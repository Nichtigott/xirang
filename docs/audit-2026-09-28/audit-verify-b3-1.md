# B3 独立验证（t10）：桥的承诺面与内聚

本文件是**非作者**对 B3 两组修复的独立验证：t5（B3-1 桥的承诺面与文档）与 t6（B3-2 桥的内聚与类型化）。
验证者 surface-auditor 未参与这两条的任何改动；本文件是本次任务**唯一**的写入路径，未改动任何源码
（变异只在 `/tmp/t10/mut` 的检出副本里做，且每次还原后与仓库逐字节相同）。

## 0. 钉住的修订与装置

| 项 | 值 |
| --- | --- |
| `git rev-parse HEAD` | `a0ea4dbeffde85cc2b5bcda75556259320671ce0` |
| 工作树状态 hash（`git status --porcelain` 的 sha256，运行前后各一次） | `9b2aff62cb774246eb15c231869299fefa4e7d6ccebe7a1e573e294154e1626c`（**前后一致 ⇒ 五条门禁跑在静置的树上**） |
| mcp 源码 + 两份 README 的聚合 hash（前后各一次） | `21ab1eaeffff9e9e139393baccef168febcb9e9185ca56c2e8c3e12198e38fe2`（前后一致） |
| 被验证二进制 | `target/debug/xirang-mcp` sha256 `11c25224cb9050f15e8ab6bb333984c14130563432bc56c3363a397e13115bc1` |
| 探针 | `/tmp/t10/probe/{common,buckets,dispatch,apply_e2e,usages_labels}.py`（均为本次自写，不调用仓库里的测试辅助） |
| 变异副本 | `/tmp/t10/mut`（工作树副本，`CARGO_HOME=/tmp/t10/cargo-home`，`CARGO_TARGET_DIR=/tmp/t10/mut-target` ← **独立 target**，与工作区 `target/` 不共享） |
| 还原备份 | `/tmp/t10/orig/`（每个被变异文件一份，逐字节还原后比对 sha256） |

逐文件 hash（复核者可直接比对）：

```
ff5af699291a8d20276fbe9d69d0eacb040470005351a76d8ea86bd1590da1e0  mcp/diff.rs
5190c80f64d9b5c70513ef87288ad88dd90697d4906ae2458f776254f99a59a6  mcp/tools.rs
5124df53f30b6d9619cefe3b69febc3826b08bf5ab21f52047f8384618b28381  mcp/evidence.rs
aa0558f90a650e867b90a8900db35c631cc8f322a5aa3897875395b266863294  mcp/overlay.rs
f3dd654fe4620f2d09240097a988fdeacecaec798224a6a4a9a3a7ff33630d47  mcp/converge.rs
7ac8bd170d921d5434e144510282c44ceb9b062353a7243ddff94e56bf34a7d3  mcp/tree_delta.rs
3fbbf1471f7b5a6d5698e41b6ed9fcb45a9d80ae1d3d30df6dfb9ad37b86f190  mcp/index.rs
a556e00b3c7d53ae6bfbcdd8cddf70f4a566237a5c64c5fb632ac9560e61543b  mcp/index_tests.rs
e8300cc94ca26102a9b08cd7f37ba4e8c4edd2ea369b12a1bd4be5f4cbe6d546  mcp/usages.rs
ff0379892ea69eea4578935e2b9fb9e1df4fc22f4b14d32fb30bb594c23722c1  mcp/README.md
a5503bf61a9021532c238f88dc1f2ea68cc8c75d4b7c48ce3792a60b80f3457b  mcp/README.zh-CN.md
```

变异纪律（按队长提示）：每次变异前备份 → 就地改写 → `touch` → 比对 sha256（确认变异生效）→ 在
**独立 target** 里构建/测试 → 判定只取 hash 稳定那一次 → 用备份逐字节还原并再核 sha256。五处变异
全部还原后，副本 `cargo test -p xirang-mcp --offline --lib` 为 **108 passed / 0 failed**，且副本的
`mcp/{tools,index,tree_delta,evidence}.rs` 与两份 README 与仓库**逐字节相同**（`diff -q` 无输出）。

我自己装置里被自身抓出的三个解析缺陷（说明探针确实独立、且结论不是"解析器恰好同意"）：
① README 的桶枚举跨行折行、末项以 `and` / `与 ` 连接，最初的切分把它读成 2 项；
② diff 预览的首行 `+ <path>` 是头部，我的"跳过 `+ ` 开头行"启发式同时误吞了缩进的正文行（`+    kind: …`），
   造成"预览与落盘不一致"的**假红**——修正后逐行 unified diff 为 0 差异；
③ `usages.rs` 的 22 个标签分布在**两半**（多行 `(label, value)` 条目 + 单独的 `format!("  needs_registry …")`），
   只解析数组会得到 21 个，正好漏掉 `getting_from_other_registry` 与 `needs_registry`——原缺陷的藏身处。

## 1. 逐条判定

| 项 | 判定 | 装置 | 证据 |
| --- | --- | --- | --- |
| **BR-1** `unmatched` 桶 | **证实** | `buckets.py` | 三条边**逐字有序相等**：`diff.rs` 计数行字面量 `ok {}  undeclared {}  stale {}  re-identified {}  unreadable {unreadable}` → `['ok','undeclared','stale','re-identified','unreadable']`；真二进制 `xirang.diff {"records": true}` 回一行 `ok 0  undeclared 0  stale 0  re-identified 0  unreadable 0`；两份 README 的枚举句按各自的写法（`,` + `and` / `、` + `与 `）解析出**同一有序列表**。九条断言全 PASS，含"五桶"与"`unmatched` 在代码桶/两份 README/运行时回复中 0 命中"。 |
| **BR-2** "树 diff 仍待做"整句 | **证实**（附**行号更正**） | `grep` | `grep -rn "still to come\|仍待做\|item 7\|第 7 条" mcp/README.md mcp/README.zh-CN.md` → **0 命中**（exit 1）；收尾能力句确实存在，但位置是 `mcp/README.md:174-175`（"Everything this bridge reads or writes is the two lists above."）与 `mcp/README.zh-CN.md:120`（"本桥读取与写入的全部能力即上面两张清单。"）——作者报的 `168-171` / `115-117` 各偏后 3–5 行，句子本身逐字相符。 |
| **BR-7** `xirang.usages` 的字段清单 | **证实** | `usages_labels.py` | 从 `usages.rs` 独立解析出 **22** 个标签（两半合计），逐个在 `tools.rs` 的 `xirang.usages` 描述与两份 README 里命中：**66/66 PASS**。 |
| **BR-12** 目录 ↔ 分派齐全与顺序 | **证实** | `dispatch.py`（端到端 stdio） | 用 live `tools/list` 的 **17** 个 name 逐个 `tools/call`：无一回复 `unknown tool`；先用一个 Cargo 说不出名字的 root 捕获 **root 解析错误文本**，断言 17 个回复都不等于它（排除"root 失败 ⇒ 其实从未分派"的假绿）；再从源码独立解析 `DISPATCH` 表：集合与顺序都与目录一致、无重名。 |
| **BR-9** 可移植路径一份规则 | **证实** | `grep` + 变异 B | `grep -rn "replace('\\\\', \"/\")" mcp/src` → **0 命中**；`mcp/index.rs:179` 转发内核 `xirang::declaration::portable_path`。变异 B（把该转发里的折叠去掉）⇒ `index_tests::a_backslash_in_a_file_name_is_spelled_one_way` **红**，症状 `left: ["a\\b.rs"] right: ["a/b.rs"]`，还原后绿。 |
| **BR-11** index 的内联测试迁出 | **证实** | `grep` + 测试 | `grep -rn "^mod tests {" mcp/src` → **0 命中**；`mcp/index_tests.rs` 存在，`cargo test -p xirang-mcp --offline` 的 108 条里含 index 的 6 条全绿。 |
| **BR-C5①** `slot` → `registry_name`、`slots:` → `faces:` | **证实** | `grep` | `grep -rn "slot {}\|slots:" mcp/src` → **0 命中**；`mcp/evidence.rs:127` 的逐面表头写 `registry_name`（值仍是 `face.registry_name`），`tools.rs` 的描述同词。 |
| **BR-C5②** 状态词形单源 | **证实** | `grep` + 变异 D | 词形常量之外，`"added since build"` / `"re-identified"` 在 `mcp/src` 只出现在 `search_tests.rs:108` 的**夹具包名**字符串里；变异 D（`FaceStatus::label()` 对 `Reidentified` 返回 `reidentified`）⇒ `search_tests::a_kind_change_under_an_unmoved_file_is_re_identified` **红**，而 `diff_tests` 的计数行钉子（读常量）**仍绿**——两面各自的词形都有钉子。 |
| **BR-C6** 无名三元组类型化 | **证实（审计要求①）／附加声明部分证实** | `grep` + 变异 E + 端到端 | ① 审计的最小修复方向是"收成 `struct { current, scope, pruning }`，文档就不必再解释位置"：`mcp/evidence.rs` 的 `BuildEvidence` 具名字段已落地，`evidence.rs:141/205` 用 `evidence.freshness()`，`overlay.rs` 不再按位置解构（取 `evidence.current`/`evidence.scope`；"没用 pruning"现在是代码里显式的，而不是 doc 里的 `_`）。审计要求②（`is_loaded` 改名）在 `plugin-host`，属 t6 scope 外，作者已列报；我核其仍在原位、未被本批触碰。② 作者报告里"把 current 回答什么放在**唯一**产出该词的地方"只对 explain 的两条报告成立：`mcp/overlay.rs:97-101` 与 `mcp/converge.rs:119-126` 各自**内联**了同一对词。变异 E（`freshness()` 的 current 分支改成 `healthy`）⇒ **整个 mcp lib 套件 108 条全绿**，且同一包上用变异二进制实测回 `build healthy`、用仓库二进制回 `build current`——三份词形之间**没有任何钉子耦合**（`evidence_tests.rs:79` 只钉了 stale）。 |
| **BR-C7** `apply` 的 `work` 类型化 | **证实** | `apply_e2e.py`（端到端 stdio） | `apply_target.rs` 的 `enum Target { Project, Copy(PathBuf) }` + `report_path`/`report_message` 在；端到端两条路：**预览**（无 `apply`）在一次性 host 副本上**逐文件 sha256 完全不变**、回复给出 `would write <path>` 与 diff；**落盘**（`apply: true`）恰好创建该文件，且**落盘字节 == 预览 diff 的内容**（逐行 unified diff 0 差异）。 |

### BR-C6 附加声明的建议（最小）

让 `overlay` / `converge` 消费 `BuildEvidence::freshness()`（`converge` 的 stale 分支保留它的附加从句
"; the scope and pruning below come from that build"），或至少给 `freshness()` 的 **current** 词形补一条钉子
——今天的钉子在 `freshness()` 改动时**全绿**，而叠加层/汇聚报告仍说旧词。

## 2. evidence 三个读数的行为不变（BR-C6 的"类型化不改行为"面）

`xirang.explain` 端到端，三态各取三读数（`apply_e2e.py`，6 条断言全绿）：

| 状态 | `build` 行 | `scope` 行 | `pruned` 行 |
| --- | --- | --- | --- |
| 产物匹配（仓库自带的 `examples/control-button`，只读） | `build current` | `scope mode=auto all=false reason=- selected_ids=2 selected_sources=2` | `pruned 3` |
| 指纹失配（同一包 + 已发布产物副本 + 改一行源码） | `build stale (run \`xirang check\`)` | 同上（清单仍可读） | `pruned 3` |
| 无产物（一次性包，无 `target/xirang/out`） | `build stale (run \`xirang check\`)` | `scope unknown (no source_scope.tsv; run \`xirang check\`)` | `unknown (no pruning_manifest.tsv; run \`xirang check\`)` |

三种状态都可区分，说明 `current` 回答的确实是"产物是否仍描述这批源码"（BR-C6 的语义要求），而不是构建健康度。

## 3. 变异反证（副本内，五处）

| # | 变异 | 目标钉子 | 结果 |
| --- | --- | --- | --- |
| A | 删掉 `DISPATCH` 里的 `("xirang.impact", impact)` 臂 | 我的分派探针 + 作者的 `every_listed_tool_is_dispatched` / `the_dispatch_table_follows_the_catalog` | 探针 exit 1，报 `advertised but not dispatched: ['xirang.impact']`；作者的 `every_listed_tool_is_dispatched` 红（"xirang.impact is advertised but not dispatched: … unknown tool \`xirang.impact\`"）、`the_dispatch_table_follows_the_catalog` 红。还原后两者绿、探针 exit 0 |
| B | `index.rs` 的 `portable_path` 转发改成不折叠 | `index_tests::a_backslash_in_a_file_name_is_spelled_one_way` | 红：`["a\\b.rs"] != ["a/b.rs"]`；还原后绿 |
| C | 从 `tools.rs` 的 `xirang.usages` 描述里删掉 `needs_registry` | `tools_tests::the_usages_description_names_every_field_it_prints` | 红（点名 `needs_registry is read back … and must be named`）；还原后绿 |
| D | `FaceStatus::label()` 对 `Reidentified` 返回 `reidentified` | `search_tests::a_kind_change_under_an_unmoved_file_is_re_identified`；对照组 `diff_tests` | 目标红；对照组（计数行读常量）仍绿；还原后全绿 |
| E | `BuildEvidence::freshness()` 的 current 分支返回 `healthy` | 作者的词形钉子（探针性问题） | **整套 108 条 mcp lib 测试仍全绿**；变异二进制实测同一包回 `build healthy`、仓库二进制回 `build current` ⇒ 该词形**没有**承重钉子（见 §1 BR-C6 ②与 §4） |

每个变异都记录了变异前/后与还原后的 sha256，并在还原后用 `diff -q` 与仓库逐字节比对。

## 4. 门禁（静置 + 哈希钉住的树上）

`bash /tmp/t10/probe/gates.sh`，前后 status-hash 与 mcp-source-hash 一致：

| # | 命令 | 结果 |
| --- | --- | --- |
| 1 | `cargo test -p xirang-mcp --offline` | exit 0；**108 passed / 0 failed**（+ 2 个空的 doc-test 段） |
| 2 | `cargo test --workspace --offline` | exit 0；**55 个测试二进制全 ok，791 passed / 0 failed** |
| 3 | `cargo clippy --workspace --all-targets --offline -- -D warnings` | exit 0 |
| 4 | `cargo test -p xirang-conventions --offline` | exit 0；**113 passed / 0 failed** |
| 5 | `cargo fmt --all -- --check` | exit 0 |

（第 5 条不在任务的 Verify 清单里，作为"五条门禁"的第五道一并跑；与任务清单的 4 条合计 5 条。）

## 5. 未覆盖范围（如实登记）

1. **平台**：全部装置在 Linux 上。BR-9 的折叠靠"文件名里含反斜杠"的夹具在 Unix 上暴露；真正的
   Windows 分隔符路径、以及 `Path::components()` 系（`normalized_path`）在 Windows 上的行为未实测。
2. **BR-C6 的另一半**：`plugin-host/src/lazy_wasm.rs` 的 `is_loaded` 只核了"未被本批触碰"，未做行为验证。
3. **BR-1 的语义等价**：我验证了词表三条边一致与 `diff.rs` 的 `undeclared` 分支条件，但**没有**端到端构造
   `xirang.grafts` 的 `NOT declared by the host entry` 输出去证明两者"同判"只是同一事实的两种说法。
4. **BR-C7 的动作面**：端到端只覆盖 `add`（父级为 root）一条；`edit` / `rename` / `delete` 与 `confirm`
   门未逐动作跑（类型化对四个动词共用，但按动作未覆盖）。
5. **分派探针的深度**：空参数调用只证明"每个名字都被分派到某个实现"，不证明各工具的参数面正确
   （那是各工具自己的钉子与我在 t5/t6 之外的范围）。
6. **长会话/边界**：未测 1 MiB 帧、SIGPIPE、并发客户端与进程生存期；那是 LGC/桥的其他批次题材。
7. **README 的位置类断言**：仓库的 `doc_anchors` 门禁按 AGENTS.md 的新说明只覆盖 `.rs`/`.toml`/`.yml`/`.yaml`，
   `.md` 目标不在其中——BR-2 的行号偏差（§1）只能靠人复核，这一点本身就是"散文里的数字没有机制对账"的又一例。

## 6. 结论

B3 两组修复的**实质项全部证实**：承诺面（桶枚举、字段清单、分派齐全与顺序、失效路线图句）与代码/运行时逐字一致；
内聚项（一份折叠、测试挂载、slot 词、状态词形、类型化 target、类型化三读数）均有独立装置与变异反证支持，
行为在端到端路径上不变；五条门禁在哈希钉住的静置树上全绿。

**唯一需要后续动一下的**是 §1 BR-C6 的附加声明与 §3 变异 E：`freshness()` 是 explain 两条报告的词形单源，
但 overlay / converge 仍各持一份同词副本，且没有任何钉子把三份耦合起来——建议按 §1 的最小方案收口，
否则下一次改词只会在 explain 上生效，叠加层与汇聚仍说旧词（正是 BR-C5② 消除的那类漂移）。
