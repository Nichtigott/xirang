# mcp 三轮修复独立验证（t69）

**对象**：`mcp` 本轮三个批次的修复——`t53`（`LG-23` 预览字节上限 + `LG-20` 不可预测临时目录）、
`t54`（`LG-43` callgraph 参数契约 + `search` 的 `unparsable` 缺口）、`t57`（`LG-51` 归属先于存在性）。
作者是 `bridge-auditor` 与 `kernel-auditor`，**都不是本验证者**。
**验证者**：`logic-adversary`（t69）。装置全部自建，跑在 `/tmp/t69-v`（`tar` 副本，
`CARGO_TARGET_DIR=/tmp/t69-target`）；本检出只读，唯一写入是本文件。

**被验对象哈希（门禁前后 `diff` 为空，`OBJECT_STABLE=yes`）**：

```
66698b5d5489e98e0e14be333fe3e364  mcp/preview.rs
ecc3674117e152dbb0c085a0d96278e6  mcp/mir.rs
d7f0401f2374789138835c5cdd650f96  mcp/callgraph.rs
1c2effe282503a4d980a3eb1d3f6abac  mcp/search.rs
80ef426a5ac9473d79629c5ddd03710d  mcp/tools.rs
02c52c1958001b6a3d773066c29b378c  mcp/nodes.rs
b63435f4a80b7d8eae2f17fc4e337a93  mcp/apply.rs
c1159ccb938013b53d18d0cbae557fd0  mcp/protocol.rs
```

## §1 逐条判定

| 条目 | 判定 | 我的装置（不采信作者钉子原文） |
| --- | --- | --- |
| `LG-23` 字节上限 `MAX_DIFF_BYTES = 64 KiB` | **证实** | 200 KiB 的**单行**改动文件：`diff_package` 的回复**逐字包含**通知行 `… truncated: a preview reports at most 200 diff lines or 65536 bytes; read the written files for the rest`，字节数 `< 64 KiB + 512`，且**不含**完整长行（`probe lg23_a_single_long_line_is_cut_and_the_reply_says_so`） |
| `LG-20` 不可预测临时目录名 | **证实** | 在"第 0 次尝试会用的名字"上**预置**一个含 marker 的目录：副本落在下一个候选（`occupied-1`）、预置目录与 marker **逐字节未动**且未被写入；`copy_package` **8 次调用 8 个不同文件名**；八个候选全被占时以一句点名临时目录的拒绝收场且八个目录都不受影响；复制中途失败（子目录 `chmod 000`）时 `copy_into` **删掉自己建的目录**（四条装置） |
| `LG-43` callgraph 真实契约 | **证实** | 一个函数名在 60 个文件里各定义一次：缺省 → 5 条 + `… +55 more definitions`；`limit: 75` → **夹到 50**（`… +10 more definitions`，且返回 `Ok` 不拒绝）；`limit: 0` → 夹到 1（`… +59 more definitions`）；`path: "src/m07.rs"` → `matches 1` 且**没有** `more definitions`（精确选一） |
| `search` 的 `unparsable` 与命中无关 | **证实** | 两个坏注册面文件的包：查询**完全不命中任何面**时回复就是 `unparsable faces 2`；命中一个面时同样有这一行（`probe lg11_the_unparsable_line_does_not_depend_on_a_match`） |
| `LG-51` 归属先于存在性 | **证实**（逐字节） | 五种根外形态——相对存在、相对不存在、`..` 归一化、绝对存在、绝对不存在——经**与桥同一条分派**（`tools::tool_call`）拿回的**整个 JSON 回复逐字节相同**：`{"id":1,"jsonrpc":"2.0","result":{"content":[{"text":"path must stay inside the configured source root","type":"text"}],"isError":true}}`（比较含错误码/字段在内的完整信封）；根内路径的对照回复是另一句（`is not a readable file …`） |

## §2 装置与变异反证

装置（只在副本里挂载；真检出零改动）：

```
mcp/zz_t69_preview_probe.rs   （挂在 preview.rs 内，取私有接缝）
  lg20_a_preexisting_directory_is_skipped_not_reused_or_deleted
  lg20_eight_calls_get_eight_names
  lg20_a_failed_copy_removes_what_it_made
  lg20_eight_occupied_candidates_refuse_without_touching_them
  lg23_a_single_long_line_is_cut_and_the_reply_says_so
mcp/zz_t69_probe.rs           （挂在 crate 根，经 pub(crate) 面 + tool_call 驱动）
  lg43_the_callgraph_limit_contract_holds
  lg51_root_outside_paths_share_one_reply
  lg11_the_unparsable_line_does_not_depend_on_a_match
```

**8/8 绿**；随后五处变异各自让对应装置变红（脚本 `/tmp/t69-mutations.sh`，每处还原后
`cmp` 相同并记下 sha256 前缀）：

| 变异 | 改回什么 | 红侧原文 |
| --- | --- | --- |
| M1 | `copy_package_with` 先 `remove_dir_all` 同名目录再创建（旧行为） | `lg20_a_preexisting_directory_is_skipped…` FAILED（`zz_t69_preview_probe.rs:52`） |
| M2 | `push_diff` 去掉字节上限那一支（只留行数上限） | `lg23_a_single_long_line_is_cut_and_the_reply_says_so` FAILED（`:147`，通知行不在回复里且回复无界） |
| M3 | 归属判断改回"存在性决定包含"（`let contained = if path.exists() { is_safe_child } else { true }`） | `lg51_root_outside_paths_share_one_reply` FAILED（`zz_t69_probe.rs:115`，`../absent.txt` 与 `../outside.txt` 不再是同一句） |
| M4 | `clamp(1, 50)` 改成不夹（`value as usize`） | `lg43_the_callgraph_limit_contract_holds` FAILED（`:66`，`limit: 75` 不再夹到 50） |
| M5 | `search` 把 `unparsable` 行挂进"有命中"的守卫里 | `lg11_the_unparsable_line_does_not_depend_on_a_match` FAILED（`:181`，无命中时回复退化成 `no matches`，坏文件从回复里消失） |

还原后装置 8/8 复绿，`preview.rs` / `mir.rs` / `callgraph.rs` / `search.rs` 与还原快照
`cmp` 全部 `identical`（sha256 前缀 `0195c41ee8c0fe8d` / `0e2137a7eeea706c` /
`8870a95a112a55cd` / `94c8b08b67f60777`）。

## §3 两处"容易自证"的地方（专门核过）

1. **`LG-51` 的"同一句"是逐字节的**：比较对象是 `tools::tool_call` 返回的**完整 JSON**
   （含 `id`/`jsonrpc`/`result.content[].type`/`isError`），不是只比那句 message。五种根外形态
   两两相同；`..` 归一化（`src/../../<outside>`）与绝对路径也在其中。
   **平台注记（观察，不是缺陷）**：`\\server\share\zz.txt` 在 Unix 上只是一个相对分量，因此被当成
   **根内**名字，回的是存在性相关的 `is not a readable file …`；它不构成"根外是否存在"的探针。
   归属判断用的是 `Path::is_absolute()`（平台感知），所以在把 UNC 视为绝对路径的平台上它会走边界
   那一句。探针把这个形态跑了出来并记录在此。
2. **`MAX_DIFF_BYTES` 的"可见报出"是断言那一行本身**：装置断言回复**包含**完整的 truncation
   通知行，并另断言回复长度 `< 64 KiB + 512`、且不含原始长行——即"被截断"是**说出来的**，而不是
   只靠行数上限顺带挡住（M2 证明：只留行数上限时，这一行不再出现且回复无界）。

## §4 门禁（静置/哈希钉住的树上）

| 命令 | 结果 |
| --- | --- |
| `cargo test -p xirang-mcp --offline` | **exit 0**（3 个 `test result: ok` 段，23:11:34） |
| `cargo test --workspace --offline` | **exit 0**（`test result: FAILED` 计数 0，23:11:40） |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | **exit 0**（23:12:40） |
| `cargo test -p xirang-conventions --offline` | **exit 0**（23:12:43） |
| `cargo fmt --all -- --check` | **exit 0**（23:13:01） |

被验的 8 个 mcp 文件在整轮门禁前后 `md5sum` 逐一相同（`OBJECT_STABLE=yes`）⇒ 结论针对静止对象；
本轮五条一次全绿，无需作废重跑。

## §5 未覆盖与局限（如实列出）

- **`MAX_DIFF_LINES = 200`（行数上限）没有独立装置**：我的装置只驱动字节上限那一支（验收点名
  的是它）。行数上限的行为由作者测试覆盖，本报告不为其背书。
- **没有经 `xirang.preview` 工具端到端驱动 `LG-23`**：装置直接调用该工具内部使用的
  `diff_package`（`preview.rs` 的 `pub(crate)` 函数，工具就是转发到它），因此"工具参数解析"这一层
  未覆盖。
- **`LG-20` 的"不可预测性"是经验性证据**：我证明了"预置的名字不会被复用/删除、8 次调用 8 个名字"，
  但没有做统计强度或名字熵的证明（`entropy()` 取 `RandomState` 的进程内种子；跨进程不可预测性未测）。
- **`LG-51` 只测了 `xirang.mir`**：其它读路径的工具（`read`、`callgraph` 的 `path` 等）是否也不
  泄露根外存在性未覆盖——本单范围只是 `mir` 的那一条。
- **并发/竞态未测**（三条修复都与并发无关，故未构造并发装置）。

## §6 新发现

1. **`search` 的 `unparsable` 行与 `limit` 的交互值得留意（低危，建议记账）**：`derived_faces` 的
   行被无条件推进 `results`，而 `limit` 在**别的循环**里限制结果条数——因此一条极小 `limit` 的查询
   仍会带上 `unparsable faces N`。这与 "这是被读到的树的事实" 的设计一致（我按此预期测的），但值得
   写进该工具的文档，免得读取方把它当成一条"结果"来计数。
2. **UNC 形态的平台差异**（§3 注记）：建议在 `LG-51` 的注释里补一句"`is_absolute()` 是平台感知的，
   因此 Unix 上的 `\\server\share` 是根内相对名"——这是本次验证实测到的边界，不是缺陷。
3. **`LG-20` 的失败路径现在有两层**（本次实测确认）：候选耗尽（8 次）与复制中途失败都会留下零
   残留；建议在工具文档里保留"最多尝试 8 次"这个数字，因为它是可观测的契约的一部分。
