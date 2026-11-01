# 独立验证：B4-conventions（t38 的三条门禁修复：LG-24 / LG-26 / LG-53）

- 验证者：**boundary-architect（t71，非作者）**。作者：`LGC-LG-24`/`26`/`53` 的修复由 **run-engineer** 在 t38 落地；t38 的第一次尝试是 gates-auditor（只逐条核实、未改动、报 failed）。
- 日期：2026-09-28。**真实树零改动**：本轮只写本文件；所有变异、探针、夹具都在 `/tmp`。
- 装置：工作树副本 `/tmp/t71/base`（47M，`tar` 自真实树，排除 `target/`/`.git/`）+ 参照副本 `/tmp/t71/pristine` + **自带 `CARGO_TARGET_DIR=/tmp/t71/target`**（另开 `/tmp/t71/target-n3` 用于一次需要干净重建的探针）+ 我自写的 rustc/clippy 前提探针 7 条 + **10 组变异**。
- 全部变异在副本内做，逐组还原；每组后用 `diff -r -q --exclude=target /tmp/t71/pristine /tmp/t71/base` 复检，**最终零残留**。

## 0. 判定一览

| 条目 | 判定 | 一句话 |
| --- | --- | --- |
| `LGC-LG-24` 去掉 `#![deny(warnings)]` 特例 | **证实**（含前提复测） | 把某 crate 根的 `#![warn(missing_docs)]` 换成 `#![deny(warnings)]` ⇒ `the_published_roots_keep_the_lint_on` **红并点名**该文件；把特例加回门禁 ⇒ **同一棵树转绿**（洞复现）。前提：我用**自己**的 rustc 与 cargo-clippy 两条不同调用复测 `#![deny(warnings)]`+未文档化 `pub fn` ⇒ exit 0、零诊断（§3） |
| `LGC-LG-26` 新增 token-job action pin 门禁 | **证实（两半）** | 把真实 `release.yml` 的 checkout SHA 改成 `@v4`（及 7 位短 SHA）⇒ 门禁**红**且点名 job、action、实际 ref、40 位规则；把该 job 的 token 名字拿掉后同一个 `@v4` **转绿** ⇒ 规则真是 **job 作用域**（`ci.yml` 的 tag pin 不受影响） |
| `LGC-LG-53` 豁免改为「`root` 下任一路径分量前缀」 | **证实（两半）** | 在 `docs/audit-2026-09-28/` 放一个**名字不以 `audit-` 开头**的坏围栏报告 ⇒ 仍**豁免**（绿）；把规则改回「只看文件名」⇒ 同一文件**被报**（红）。另测 `strip_prefix(root)` 那一半：副本路径含 `design-base` 分量时活文档仍被覆盖（红），去掉 `strip_prefix(root)` ⇒ 整棵检出被豁免（绿=静默大洞） |
| **N-1**（新发现·边界） | 记录 | `#![cfg_attr(all(), warn(missing_docs))]` 让 lint 实际打开，但门禁报该根「缺属性」。方向保守（要求直写形式），且不评估 cfg 才能挡住 `cfg_attr(any(), …)` 这种「声称开启、其实关闭」；建议记为已知边界 |
| **N-2**（新发现·衰减风险） | 记录 | `conventions/src/release_workflow.rs` 现 **599/600** 行（作者也在 t38 报告里自陈）：下一次一行改动就撞棘轮 |
| **N-3**（新发现·假阴性） | 记录 | 新版 pin 规则的「持有者」判据是**字面量名字**：一个用 `secrets: inherit` 的**可复用工作流调用 job**（合法形态）真的持有 token，却对门禁不可见 ⇒ 它的 `uses: …@v1` 不被报。今天出厂 `release.yml` 是显式点名（保护成立），但这是可达的绕过路径（§2.2 有对照） |

作者主张与我的复测**无冲突**：三条修复都成立、都真的有牙、都不误报；作者自陈的 `release_workflow.rs` 599/600 我核实无误；作者提到的 `docs/notes.md`/`docs/audit-probe.md` 对照文件在树上已不存在（那是作者当时的临时探针），我用自建探针重做了同样的对照。

## 1. 装置与变异（10 组，判定只取 hash 稳定那次）

| 变异（副本内） | 我自己跑出来的红侧 | 还原 |
| --- | --- | --- |
| M1a `debug_method/src/lib.rs` 的 `#![warn(missing_docs)]` → `#![deny(warnings)]` | `lint::tests::the_published_roots_keep_the_lint_on` **FAILED**，`[\"debug_method/src/lib.rs\", …]` | byte-identical |
| M1b 门禁里把 `#![deny(warnings)]` 特例加回（同一棵 M1a 树） | **同上测试转 ok**（这就是那条被修掉的洞） | byte-identical |
| M2 `debug_method/src/lib.rs` 末加 `#[allow(missing_docs)] pub struct ProbeItem;` | `lint::tests::no_item_silences_the_lint` **FAILED**，`\"debug_method/src/lib.rs:43\"` | byte-identical |
| M3a `release.yml` 的 `actions/checkout@11d5960…` → `@v4` | `release_action_pin::…::the_shipped_workflows_hold_the_rule` **FAILED**：`job \`release\` holds \`CARGO_REGISTRY_TOKEN\`, so \`actions/checkout\` must be pinned by a 40-character commit SHA; it is \`v4\`` | byte-identical |
| M3b 同上 `@v4`，但把该 job 的 token 名换掉（`NICH_PUBLISH_TOKEN` + `secrets.NICH_PUBLISH_TOKEN`） | **ok**——同一处 tag pin 不再被报 ⇒ job 作用域属实 | byte-identical |
| M3c 把 pin 改成 7 位短 SHA `@11d5960` | **FAILED**，消息里 `it is \`11d5960\`` ⇒ 只有**恰好 40 位**十六进制算 pin | byte-identical |
| M4a 在 `docs/audit-2026-09-28/` 放 `zz-probe-not-audit-named.md`（坏围栏） | **ok**（13 条 doc_blocks 测试全绿，含 `documented_rust_blocks_parse`、`the_exemption_does_not_swallow_living_documents`） | 探针删除 |
| M4b 同文件 + `is_record` 改回「只看文件名」 | `documented_rust_blocks_parse` **FAILED**：`file: \"docs/audit-2026-09-28/zz-probe-not-audit-named.md\"` | byte-identical |
| M4c 同一个坏围栏放到 `docs/zz-lg53-probe.md`（记录目录之外） | **FAILED**：`file: \"docs/zz-lg53-probe.md\"` ⇒ 豁免是边界，不是一刀切 | 探针删除 |
| P-strip 副本路径含 `design-base` 分量 + `docs/zz-probe-design-path.md` | 出厂码 **FAILED** 并点名该文件（`cannot parse string into token stream`）；把 `strip_prefix(root)` 去掉后**转 ok**（整棵检出被豁免） | byte-identical + 探针删除 |

**一次装置自纠（记账）**：N-3 的前两次尝试给出的「绿」是**无效装置**，不是结论——第一次我把新 job 插在 `jobs:` 映射**之外**（`release.yml` 的 `permissions:`/`concurrency:` 在 `jobs:` 之前，我插到了 yaml 末尾），解析器根本看不到它；第二次的形状 `secrets: inherit` 配 `steps:` 在 GitHub 里不合法（`secrets: inherit` 只属于**可复用工作流调用** job）。改到合法形态并放回 `jobs:` 内部后才得到 §2.2 的结论。

## 2. 逐条验证

### 2.1 `LGC-LG-24`：`carries_the_lint` 不再接受 `#![deny(warnings)]`

**读到的出厂码**（`conventions/src/lint.rs:215` 起的 `carries_the_lint`，接受分支在 `:231`）：只接受 `#![warn(…missing_docs…)` / `#![deny(…missing_docs…)`，且属性必须出现在第一个条目之前；旧的特例（`trimmed == "#![deny(warnings)]"`）已删，删除理由写在 `:247` 起的注释里。

- **有牙**：M1a（crate 根的属性被换成 `#![deny(warnings)]`）⇒ 门禁红并**点名** `debug_method/src/lib.rs`；M2（`#[allow(missing_docs)]`）⇒ 红并点名 `debug_method/src/lib.rs:43`。两条消息都给了「改为补文档」的行动指引。
- **不误报**：副本基线 131 passed；真树上 `the_published_roots_keep_the_lint_on` 与 `no_item_silences_the_lint` 均绿（九个已发布库根 + `conventions` 自己 + `mcp/main.rs`）。
- **变异反证**：M1b——把特例加回门禁，**同一棵坏树**立刻转绿 ⇒ 删掉特例正是这条门禁的牙。

### 2.2 `LGC-LG-26`：新的 token-job pin 门禁

**读到的出厂码**：`release_action_pin.rs` 由 `release_workflow.rs:116` 调用；`jobs()` 按缩进切出 job 正文，`unpinned_actions_where_the_token_is` 要求「**非注释**行里出现 `CARGO_REGISTRY_TOKEN`」的 job 内每个远程 `uses:` 是 40 位十六进制；`./`、`docker://`、`@<sha> # v4.4.0` 尾注释均不误判。

- **有牙**：M3a（`@v4`）、M3c（7 位短 SHA）⇒ 红，消息含 job 名、action 名、实际 ref 与「40 位」规则。
- **不误报**：出厂 `release.yml`（两个 action 都是 40 位 SHA）与 `ci.yml`（tag pin、无 token）在基线与真树都绿；M3b 证明同一条 `@v4` 在没有 token 的 job 上不被报 ⇒ **job 作用域**是真实的，而不是碰巧。
- **N-3 假阴性（合法形态，见 §0 与 §1 的自纠）**：把一个新的**可复用工作流调用** job 放进 `jobs:` 内部：

  ```yaml
    relay:
      uses: octo/relay/.github/workflows/publish.yml@v1
      secrets: inherit
  ```

  门禁 **ok**（漏报）；把 `secrets: inherit` 换成显式映射点名 `CARGO_REGISTRY_TOKEN` ⇒ **FAILED**，消息 `job \`relay\` holds \`CARGO_REGISTRY_TOKEN\`, so \`octo/relay/.github/workflows/publish.yml\` must be pinned by a 40-character commit SHA; it is \`v1\``。
  判据是**字面量名字**，而 `secrets: inherit`（以及任何不点名 token 的间接获得方式）不在其中。「谁持有 token」这件事在 YAML 层面上无法完全判定，但可枚举的两种合法间接形态至少应当被覆盖：`secrets: inherit`，以及把 registry secret 通过 `jobs.<id>.secrets` 之外的别名传递。今天出厂工作流是显式点名，所以**当前保护成立**；这是未来编辑可达的绕过路径。

### 2.3 `LGC-LG-53`：豁免从「文件名前缀」改为「`root` 之下任一路径分量前缀」

**读到的出厂码**（`conventions/src/doc_blocks.rs:205` 的 `is_record`）：先 `path.strip_prefix(root)`，再对**每个分量**（文件名与各级目录）测 `RECORD_PREFIXES = [\"audit\", \"design\"]` 前缀。

- **有牙**：M4c（坏围栏放到记录目录之外）⇒ 红并点名；P-strip（副本路径含 `design-base` 分量 + 活文档坏围栏）⇒ 红并点名 ⇒ 检出自己放在 `design-*`/`audit-*` 目录下**不会**把整棵检出豁免。
- **不误报**：M4a（记录目录内、名字不以 `audit-` 开头的报告）⇒ 绿（13 条 doc_blocks 全绿）。
- **变异反证**：M4b（改回只看文件名）⇒ M4a 的文件被报（红）；P-strip 反证（去掉 `strip_prefix(root)`）⇒ 整棵检出被豁免（绿=大洞）⇒ 两半都不可省。

## 3. `LGC-LG-24` 的前提复测（我自己的装置，不同调用方式）

工作区 rustc：`rustc 1.96.0 (ac68faa20 2026-05-25)`。探针在 `/tmp/t71/premise`（最小单文件 crate + 一个最小 cargo crate）。

| 探针 | 源 | 调用 | exit | `missing documentation` 诊断数 |
| --- | --- | --- | --- | --- |
| P1 | `#![deny(warnings)]` + `pub fn undocumented() {}` | `rustc --edition 2021 --crate-type lib -D warnings` | **0** | **0** |
| P2 | `#![warn(missing_docs)]` + 同函数 | 同上 | 1 | 2 |
| P3 | `#![deny(missing_docs)]` + 同函数 | `rustc --crate-type lib` | 1 | 2 |
| P4 | `#![warn(missing_docs)]` + `#![deny(warnings)]` | `rustc --crate-type lib` | 1 | 2 |
| P5 | `#![deny(warnings)]` + 同函数 | `rustc -D warnings -W missing_docs` | 1 | 2 |
| C1 | `#![deny(warnings)]` + 同函数 | `cargo clippy --offline -- -D warnings` | **0** | **0** |
| C2 | `#![warn(missing_docs)]` + 同函数 | 同上 | 101 | 2 |

**结论：前提成立**——`missing_docs` 默认 `allow`、不属于 `warnings` 组，因此 `#![deny(warnings)]`（乃至 `-D warnings`）**一句诊断都不产生**；它只有在被点名（P2/P3）、被显式打开（P5）或与 `#![warn(missing_docs)]` 同时存在（P4）时才生效。作者那条修复的理由（「仅仅声称强度的声明不能当作该属性」）**无需重写**。

## 4. 未覆盖（如实）

- **YAML 不是真解析器**：门禁是手写的按行/按缩进扫描。我**没有**测：多文档 YAML、`uses:` 出现在 `with:` 字符串里、`matrix` 展开、锚点/别名（`&`/`*`）、`jobs.<id>.environment` 之类的间接。N-3 只覆盖了 `secrets: inherit` 这一种合法间接。
- **`LG-24` 的其余拼法**：我复测了「换成 `#![deny(warnings)]`」与「`#[allow(missing_docs)]`」两条主路径；作者自己的钉子覆盖了折行属性、`#[allow (missing_docs)]`、`#![allow(missing_docs)]`、同行第二个属性、`cfg_attr(all(), allow(missing_docs))`、模块内属性。我**没有**独立重做这些形状，只跑了作者的钉子（在 131 条里）。
- **`LG-53` 的其余形状**：我未测大小写（`Audit-…`）、符号链接文档、`docs/` 之外的深层记录目录（作者的钉子覆盖了嵌套）。
- **我用的观测装置是作者的门禁测试本身**（那是被验对象，也只能如此），所以我的独立性体现在**输入**上：变异与探针都是我自己造的，作者的钉子只作为附带证据被跑到。我没有另写一个「门禁探针 crate」（t34 那种风格）。
- **`release_workflow.rs` 的 599/600 只做了行数核实**，没有做「再加一行会怎样」的实验（那需要改源码，超出本单范围）。

## 5. 门禁与并发（静置后的真树，五条命令）

| 命令 | 结果 |
| --- | --- |
| `cargo test -p xirang-conventions --offline` | **exit 0，131 passed / 0 failed** |
| `cargo test --workspace --offline` | **exit 0，934 passed / 0 failed**（本机未出现 ENV-1 那条 studio 离线用例，故无需豁免） |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | **exit 0**，无输出 |
| `cargo fmt --all -- --check` | **exit 0**，无 Diff |
| `tools/xirang-publish --check-table` | **exit 0** |

副本基线（同一套装置）：131 passed / 0 failed，exit 0。本轮未遇并发红（t38 报告里的并发写者 `run_method/authoring/manifest/face/render.rs` 640 行已在 t43 拆走，现在 370 行）。

## 6. 结论

三条修复**全部证实**：`LG-24` 有牙且前提经我独立复测成立；`LG-26` 的规则真的 job 作用域（有牙 + 不误报），并留下一条可枚举的假阴性（N-3）；`LG-53` 的豁免边界两半都成立（目录分量前缀 + `strip_prefix(root)`），且不会一刀切。三条新发现都是**MINOR**：N-1 是保守方向的边界（建议写明），N-2 是衰减风险（下一次改动前宜拆），N-3 是未来编辑可达的绕过路径（建议在 `unpinned_actions_where_the_token_is` 里把 `secrets: inherit` 也算「持有 token」）。
