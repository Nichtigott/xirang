# 横切审计（logic-hunt）：大逻辑 / 假实现 / 安全与可靠性

- 审计对象：工作树 `HEAD=cf0c378`（120 项未提交改动）。本页不按目录划分，按**最危险的流**走：源码 → 宏/解析 → `NodeId` 身份 → 构建期 plan/graft → 运行期 trace/artifact → MCP/CLI 查询。
- 口径：本轮只出报告，源码一行不改；写权限只用于本文件。所有 `file:line` 都是本次亲自 read 到的当前内容（`sed -n`/`read` 逐条核对），配对写法里的 token 确实在那几行上。
- 严重度：CRITICAL＝静默错答 / 身份或数据损坏 / 安全不可逆；MAJOR＝明确行为错误或维护性硬伤，含「注释承诺与实现不符」；MINOR＝局部一致性 / 错误上下文。
- 探针纪律：每条发现优先给**能跑出来的**最小复现；探针与 fixture 全部落在 `/tmp`（`/tmp/nichprobe/*`），仓库源码树与 `.agent-teams/` 未被改动。跑不出来的显式标「代码阅读佐证」，不写成「已实测」。
- 命名约定：`NICH-` 前缀留给片区审计；本页用 `LH-`。

## 方法（跑通了哪几条流）

| 流 | 手法 | 探针 |
| --- | --- | --- |
| MCP 入口 → 工具分派 → 回复 | 直接驱动 `target/debug/xirang-mcp` 的 stdio JSON-RPC（`/tmp/nichprobe/probe.py`），对 `~/.agent-teams` 之外无副作用 | 20+ 次 `tools/call` |
| 源码 → face 解析 → 逻辑路径 / 身份 | `/tmp/nichprobe/pkg` 一个**可解析的** fixture 包（拷贝 `examples/control-button/src`，含 typed `static_graft_plan!` 声明），可用 `cargo metadata --offline` 命名 | `xirang.registry` / `search` / `diff` / `grafts` |
| graft 记录 → 「是否被保住」两个执行面 | 手工放 4 份 `graft.plan`（好/身份漂移/幽灵/不可解析）后同时跑 CLI `xirang grafts` 与 MCP `xirang.grafts` / `diff records:true` | 见 LH-03 |
| 构建期管线 → 身份缓存 → 发布清单 | `/tmp/nichprobe/idprobe`：一个进程里对两个同形包 A、B 交替跑 `check_for`，冷/热 unit 缓存各一遍 | 见 LH-01 |
| trace artifact 读取 → 报告 | `/tmp/nichprobe/libleak`：直接调 `TraceArtifact::parse`，用 `/proc/self/status` 量 RSS | 见 LH-05 |
| 进程插件调用 → 帧 / 超时 / 环境 | `/tmp/nichprobe/procprobe`：直接调 `ProcessBackend`（`process-tools` 特性） | 见 LH-08 |
| 写路径预览 → 包副本 | 在 fixture 里放符号链接后调 `xirang.apply`（预览） | 见 LH-06 / LH-07 |
| wasm 限额 / 准入 | 只读代码 + 读 `plugin-host/tests/fault_matrix.rs` 既有钉子（本轮未新增 wasm 探针） | 见「查了、干净」 |

---

## CRITICAL

### LH-01 CRITICAL — 大逻辑（进程级身份缓存的键里没有命名空间，跨包污染 `NodeId`，并让构建/校验结论随「同进程里先跑过谁」翻转）

- 位置：`build_method/node_id.rs:36-38`（读缓存）、`build_method/identity_cache.rs:58-64`（写入时的校验与 `OnceLock::set`）、消费端 `build_method/static_plan.rs:117-118`（`node_id` 与 `cached_parent_id` 并排使用）、`build_method/cache.rs:56/63/208/224/241`、`build_method/contracts.rs:43/84`、`build_method/graft_plan_check.rs:105`
- 现象：
  1. `CACHED_NODE_IDS` 是 `static OnceLock<BTreeMap<String, (NodeId, String)>>`，**键只有相对源码路径**（`build_method/node_id.rs:25`、`:36`），值是某个命名空间下算出的 `NodeId`。
  2. 写入侧**校验**了命名空间：`build_method/identity_cache.rs:58` 要求 `id == package_node_id(&relative, &kind)`（该函数用当前运行的命名空间）。但读取侧 `build_method/node_id.rs:37` 直接 `return Some(*id)`，**不再复核命名空间**；而 `OnceLock::set` 先到先得（`build_method/identity_cache.rs:64`），因此本进程里第二次以后的 `prime` 全部被丢弃——缓存内容永久是「本进程第一个 prime 命中的那个包」的。
  3. 于是同一进程里服务两个包时会出现**混合命名空间的一棵树**：注册树里文件派生的 id 来自缓存（A 的命名空间），而 `cached_parent_id`/`package_root_node_id`（`build_method/cache.rs:186-201`）用当前运行的命名空间（B）重算父级 → 子面声明的父级在树里找不到 → `static-plan: parent node is missing`。
- 触发条件（全部满足即可，不需要恶意输入）：
  1. 一个**长寿命进程**跑多次管线：MCP bridge（`xirang.verify`）、Studio、任何一次跑多包的测试二进制——`build_method/identity.rs:24-45` 的注释自己点名了「桥或 Studio 会话会在一个进程里服务多个包」；
  2. 两个包有**相同的相对源码路径**（`control/control.rs` 这种在脚手架宿主之间必然重合）；
  3. 进程的**第一次** `prime_node_id_cache` 能读到 unit 文件。也就是说，在此之前曾有任何一次运行把 `target/xirang/cache/units/*.tsv` 写出来——正常开发流程里的 `cargo build` / `cargo xirang check` 就会写（`build_method/cache.rs:99-162`）。若 `CARGO_TARGET_DIR` 被设置，unit 目录在包之间**共享**（`build_method/identity_cache.rs:14-26` 与 `:88-100`），则只需「A 先跑、B 后跑」两步即可。
- 最小复现（实测；fixture 全在 `/tmp`）：

  ```sh
  # 1) 一个可解析的宿主包，复制成 A/B 两个同形包，只改包名
  mkdir -p /tmp/nichprobe && cd /home/nich/Moirai_N3/nichlink
  cp -r examples/control-button/src /tmp/nichprobe/src-template   # 任意同构包均可
  # 2) 探针：一个进程里 check_for(A) 然后 check_for(B)
  cd /tmp/nichprobe/idprobe && cargo run --offline --release        # 冷 unit 缓存
  rm -rf /tmp/nichprobe/A/target/xirang/out /tmp/nichprobe/B/target/xirang/out
  ./target/release/idprobe                                         # 热 unit 缓存
  ```

  实测输出（同一份 fixture，只差 unit 缓存是否存在）：

  ```text
  ===== cold: A then B =====
  probe-a: ok
  probe-b: ok
  ===== warm: A then B =====
  probe-a: ok
  probe-b: FAILED ... phase=static plan | source=control/object/button/button.rs |
          field=parent | expected=registered parent | actual=69fb465d6ebde38d1d821bb7f178e564 |
          `-- parent node is missing
  ```

  `69fb465d6ebde38d1d821bb7f178e564` 正是 **B 的** `control/control.rs` 身份（独立算出），而树里注册的 `control/control.rs` 是 **A 的** `925889c2e6bb18021c56fbfedcf98536`——混合命名空间被诊断当场打印出来。同一份代码在冷缓存下判 `ok`，在热缓存下判 `FAILED`。

  **完整判据（A/B/C 三包 6 种顺序的实测矩阵，全部在同形包、热缓存、一个进程内两跑）**：

  ```text
  A->B  FAIL(B)      B->A  ok        C->A  ok
  A->C  FAIL(C)      B->C  FAIL(C)   C->B  FAIL(B)
  ```

  「第二个包」失败当且仅当**第一个包**的 unit 缓存对本包**有效**：A 的 unit 是有效条目，所以 A 先跑就把 map 钉成 A 的 id；B、C 的 unit 里存的是别的命名空间的 id（B 的 `control/control.rs` 条目是 `8ea7eb1142a2c59166d9d731d62dee66`，实测等于 `probe-c` 命名空间下的值，而非 B 自己的 `69fb465d…`），于是 B/C 先跑时 `build_method/identity_cache.rs:58` 的**写入侧**校验把整份 `values` 拒成空 map，`set` 一个空 map，后来的包不受影响——两条 `ok` 由此解释。这条对照同时说明：**校验只存在于 prime 的写入侧，`build_method/node_id.rs:37` 的读取侧没有**。

  同一次污染还留下两个可独立观察的痕迹：
  1. **同一次运行里两处 id 来源不一致**：`build_method/static_plan.rs:117` 用缓存版 `node_id`，而发布清单 `build_method/manifests.rs:181-184` 用 `package_node_id` 重算。于是失败那次运行的 `pruning_manifest.tsv` 里是**正确的 B id**（实测），拓扑校验却拿 A 的 id 当树内容——同一份证据自相矛盾，正是「两个 id 来源」的指纹。
  2. **污染会被写进下一份 unit 缓存**：`build_method/cache.rs:104`（`collect_discovery_rows` → `node_id`）与随后的 unit 写入把 `node\t{id}` 落盘。这正是 B 的 unit 里出现 `probe-c` id 的原因（B 在 C 之后跑的进程里写的）。也就是说污染不是只读的：它会落盘，只靠写入侧校验在**下一个进程**才被拒掉。

- 判据：
  - `NodeId` 是命名空间 + 相对路径 + kind 的散列；`build_method/identity.rs:24-45` 的注释把「用别的包命名空间盖下的身份」定义为「一个看起来正确的错误答案——它是这次构建发布的每一个 `NodeId` 的输入」，并明确说线程局部的 `RUN_NAMESPACE` 就是为消除它而存在。`CACHED_NODE_IDS` 把这个已被文字排除的故障又请了回来。
  - 本轮实测到的方向是**假失败**（构建拒绝，方向安全）；同一个缓存值也流进 `collect_active_ids` / `source_is_active` / `face_source_is_active`（`build_method/cache.rs`，作用域与发布剪枝的选择集）与 `build_method/contracts.rs`，那里没有第二道「父级对不上」的交叉校验，方向是**假通过 / 剪错符号集**（这一半是代码阅读佐证，未实测）。
  - 注释型假实现：`build_method/node_id.rs:23`「Node identities primed from the discovery cache before scope inference」与 `build_method/identity_cache.rs:88` 把缓存描述成「增量」加速，读者会以为它与命名空间无关地安全；实际它是本进程第一个**成功 prime** 的包的切片。
- 修复方向（任选其一，建议 1+3）：
  1. 缓存键加上命名空间：`BTreeMap<(String /*namespace*/, String /*relative*/), (NodeId, String)>`，读侧只命中当前 `package_namespace()` 的条目；
  2. 读侧复核：`node_id()` 命中后仍验证 `id == package_node_id(&relative, &kind)`（代价与一次散列相同），或把 `CACHED_NODE_IDS` 从 `OnceLock` 改成「按命名空间分桶、可重入」的结构，让第二次 `prime` 对**新的**命名空间仍然生效；
  3. `prime_node_id_cache` 里把「本次命名空间」一并记进 map，或在 `run_as_package` 作用域结束时清空。
- 复核手段：`idprobe` 冷/热两跑（上面的命令）加三包 6 顺序矩阵（`./target/release/idprobe /tmp/nichprobe/A /tmp/nichprobe/B`，参数即顺序）；建议落成 `build_method` 的一条单测：同一进程内 `check_for(A)` → `check_for(B)`，断言 B 的 `static_plan` 无诊断（需要先把 A 的 unit 缓存预热，即先 `check_for(A)` 两次或用预置 unit 文件）。

---

## MAJOR

### LH-02 MAJOR — D 大逻辑（只读视图静默丢弃解析失败的注册面文件，并把它的子面改挂到根上；构建报错、视图不报）

- 位置：`build_method/face_view.rs:211-217`（`collect` 用 `parsed_face(...)` 的 `Option` 过滤）、`build_method/validation.rs:180-182`（`parsed_face` 把解析失败折叠成 `None`）、`build_method/face_view.rs:280-292`（`logical_path` 找不到父级时回退 `root`），消费端 `mcp/registry.rs`、`mcp/search.rs:60`、`mcp/diff.rs:41`、`mcp/impact.rs:48`、`mcp/usages.rs:35`、`mcp/trace.rs:108`、`mcp/impact.rs`、`mcp/usages.rs`
- 现象：一个面文件只要**不是恰好一个可解析的注册面**（例如被追加了第二个宏调用、宏体写坏），`parse_face` 返回 `Err`（`core/src/registry_core/syntax/face.rs:175-183`），`parsed_face` 把它变成 `None`，`face_views` 就把这个文件**当成没有面**跳过——不计数、不诊断。它的子面因此解析不到父级，在 `logical_path` 里回退成 `root/<slot>`。于是「这个包声明了什么」这个问题，构建与桥给出**不同规模的答案，且桥的那份带有一条运行期从未存在过的逻辑路径**。
- 触发条件：包内任意一个注册面文件解析失败（最常见：两处宏调用、字段写坏、缺 `kind`）。
- 最小复现（实测）：

  ```sh
  P=/tmp/nichprobe/pkg           # LH-03 的同一个 fixture
  printf '\npub struct Twin;\ncrate::root_object! {\n  kind: Twin,\n  needs_registry: false,\n  registry_name: "twin",\n  parent: crate::root_node_id(env!("CARGO_PKG_NAME")),\n}\n' >> $P/src/control/control.rs
  # 构建侧（报错）
  cd $P && /home/nich/Moirai_N3/nichlink/target/debug/cargo-xirang xirang check
  # 视图侧（不报错、少一个面、子面改挂根）
  python3 /tmp/nichprobe/probe.py xirang.registry '{}' $P
  ```

  实测：

  ```text
  # check: phase=face-syntax source=control/control.rs:43 `-- expected one registration face in this file
  #        (另加两条 static plan: parent declaration cannot be resolved)
  # xirang.registry:
  namespace probe-host
  faces 2
  root/button      Button  control/object/button/button.rs  371236fe8d53731db5ab0b9db8dfd46c  parent-unresolved
  root/slider      Slider  control/object/slider/slider.rs  8ebd5ccbb6b632c4752546941068f3b3  parent-unresolved
  ```

  即：`Control` 面整个消失、`root/control/button` 退化成 `root/button`，回复里没有任何一句说「有文件解析不了」。
- 判据：
  - `face_view.rs` 的模块文档自己写着这条视图给的是「构建计算出的路径与身份」，并专门说过「命令若猜成别的，就会打印出运行期从未有过的逻辑路径」（`build_method/face_view.rs:222-231` 对 `registry_name` 默认值的说明）。这里它正是这么做的：`root/button` 在这棵树里从未存在过。
  - 而 `xirang.apply` 的契约是「`node` 用逻辑路径点名面——就是 `xirang.registry` 报告的那个」（`mcp/tools.rs` 的 `xirang.apply` 描述）。视图少报 + 改名会让一次编辑落到另一个面上。
  - `check` 有 `face_syntax` 诊断（`build_method/validation.rs:191-210`），说明「解析失败」在构建侧是一等事实，视图侧却没有等价的输出。
- 修复方向：`collect` 把解析失败的区别**带出来**——`face_views` 返回 `(Vec<FaceView>, Vec<UnreadableFace>)` 或让 `FaceView` 带 `unparsed: bool`；MCP 的 `registry`/`search`/`diff`/`impact`/`usages` 在回复里加一行 `unparsable faces N (see xirang check)`，并把不回退父级的子面标成 `parent-unresolved`（这一半已有）。不要靠「少一个面」来表达错误。
- 复核手段：上面的 fixture；断言 `xirang.registry` 的回复里出现被解析失败的文件名或其计数。

### LH-03 MAJOR — 假实现 / 两端不一致（`diff records:true` 只在「身份在树里」那一支查「是否被声明」，于是 `undeclared 0` 与 `xirang.grafts` 的 `unkept plans N` 互相矛盾）

- 位置：`mcp/diff.rs:150-171`（身份命中分支，唯一读 `row.declared` 的地方，见 `:162`）与 `mcp/diff.rs:177-195`（路径命中 → `reidentified`；路径缺失 → `stale`，两支都不读 `declared`）；产生 `declared` 的规则在 `build_method/graft_view/plan_rows.rs:152-166`；两个执行面的出口 `mcp/grafts.rs:115`（`unkept plans {unkept}`）
- 现象：同一条 graft 记录，两个 MCP 工具给出**互相矛盾的结论**：`xirang.diff records:true` 把它算进 `re-identified` 或 `stale` 并报 `undeclared 0`；`xirang.grafts` 对同一份记录报 `[NOT declared by the host entry]` 与 `unkept plans 1: the release prunes these slots, so the records can never take effect`。前者是「身份漂移，修一下就好」，后者是「这条记录永远不会生效（发布剪枝）」——两句话的行动含义相反，而 `undeclared 0` 是错的。
- 触发条件：一条记录的身份不在树里（典型的「槽位没动、身份换了」）且宿主入口没有用**字符串**切口点名该路径时，即命中。类型化切口（`cut(crate::…::NODE_ID)`）在身份缺席时无法解析模块（`build_method/graft_view/plan_rows.rs:152-155` 只在身份命中时取 `module`），因此这种记录几乎必然落在这个分支——正是本项目 scaffold 生成的宿主入口的写法（`examples/control-button/src/lib.rs` 的 typed `cut(...)`）。
- 最小复现（实测，只有一条 `moved_identity` 记录，其余删掉）：

  ```sh
  P=/tmp/nichprobe/pkg
  rm -rf $P/.xirang/external-grafts
  mkdir -p $P/.xirang/external-grafts/moved_identity
  printf 'version=1\ntarget=00000000000000000000000000000000\ntarget_path=root/control/button\ngraft=button_fast\nfull=false\n' \
    > $P/.xirang/external-grafts/moved_identity/graft.plan
  python3 /tmp/nichprobe/probe.py xirang.diff     '{"records":true}' $P
  python3 /tmp/nichprobe/probe.py xirang.grafts   '{}'               $P
  ```

  实测：

  ```text
  # xirang.diff records:true
  records 1 (external graft plans)
  ok 0  undeclared 0  stale 0  re-identified 1  unreadable 0
  re-identified:
    ~ moved_identity 00000000000000000000000000000000 -> 371236fe8d53731db5ab0b9db8dfd46c  (root/control/button)
  # xirang.grafts
  plans 1
    moved_identity: target=root/control/button graft=button_fast full=false [NOT declared by the host entry]
  unkept plans 1: the release prunes these slots, so the records can never take effect. ...
  ```

- 判据：`build_method/graft_view/plan_rows.rs:8-15` 的模块文档把这条规则的目的写成「同一个问题有两个执行面在问……规则住在这里，两者就不可能给出不同答案」；`mcp/diff.rs:154-161` 的注释把「称它 ok 会让同一条记录有两个健康结论」记为本轮修过的 `M1` 缺陷。修的是身份命中那一支，**路径命中/缺失两支仍然只报身份结论**，于是同类缺陷在另两支原样存在。`undeclared 0` 是可被 agent 直接引用的计数，错在这里比错在文案上严重。
- 修复方向：
  1. 把 `row.declared` 的应用提到桶分流**之外**：先算 `(identity_verdict, declared)` 两维，再决定输出行；`undeclared` 计数与 `xirang.grafts` 的 `unkept` 用同一个数；
  2. `plan_rows.rs` 在身份缺席时也解析模块（用 `faces` 里 `path == document.target_path` 的那个面），这样 typed 切口在 re-identified 情形下也能被判成「已声明」——否则至少在那一行注明「无法判断是否被声明」而不是让它落进 `undeclared 0`。
- 复核手段：上面的最小复现，断言两个工具的计数一致（`unkept plans N` == `undeclared N`，当 `N` 只来自被声明/未声明的区分时）。

### LH-04 MAJOR — 假实现（MIR 的 `jsonl:true` 可以给**任意可读文件**盖上本包的快照表头，把「来源可证」变成「来源可造」）

- 位置：`mcp/mir.rs:73-87`（`jsonl:true` 分支 → `confirmed_snapshot`）、`:259-280`（`confirmed_snapshot` 直接写 `graph.snapshot = Some(...)`）、`:217-237`（`load` 按扩展名选解析器；非 `.jsonl` 一律走 `from_mir_text`，而它**从不失败**）、`core/src/registry_core/mir/render.rs:51-66`（`to_jsonl` 把表头写在第一行）
- 现象：`xirang.mir` 只要给的路径不以 `.jsonl` 结尾，就用「不是调用的行就不是调用」的宽松文本解析器读；对一个完全不是 MIR 转储的文件，它返回一张**空**图并判 `isError:false`。此时 `jsonl:true` 会把这台空图**盖上本包的命名空间与 root** 输出成一份合法 artifact。快照表头正是这套工具链里「这份 artifact 描述哪棵树」的**唯一凭据**（同文件的 `delta_report`、`unified`、`trace.rs` 的身份校验都靠它），而它可以由任意文件凭空产生。
- 触发条件：`path` 指向包内任意可读的非 `.jsonl` 文件（工具自述里 `path` 是必填，扩展名决定解析器，这一点没有别的校验）。
- 最小复现（实测）：

  ```sh
  P=/tmp/nichprobe/pkg
  printf 'This is not a MIR dump.\nJust prose.\n' > $P/notes.txt
  python3 /tmp/nichprobe/probe.py xirang.mir '{"path":"notes.txt"}'              $P
  python3 /tmp/nichprobe/probe.py xirang.mir '{"path":"notes.txt","jsonl":true}' $P
  ```

  实测输出（原样）：

  ```text
  file /tmp/nichprobe/pkg/notes.txt
  functions 0 calls 0 locals 0
  calls:
  locals:

  {"kind":"snapshot","namespace":"probe-host","root":"cc05a41d58a3a71153bfcd58b7a05e9d"}
  ```

  把它写回 `fabricated.jsonl` 再读，回复是 `functions 0 calls 0 locals 0` + `snapshot namespace=probe-host root=cc05a41d…`——一份「本包、零调用」的可信快照。
- 判据：模块文档自己说「让写出的 artifact 成为快照的是本工具盖进去的表头」，并要求「外来快照按名字拒绝」。表头若能与内容无关地生成，「按名字拒绝」这道门守的是**标签**而不是**事实**：一份在 A 包生成、内容来自 B 包的快照，照样会被当成本包快照接受（并把 delta 的另一侧判成「不同树」或「同一树」）。这与 `confirmed_snapshot` 注释里「re-stamping would turn a foreign artifact into one that claims to be this package's」的意图正好相反——这里不需要 re-stamp，直接造一份新的。
- 修复方向：
  1. `jsonl:true` 前要求图非空且来自文本转储时至少解析出 1 个 `fn` 头（`from_mir_text` 无法区分「空 MIR」与「非 MIR」，那就把「零记录」当拒绝理由：`refusing to stamp a snapshot onto an artifact with no MIR records`）；
  2. 或者给 `confirmed_snapshot` 加一个 `source_kind` 参数，只有 `from_jsonl`（可能自带表头）与「文本转储里出现过 MIR 函数头」两种情形允许盖戳；
  3. 文档同步：`xirang.mir` 的自述应当说清「文本转储零记录时不产出快照」。
- 复核手段：上面的两行命令；断言 `jsonl:true` 对 prose 文件返回错误或至少不产出 `kind:snapshot` 行。

### LH-05 MAJOR — 可靠性（trace artifact 的字符串驻留是一座进程级、无淘汰的 `Box::leak` 集合；长寿命 MCP 进程可被逐次喂大的 artifact 持续抬高 RSS）

- 位置：`run_method/runtime/trace/artifact/parse.rs:294-304`（`fn intern`：`static INTERNER: Mutex<BTreeSet<&'static str>>` + `Box::leak`）、调用点 `:94`（frame 的 function）、`:126`（local 的 file）、`:233`（source 的 file）
- 现象：解析 trace artifact 时，每个**新见到的**字符串都被 `Box::leak` 成 `'static` 并记进一个进程级 `static` 集合，**永不释放**。进程内的每份 artifact 都共享同一张表；`read_trace_artifact` 是公开入口，MCP 的 `xirang.trace` / `xirang.unified` / trace 驱动的 `xirang.converge` 都按请求读一遍宿主指定的路径。
- 触发条件：长寿命进程 + 每次喂进「词表不同」的 artifact。CLI 一次性进程无害；MCP/Studio（本项目的设计用法）不是。
- 最小复现（实测；RSS 用 `/proc/self/status` 的 `VmRSS`）：

  ```sh
  cd /tmp/nichprobe/libleak && cargo run --offline --release
  ```

  实测（A 组：每轮 10 万个**新**函数名；B 组：每轮同一批名字）：

  ```text
  baseline rss_kb=2264
  A round 0: rss_kb=13644 (+11380)
  A round 1: rss_kb=38136 (+35872)
  A round 2: rss_kb=44704 (+42440)
  A round 3: rss_kb=51264 (+49000)
  B round 0: rss_kb=57924 (+6660)
  B round 1: rss_kb=57924 (+6660)
  B round 2: rss_kb=57924 (+6660)
  B round 3: rss_kb=57924 (+6660)
  ```

  A 组每轮单调上升约 6.5 MB/10 万字符串（artifact 本体已被 `drop`），B 组平——差值就是驻留表，而且从不回落。
- 判据：`run_method/runtime/trace/artifact/parse.rs:289-293` 的注释把这条设计的边界写成「this leaks once per distinct string instead, **bounded by the artifact's vocabulary**」。对**单次调用**为真；对**进程**为假：真正的上界是「本进程解析过的所有 artifact 词表的并集」，没有任何淘汰、没有按 artifact 释放、没有上限。注释承诺的界与实现不符，而这条路径是外部输入（artifact 路径由调用方给）。
- 修复方向：驻留表按 artifact 生命周期回收——把 `&'static str` 换成 `Rc<str>`/`Arc<str>` 放进 artifact 自己的 arena（`TraceArtifact` 持有一个 `Bump`/`Vec<String>`，帧与局部值持索引或 `Arc`），或在 `intern` 前加「表大小上限 + 命中率衰减」的显式策略并把它写进文档；至少要把注释改成「进程级、不回收、上界是历次并集」，并把这条风险写进 `docs/threat-model.md`。
- 复核手段：`libleak` 的 A/B 两跑；或对运行中的 MCP 进程连续发 N 次 `xirang.trace`（每次换一份 artifact），观察 `VmRSS` 单调上升。

### LH-06 MAJOR — 安全（预览的包副本跟随目录符号链接走出包根，且没有环/深度守卫；读侧同一条规则却是拒绝的）

- 位置：`mcp/preview.rs:37-58`（`copy_directory`；`:50` `source.is_dir()`、`:52` `source.is_file()`、`:53` `std::fs::copy`——三者都跟随符号链接，`:45` 只按名字跳过 `target` 与 `.xirang`），入口 `mcp/apply.rs:84-88`
- 现象：`xirang.apply`（默认预览）把整个包复制到 `std::env::temp_dir()` 下一个临时目录。复制只按**目录项名字**过滤 `target`/`.xirang`，对符号链接既不看解析结果、也不设深度上限：
  - 包内一个指向外部目录的链接 → 外部整棵树被复制进 temp（实测：错误信息点名了包外的文件）；
  - 一个自指链接（`ln -s . src/self`）→ 递归到路径长度上限，把同一批文件复制几十份，并且**照样成功**，回复里出现 124 个面、逻辑路径 `root/control` 重复出现多次（实测）。
- 触发条件：`xirang.apply` 且 `apply != true`（即默认预览），包内存在目录符号链接。链接是常见工程事实（monorepo、`node_modules` 式共享目录、Windows junction）。
- 最小复现（实测）：

  ```sh
  P=/tmp/nichprobe/pkg
  mkdir -p /tmp/nichprobe/outside && echo secret > /tmp/nichprobe/outside/secret.txt
  ln -sfn /tmp/nichprobe/outside $P/src/outside_link
  python3 /tmp/nichprobe/probe.py xirang.apply \
    '{"action":"add","parent":"root","fields":{"module":"probe_add","kind":"ProbeAdd"}}' $P
  # 实测: cannot copy /tmp/nichprobe/pkg/src/outside_link/secret.txt: Permission denied (os error 13)
  #       ——包内一个链接，把包外的文件拖进了复制路径

  ln -sfn . $P/src/self
  python3 /tmp/nichprobe/probe.py xirang.apply \
    '{"action":"add","parent":"root","fields":{"module":"probe_add","kind":"ProbeAdd"}}' $P
  # 实测: action preview ... faces 124
  #         root/control  Control  self/self/self/self/control/control.rs
  #         root/control  Control  self/self/self/self/self/self/self/self/control/control.rs
  #         root/control  Control  self/self/self/self/.../control/control.rs
  ```

  （第一次探针把 `outside/secret.txt` 设成 `chmod 000` 以把「已走出包根」变成可读的错误信息；复现时用可读文件会静默复制成功。）
- 判据：同一个桥的**读侧**对这件事有明确规则并且实现了它——`mcp/index.rs:42-50`「解析到根外的链接不属于本索引」、`:66-68`「每个事实都经路径的规范形式」、`:167-174`（`is_safe_child` = 两侧 `canonicalize` 后 `starts_with`）。写侧预览既不 canonicalize 也不拒绝，两侧对同一棵树给出两套边界。`mcp/preview.rs:4-10` 的模块文档承诺的是「预览是安全的：项目从未被碰过」——它说的确实是**项目**没被碰，但没有说清副本可以把包外的东西读进来，也没有说包内一个链接就能让预览变成无界复制。威胁模型对「宿主目录约束」只有资产与边界的一般陈述，没有豁免预览路径。
- 修复方向：
  1. `copy_directory` 用 `symlink_metadata` 判断条目类型，遇到符号链接**不跟随**（复制链接本身，或直接跳过并在回复里计数），与读侧 `is_safe_child` 同一条规则；
  2. 跟随不可免时，至少 `canonicalize` 后要求 `starts_with(root)`，并带 `visited` 集合 + 深度/条目数上限（超出即失败，理由写进回复）；
  3. 把「预览副本的边界」写进 `preview.rs` 文档与威胁模型的残余风险段（现状两者都没写）。
- 复核手段：上面两条命令；断言 `outside_link` 的内容不被复制、自指链接要么被跳过要么以错误结束而不是产出 124 个面。

### LH-07 MAJOR（成对缺陷，与 LH-06 同源）— 可靠性（`copy_package` 失败时把半份副本永久留在 temp；清理只覆盖「副本已成功、执行器失败」这一支）

- 位置：`mcp/apply.rs:84-88`（`copy_package(root)?` 在 `remove_copy` 可达之前就 `?` 传播）与 `:128-135`（唯一的失败清理），`mcp/preview.rs:21-30`（`copy_package` 内部的 `?`）
- 现象：`copy_package` 在复制中途失败（LH-06 的权限例子、磁盘满、路径过长）时直接带着错误返回，`apply` 的 `Err` 分支（`mcp/apply.rs:128-135`）根本到不了——那份**半成品包副本**留在 `std::env::temp_dir()` 里。实测留下 `/tmp/xirang-mcp-preview-343377-0`，里面 7 个文件（含 `Cargo.toml`、`src/**`）。
- 触发条件：任何一次预览复制失败；MCP 客户端可以凭 LH-06 的链接稳定触发，反复调用即持续堆积。
- 最小复现（实测）：

  ```sh
  # 用 LH-06 的 outside_link（其目标文件不可读）触发复制失败
  ls -d /tmp/xirang-mcp-preview-*      # 实测: /tmp/xirang-mcp-preview-343377-0
  find /tmp/xirang-mcp-preview-343377-0 -type f | wc -l   # 7
  ```

- 判据：`mcp/apply.rs:8-11` 与 `mcp/preview.rs:14-20` 的文档把「一次性副本」写成这个设计的安全根据；一次性意味着**无论成功失败都该消失**。同文件里已经为「失败的预览不必留下副本」写了清理（`:128-135`），说明作者知道该清，只是漏了 `copy_package` 自身失败这一支。
- 修复方向：把复制纳入同一个 `Result`-to-cleanup 结构——例如 `copy_package` 失败时自己 `remove_dir_all(destination)` 再返回 Err（destination 已构造出来，指针就在手边），或让 `apply` 用 `let work = copy_package(root).inspect_err(|_| ...)` 形式统一收尾；顺便把 `std::env::temp_dir()` 的副本前缀写进文档。
- 复核手段：触一次复制失败后 `ls -d /tmp/xirang-mcp-preview-*`，断言不存在新目录。

### LH-08 MAJOR — 注释型假实现 / 可靠性（进程插件适配器：注释说「已送达的答案不再被报成超时并丢掉」，实现仍会把它报成 `Timeout` 并丢掉；帧损坏时又只剩一句无上下文的 io 错误）

- 位置：`plugin-host/src/process.rs:341-354`（stdout 排空线程与它的注释）、`:372-409`（轮询循环：只有子进程退出或到 deadline 才收尾）、`:404-407`（到点即 `kill_and_reap` + `return Err(HostError::Timeout)`）；对照 `plugin-host/src/process/child.rs:119-135`（`drain_to_eof` 的注释）
- 现象：
  1. 子进程**已经写完整帧**、然后不退出（收尾慢、等信号、睡眠）时，宿主在 deadline 处返回 `Err(Timeout)`，**把手里已收到的帧丢掉**。`plugin-host/src/process/child.rs:119-127` 的注释把这件事描述成已被修掉的缺陷（「当它阻塞时，宿主会等一个不可能到来的退出、在超时点杀掉子进程并报出超时——同时丢掉它其实已经拿到的答案」），修的是「管道满导致阻塞」这一子类，而注释描述的是**整个类**；
  2. 子进程写了一帧**不完整**的内容后正常退出（status 0）时，调用方拿到的是 `HostError::Io(UnexpectedEof, "failed to fill whole buffer")`——既没说是哪个插件、哪个操作，也没说是帧坏了（`read_frame` 的 `read_exact` 经 `From<io::Error>` 升格，`plugin-host/src/process.rs:392` 的 `refusal` 原样返回）。
- 触发条件：任何「先作答、后不退出」的插件；任何写坏帧的插件。
- 最小复现（实测，`/tmp/nichprobe/procprobe`，直接调 `ProcessBackend`（`process-tools`））：

  ```sh
  cd /tmp/nichprobe/procprobe && cargo run --offline --release
  # 实测:
  # answers-then-lingers: Err(Timeout) after 303.936524ms      # 帧已送达（6 字节 "answer"）
  # partial frame, exit 0: Err(Io(Error { kind: UnexpectedEof, message: "failed to fill whole buffer" }))
  ```

- 判据：`plugin-host/src/process.rs:247-283` 的注释与 `plugin-host/src/process/child.rs:119-127` 是同一份主张的两个副本，措辞是「a delivered answer was reported as a timeout」——这是一个**错误诊断 + 丢弃结果**的复合缺陷，作者已判定它不该存在。当前实现只覆盖了它的一种成因（管道阻塞），并把另一种成因（子进程作答后不退出）当成无条件的 deadline 语义。同时 `HostError` 的文档（`plugin-host/src/error.rs:46-48`）说 `Io` 是「唯一会暴露源错误的变体」，而这里暴露的源错误不含任何宿主侧上下文。
- 修复方向：
  1. 循环里把「帧已收到」变成一条**完成条件**：`if let Some(bytes) = frame { ... }` 时先尝试优雅收尾（`try_wait` + 短 grace），要么返回答案，要么在超时后用专门的错误变体说明「答案已收到，但子进程未在 deadline 内退出」（不要复用 `Timeout`）；
  2. 若坚持「deadline 覆盖整个调用」，则修改 `process.rs`/`child.rs` 的注释，明确写出「得到答案后仍不退出者报 `Timeout`，且答案被丢弃」，并把这条语义写进威胁模型的进程插件段；
  3. 帧读取失败时包装上下文：`HostError::Process(format!("plugin `{operation}` answered with a broken frame: {error}"))`，至少带上操作名与「frame」这个词。
- 复核手段：`procprobe` 的两种插件；或在 `plugin-host/tests/fault_matrix.rs` 的 `process_faults` 里加一条 `answers-then-sleeps` 与一条 `half-frame-exit-0` 的钉子。

---

## MINOR（成组）

### LH-09 MINOR 组 — 自我描述与行为不一致（本项目的老毛病，两处不一致都算缺陷）

| # | 位置 | 自述 | 实际 |
| --- | --- | --- | --- |
| a | `mcp/tools.rs:139-157`（`xirang.diff` 描述，`:149` 起） | 「A record comes back `ok`, `stale` …, or `re-identified` …；`unreadable` counts a record file that could not be read」 | 回复里还有一个 `undeclared` 桶（`mcp/diff.rs:143/169/206-239`），而且它按 `mcp/grafts.rs:115` 的说法意味着「这条记录永远不会生效」。工具的**自述里没有这个桶**，agent 只能从回复正文里第一次见到它 |
| b | `mcp/diff.rs:26-38`（模块文档，`:34`） | 「Both borrow the same vocabulary (added/gone/re-identified) on purpose」 | `records:true` 的词汇是 `ok / undeclared / stale / re-identified`，**没有 added/gone**。两种比较共用的只有「面级」这个抽象，不是词汇 |
| c | `run_method/runtime/trace/artifact/parse.rs:289-293` | 「this leaks once per distinct string instead, bounded by the artifact's vocabulary」 | 界是进程级的（LH-05），见那里的实测 |
| d | `build_method/node_id.rs:23-25` 与 `build_method/identity.rs:24-45` | 缓存被写成「增量加速」；命名空间污染被写成「已被线程局部消除」的故障 | LH-01：缓存键不含命名空间，故障回来 |

- 判据：项目自己的规则——「一个工具/检查的自我描述必须与行为一致，两处不一致都算缺陷」。
- 修复方向：把 a/b/c/d 的自述改成实际行为（或让行为回到自述）；a 尤其重要，因为它是 agent 唯一的工具契约。
- 复核手段：`xirang.diff` 的 `records:true` 输出与 `mcp/tools.rs` 里该工具的 description 逐词对照（本页 LH-03 的复现命令同时给出两侧）。

### LH-10 MINOR 组 — 把「无法判断」放在正确位置、但错误上下文丢在最后一步

| # | 位置 | 现象 |
| --- | --- | --- |
| a | `plugin-host/src/process.rs:392`（`refusal` 原样返回） | 坏帧 → `HostError::Io("failed to fill whole buffer")`，无操作名、无「帧」字样（LH-08 第 2 点，实测） |
| b | `mcp/mir.rs:217-237`（`load`：先 `is_file()` 再 `is_safe_child`） | 代码注释说这顺序是有意的（「存在性有意先于归属检查」），但副作用是：MCP 客户端能用 `xirang.mir` 探测**宿主文件系统上任意路径是否存在**（实测：`/etc/hostname` → 「path must stay inside the configured source root」；`/etc/definitely-not-here-12345` → 「is not a readable file」）。两条回复的差别就是一个存在性 oracle。风险低（客户端本来就有宿主 uid，多数情况也能读写），但这是唯一一处把根外路径的存在性放进回复里的地方；建议对根外路径统一回「不在配置的源码根内」，不带存在性信息 |

### LH-11 MINOR（以**证伪**为主）— 宏里的 `::xirang_debug_method::submit!`：opt-in 且 `cfg(debug_assertions)` 门控，不是「无条件悬空路径」

- 位置：`run_method/macros/face_registration.rs:160-170`（`__submit_registration!` 的三个 arm）、默认走 `development` 的入口 `run_method/macros/face_objects.rs:13`（对象面）与 `run_method/macros/face_external.rs:51`（外部面默认 `linked`）
- 核实结论（独立复核 t7 的「幻影依赖」说法）：
  1. **不是无条件发出**。被转达的那条路径是 `(debug; $registration:ident) => { #[cfg(debug_assertions)] ::xirang_debug_method::submit! { $registration } }`（`run_method/macros/face_registration.rs:164-167`，`:166` 即 `submit!` 那一行）。同一 macro 的 `(development; …) => {}`（`:163`）与 `(linked; …) => {}`（`:168-171`）什么都不发。
  2. **树内没有生产方**。`collector` 只在两处被默认/显式写下：对象面省略时 `development`（`run_method/macros/face_objects.rs:13`），外部面省略时 `linked`（`run_method/macros/face_external.rs:51`）；生成计划（实测 `target/debug/build/xirang-example-control-button-2b5957624b0ef577/out/generated_lib.rs:29-37`、`:64-66`）调的是 `__xirang_object! { $($tokens)* }`，不带 `collector` → 走 `development` → 空 arm。全树唯一的 `collector: debug` 生产者是一条**测试**：`debug_method/tests/collector_integration.rs:7`（dev-dependency 传递的 `--extern` 让它解析得到）。
  3. **只在 debug 下需要该 crate**。实测：一个只依赖 `xirang-run-method`、显式写 `collector: debug` 的宿主（`/tmp/nichprobe/hostprobe`）在 `cargo check --offline` 下报 `E0433 could not find xirang_debug_method in the list of imported crates`，并在同一份源码上 `cargo check --offline --release` **通过**——`#[cfg(debug_assertions)]` 在名字解析之前就把这一句移除了。
- 因此：`examples/` 两个只依赖 `run_method` 的宿主**不会**拿到无法解析的路径（它们走 `development`），t7 的「幻影依赖」在出厂路径上不成立。**证伪成立**。
- 仍值得记的窄缺口（这才是本条 MINOR 的实体）：`collector: debug` 是任何人都能写的**隐藏开关**，但只有本工作区内（`mcp`/`studio` 各自直接依赖 `debug_method`，见各自 `Cargo.toml`）才解析得到；arm 上没有 doc 注释说明「用这个要自己加 `xirang-debug-method` 依赖」，`run_method` 也没有对应特性来打开它。第三方宿主照抄这个名字会在 debug 构建里撞 E0433。
- 修复方向（择一，都不大）：给 `debug` arm 补双语 doc 注释写明前置依赖；或改为 `$crate::__debug_submit!` 一类的间接层（由 `run_method` 在特性下 re-export），使 opt-in 不要求调用方知道第三个 crate 的名字；或在 `run_method` 侧拒绝非本工作区的 `collector: debug`（宏里无法判定，故建议前两条）。
- 复核手段：`/tmp/nichprobe/hostprobe` 的 debug/release 两跑（上面的命令）。

---

## 横切总账：注释作为一类「自我描述」、术语一致性

（队长 2026-09-28 追加的轴。只列本页有探针或逐行证据的条目；复现见对应发现。）

**A. 注释/自述承诺与实现不符（都是行为问题，不只是文字）**

| # | 位置 | 注释承诺 | 实现 | 见 |
| --- | --- | --- | --- | --- |
| 1 | `build_method/identity.rs:24-45`、`build_method/node_id.rs:23` | 用别的包命名空间盖下的身份「已被线程局部消除」 | 进程级 `CACHED_NODE_IDS` 把同一个故障请了回来，实测可复现 | LH-01 |
| 2 | `plugin-host/src/process/child.rs:119-127`、`plugin-host/src/process.rs:341-350` | 已送达的答案不会再被报成超时并丢掉 | 「作答后不退出」的插件仍得到 `Err(Timeout)`，答案被丢弃 | LH-08 |
| 3 | `run_method/runtime/trace/artifact/parse.rs:289-293` | 泄漏「bounded by the artifact's vocabulary」 | 界是**进程级**历次并集，实测单调增长 | LH-05 |
| 4 | `mcp/mir.rs:20-29`（模块文档） | 快照表头是「这份 artifact 描述哪棵树」的凭据 | 表头可被任意可读文件凭空盖出 | LH-04 |
| 5 | `mcp/diff.rs:26-38`（模块文档，`:34`） | 两种比较「共用同一套词汇（added/gone/re-identified）」 | `records:true` 用的是 `ok/undeclared/stale/re-identified` | LH-09 b |
| 6 | `mcp/tools.rs:139-157`（`xirang.diff` 自述） | 记录只有 `ok`/`stale`/`re-identified` 与 `unreadable` | 还有 `undeclared` 桶，且它与 `xirang.grafts` 的 `unkept` 计数冲突 | LH-03 / LH-09 a |
| 7 | `build_method/face_view.rs:222-231` | 视图「绝不打印运行期从未有过的逻辑路径」 | 解析失败时子面被改挂根，打印出 `root/button` | LH-02 |

**B. 术语一致性**

| # | 现象 | 位置 |
| --- | --- | --- |
| 1 | 同一个状态两个词：`xirang.diff records:true` 叫 `undeclared`，`xirang.grafts` 叫 `unkept plans` | `mcp/diff.rs:207` vs `mcp/grafts.rs:115` |
| 2 | 同一个工具里同一个状态两种拼法：默认报告印 `reidentified`，记录报告印 `re-identified`，而工具自述与 `search` 都用带连字符的 | `mcp/diff.rs:100`、`mcp/diff.rs:117` vs `mcp/diff.rs:207`、`mcp/search.rs:140`、`mcp/tools.rs:65` |

**C. 「需要靠注释才看得懂」的结构信号（判断，不是缺陷计数）**

- `plugin-host/src/process.rs:246-435`（`call`）：函数体近 190 行，前面挂了约 50 行注释解释「直白写法为什么错」。注释本身是资产（两条实测边界都有出处），但它承担的是**控制流契约**——输入、管道、超时、四类失败的排序。这类内容更适合抽成一个具名的小状态机，让注释退回「为什么」。
- `build_method/node_id.rs:23-25` 的缓存说明与 `build_method/identity_cache.rs:88-100` 的路径推导，是「读者必须读注释才知道值从哪来」的典型——而这正是 LH-01 埋雷之处：说不清来源的全局值，注释也守不住。

## 查了、干净（本轮走通 / 探过的流）

1. **MCP 读侧路径约束（实测干净）**：`xirang.read` / `inspect` 对 `../../etc/passwd`、`/etc/passwd` 一律回 `path must stay inside the configured source root`；`load_one` 还额外要求 `.rs` 扩展名（实测 `../xirang/Cargo.toml` → `only Rust source files can be read`）。`is_safe_child`（`mcp/index.rs:167-174`）两侧 `canonicalize` 后 `starts_with`，是组件级比较，`/a/bc` 不会匹配 `/a/b`。`resolve_root`（`mcp/index.rs:176-196`）把 `root` 参数限在 `XIRANG_PACKAGE_ROOT` 之下且必须存在且是目录——实测仓库根（12 个成员的工作区）会得到「is not a package… set XIRANG_NAMESPACE」而不是错答。
2. **graft 计划文档解析（读码干净）**：版本不符 / 重复键 / 未知键 / 缺键 / `target_path` 空段 / selector 含分隔符、空白、引号、前导点 全部拒绝（`core/src/registry_core/plugin/graft/document.rs:69-151`、`:186-244`，自带钉子）。`graft.plan` 的 selector 因此不能把写入方带出记录根。
3. **计划行读取（实测干净）**：`read_dir` 的 `Err` 半边、读不了的 plan 文件、解析失败的 plan 都会变成被计数的行（`build_method/graft_view/plan_rows.rs:93-151`）；`grafts` 的 CLI 与 MCP 两条出口对同一份 fixture（好/漂移/幽灵/不可解析各一条）逐行一致——唯一不一致的是 LH-03 那处**结论词**而不是行内容。
4. **JSONL 解析（读码 + 既有钉子干净）**：重复字段、第二条 `snapshot`、未知 `kind`、`mir_line` 非整数、非 UTF-8、坏转义、代理对半只 全部拒绝（`core/src/registry_core/mir/jsonl.rs`）；`delta` 的方向与自述一致（`added = after − baseline`，`core/src/registry_core/mir/delta.rs:60-77`）。
5. **`tree_delta::by_source` 的「静默后者胜出」怀疑（已排除）**：`by_source` 是 `HashMap<String, NodeId>`，重复 `source` 会让后者覆盖；但 `parse_face` 明确拒绝「一个文件里两个注册面」（`core/src/registry_core/syntax/face.rs:175-183`），脚手架与解析器都保证一文件一面，因此同一 `source` 不可能对应两个不同 id；多符号面产生的多行共享同一 `id`，覆盖无害。读码结论：安全。
6. **wasm 限额（读码 + 读既有钉子）**：`WasmLimits` 的每一项都接到实现上——工件上限在编译前手工检查、被动元素段负载在实例化前按段头量出、`memory_size`/`table_elements`/`instances`/`memories`/`tables` 接进 `StoreLimits`、`EnforceLimits::strict()` 接进 `Config`、燃料在 `instantiate_and_start` 之前设定、输出上限在分配前按声明长度检查（`plugin-host/src/wasm.rs:149-233` 与 `:346-393`）。`plugin-host/tests/fault_matrix.rs` 已对表、被动段、声明内存、工件大小、燃料各有一条钉子（`PH-2` 等）。本轮**没有**新增 wasm 探针——这是「只读了代码 + 既有测试」的部分。
7. **进程插件限额与环境（实测干净）**：`max_output_bytes` 在分配前拒绝（`plugin-host/src/process/child.rs:156-167`）、stderr 读到 EOF 只保留前缀（`plugin-host/src/process/child.rs:104-117`）、stdin 写入与 stdout/stderr 排空都在自己的线程上（`plugin-host/src/process.rs:323`、`:341`、`:365`）、`inherit_env:false` 用 `env_clear` + 程序自带变量（`plugin-host/src/process/child.rs:70-75`）、`ETXTBSY` 有界重试（`plugin-host/src/process/child.rs:60-89`）。LH-08 的两条是这组能力之外的两个缝，不是限额失效。
8. **创作写路径的根约束（实测 + 读码干净）**：`module` 名走 `validate_name`（仅 `[a-z0-9_]`：`run_method/authoring/operations/operations.rs:210`、`:231`、`:328`），`registry_rule_path` 走 `generated_paths`——相对路径拒绝任何 `..` 组件、绝对路径 `canonicalize` 后要求仍在源码根内、并且要求文件带 XiRang 生成标记（`run_method/authoring/operations/paths.rs:6-39`）。这一处比 LH-06 的预览复制更严格，正好构成对照。
9. **`verified vs checksum` 准入（读码干净）**：`PluginAdmission::admit` 先过 `PluginPolicy::decision`，再按来源分派 `verify_signed` / `verify_artifact`（`plugin-host/src/admission.rs:165-185`），与威胁模型「官方要签名+信任策略、用户来源要摘要」一致；`core/.../plugin/trust/trust.rs` 的 `verify` 覆盖密钥指纹缺失、撤销表缺失、撤销命中三类拒绝，并有钉子。
10. **CLI 与 MCP 的 `explain --overlay` 互相排斥声明（实测干净）**：`xirang.explain` 同时给 `overlay` 与 `node` 时回 `overlay renders the whole effective tree; drop node`，与工具自述的「mutually exclusive」一致。

## 未覆盖 / 明确没做的事

- 没有对 `studio/`、`conventions/` 的门禁逻辑、`.github/` 做独立扫荡（属于其它片区）。
- wasm 侧只读代码与既有钉子，未新增恶意模块探针（时间预算给了身份/trace/预览三条线）。
- LH-01 的「危险方向」（假通过 / 剪错符号集）是**代码阅读佐证**，未构造出端到端复现；本轮实测到的是同一条机制产生的假失败。
- LH-02 的 `face_views` 消费端里，「一次编辑落到另一个面」是推论（读 `mcp/tools.rs` 的 `apply` 契约 + LH-02 实测的路径改名），未真的执行 `apply` 落盘。

## 探针清单（全部在 `/tmp`，可重建）

| 路径 | 用途 |
| --- | --- |
| `/tmp/nichprobe/probe.py` | 驱动 `target/debug/xirang-mcp` 的 stdio JSON-RPC：`python3 probe.py <tool> '<json args>' <cwd> [env]` |
| `/tmp/nichprobe/pkg` | 可解析的 fixture 宿主包（`examples/control-button/src` + 只含 `[package]` 的 `Cargo.toml`）；`.xirang/external-grafts/*/graft.plan` 按 LH-03 放置 |
| `/tmp/nichprobe/idprobe` | LH-01：一个进程里对 A/B 两包交替 `check_for`；`cargo run --offline --release [swap]` |
| `/tmp/nichprobe/libleak` | LH-05：`TraceArtifact::parse` 的 RSS 增长 A/B 组 |
| `/tmp/nichprobe/procprobe` | LH-08：`ProcessBackend`（`process-tools`）的「作答后不退出」「半帧后 exit 0」 |

三条 `/tmp` 探针 crate 都通过 `path` 依赖本检出（`xirang-build-method` / `xirang-core` / `xirang-run-method` / `xirang-plugin-host`），`cargo --offline` 可构建；不写入本检出。
