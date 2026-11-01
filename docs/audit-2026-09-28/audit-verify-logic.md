# 独立复核：逻辑与安全总账的 CRITICAL / MAJOR（audit-verify-logic）

- 复核对象：`docs/audit-2026-09-28/audit-logic-safety-report.md`（t15，618 行，3 CRITICAL + 29 条 MAJOR 行 LG-04…LG-32）。
- 复核者：bridge-auditor（桥与宿主那片区的审计员，**不是**总账作者；本报告与 t15 无共享探针）。
- 工作目录 `/home/nich/Moirai_N3/nichlink`；**源码一行未改**。文件名按队长 2026-09-28 规则加 `audit-` 前缀
  （任务书写的是 `verify-logic.md`）。写入只落在本文件；探针全在 `/tmp/verifyprobe/`，`cargo` 一律 `--offline`。
  （构建产物写在各自 `target/`，不触碰源码；未跑 `git checkout/restore/stash`，未跑 `cargo fmt`。）
- 纪律：**默认怀疑作者**。每条先问「能不能用与作者不同的装置复现」，不能的明确写「代码阅读佐证」或「无法判定」；
  凡「症状为真、成因/修法说错」的单独成节。**没有重跑作者的任何脚本**：所有探针都是我本轮新写的
  （`/tmp/verifyprobe/vp`，10 个 bin + 端到端驱动 MCP 二进制 + `rustc`），夹具也是新的。

## 0. 复核装置（全部为本轮新建）

| 装置 | 依赖 | 命中条目 |
| --- | --- | --- |
| `/tmp/verifyprobe/vp/src/bin/p_admission.rs` | `xirang-core`（syntax） | LG-02 |
| `p_catalog.rs` | `xirang-core` | LG-04、LG-40（旁证）、LG-03 的核侧一半 |
| `p_traits.rs` | `xirang-core` | LG-30、LG-05 的守卫一半 |
| `p_syn_abort.rs` | `syn` + 内核守卫 | LG-05 |
| `p_gates.rs` | `xirang-conventions`（直接调门禁函数） | LG-25、LG-26 |
| `p_lint.rs` + `rustc` 三文件 | `xirang-conventions::lint` + `rustc` | LG-24 |
| `p_faceview.rs` + 三棵树夹具 | `xirang-build-method::face_views` | LG-08、LG-11 |
| `p_idcache.rs` + A/B 两包（两次调用） | `xirang-build-method::run_for` | LG-01 |
| `p_plugin_timeout.rs` | `xirang-plugin-host`（process-tools） | LG-32 |
| 端到端驱动 `target/debug/xirang-mcp`（原始字节 stdin） | 已构建的 MCP 二进制 + 自建包夹具 | LG-18、LG-19、LG-20、LG-21、LG-22、LG-23 |

作者原探针（`/tmp/nichverify`、`/tmp/nichprobe`）**我一次都没有运行**，只在正文里注明「采信作者实测」时引用其结论。

## 1. 复核结论总表

| id | 原严重度 | 复核结论 | 独立装置 | 严重度变化 |
| --- | --- | --- | --- | --- |
| LG-01 | CRITICAL | **证实** | `p_idcache`（冷/热两次调用） | 维持 |
| LG-02 | CRITICAL | **证实** | `p_admission` | 维持 |
| LG-03 | CRITICAL | **证实**（核侧探针 + 写侧读码） | `p_catalog` + Studio 写盘读码 | 维持 |
| LG-04 | MAJOR | **证实**（比我测的四种多两种旁路） | `p_catalog`（13 形态） | 维持 |
| LG-05 | MAJOR | **部分证实**（缺陷成立；**修法与触发形状有别**） | `p_syn_abort` + `p_traits` | 维持 MAJOR，**修法需改** |
| LG-06 | MAJOR | **部分证实**（分叉读码成立，症状未独立复现） | 读码 `overlay.rs` / `graft_ops.rs` | 维持 |
| LG-07 | MAJOR | **证实**（两处静默 + 一处伪造键） | 读码 `index.rs` / `resolution.rs` / `transaction.rs` | 维持 |
| LG-08 | MAJOR | **证实**（直接演示：同形状换名字两个答案） | `p_faceview`（两棵夹具） | 维持 |
| LG-09 | MAJOR | **部分证实**（写序读码成立；混代被读成 current 未注入失败复现） | 读码 `pipeline.rs` / `cache.rs` | 维持 |
| LG-10 | MAJOR | **证实**（读码；对照口径存在） | 读码 `migration.rs` / `scope_view.rs` | 维持 |
| LG-11 | MAJOR | **证实** | `p_faceview`（broken 夹具） | 维持 |
| LG-12 | MAJOR | **证实**（两侧读码闭环） | 读码 `diff.rs` + `plan_rows.rs` + `declared.rs` | 维持 |
| LG-13 | MAJOR | **证实**（BR-6 本人原发现；本轮补两侧读码） | 读码 `verify.rs` / `registry.rs` / `tree_delta.rs` | 维持 |
| LG-14 | MAJOR | **证实**（读码） | 读码 `face_registration.rs` | 维持（备注：零运行期后果） |
| LG-15 | MAJOR | **证实**（grep 全仓） | `grep` + 读码 `manifests.rs` | 维持 |
| LG-16 | MAJOR | **部分证实**（四处调用点读码成立；出厂路径无受害者） | 读码四处调用点 | 维持（标注「潜伏」） |
| LG-17 | MAJOR | **证实**（读码） | 读码 `authoring/parse/parse.rs` | 维持 |
| LG-18 | MAJOR | **证实**（端到端） | MCP 原始字节驱动 | 维持 |
| LG-19 | MAJOR | **证实**（第一手：走过外链并命名包外文件） | MCP 驱动 + 夹具外链 | 维持（补一条「diff 看不见」的细节） |
| LG-20 | MAJOR | **证实**（第一手残留目录 + 读码） | MCP 驱动（失败预览） | 维持 |
| LG-21 | MAJOR | **证实**（症状比报告更重：traits/contracts 互相矛盾） | MCP 驱动（add→edit） | 维持（症状升级） |
| LG-22 | MAJOR | **证实**（三类畸形帧对照） | MCP 原始字节驱动 | 维持 |
| LG-23 | MAJOR | **证实**（端到端假阳性） | MCP 驱动（预览 diff） | 维持 |
| LG-24 | MAJOR | **证实**（**跑门禁本身** + rustc 两半） | `p_lint` + `rustc` | 维持 |
| LG-25 | MAJOR | **证实**（多出三种等价否定拼法） | `p_gates`（门禁函数） | 维持 |
| LG-26 | MAJOR | **证实**（门禁对 `uses:` pin 沉默） | `p_gates`（pin 夹具） | 维持 |
| LG-27 | MAJOR | **证实**（读码：合并清单里没有 runtime_checks） | 读码 `owned.rs` | 维持 |
| LG-28 | MAJOR | **证实**（读码：两个解析器一拒一丢） | 读码 `rules.rs` + 桥两处 | 维持 |
| LG-29 | MAJOR | **证实**（读码：`visit_item_mod` 无条件 return） | 读码 `entries/graft.rs` | 维持 |
| LG-30 | MAJOR | **证实**（probe：带逗号泛型 → `Err`） | `p_traits` | 维持 |
| LG-31 | MAJOR | **证实**（读码：进程级 `static` 集合 + `Box::leak` 不回收） | 读码 `artifact/parse.rs` | 维持 |
| LG-32 | MAJOR | **证实**（第一手：完整帧 + 不退出 → `Err(Timeout)`） | `p_plugin_timeout` | 维持 |

**合计：证实 28 条、部分证实 4 条（LG-05/06/09/16，另 LG-19/21 各有一处细节修正）、证伪 0 条。**
没有一条被推翻；四条「部分证实」缺的都是**症状侧的注入复现**，机制侧都能读码闭环。

---

## 2. 逐条复核

### LG-01（CRITICAL）进程级身份缓存跨包污染 → **证实**

- **我的装置**：`/tmp/verifyprobe/vp/target/debug/p_idcache`——A/B 两包源码逐字节相同、只改包名，
  在**同一个进程**里依次 `run_for(A)`、`run_for(B)`；整个二进制**跑两次**（第一次冷 unit 缓存、第二次热）。
- **证据**（`./target/debug/p_idcache; echo exit=$?`）：
  ```text
  第 1 次（冷）：pkg-a => Ok      pkg-b => Ok
  第 2 次（热）：pkg-a => Ok      pkg-b => Err("NICHLink BUILD CHECK FAILED")
      +-- phase=static plan / 静态计划  source=control/button/button.rs  field=parent
      |   expected=registered parent   actual=edd426938bc250d3416e377d45ab7eac
      pkg-b 的 pruning_manifest.tsv：69e3bdd8… control/button/button.rs / edd42693… control/control.rs
  exit=0
  ```
  失败的 `actual`（父身份）与 pkg-b **自己清单里的** `control/control.rs` 身份逐字节相同——同一次运行里
  两处 id 来源不同，正是总账写的签名词（`static_plan` 用缓存、`manifests` 重算）。
- **代码闭环**（代码阅读佐证）：`build_method/node_id.rs:25` 的 `CACHED_NODE_IDS` 是
  `OnceLock<BTreeMap<String, (NodeId, String)>>`（键**只有相对路径**）；`identity_cache.rs:64`
  用 `let _ = CACHED_NODE_IDS.set(values)`（先到先得）；`pipeline.rs:55` 在作用域阶段之前 prime。
- **严重度**：维持 CRITICAL。**备注**：我的装置与作者的不同点在于——不手写 unit 文件、用公开
  `run_for` 走完整管线、夹具是 control/button 两层，且两次调用是**同一二进制**而不是他们的 idprobe 工程。

### LG-02（CRITICAL）创作面读回丢 deny → **证实**

- **我的装置**：`p_admission`——`parse_admission_expression` → `render_admission` → `parse_admission_owned`
  → `OwnedAdmission::accepts("ui/experimental")`，把「三跳之后门禁是否放宽」直接跑出来。
- **证据**（`./target/debug/p_admission; echo exit=$?`，exit=0）：
  ```text
  直接 parse_admission_owned(App...) = 门禁原样：deny 优先，accepts(ui/experimental) = false
  A input = crate::Admission::new(&["ui"], &["ui/experimental"])
    read_back = allow:ui
    rendered  = Ok("crate::Admission::new(&[\"ui\"], &[])")
    accepts(ui/experimental) = Ok(true)        ← 一次读回往返即放宽（不可逆：写回源码）
  B deny only  -> deny:ui/experimental, accepts = Ok(false)   （对照：deny 本身有效）
  ```
- **代码闭环**：`authoring/parse/admission.rs:38-45` 的 `match (allow.is_empty(), deny.is_empty())`
  在两张表都非空时走 `(false, _) => format!("allow:{}", …)` —— deny 被丢。
- **严重度**：维持 CRITICAL。**备注（作者未写）**：`parse_admission_expression` 与
  `parse_admission_owned` 接受的拼法是**不相交**的——前者读 `Admission::new(...)`/`ANY`，
  后者读 `allow:`/`deny:`/`ANY`（我的探针把构造函数形式喂给后者得到 `Err("admission must be ANY,
  allow:path/prefix, or deny:path/prefix")`）。所以「读一次再写回」这条链上，读半与校验半用的
  是两个词表；这是同一处缺陷的第二个成因，修的时候要一起收。

### LG-03（CRITICAL）Studio 插件写盘能产出解析器拒绝的锁 → **证实**

- **核侧一半（探针）**：`p_catalog` 形态 9：
  ```text
  9 duplicate identity -> Err(plugin lock line 2: duplicates package identity `pkg` `1.0.0`)
  ```
  即同一身份五元组出现两次时 `PluginCatalog::parse` **整份拒绝**——而宿主准入读的就是整份锁。
- **写侧一半（代码阅读佐证）**：`studio/studio/app/mutations.rs:257` 手拼 7 字段记录；
  `:260` 的唯一闸是**整行字符串相等**（`existing.lines().any(|line| line.trim() == record)`）；
  `:299` 的 `contains_record` 只对 `source == "official"` 生效；`:341` 之后直接 `append_line(&lock, …)`，
  **不把追加后的文本回读给解析器**。因此 user 来源改 checksum、或 official 改 mode 而五元组不变，
  都会追加出重复身份 → 整份锁变成 `Err`。
- **严重度**：维持 CRITICAL（写坏的锁让宿主对**该文件里所有插件**失去准入能力，恢复要手改文件）。
  **备注**：作者说「未给可跑探针」——TUI 驱动我也没有构造（成本不成比例）；但「写盘闸门不足以产生
  内核接受的记录」这一半我用**门禁函数本身**独立证实了，比 t11 的装置更直接。

### LG-04（MAJOR）锁 schema 门禁可按外观与行序绕过 → **证实（并多两种旁路）**

- **我的装置**：`p_catalog`（13 形态，自建字符串，不碰作者夹具）。
- **证据**（`./target/debug/p_catalog; echo exit=$?`，exit=0）：
  ```text
  1 正确表头 v3 + 记录          -> Ok(1)            （对照：门禁在工作）
  2 版本错、表头在前            -> Err(… uses identity schema v9, expected v3)
  3 版本错、**没有记录**        -> Ok(0)            ← 检查根本不跑（作者 K07）
  4 `# xirang-schema v9`（少 `=`）-> Ok(1)         ← 当普通注释吞掉（作者 X-2）
  5 `# xirangschema=v9`（少连字符）-> Ok(1)        ← **本轮新增旁路**
  6 表头在记录之后              -> Ok(1)            ← 之前的记录从不检查（作者 K07）
  7 CRLF 版本错 / 8 前导空白     -> Err              （对照：trim 救了这两种拼法）
  ```
- **代码闭环**：`core/.../plugin/catalog/catalog.rs:131-137`（`strip_prefix` 失败即落 `starts_with('#') => continue`）
  与 `:138-145`（schema 检查在**记录循环内**）。
- **严重度**：维持 MAJOR（门禁可静默失效）。**备注**：作者只举了「少 `=`」，我另测出「少连字符」
  同样绕过——说明修法不能只救一种拼写错，要按「以 `# xirang-schema` 开头就进 schema 处理」改。

### LG-05（MAJOR）`flow_provider` 编辑入口缺嵌套守卫 → **部分证实（修法与触发形状需改）**

- **我的装置**：`p_syn_abort`——先在**主线程**（8 MiB 栈）上问内核守卫与受守卫的兄弟函数，再在
  **256 KiB 栈**的线程上做**无守卫**的 `syn::parse_str::<syn::Path>`；两种形状各跑一次。
- **证据**（`./target/debug/p_syn_abort generics; echo exit=$?`）：
  ```text
  shape=generics input_bytes=301
  guarded trait_names_from_paths -> Ok(1)
  guarded guard_nesting          -> Ok(())        ← 守卫对这份输入放行！
  --- 无守卫调用（256 KiB 栈）---
  thread '<unknown>' has overflowed its stack
  fatal runtime error: stack overflow, aborting
  exit=134
  ```
  （`./target/debug/p_syn_abort segments`：同样的 301 字节、100 层 `::`，`syn` 正常解析出 101 段，
  `exit=0`，**不 abort**。）
- **结论**：**缺陷成立**——`run_method/.../face/face.rs:143` 的 `syn::parse_str::<syn::Path>`
  确实没有守卫（同一份代码在 `core/.../authoring/parse/parse.rs:217` 有 `guard_nesting`），
  301 字节输入在 256 KiB 栈上以 SIGABRT 结束。
  **但作者的两条说法要改**：① 触发形状是**嵌套泛型实参**（递归），不是「类型路径文本的深度」——
  同样深度的 `::` 链不 abort；② 「把 `guard_nesting` 接到这一处」**不足以闭合**：我这份 100 层输入在
  `guard_nesting` 下是 `Ok(())`（阈值按加权深度 128），而它在小栈线程上照样 abort。
  守卫量的是**嵌套度**，不是「本线程还剩多少栈」；宿主在 256 KiB 工作线程上调用编辑 API 时，
  129 层以下仍然会打死进程。修法应写成：接 `guard_nesting` **并且**在文档/API 上说明
  「栈仍是调用方的责任」，或把解析放进一个自带足够栈的线程。
- **严重度**：维持 MAJOR。**备注**：这是本轮唯一「症状为真、作者对成因/修法的解释不完整」的条目。

### LG-06（MAJOR）overlay 非 full 分支不查 `needs_registry` → **部分证实（读码）**

- **代码阅读佐证**：`core/.../tree/graft_ops/overlay.rs:256-309` 的 else 分支只做
  `registry.reconfigure(&candidate)` + `child_violating(...)`（按新 `registry_rule` 判），
  然后 `entry.child = kept` —— 通篇没有 `candidate.needs_registry` 的判定；
  而 `graft_ops.rs:261-264` 的 `replace_info` 对同一种情形
  `(Some(child_registry), false) if !child_registry.entries.is_empty() => … return false`（拒绝）。
  两处对同一不变量不同判。
- **严重度**：维持 MAJOR。**备注**：症状（「该叶子仍可注册」）采信 t9 探针，本轮我**没有**独立复现
  （构造两棵注册树 + GraftPlan 的夹具成本超过本条的边际收益）；机制侧读码闭环。

### LG-07（MAJOR）兄弟同名 `registry_name` 三处静默碰撞 → **证实（读码，机制更精确）**

- **代码阅读佐证**：
  1. `core/.../tree/index/index.rs:88-93`：`by_path` 是 `BTreeMap<String, NodeId>`，逐面
     `insert(entry_path, id)` —— 同名兄弟时**后者覆盖前者**（无报错、无计数）。
  2. `core/.../tree/graft_ops/resolution.rs:22-27`：`resolve_path` 走 `depth_first()` 取
     `find_map`（**第一个**命中），无歧义报错。
  3. `core/.../tree/query/query.rs:86-96`：`path_for` 把 `path_for(parent)/registry_name` **铸**出来——
     它是重复键的来源（本身不丢信息）。
- **判据**：`transaction.rs:98-179` 的 `plan_batch` 只查重复**节点身份**、重复 stable name、父环、缺父，
  **没有**任何兄弟 `registry_name` 唯一性检查。
- **严重度**：维持 MAJOR。**备注**：总账说「三处」——按我的读码应表述为「一处铸键（`path_for`）
  + 两处静默取用（`by_path` 覆盖、`resolve_path` 取第一个）」。

### LG-08（MAJOR）`FaceView.path` 顺序依赖 → **证实（直接演示）**

- **我的装置**：`p_faceview` + 两棵**同形状、只换名字**的夹具：
  `order` = 子目录 `aaa_child/`（父在 `zzz_parent/`）、`order2` = 子目录 `zzz_child/`（父在 `aaa_parent/`）。
- **证据**（`./target/debug/p_faceview; echo exit=$?`，exit=0）：
  ```text
  order : path=root/aaa_child          parent_resolved=true   ← 该是 root/zzz_parent/aaa_child
  order2: path=root/aaa_parent/zzz_child  parent_resolved=true   ← 同一个形状，答案正确
  ```
- **代码闭环**：`build_method/face_view.rs` 先在 `collect` 顺序里逐面算路径（`:168` `logical_path`），
  **之后**才 `views.sort_by`（`:172`）；`logical_path`（`:280-298`）只查一层 `paths.get(parent)`，
  未命中回退 `root`（其文档却写「沿已解析的父级链行走」）。
- **严重度**：维持 MAJOR。**备注**：这条是我用**两个夹具对照**直接演示的，比「读码 + 单点复核」更强。

### LG-09（MAJOR）货币凭据先于载荷发布 → **部分证实（读码）**

- **代码阅读佐证**：`build_method/pipeline.rs:155-164` 在 `compile_errors.is_empty()` 时先写
  `discovery.fingerprint`，`:174-184` 才写五份载荷（并把失败收进 `write_errors`）；
  `cache.rs:36-39` 的 `write_if_changed` 是就地 `fs::write`（非临时文件 + rename）。
  因此「载荷写失败/被截断 + 指纹已发布」是可达状态，`build_output_is_current` 只读指纹。
- **严重度**：维持 MAJOR。**备注**：症状（读者把上一代行当本代作答）需要注入写失败（磁盘满/权限/中断），
  本轮没有构造；写序本身是决定性证据，未复现症状的部分如实标注。

### LG-10（MAJOR）模块迁移无边界文本替换 → **证实（读码）**

- **代码阅读佐证**：`run_method/authoring/operations/migration.rs:179-181`
  `.replace(&rust_old, &rust_new).replace(&old_prefix, &new_prefix)`（`rust_old = "crate::<old module>"`
  无 `::` 尾界 → 会命中 `crate::control_extra`；`old_prefix = "src/<old>/"` → 命中 `src/control_extra/`）。
  同仓对照口径在 `build_method/scope_view.rs:109-118`（`selected.as_bytes().get(module.len()) == Some(&b':')`）。
- **严重度**：维持 MAJOR。

### LG-11（MAJOR）`face_views` 静默丢面并把子面改挂根 → **证实**

- **我的装置**：`p_faceview` 的 `broken` 夹具——`src/bad/bad.rs` 里两个宏调用（解析失败），
  `src/bad/child/child.rs` 声明 `parent: crate::bad::NODE_ID`。
- **证据**：
  ```text
  broken: faces 1
    path=root/child  kind=Child  module=bad::child  parent_resolved=false
  ```
  即：`bad` 面**静默消失**（回复里没有任何计数或提示），子面被改挂根、报出一条运行期从未有过的
  `root/child`。与总账描述逐字一致。
- **严重度**：维持 MAJOR。**备注**：这个夹具同时验证了 `parent_resolved=false` 与「路径被改写」两半。

### LG-12（MAJOR）`diff records:true` 只在身份在树里那一支查「已声明」→ **证实（两侧读码闭环）**

- **代码阅读佐证**（`mcp/diff.rs:145-175`）：`if faces.iter().any(|face| face.id == target)`
  **先**判定；只有进去以后才看 `row.declared`（`Some(false)` → `undeclared`）。身份缺席时直接走
  `re-identified`/`stale`，**从不看 `declared`** → 计数行永远是 `undeclared 0`。
- **对侧为什么会出现 `declared=false`**（这是总账没说全的一半）：
  `build_method/graft_view/plan_rows.rs:155-175` 里 `module` 由「树里有这个身份的**面**」决定，
  身份缺席 → `module=None`；`declared.rs:147-149` 的 `names_face(path, None)` 对**类型化切口**直接
  `return false`；于是 `declared_state = Some(false)`。字符串切口（`expressions: None`）走
  `self.cut == path` 比对，不受影响——这正好解释了总账「触发条件：宿主入口用类型化切口」。
- **严重度**：维持 MAJOR。

### LG-13（MAJOR）`XIRANG_NAMESPACE` 下 verify 与 diff/search 各用一套命名空间 → **证实**

- **代码阅读佐证**：`mcp/verify.rs:43` 用 `package_name(&manifest)`（Cargo 名）作 `check_for` 的
  package；`mcp/registry.rs:59-61` 的 `namespace()` 先读 `XIRANG_NAMESPACE` 覆盖；
  `tree_delta.rs:101-119` 的 `by_source` 按**源码路径**命中（与命名空间无关），于是每个面都落进
  `Reidentified(previous)`；`build_output_is_current` 的指纹只散列路径与内容，仍报 `current`。
- **严重度**：维持 MAJOR。**备注**：这条是**我本人**在 t5 的 BR-6 里提出的（作者标了别名 BR-6）；
  本轮我补的是两侧实现与「为什么每个面都命中」的闭环，作者原来的探针（t12 的 `reidentified 3`）我没有重跑。

### LG-14（MAJOR）`linked` collector 只留注释 → **证实（读码）**

- **代码阅读佐证**：`run_method/macros/face_registration.rs:9-10` 模块文档「收集器标识决定注册信息
  进入哪个链接器段」；`:168-171` 的 `(linked; $registration)` 展开为一个**注释**
  （「采集由 xirang-debug 持有；core 只保留纯声明」）。`development` 臂也是空。
- **严重度**：维持 MAJOR（自述承诺一类，按本仓标准起记 MAJOR）。**备注**：该臂**零运行期后果**
  （静态计划仍由 `static_plan` 生成），修法可以是实现，也可以只是把文档改成「保留位」。

### LG-15（MAJOR）剪枝符号列由魔法标识符产出，一支硬编码 `Button::…` → **证实**

- **证据**（`grep -rn "optional_pruning_probe\|PRUNING_TABLE" --include=*.rs . | grep -v target | grep -v _tests`）：
  ```text
  ./build_method/src/manifests.rs:202  let name = if line.contains("PRUNING_TABLE")
  ./build_method/src/manifests.rs:208  } else if line.contains("optional_pruning_probe") && line.contains("fn optional_pruning_probe")
  ./build_method/src/manifests.rs:210      "Button::optional_pruning_probe"
  ```
  全仓只有这三行；`Button::optional_pruning_probe` 是**与当前面无关的字面量**（被命名的面根本不存在），
  且它由**行文本**匹配产生。
- **严重度**：维持 MAJOR。

### LG-16（MAJOR）`compile_error_demo` 魔法名改写构建结果 → **部分证实（读码；触发狭窄）**

- **代码阅读佐证**（四处调用点，逐处读原文）：
  `build_method/contracts.rs:78`（跳过契约聚合）、`registration_check.rs:86-87`（跳过注册检查）、
  `static_plan.rs:74`（不进静态计划）、`renderer/tree.rs:68-70`（给该模块包一层
  `#[cfg(feature = "compile_error_demo")]`）。
- **严重度**：维持 MAJOR，但**标注「潜伏」**：触发需要宿主存在名为 `compile_error_demo` 的模块/文件，
  出厂路径上没有受害者。它仍是本仓明文的「判定落在名字上」反例（§5.2 #12），修法（改成显式宿主意愿）
  与严重度无关。

### LG-17（MAJOR）authoring 的 source 回落链硬编码夹具目录名 → **证实（读码）**

- **代码阅读佐证**：`run_method/authoring/parse/parse.rs:27-42`：
  ```text
  let roots = ["compile_error_demo", "control", "engine", "trimmed_core"];
  … if roots.contains(&text.as_ref()) { 从这里截断成相对路径 }
  ```
  只在 `path.strip_prefix(source_root())` 失败（即路径不在配置的源码根下）时才走到——但那时它把
  宿主的绝对路径按**本仓夹具名**截断，写出一个错的 `source`。
- **严重度**：维持 MAJOR。

### LG-18（MAJOR）MIR `jsonl:true` 给任意可读文件盖快照表头 → **证实（端到端）**

- **我的装置**：自建包夹具 `mcpfix`（`notes.txt` = 普通文本），raw JSON-RPC 驱动
  `target/debug/xirang-mcp`。
- **证据**（`printf … | XIRANG_PACKAGE_ROOT=$F ./target/debug/xirang-mcp`，exit=0）：
  ```text
  {"id":1,…,"text":"{\"kind\":\"snapshot\",\"namespace\":\"mcp-host\",\"root\":\"08806abc…\"}\n"}
  {"id":2,…,"text":"file …/notes.txt\nfunctions 0 calls 0 locals 0\ncalls:\nlocals:\n"}
  ```
  即：一个零记录的文本文件被盖上**本包身份快照**，且回读被当作合法快照。
- **严重度**：维持 MAJOR（来源凭据可造）。

### LG-19（MAJOR）预览副本跟随目录符号链接走出包根 → **证实（第一手；一处细节修正）**

- **我的装置**：`mcpfix/outside_link -> /tmp/verifyprobe/outside`（包外目录含 `secret.txt`），
  用 apply 预览驱动。
- **证据 A（跟随被证实）**：把包外文件置为不可读后预览——
  ```text
  isError: True   text: cannot copy /tmp/verifyprobe/fix/mcpfix/outside_link/secret.txt: Permission denied (os error 13)
  ```
  报错路径字面在包内、实际解析到**包外**文件：拷贝遍历确实走进了链接。
- **证据 B（细节修正）**：包外文件可读时，预览回复里**看不到**任何 `outside*` 行——因为项目侧与副本侧
  都经同一个链接读到同一份字节，diff 判定「未变」。即：**外链数据确实被拉进 `/tmp` 副本，而 diff 不会说**。
  残留目录 `/tmp/xirang-mcp-preview-485308-0` 里就有 `outside_link` 这一项（配合 LG-20 的泄漏）。
- **代码闭环**：`mcp/preview.rs:50-52` 用 `source.is_dir()`（跟随链接）且全函数无 visited/深度上限。
- **严重度**：维持 MAJOR。**备注**：作者只测了「自指链接 → faces 124」；我补的这条说明
  **包外数据**这一半也能独立成立，且 diff 不是它的观察面。

### LG-20（MAJOR）预览副本临时路径可预测：删除重建 + 失败留残留 → **证实（第一手 + 读码）**

- **证据（我自己跑出来的残留）**：我这次失败的预览留下
  ```text
  /tmp/xirang-mcp-preview-485308-0   13:30:13（本轮） 内容：binfile.dat Cargo.toml notes.txt outside_link src
  （另有两个更早的：-343377-0 12:38、-416016-0 13:00，是作者/t12 那几轮留下的）
  ```
  失败路径的清理缺口比总账写的更宽：`mcp/apply.rs:84-88` 里 `copy_package(root)?` 一旦失败就
  直接返回，**连 `remove_copy` 都没到**（`:144` 的清理只在更后面）。所以「复制中途失败」本身就足够泄漏。
- **代码阅读佐证**：`preview.rs:22-29` 手工拼 `temp_dir()/xirang-mcp-preview-{pid}-{seq}` +
  `remove_dir_all` 预清 + `create_dir_all`（无排他创建、无权限收紧）。
- **严重度**：维持 MAJOR。**备注**：**我 t5 的 BR-19 里「符号链接被顺着写入」那半是错的**——
  `remove_dir_all` 先删掉了链接本身，t12 的证伪与我本轮读码一致；总账把它改成「删除并重建 + 失败泄漏」
  是对的，我确认这一版描述。

### LG-21（MAJOR）`apply edit` 静默忽略 `handle_contracts`/`part_contracts` → **证实（症状比报告更重）**

- **我的装置**：`mcpfix` 上两次 apply（`add` 带 `handle_contracts`+`handle_traits`，再 `edit` 只给
  `handle_contracts: "crate::Baz::Qux"`），每次读生成文件。
- **证据**：
  ```text
  add  → 文件：handle_traits: ["Bar"]        handle_contracts: [crate::Foo::Bar]
  edit → 文件：handle_traits: ["Qux"]        handle_contracts: [crate::Foo::Bar]   ← 两行互相矛盾
  两次回复都是 isError:false / “applied …”，一个字都没提。
  ```
  报告说「文件只有 `handle_traits`、无 contract 行」；我的探针给出的是**更坏**的状态：`handle_traits`
  被写成**新契约派生的标签**（`Qux`），而 `handle_contracts` 还是旧值——同一个面上两条本该由同一规则
  产出的行**互相矛盾**。
- **代码闭环**：`run_method/.../operations/face_write.rs:58-79` 的 `EDIT_FIELD_ORDER` 有
  `handle_traits`/`part_traits`，**没有** `handle_contracts`/`part_contracts`；而
  `core/.../syntax/fields.rs:103-106` 的 `string_list("handle_traits")` 是**从 `handle_contracts` 推导**的
  ——推导读内存值（新），写出读字段序（旧），矛盾由此产生。
- **严重度**：维持 MAJOR（症状升级）。

### LG-22（MAJOR）非 UTF-8 请求帧结束整个桥 → **证实（三类对照）**

- **我的装置**：raw 字节喂 `xirang-mcp`，每类后面跟一条 `ping`。
- **证据**：
  ```text
  非法 UTF-8 帧： exit=1 stdout lines=0 stderr=xirang-mcp: cannot read stdin: request line is not valid UTF-8
  坏 JSON    ： exit=0 stdout lines=2 （-32700 + ping 回复）
  超长行      ： exit=0 stdout lines=2 （-32600 + ping 回复）
  ```
- **严重度**：维持 MAJOR。**备注**：与两份 README 的「不写 stderr / 失败是 stdout 上的错误响应」承诺冲突。

### LG-23（MAJOR）预览 diff 把非 UTF-8 文件报成新增、无上限、复制 `.git` → **证实（端到端）**

- **我的装置**：`mcpfix` 放一个 512 字节随机 `binfile.dat` + 伪造 `.git/index`，跑 apply 预览。
- **证据**：
  ```text
  + .git/index                  ← 本次编辑没碰过
  + binfile.dat                 ← 两侧逐字节相同
  + src/probe_add/probe_add.rs  ← 唯一真实变化
  （reply 的 '+' 行共 17 行；exit=0，没有任何“可能有假阳性”的提示）
  ```
- **代码闭环**：`preview.rs:108` 的 `read_to_string(...).unwrap_or_default()` + `:115` 的 `Err(_) => "+ …"`。
- **严重度**：维持 MAJOR。

### LG-24（MAJOR）`lint` 把 `#![deny(warnings)]` 当作「带着 missing_docs」 → **证实（跑门禁本身 + rustc）**

- **我的装置 A（门禁函数本身，比作者的 rustc 探针更直接）**：`p_lint` 在
  `/tmp/verifyprobe/lintfix`（一个真工作区：根 `[workspace] members=["zznew"]`）上依次写四种 crate 根，
  调 `xirang_conventions::lint::missing_roots`。
  ```text
  #![deny(warnings)]            -> missing_roots = []
  #![warn(missing_docs)]        -> missing_roots = []
  （无属性）                     -> missing_roots = ["zznew/src/lib.rs"]
  #![deny(warnings, missing_docs)] -> missing_roots = []
  allow_workarounds = ["zznew/src/lib.rs:2"]        ← 同门禁另一半仍工作
  ```
- **我的装置 B（rustc 两半）**：`printf '…' | rustc - --crate-type lib --crate-name probe_x -o /tmp/…`
  ```text
  #![deny(warnings)]    : exit=0 stderr_lines=0     （lint 仍是 allow）
  #![warn(missing_docs)]: exit=0 stderr_lines=22    （2 warnings）
  ```
- **严重度**：维持 MAJOR。**备注**：作者自记的探针 bug（第一版多写了一行 `#![warn(missing_docs)]`）
  不影响结论；我的两个装置都独立得到「门禁放行、rustc 沉默」。

### LG-25（MAJOR）tag 守卫用子串黑名单判否 → **证实（并多三种等价拼法）**

- **我的装置**：`p_gates` 直接调 `xirang_conventions::release_workflow::findings(text)`，自建 9 种
  `if:` 写法 × 两种上传命令。
- **证据**（节选，全表见装置输出）：
  ```text
  control 肯定式            findings=0   （对照：正确形状不报）
  `== false`                findings=0   ← 作者 G-02
  `== 0`                    findings=0   ← 本轮新增
  `! startsWith(...)`（空格）findings=0   ← 本轮新增
  `!(startsWith(...))`      findings=0   ← 本轮新增
  `!=` / `||` / `!startsWith`（连写） findings=1  （对照：黑名单认识的那些仍能报）
  ```
- **代码闭环**：`release_workflow.rs:96-111` 的 `contains("!startsWith") || contains("!=") || contains("||")`。
- **严重度**：维持 MAJOR。**备注**：修法必须是「只允许肯定式形状」的白名单，补黑名单救不了。

### LG-26（MAJOR）持 token 的 job 的 action pin 无门禁 → **证实（探针 + 读码）**

- **证据**：`p_gates` 末例——一个只有 `uses: some/action@v1` + `token: ${{ secrets.CARGO_REGISTRY_TOKEN }}`、
  没有任何上传命令的 job：`findings=[]`。
- **代码阅读佐证**：`conventions/src/release_workflow.rs:236-250` 是**唯一**读 `uses:` 的地方，且它
  专门 `split('@').next()` **丢掉** ref——门禁从头到尾没有「pin 必须是 40 位 SHA」这条判定。
- **严重度**：维持 MAJOR（供应链：持有发布 token 的 job）。

### LG-27（MAJOR）`merge_authored` 静默忽略作者侧 `runtime_checks` → **证实（读码）**

- **代码阅读佐证**：`core/src/registry_core/declaration/owned.rs:280-315` 的合并逐字段赋值清单里
  有 `namespace/kind/preset/parts/params/handle/stable_name/name/summary/exports/needs_registry/
  registry_name/getting_from_other_registry/registry_rule_path/registry_rule/admission/requires/provides/
  handle_traits/part_traits/source`，`flow`/`flow_provider` 是条件赋值，**没有任何一行处理 `runtime_checks`**。
- **严重度**：维持 MAJOR。

### LG-28（MAJOR）畸形 `requires` 条目三种口径 → **证实（读码）**

- **代码阅读佐证**：
  ```text
  core/src/registry_core/authoring/parse/rules.rs:108-119 parse_requirements_owned：filter_map(item.split_once("=>")?) → 静默丢
  core/src/registry_core/authoring/parse/rules.rs:121-135 parse_requirements：同一输入 Err("requires entries must use capability=>provider syntax")
  mcp/impact.rs:246-255  与 converge.rs:192-207：前者不 push（少一条边），后者 push provider:"?" 的 UNANSWERED
  ```
- **严重度**：维持 MAJOR。**备注**：桥那两处是我 t5 的 BR-10（同一现象），本轮补上内核两个解析器的原文。

### LG-29（MAJOR）模块级 `cfg` 一律跳过 → **证实（读码）**

- **代码阅读佐证**：`core/src/registry_core/syntax/entries/graft.rs:104-116`（总账写作 `entries/graft.rs`——
  全路径是 kernel 的这份）的 `visit_item_mod` 里 `attrs.iter().any(|a| a.path().is_ident("cfg"))` 即
  `return`，**不求值**特性是否开启；而同一仓库有现成的 `build_method/static_plan.rs:157`
  `face_cfg_enabled` 给「面」那一边用。
- **严重度**：维持 MAJOR。**备注**：这条的 `file:line` 在总账里少了目录前缀（`entries/graft.rs` 不存在于
  `build_method/`），引用时应写 `core/src/registry_core/syntax/entries/graft.rs:105-116`。

### LG-30（MAJOR）trait 标签派生遇带逗号泛型实参失败 → **证实**

- **证据**（`./target/debug/p_traits; echo exit=$?`，exit=0）：
  ```text
  trait_names_from_paths("crate::Foo")            = Ok("Foo")
  trait_names_from_paths("crate::Foo<A>")         = Ok("Foo")
  trait_names_from_paths("crate::Foo<A, B>")      = Err("`crate::Foo<A` is not a Rust trait path")
  trait_names_from_paths("crate::Foo<A, B>, crate::Bar") = Err("`crate::Foo<A` …")
  ```
- **代码闭环**：`core/src/registry_core/syntax/fields.rs:124-131` 把路径 `join(",")` 后再交给
  `core/src/registry_core/authoring/parse/parse.rs:205-226` 的 `trait_names_from_paths`，后者又
  `split(',')`——先拼后拆，泛型实参里的逗号被当成路径分隔符。
- **严重度**：维持 MAJOR。

### LG-31（MAJOR）trace 字符串驻留是进程级无回收 `Box::leak` → **证实（读码）**

- **代码阅读佐证**：`run_method/runtime/trace/artifact/parse.rs:293-303`：
  ```text
  static INTERNER: Mutex<BTreeSet<&'static str>> = …;
  let leaked: &'static str = Box::leak(value.to_owned().into_boxed_str());
  interner.insert(leaked);
  ```
  表是 **`static`（进程级、永不清理）**，每个新字符串永久泄漏；文档写的「规模由 artifact 的词表决定」
  只对**单份** artifact 成立，而表面向的是进程里历次读过的**并集**。
- **严重度**：维持 MAJOR（长寿命桥）。**备注**：内存数字（+6.5 MB/10 万串）采信 t8，本轮未复现。

### LG-32（MAJOR）子进程给出完整帧却不退出时答案被丢成 `Timeout` → **证实（第一手）**

- **我的装置**：`p_plugin_timeout`（`xirang-plugin-host`，`process-tools`）——插件脚本
  `printf '\005\000\000\000hello'; sleep 5`，`timeout = 300 ms`；对照脚本写完即退出。
- **证据**：
  ```text
  outcome = Err(Timeout)          elapsed = 304.036123ms      ← 帧已完整送达仍被丢弃
  control (exits) = Ok([104, 101, 108, 108, 111])             ← 同一帧，只因子进程退出就交付
  ```
- **代码闭环**：`plugin-host/src/process.rs:372-409`（deadline 分支先于任何用 frame 的路径）
  与 `:421-425`（只有 `status.success()` 之后才 `return Ok(bytes)`）。
- **严重度**：维持 MAJOR。**备注**：这条同时也是**我 t5 的 BR-20**——我用自己的新探针复核自己的发现，
  结论不变（完整帧 + 不退出 = 丢答案 + 错的失败种类）。

### 已降级条目（LG-33 / LG-34）

两条在总账里已是 MINOR，按任务范围（CRITICAL/MAJOR）不在本轮逐条复核之列。我读到的两个事实与总账一致：
LG-33 的机制已被 t9 证伪（只剩文案）；LG-34 的「数据丢失」已被 t12 证伪（只剩表单显示歧义）。

---

## 3. 作者错在哪（尤其是「症状为真、成因/修法说错」）

1. **LG-05 的修法不充分（唯一一条需要改结论方向的）**：报告写「把内核的 `guard_nesting` 接到
   `run_method` 这一处」。我实测：同一条 100 层、301 字节的嵌套泛型输入，`guard_nesting` 返回
   `Ok(())`，而无守卫的 `syn::parse_str::<syn::Path>` 在 256 KiB 栈上仍然 `abort`（exit 134）。
   守卫量的是嵌套度（阈值 128），不是本线程可用栈；修法要补上「栈是调用方责任」或自带足够栈。
   另一处：报告的触发条件写成「类型路径文本深度」，实测**只有嵌套泛型实参**会递归到溢出——
   同样 301 字节的 100 层 `::` 链 `syn` 正常解析（`segments=101`）。
2. **LG-19 只观察到了症状的一半**：外链**可读**时，预览回复里没有任何 `outside*` 行（diff 两侧都经链接
   读到同一份字节，判为未变），所以「包外数据被拉进副本」这件事**不能靠 diff 观测**；只有让包外文件
   不可读（我的做法）或在失败残留里才看得见。报告举的自指链接之所以看得见，是因为它改变了路径结构。
3. **LG-20 的清理缺口比报告写的更宽**：报告说「失败会留残留」（以 t12 的 `diff` 失败为例）；
   我实测 `copy_package` 自身失败（包外文件 permission denied）也留残留——`apply.rs:84-88` 的 `?`
   在 `remove_copy`（`:144`）之前，这条路径连清理代码都到不了。
4. **LG-21 的症状比报告写的更重**：报告说 edit 后「文件只有 `handle_traits`」；我的两跳探针得到的是
   `handle_traits: ["Qux"]`（新契约派生）与 `handle_contracts: [crate::Foo::Bar]`（旧值）**并存且矛盾**。
   成因链条要在「`EDIT_FIELD_ORDER` 少两个字段」之上再加一环：traits 是**从 contracts 推导**的，
   推导读内存新值、写出读字段序旧值。
5. **LG-25 的黑名单只堵一种拼法**：报告只举 `== false`；我另测出 `== 0`、`! startsWith(...)`（`!` 后有空格）、
   `!(...)` 三种**等价否定**同样 0 发现。补黑名单的错误方向在总账自己的 §5.2 里已经写明，但 LG-25 的
   修法第一句仍写「至少拒绝 TAG_GUARD 后紧跟 `== false`/`= false`/`== 0`」——那仍然是黑名单思维。
6. **两处计数/引用不精确**：① 交付摘要说「23 MAJOR」，而总表实际有 **29** 条 MAJOR 行（LG-04…LG-32）；
   ② §5.1 第 23 行把 LG-11 与 `face_view.rs:222-231` 的引用配对，但那几行的注释讲的是
   `registry_name` 的**默认取值**（「命令若猜成别的，就会打印出运行期从未有过的逻辑路径」），
   不是「解析失败时子面回退到 root」——LG-11 的行为我实测成立，只是那句引文指的不是同一件事。
   ③ LG-29 的 `entries/graft.rs` 缺目录前缀（真实路径在 kernel 的 `core/src/registry_core/syntax/entries/graft.rs`）。

## 4. 作者漏项（不在总账的 CRITICAL/MAJOR 清单里）

从片区报告对照总表逐 id 查（`grep -c`）：**studio 片区有 5 条 MAJOR 完全没进总账**——

| 漏项 | 片区报告的严重度与位置 | 总账里出现的次数 |
| --- | --- | --- |
| `S-02` | MAJOR，`app/lifecycle.rs:152-158` × `ui/panels.rs:119-159`（检视器行数两处真值 → 光标可越出列表） | 0 |
| `S-03` | MAJOR，`app/lifecycle.rs:108-183`（reload/hot reload 不重校验已装入的 trace/MIR） | 0 |
| `S-04` | MAJOR，`ui/ui.rs:66` 等（渲染期改写 App） | 0 |
| `S-05` | MAJOR，`app/search_queries.rs:18-136`（列表模式每帧至少三次全量重扫） | 0 |
| `S-06` | MAJOR，`app/search_queries.rs:74-133`（手写第二套符号词法器） | 0 |

另有两处**合并时丢了严重度/没交代**：

- `S-07`：studio 片区评 **MAJOR**（`app/graft.rs:127-158` 错误吞掉），总账把它与 `S-24` 合并成
  LG-50 一行并标 **MINOR**，没有写「降级理由」。按总账自己的规则（重复只留一条、严重度取最高），
  这一行应为 MAJOR 或至少写明为何降。
- `S-30`（node-editor 夹具的存在方式，设计问题）在总账里 0 次引用；它不是缺陷、可以只留在片区报告里，
  但既然队长专门转达过，建议在总账的附录里记一行归属。

**MINOR 侧的完整性**（不影响 CRITICAL/MAJOR 结论）：我 t5 的 `BR-9`（可移植路径两种拼法）、
`BR-11`（测试挂载方式）、`BR-12`（目录顺序 ≠ 分派顺序且无钉子）、`BR-15`（中文 CLI README 少
`snippets` 行）、`BR-17`（`u32` 帧宽那条消息报错上限）都没有出现在总账的 §2/§5/§6 任何表里。
它们都是 MINOR，但总账那节的标题自称收纳「其余一致性/错误上下文/自述欠说明条目」。

## 5. 我**没有**独立复现的部分（如实列出，供下一轮别把它们当已证实前提）

- **症状未复现、机制已读码闭环**：LG-06（叶子仍可注册）、LG-09（混代产物被读成 current）、
  LG-16（宿主真的叫 `compile_error_demo` 时的构建差异）。
- **采信作者/其他复核的实测数字**（我未跑）：LG-01 的六序矩阵与 unit 文件内容对照（我的装置另证了
  冷/热两态）、LG-19 的自指链接 `faces 124`、LG-31 的 RSS 增长、LG-13 的 t12 `reidentified 3` 探针、
  LG-07/29/30 的 t9 探针。
- **只能读码的**：LG-03 的 **TUI 写盘动作**本身（核侧拒绝我用门禁函数跑了）、LG-14/15/17/27/28/29 的
  文字与分支、LG-25/26 的 YAML 规则（我用门禁函数跑了，没构造完整 workflow 文件）。

## 6. 对总表的整体判断与两条口径建议

- **整体**：32 条 CRITICAL/MAJOR **无一被推翻**；28 条我用与作者不同的装置第一手证实，
  4 条部分证实（缺的是症状注入复现，机制闭环），0 条证伪。总账的严重度定级我逐条对照后**全部维持**，
  只有 LG-16 建议标注「潜伏触发」，LG-05 建议把「修法」改掉。
- **建议一**：把 LG-05 的修法改成两段——「接 `guard_nesting`（挡 129 层以上）」+
  「在 `run_method` 的编辑入口文档里写明：解析在调用方线程的栈上发生，宿主需自备足够栈；
  或把解析放进固定栈的专用线程」。否则按现修法改完，我这份 301 字节输入仍会打死小栈宿主。
- **建议二**：LG-25 的「最小修复方向」删掉黑名单那半句，只留「只允许肯定式形状」——我实测的四种
  等价否定拼法说明黑名单永远落后一步；这也是总账 §5.2 自己立的标准（判定取结构化位置，不取子串）。
