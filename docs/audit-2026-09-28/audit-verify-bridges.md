# 独立复核（verify-bridges）：D 路 `lane-bridges` 的 20 条 + 注释轴 7 条

- 复核对象：`docs/audit-2026-09-28/audit-lane-bridges.md`（作者：bridge-auditor，t5）。
- 复核者：logic-adversary（t12）。**本轮只出报告，源码一行未改**；写权限只落在本文件。
  文件名按队长 2026-09-28 规则用 `audit-` 前缀（任务书写的是 `verify-bridges.md`，同批的 studio 复核文件为 `audit-verify-studio.md`）。
- 对象工作树：`HEAD=cf0c378`（120 项未提交改动）。
- **手段与作者不同**：作者以「逐文件读源码 + 只读 grep/计数」为主（其报告自述未跑 `cargo`）。本次复核以**可执行的独立探针**为主：
  1. 自建 MCP stdio 驱动（`/tmp/nichverify/harness.py`）——**原始字节**喂 stdin、读 `tools/list` 的**客户端可见 schema**、读 stdout/stderr/退出码；
  2. 自建 `/tmp` fixture 包（从 `examples/control-button/src` 复制 + 手写 `Cargo.toml`），在其上跑 `xirang.apply`/`verify`/`diff`/`search`/`usages`/`callgraph` 的**端到端**调用；
  3. 自建三个 `/tmp` 探针 crate（`path` 依赖本检出）：`wasmprobe`（plugin-host wasm）、`procprobe`（plugin-host process-tools）、`harness.py`；
  4. CLI 直接跑 argv 矩阵并记录**退出码**。
  未重跑作者的任何脚本（作者未提供脚本）；作者的 grep 判据一律换成「活体回复或活体 schema」再判一次。
- 每条给出：**结论 → 证据（命令 / exit code / 关键输出行）→ 严重度是否改变 → 备注**。只读代码的显式标「代码阅读佐证」。

## 汇总

| 原 id | 作者严重度 | 复核结论 | 严重度 |
| --- | --- | --- | --- |
| BR-1 | MAJOR | **证实** | 不变 |
| BR-2 | MAJOR | **证实**（引用失效也成立） | 不变 |
| BR-3 | MAJOR | **证实**（活体 schema + 活体回复自相矛盾） | 不变（偏上限的 MAJOR） |
| BR-4 | MAJOR | **证实**（端到端复现：两个非 UTF-8 文件被报成新增） | 不变 |
| BR-5 | MAJOR | **证实**（exit 1 + 0 行 stdout，三类同类畸形帧均被作答） | 不变 |
| BR-6 | MAJOR | **证实**（活体复现：`reidentified 3` + `build current`） | 不变（可争 CRITICAL，见备注） |
| BR-C1 | MAJOR | **证实**（探针：`Err(Timeout) after 303ms`） | 不变 |
| BR-C2 | MAJOR | **证实**（`.git` 被复制、被列举） | 不变 |
| BR-7/8/9/10/11/12/13/14/15/16/17/18/20 | MINOR | **证实**（逐条见下） | BR-10 建议升 MAJOR（分歧半）、BR-13 修正为**五种**下场 |
| BR-19 | MINOR | **部分证实**：卫生面证实；作者给的**符号链接机制证伪**；泄漏面换成可复现的另一条路径 | 不变（描述要改） |
| BR-C3/C4/C5 | MINOR | **证实** | 不变 |
| BR-C6/C7 | MINOR | **无法判定/保留**（纯结构性建议，无独立可跑判据） | 不变 |
| —（新增） | — | **NEW-B1**：`apply edit` 静默忽略 `handle_contracts`/`part_contracts`，而 `EDITABLE_FIELDS` 与工具自述都把它们列为可写 | MAJOR（新增） |
| —（复核队长转达） | MINOR | **V-03 证伪**：Studio Edit 不会抹掉 contract 字段（机制见下）；真问题转为 NEW-B1 | V-03 应撤下「数据丢失」定性 |

MINOR 抽检覆盖率：14 条主 MINOR 中复核 13 条、注释轴 5 条全部复核（远超 1/3）。

---

## CRITICAL / MAJOR 逐条

### BR-1 —— 两份 mcp README 仍承诺已删的 `unmatched` 桶 → **证实**

- **证据**：`sed -n '40,50p' mcp/README.md` 第 47 行原文 `stale`, `unmatched` (a typed cut stores an expression, …；`sed -n '30,40p' mcp/README.zh-CN.md` 第 35 行同义。活体回复（`/tmp/nichverify/harness.py` 调 `xirang.diff {"records":true}`）的计数行是
  `ok 0  undeclared 0  stale 0  re-identified 1  unreadable 0`（本文件 BR-3 复现命令同一驱动），**没有 `unmatched`**。
  `grep -rn unmatched mcp/src` → 仅 `mcp/diff_tests.rs:115/125/165`（负断言）与 `mcp/diff.rs:179/188`（删除说明）。
- **严重度**：不变（MAJOR，文档以现在时承诺一个不存在的桶）。
- **备注**：作者引的 `docs/audit-3p-2026-09-27.md:74` 我也核对了：上一轮只同步了 `tools.rs`，两份 README 未同步。修法照作者即可。

### BR-2 —— mcp README 自相矛盾（「树 diff 仍待做」 vs 120 行前的 `xirang.diff`）→ **证实**

- **证据**：`sed -n '160,170p' mcp/README.md` 第 164-166 行 `Graft writes, plugins, project scaffolding, tree diffs, and the consistency analysis are still to come (docs/roadmap-1.0.md item 7)`；同文件 `:41-49` 整段描述 `xirang.diff`（含 `records: true`）。
  引用是否失效我独立查了 `docs/roadmap-1.0.md` 的**三张编号清单**：「下一批」第 7 条 = `C19:诊断字段表`（`docs/roadmap-1.0.md:46`，✅ 已完成）；第四轮第 7 条 = `petgraph 0.6 → 0.8.3`（`:379`，在「已修」节）；「未做，等你定」是**无序**列表（`:415` 起）。三者没有一条把「graft 写入/插件/脚手架/树 diff」列为待做 → 作者的「引用也已失效」成立。
- **严重度**：不变。**备注**：这条是「同一份文件内自相矛盾」，比单纯过期更硬，建议按作者的第一种修法直接删句。

### BR-3 —— `xirang.callgraph` 读未声明的 `limit`（回复又叫调用方 raise 它）→ **证实**

- **证据**（活体 schema，客户端可见契约）：`tools/list` 返回的 `xirang.callgraph.inputSchema.properties` = `['function','path','root']`，`required=['function']` —— **没有 `limit`**。
  活体行为（同一驱动，仓库根作 root）：
  ```text
  limit=None → 5 条定义行 + "… +34 more definitions (raise `limit` or pass `path`)"
  limit=1    → 1 条 + "… +38 more"
  limit=10   → 10 条 + "… +29 more"
  limit=50   → 39 条（无截断行）
  ```
  即：未声明的键**确实改变答案长度**，而回复给出的行动建议（raise `limit`）按 schema 无法执行。
- **严重度**：不变（MAJOR；按本仓「自述必须与行为一致」的口径成立）。**备注**：这是「一个回合内自相矛盾」，比一般 schema 漂移更值得修；作者建议的「顺便把 `callgraph` 加进那张表」是正确的最小钉子。

### BR-4 —— 预览 diff 把每个非 UTF-8 文件报成「新增」→ **证实（端到端）**

- **装置**：`/tmp/nichverify/fix`（可解析 fixture）里放两个**两边完全相同**的非 UTF-8 文件：`binfile.dat`（22 B，`\xff` 开头）与 `.git/index`（`\xff` 开头）；另放 UTF-8 的 `.git/config`、`plain.txt` 作对照。然后调 `xirang.apply {"action":"add",…}`（默认预览）。
- **关键输出**（作者脚本之外的独立探针）：
  ```text
  action preview
  would write /tmp/nichverify/fix/src/probe_add/probe_add.rs
  diff:
  + .git/index          ← 从未被这次编辑碰过，被报成新增
  + binfile.dat         ← 同样，两边完全相同
  + src/probe_add/probe_add.rs   ← 唯一真实变化
  ```
  对照成立：UTF-8 且未变的 `.git/config`、`plain.txt` **没有**出现 —— 说明触发条件正是「读不出 ⇒ 当新增」，与作者判断一致。
  另：`.git/config` 能被比较（未变），**反证副本里含 `.git`**（见 BR-C2）。
- **严重度**：不变（MAJOR：一次没碰过该文件的编辑会让 agent 以为它会变；也是 `BR-C2` 与「无界回复」的来源）。
- **备注**：作者「本检出 36 个非 UTF-8 文件」的计数我未重跑脚本；但**机制**已由上面的最小 fixture 独立钉死，计数只是量级。修法照作者 ①（两侧原始字节相等即 continue）②（跳过 `.git`）③（加行数上限）。

### BR-5 —— 一帧非 UTF-8 请求结束整个桥 → **证实**

- **证据**（`harness.py` 原始字节，先喂一条含 `\xff\xfe` 的 `tools/list`，再喂一条合法 `ping`）：
  ```text
  exit code: 1
  stdout lines: 0
  stderr: xirang-mcp: cannot read stdin: request line is not valid UTF-8
  ```
  同类畸形帧的对照（同一驱动，各带一条后续 `ping`）：
  ```text
  too-long line     : exit=0 replies=2 first={"error":{"code":-32600,…"line exceeds 1048576 bytes"},"id":null}
  bad json          : exit=0 replies=2 first={"error":{"code":-32700,"message":"invalid JSON: …"},"id":null}
  non-object member : exit=0 replies=4 first={"error":{"code":-32600,…"not a JSON object"},"id":null}
  ```
  四类里只有非 UTF-8 这一类是致命的，且 `ping` 再也得不到回复 —— 与作者描述逐字一致；两份 README 的「不写 stderr」承诺（`mcp/README.md:7`、`README.zh-CN.md:5`）确实被这句 stderr 打破。
- **严重度**：不变（MAJOR：协议可靠性 + 明文承诺相反）。**备注**：这条的**修法**很便宜（把非 UTF-8 归入坏帧作答），作者给的钉子形状（坏行 + `ping`）可用；我已用同一形状独立验证过「当前会红」。

### BR-6 —— `XIRANG_NAMESPACE` 下 `verify` 与 `diff`/`search` 各自一套命名空间 → **证实（活体复现）**

- **证据**：fixture 先跑一次 `verify`（无覆盖名）→ `verdict ok`、`diff` 全零（`added 0 gone 0 reidentified 0`，`build current`）。
  随后**同一 fixture**带 `XIRANG_NAMESPACE=alternate-ns` 再跑：
  ```text
  VERIFY: verdict ok (the kernel accepted the tree)
  --- diff ---
  namespace alternate-ns
  build current
  faces 3 (source) vs 3 (build)
  added 0  gone 0  reidentified 3
  reidentified:
    ~ root/control 12df60c352563b8ca989a1a2116b02ae -> 8ea79f011cf05f035f835898232be993
  --- search Button ---
  face  root/control/button … [re-identified (371236fe8d53731db5ab0b9db8dfd46c -> df5b6a7e1b36157eb7e97faa6771a7f5)]
  ```
  这里 `12df60c352563b8ca989a1a2116b02ae` / `371236fe8d53731db5ab0b9db8dfd46c` 正是 **Cargo 名 `probe-host`** 下的身份（我在同期探针里独立算过该值），即**发布侧用了 Cargo 名、读取侧用了覆盖名**，而 `build current` 让这份错答案看起来是新鲜的。作者只有 `[代码]`，这里是活体实证。
- **严重度**：**不变（MAJOR）**，但注明：按本仓「静默错答=CRITICAL」的定义，把这条升为 CRITICAL 也站得住；我保持 MAJOR 的理由是触发需要宿主显式设 `XIRANG_NAMESPACE`（虽然桥自己在无分包根时就会建议设它）。**备注**：修法（`verify` 用 `registry::namespace(root)` 同一个名，或两者不一致时明确拒绝）与作者一致。

### BR-C1 —— stdout 线程注释把「已修的范围」说过头 → **证实**

- **证据**（我自己的 `procprobe`，非作者脚本）：插件「先写完整帧 `"answer"`，再 `sleep 3`」，`timeout=300ms`：
  ```text
  answers-then-lingers: Err(Timeout) after 303.152728ms
  ```
  而 `plugin-host/src/process.rs:341-350` 的注释说「只读一帧就停下曾是缺陷：……一个已经送达的答案被报成超时」——现在「作答后不退出」仍走到 `:404-406` 的 `return Err(HostError::Timeout)`，早于 `:421` 的 `let Some(bytes) = frame`。
- **严重度**：不变（MAJOR，属维护者新加的注释轴：注释承诺的行为不成立）。**备注**：与 `BR-20` 是同一条机制的两面，修一次即可。

### BR-C2 —— `preview.rs` 说「`target/` 是唯一可能很大的目录」→ **证实**

- **证据**：`sed -n '14,20p' mcp/preview.rs` 原文 `target/` **is the one directory that can be large（`target/` 是唯一可能很大的目录）`；而 `sed -n '43,48p'` 的跳过条件只有 `name == "target" || name == lexicon::XIRANG_DIR`。
  量化（我自己的命令）：`du -sh .git` → **9.9M**；`du -sh --exclude=target --exclude=.xirang .` → **47M**。
  行为侧由 BR-4 的探针补上：`.git/index` 出现在「新增」清单、`.git/config` 被逐字节比较 —— 只有 `.git` 被**整份复制**才可能发生。
- **严重度**：不变（MAJOR）。**备注**：作者把 BR-4 与 BR-C2 分成「行为」与「注释」两面是对的；落地时改跳过多一条 `.git` 并把这句注释改成清单式。

---

## MINOR 抽检（14 条抽 13 条 + 注释轴 5 条）

| id | 结论 | 我跑出来的证据（关键行） |
| --- | --- | --- |
| BR-7 | **证实** | 活体 `tools/list` 的 `xirang.usages` 描述只列到 `runtime checks`；活体 `xirang.usages`（对**生成面** `root/probe_add`）打印 `module / stable_name / getting_from_other_registry / flow_provider / needs_registry` 五项之外还在描述之外的标签。五项都在 `mcp/apply.rs:172-196` 的 `EDITABLE_FIELDS` 里（作者判据成立） |
| BR-8 | **证实** | `grep -c undeclared mcp/tools.rs` → **0**；`mcp/diff.rs` → **7**；活体 `diff records:true` 的计数行含 `undeclared 0` |
| BR-9 | **证实**（并补一处） | `mcp/index.rs:159` 用 `MAIN_SEPARATOR`；`mcp/preview.rs:85` **与 `:152`**、`mcp/converge_trace.rs:246` 无条件折 `'\\'`。作者只列了 `mcp/preview.rs:85`，实际该文件有两处 |
| BR-10 | **证实**（建议升 MAJOR） | 三个 `tokens()` 我逐个 `sed` 出来**逐字节相同**（impact/usages/converge）。语义分歧半：`mcp/impact.rs:248` 的 `if let Some((capability, provider)) = entry.split_once("=>")` 对裸能力名不匹配 → **不 push（静默丢）**；`mcp/converge.rs:197` 的 `let … else` 对同一输入走 `:201-205` 的 `push(RequirementVerdict { provider: "?".to_owned(), … })`（UNANSWERED 裁决）。同一份 `requires` 文本两个工具给出不同依赖结论 → 建议把「分歧」这半升 MAJOR（`impact` 少报一条依赖边＝爆炸半径假阴性）；作者判据写的是同一函数，我把两侧都读到了 |
| BR-11 | **证实** | `grep -rn "^mod tests {" mcp/src` → 恰好 `mcp/tools.rs:409`、`mcp/index.rs:219`；`#[path = "…_tests.rs"]` 的文件数 → **16**。作者的「16 + 2，`tools.rs` 两者兼有」精确成立 |
| BR-12 | **证实** | 我用脚本从 `tools()` 与 `match name` 各自抽名字：**集合相同、顺序不同**（`apply` 在目录第 6、分派最后；`grafts`/`impact` 次序互换）。另用**活体分派**逐个调 17 个工具：无一返回 `unknown tool`（即作者「两张名单齐全」的干净结论我独立复现） |
| BR-13 | **证实并修正为五种下场** | CLI argv 矩阵（`target/debug/xirang`，fixture 内，记录退出码）：裸调/`--help`/`-h`/`help` → exit 0 + 43 行 usage；`check|explain|grafts|snippets --help` → **exit 1**、stderr 1 行、stdout 0；`studio --help` → exit 0 + usage；`new --help` → exit 1；`bogus` → exit 1；**`build --help` → exit 0**（先跑注册校验并打印 `xirang build: registration ok (probe-host)`，再把 `--help` 交给 cargo，后者打印 `Compile a local package and all of its dependencies`）；**`mcp --help` → exit 0 且两流 0 字节**（桥把 `--help` 当空气，EOF 即干净退出）。作者说「四种下场」，实际至少**五种**（`mcp` 这一类他们漏了） |
| BR-14 | **证实** | `xirang new probe2 --path /home/nich/Moirai_N3/nichlink --git https://example.invalid/repo` → exit 0、`created binary project at /tmp/nichverify/probe2`，生成的 `Cargo.toml` 里只有 `path = "…/run_method"`，**没有任何 `git =` 键** → `--git` 被静默忽略。作者判据成立 |
| BR-15 | **证实** | 逐行解析两份表格：`cli/README.md` **9** 行（含 `snippets`），`cli/README.zh-CN.md` **8** 行（缺 `snippets`）；两份的 studio 行都是 `xirang studio`（缺 `[path]`） |
| BR-16 | **证实** | `procprobe --overflow`：`ProcessLimits { timeout: Duration::MAX }` 下一次 `call` → `panicked at library/std/src/time.rs:429: overflow when adding duration to instant`，**exit 101**。作者判据无误；MINOR 合适（触发需要荒谬配置，但公开字段 + 库内 panic 违背本 crate 的 `Result` 口径） |
| BR-17 | **证实（代码阅读佐证）** | `plugin-host/src/process.rs:296-302`：一个 `if` 两个条件（`> max_input_bytes` 或 `> u32::MAX`）共用一条消息，消息里报的是 `max_input_bytes`——落在两者之间时拒绝它的是**帧宽**。未构造 4 GiB 切片（与作者注一致） |
| BR-18 | **证实** | `wasmprobe`：装坏工件 → `activate_pending #1` → `Err("plugin ABI error: … 99 …")`、`is_loaded=Ok(false)`；`activate_pending #2` → **`Ok(false)`**（待定已被吃掉，无法重试）；对照：好工件 `activate_pending → Ok(true)`、`call echo → Ok("abc")`。持锁编译半：`plugin-host/src/lazy_wasm/slot_state.rs:75-105` 的 `pending.lock()` 作用域**包含** `backend.load(…).and_then(health_check)`（代码阅读佐证） |
| BR-20 | **证实** | 同 BR-C1 的探针（`Err(Timeout) after 303ms`，帧已完整送达）。作者说它 MINOR、BR-C1 是同一机制的注释面 —— 分法合理 |
| BR-19 | **部分证实（并改写机制）** | 见下 |
| BR-C3 | **证实** | `sed -n '8,32p' mcp/callgraph.rs`：模块文档 `mcp/callgraph.rs:10` 说 **142**，函数内注释 `mcp/callgraph.rs:25` 说 **151**，指的是同一件实测（`new` 定义数、4.5 MB 回复） |
| BR-C4 | **证实** | `cli/lib.rs:169` 与 `:175`（英/中两半）都写 `nihlink build`；`grep -rn nihlink cli/src mcp/src plugin-host/src` 只有这两行 |
| BR-C5 | **证实** | `reidentified`（`mcp/diff.rs:100`、`:117`）对 `re-identified`（`mcp/diff.rs:207` 等、`mcp/search.rs:140`）；`slot` 三义：`mcp/evidence.rs:112` 打印的是面的 `registry_name`，`mcp/overlay.rs:117` 是 graft 切口槽位，`plugin-host/src/lazy_wasm.rs` 是插件槽 |
| BR-C6/C7 | **无法判定（保留）** | 两条都是结构性建议（具名结构体、类型承载不变量），没有可独立执行的判据；我读了两处引用的确存在（`mcp/evidence.rs:40-55` 的三元组被 `mcp/overlay.rs:56` 丢弃第三项；`mcp/apply.rs:94-127` 的路径改写），但不构成「行为错误」，维持作者的 MINOR 与「建议」定性 |

### BR-19 的更正（重要）：卫生面成立，但**符号链接机制证伪**，泄漏面换一条可复现路径

- **装置**（全在 `/tmp`，用 `exec` 保住 pid，使预测路径可提前构造）：`/tmp/nichverify/victim` 作为受害目录；wrapper 先 `ln -sfn …/victim /tmp/xirang-mcp-preview-$$-0` 再 `exec xirang-mcp`，于是目标路径正是进程会用的那条。
- **结果 1（符号链接）**：预览正常完成，`victim/` **未被写入**（只有我放的 `marker.txt`，没有 `src/`）。原因是 `mcp/preview.rs:27` 在 `create_dir_all` 之前先 `let _ = std::fs::remove_dir_all(&destination);` —— 它把**符号链接本身**删掉了，而不是顺着它写。作者设想的「顺着链接把副本写进目标目录、`remove_copy` 再删掉目标」没有发生 → **该机制证伪**。
- **结果 2（同名真实目录）= 真正的后果**：预先把**同名真实目录**（内含 `important.txt`）放在预测路径上，再跑一次预览：
  ```text
  pre-created real dir with marker: /tmp/xirang-mcp-preview-415521-0
  predicted path was: /tmp/xirang-mcp-preview-415521-0
  GONE after the run (pre-existing data destroyed)
  ```
  即：可预测路径 + 无排他创建 + 先删后建 = **任意同名既有目录会被静默删除**（同一台机器上的另一个用户放的同名目录亦然，前提是进程对该路径有写权限）。这比「写进别人目录」更需要写进报告。
- **结果 3（泄漏）**：作者指认的路径是 `mcp/apply.rs:139-143` 的 `diff_package(…)?` 早于 `:144` 的 `remove_copy`。我**没能构造出**「copy 成功而 diff 失败」的输入（目录不可读会让 copy 先失败；两棵树由同一次复制产生），因此该具体路径标**无法判定**。但同一类泄漏**另有可复现路径**：`copy_package` 自身失败时 `mcp/apply.rs:87` 的 `?` 在 `remove_copy` 可达之前就返回 —— 用 fixture 里一个 `chmod 000` 的文件复现：
  ```text
  cannot copy /tmp/nichverify/fix2/unreadable.txt: Permission denied (os error 13)
  /tmp/xirang-mcp-preview-416016-0     ← 半份副本留在 /tmp（find 到 .git/config、.git/index、binfile.dat…）
  ```
  （本轮另见一条更早的残留 `/tmp/xirang-mcp-preview-343377-0`，来自 t8 的同款探针。）
- **严重度**：不变（MINOR，本地前提）；**备注**：修法照作者 ①`Drop` 守卫 ②排他创建 + 0700；报告文字建议改成「可预测路径会被**删除并重建**；失败路径（`copy_package` 出错）会永久留下半份工程源码副本」，把符号链接那半删掉。

---

## 作者漏掉 / 我顺手发现的同类问题

### NEW-B1（建议 MAJOR）—— `xirang.apply edit` 静默忽略 `handle_contracts` / `part_contracts`，而 `EDITABLE_FIELDS` 与工具自述都把它们列为可写

- **位置**：`mcp/apply.rs:172-196`（`EDITABLE_FIELDS` 含 `handle_contracts`/`part_contracts`）与工具自述「`edit`（change the `fields` the request names and keep the rest）」；实现 `run_method/authoring/operations/face_write.rs:59-79`（`EDIT_FIELD_ORDER`）**不含**这两个字段与 `:114-127`（`handle_traits` 由 `apply_trait_contract(labels, contract_paths)` 处理）。
- **探针与输出**（同一 fixture，`action: add` 与 `action: edit` 各一次，都 `apply: true`）：
  - **add** `{module: probe_d, kind: ProbeD, handle_contracts: "crate::Foo::Bar"}` → 文件拿到
    `handle_traits: ["Bar"],` **与** `handle_contracts: [crate::Foo::Bar],` —— 键有效。
  - **edit** `{node: root/probe_c, handle_contracts: "crate::Foo::Bar"}`（对已存在的生成面）→ 回 `action apply / applied …`（**成功**），文件只拿到
    `handle_traits: ["Bar"],`，**没有 `handle_contracts:` 行** —— 键被接受但契约路径被丢弃。
- **判据**：`handle_contracts` 只出现在 `CREATE_FIELD_ORDER`（`:48-49`，`CREATE_FIELD_ORDER` 从 `:32` 起），不在 `EDIT_FIELD_ORDER`（`run_method/authoring/operations/face_write.rs:59-79`）；而 `mcp/apply.rs` 的字段白名单与自述把它当可编辑键。于是同一个键在同一个工具的 add 与 edit 里语义不同，edit 侧是**静默降级**：作者要的是「参与编译检查的契约」，拿到的是「未经检查的标签」——正好违反本 crate 自己声明的规则（`run_method/authoring/manifest/face/render.rs:12-28` 对 `plugin:` 字段写着「响亮拒绝胜过静默删掉作者写的字段」）。
- **修复方向**：① 把两个字段加进 `EDIT_FIELD_ORDER`（`edit_optional(…, FaceWrite::Edit)` 对空值会清空，因此还要决定「空 ⇒ 清空」是否可接受）；或 ② 在 edit 路径上拒绝这两个键（`invalid_field` 增加按动作区分的白名单），并在工具自述里写明「contract 只在 `add` 上可设」。两条都要顺手给 `apply_tests` 加一条钉子。
- **备注**：这条与下面 V-03 是**同一根因的两面**，建议合并成一条修。

### NEW-B2（MINOR，对 BR-13 的补充）—— `xirang mcp --help` 是一种「安静的成功」

`mcp --help` → exit 0、stdout/stderr 全空：桥把 `--help` 当普通参数吞掉，按 stdio 服务器启动，遇 EOF 干净退出。于是「`--help` 至少应该告诉你它是 mcp」这件事在七个子命令里第五种下场。修法随 BR-13 的共享 `help()` 一起做。

---

## 独立复核队长转达的 V-03：**证伪（不是数据丢失）**

- **被复核的断言**：Studio Edit 表单用「读文件失败 ⇒ 空契约」填 `handle_contracts`/`part_contracts`，提交走全量重渲染，`run_method/authoring/manifest/face/render.rs:93-105` 空则不发行 → 改任意别的字段会把源文件里原有的两个 contract 字段**静默抹掉**。
- **我复核的两半**：
  1. **表单那一半成立**（代码阅读佐证）：`studio/studio/app/keyboard.rs:51-56` 确是
     `std::fs::read_to_string(source_path_for(&info.source.file)).ok().map(|s| declaration_contract_paths(&s)).unwrap_or_default()` —— 读失败与「没写 contract」在**表单显示**上不可区分。
  2. **提交那一半不成立**：`submit_edit`（`studio/studio/app/mutations.rs:152-189`）把表单值装进 `ModuleFacePatch` 后调用 `xirang_run_method::edit_module_face`，而该入口（`run_method/authoring/operations/operations.rs:318-332`）走的是 `apply_module_face_values(…, FaceWrite::Edit)` → **`EDIT_FIELD_ORDER`（`run_method/authoring/operations/face_write.rs:59-79`）里没有 `handle_contracts`/`part_contracts`**。这两个字段只在**创建**顺序里（`:48-49`，`CREATE_FIELD_ORDER` 从 `:32` 起）。编辑时它们**根本不被写入**，`FaceManifest` 里从源文件解析出来的值原样保留，`render.rs` 于是把它们原样重新发出。
     `handle_traits` 会经 `apply_trait_contract`（`:190-205`）处理：**两个都空就 `return Ok(())`（什么都不改）**；只有标签非空时才改写标签行，仍不碰契约行。
- **端到端探针（同一执行器，走 MCP）**：先给一个生成面手工加上
  `handle_contracts: [crate::ControlHandle],` 与 `part_contracts: […]`，再调
  `xirang.apply {action: edit, node: root/probe_add, fields:{name_en:"ProbeAddX", handle_contracts:"", part_contracts:""}, apply: true}`（刻意把两个键指名成空，模拟「读失败 ⇒ 空」的表单值）→ 回复成功，文件是：
  ```text
  name: { zh: "ProbeAdd", en: "ProbeAddX" },
  handle_traits: ["ControlHandle"],
  handle_contracts: [crate::ControlHandle],   ← 仍在
  part_traits: ["ControlHandle"],
  part_contracts: [crate::ControlHandle],     ← 仍在
  ```
  两个契约行**都没有被抹掉**。
- **结论**：**V-03 的「静默抹掉」证伪**。真实存在的只有「表单在文件读不到时显示为空」（显示歧义，无写后果），以及被这条断言遮住的**反向**缺陷 NEW-B1（Edit 路径根本写不了这两个字段，而 API 说可以）。
- **建议**：总账里把 V-03 从「数据丢失」降为「表单显示歧义（MINOR）」，并把 NEW-B1 作为同一处代码的真实缺陷记入；若作者仍坚持数据丢失，需要一个**能复现的探针**（我没有找到机制，因为 edit 字段序里没有这两个字段）。

---

## 对作者「查了、干净」的抽检

- **#1「17 个工具两处都在」** → 我用**活体分派**复核：遍历 `tools/list` 的 17 个名字各发一条 `tools/call {}`，无一返回 ``unknown tool``，也无空回复 → 成立（我的手段与作者的逐名比对不同）。顺序问题见 BR-12，不影响齐全性。
- **#5「读路径的根约束」** → 与我在 t8 的独立探针一致（`../../etc/passwd`、`/etc/passwd` 均被 `path must stay inside the configured source root` 拒绝；`load_one` 另要求 `.rs`）→ 成立。
- 其余 15 项本轮未逐条重跑；作者标注的 `[代码]`/`[实测]` 标签与我的抽检一致，未发现夸大。

## 复核者的证据清单与局限

- 新增/使用的装置（全部在 `/tmp`，不写仓库）：
  - `/tmp/nichverify/harness.py`：原始字节 MCP 驱动 + `tools/list` + 逐工具分派；
  - `/tmp/nichverify/fix`、`/tmp/nichverify/fix2`：可解析 fixture（含两个非 UTF-8 文件、`.git/`、若干生成面）；
  - `/tmp/xirang-mcp-preview-*`：BR-19 的残留与预测路径探针；
  - `/tmp/nichprobe/`：`procprobe`（process-tools）、`libleak`（t8 遗留）；`/tmp/nichverify/wasmprobe`：wasm 懒激活。
- 本轮**未做**：跨平台（Windows）验证；BR-9 只在 Linux 上成立两种拼法「在 Linux 上无差别、在 Windows 上有」这一点属代码阅读；`BR-6` 未在真实多包会话里长时间跑；作者「36 个非 UTF-8 文件」的计数未独立重跑（机制已由 fixture 钉死）。
- 纪律：未跑 `git checkout/restore/stash`，未跑 `cargo fmt`，所有 `cargo` 调用带 `--offline`；源码与 `.agent-teams/` 零改动；写权限只用于本文件。
