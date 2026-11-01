# t14 结构重构方案：目录 / 模块 / 包与包管理的合并与拆分

> 任务 t14（boundary-architect）。**本轮只出方案，源码一行不改**：本文是唯一产物。
> 命名说明：任务书写的工作路径是 `docs/audit-2026-09-28/boundary-refactor-plan.md`，队长 2026-09-28 的硬规则要求本目录下每个文件名都以 `audit-` 开头（`conventions/src/doc_blocks.rs:69` 的 `RECORD_PREFIXES` 只按**文件**名前缀豁免）。两者冲突时按队长的硬规则执行，因此实际落盘为 `docs/audit-2026-09-28/audit-boundary-refactor-plan.md`，不另建旧名文件。
> 输入：`docs/audit-2026-09-28/audit-structure-base.md`（t7，本人的事实底座）+ 五路片区的「结构观察」小节（`audit-lane-kernel.md`、`audit-lane-surfaces.md`、`audit-lane-studio.md`、`audit-lane-bridges.md`、`audit-lane-gates.md`）+ `audit-inventory.md`（t1 机械清点）。

## 0. 这份方案怎么读

**判据纪律**（维护者的真实约束）：他要的是「文件的归属的科学性」。行数只是**证据之一**，不能替内聚性判断做决定。因此本文每条问题都必须给出下列之一作为判据，缺一不可：

| 判据 | 说明 |
| --- | --- |
| **内聚性** | 一个文件/目录里装着两个及以上互不相关的职责，或一个职责被切成两页 |
| **同一规则两处实现** | 同一个事实在两个执行面各算一次，且没有唯一来源 |
| **体量 vs 职能** | 体量与它承担的职责不成比例——**仅作证据**，必须与上面两条之一并用 |
| **阅读顺序** | 读者按"先入口后细节"读时要跳文件、或最有价值的部分在最后一屏 |
| **门禁/文档边界** | 规则或文档的形状与它的意图不符（例如豁免按文件名而不按目录） |

**编号与状态**：`R-01`…`R-28`。每项标注 `结论`：
- `做`＝建议做，机械步骤在 §3；
- `条件做`＝必须先满足一个外部条件（维护者拍板 / 与某条 lane 发现同批 / 与版本推进同批）；
- `不做`＝已评估并否决，理由写在 §6。

**总量**：28 项 = `做` 22 项 + `条件做` 5 项（R-02 / R-20 / R-21 / R-22 / R-23）+ `不做` 1 项（R-25）。另有 §6.1 的 17 条"结构上明确不动"的非编号项。**没有任何一项要求改 crate 数量**——crate 边界本轮不动，理由见 §2.3 与附录 A。

---

## 1. 现状问题清单

### A 组：包与包管理

#### R-01 · `做` · 同一份依赖要求在 11 个清单里各写一遍
- **位置**：根 `Cargo.toml`（14 行，无 `[workspace.dependencies]`）；`grep -rn 'version = "0.1.6"' --include='Cargo.toml' .` → **20 处**，分布在 11 个清单（root 1 + 成员 19）。
- **判据**：同一规则两处实现（版本要求有 19 个副本，无唯一来源）+ 门禁边界（`tools/xirang-publish:462` 强制每条内部要求 == `^<workspace version>`，副本多一处就多一处漂移面）。重复的外部依赖同样：`serde_json` ×4（`build_method/Cargo.toml:25`、`cli/Cargo.toml:38`、`mcp/Cargo.toml:41`、`studio/Cargo.toml:84`）、`syn` ×5、`proc-macro2` ×3、`tempfile` ×2。
- **动作**：见 §3 的 R-01 行。工具侧零改动——`tools/xirang-publish:523` 已经检查 `[workspace.dependencies]` 里的内部版本，`tools/xirang-publish:602` 已经会读 `xirang-x.workspace = true`。
- **身份**：安全（清单不改任何模块路径）。
- **回滚**：还原根清单与 10 个成员清单（纯清单改动，无代码）。
- **验收**：`tools/xirang-publish --check-table` 绿；`cargo metadata --no-deps --format-version 1 --offline` 的依赖边集合与 feature 集合**逐条等于**改前（用 §5 批 1 的 diff 脚本比对）；`cargo test --workspace --offline` 绿。

#### R-02 · `条件做` · 已定决策写着"独立发版"，实施门禁却禁止它
- **位置**：`docs/roadmap-1.0.md:74`（"保留编译隔离与独立发版……只有真正用到新行为时才抬下界"）vs `tools/xirang-publish:462` 起的 `metadata_requirement_problems()`（要求每条内部要求 == `^<workspace version>`）。
- **判据**：门禁/文档边界——两份权威对同一件事说法相反，且**工具那一份是强制的**（CI `.github/workflows/ci.yml:134` 与发布工作流都跑 `--check-table`）。lane-gates 的 G-27 独立命中同一处。
- **动作**：二选一（推荐 a，成本一行）：
  - **(a) 改文档**：把 `docs/roadmap-1.0.md:74` 的"独立发版"改成"共享版本线：一次推进＝9 个 crate 一起发"，并在 `AGENTS.md` 写明这条；工具不动。
  - **(b) 放宽工具**：允许内部要求写成 `>=0.1.0, <0.2` 这类下界，并让 `metadata_requirement_problems()` 接受"下界 ≤ 工作区版本"的写法；同时补测试夹具覆盖两种拼法。
- **身份**：安全。**回滚**：文档改动或 awk 判断的一行。
- **验收**：改完后文档里不再出现与工具矛盾的句子（`grep -n '独立发版' docs/roadmap-1.0.md` 为空或带上相反说明）；`--check-table` 仍绿。

#### R-03 · `做` · "crate 太多了"缺一条可审计的边界契约
- **位置**：`AGENTS.md` 的 crate 表（只有"一行职责"列）；`Cargo.toml:2` 的 members 数组；`docs/roadmap-1.0.md:74` 是唯一记录合并曾被否决的地方。
- **判据**：门禁/文档边界——维护者刚刚提出"crate 太多了"，而仓库里**没有任何地方**写着"为什么这个 crate 必须独立"。数量之争因此只能靠口味。t7 §6.1 已经证明 12 个成员里 11 个能给出可复算的独立性理由（物理约束 / 打包约束 / 编译隔离 / 产品入口）——这份理由应当被写下来并让门禁守住。
- **动作**：见 §3 的 R-03 行：① `AGENTS.md` 的 crate 表加一列"为什么必须独立"（每行一句，取自 t7 §6.1 的分类）；② `conventions` 新增一个门禁模块（例如 `conventions/src/boundaries.rs`，按既有风格挂载），断言"根清单的每个 member 都在边界表里出现，且表里的每个成员都有一句理由；表只能增不能无故减"。
- **身份**：安全（只读清单与文档）。**回滚**：删门禁模块 + 还原 AGENTS.md。
- **验收**：把 `AGENTS.md` 表里某一行删掉 → 门禁**红**并点名该 member；给 `members` 加一个假成员（临时、不入库）→ 门禁红；正常状态 `cargo test -p xirang-conventions --offline` 绿。

#### R-04 · `做` · 发布层级顺序在两处文档里与活表矛盾
- **位置**：`docs/roadmap-1.0.md`（发布顺序段）与 `tools/xirang-package-audit` 头部注释 vs 活表 `tools/xirang-publish:86` 的 `levels`。
- **判据**：门禁/文档边界——按文档写的顺序发布**必然失败**（lane-gates G-16、G-21 各自实测）。包管理文档是执行指令，不是叙述。
- **动作**：二选一：① 删掉文档里的顺序副本，改成"顺序的唯一来源是 `tools/xirang-publish` 的 `levels`"；② 让文档从表派生（生成式）。推荐 ①（一行）。
- **身份**：安全。**回滚**：文本还原。**验收**：`grep -n 'xirang-core' docs/roadmap-1.0.md` 不再出现"发布顺序"语境的清单；`--check-table` 绿。

### B 组：studio 文件级职责（全私有模块 → 身份安全）

> studio 的目录分层是本轮审计里最健康的一层（lane-studio §5 原话）；问题都出在**文件级职责**上，**不需要动目录**。

#### R-05 · `做` · `writers.rs` 的名字与自述不符（**改名，不并入**；原修法已被复核否决）
- **位置**：`studio/studio/app/writers.rs:1`（全 42 行；**首行自述是 "Write guards"** —— 它是"写入的守卫缝"，不是"拥有全部写入"）与 `studio/studio/app/mutations.rs:1`（395 行，唯一的写入方）。
- **判据**：**名实不符**（本行原文把 `writers` 读成了"写入方"，lane-studio C-02 的原始表述同样如此）。该文件第二段写明"写入必须走无回落根"——**并进 `mutations.rs` 会拆掉这道缝**，因此原文的"合并"修法被 `audit-naming-review.md` 的 **NAM-06**（t27 复核提出、t30 修订、t31 复判通过）否决。
- **动作**：**改名而不合并**——`git mv studio/studio/app/writers.rs studio/studio/app/write_guard.rs`，`studio/studio/app/app.rs:35` 的挂载行随之改名（`mod writers;` → `mod write_guard;`）；文件内容与守卫缝保持不变。
- **身份**：安全（studio 无生产注册面声明，§4 实测）。**回滚**：`git mv` 反向 + 还原 `app.rs` 一行。
- **验收**：`cargo test -p xirang-studio --offline` 与 `--features prototype-fixtures` 两档绿；`grep -rn 'writers' studio/src` 为空（`write_guard` 不含该子串）；`write_guard.rs` 存在且模块文档仍写着守卫语义。

#### R-06 · `做` · `support.rs`（509 行）装了四类互不相关的关注点
- **位置**：`studio/studio/app/support.rs:1`（509 行；lane-studio C-04 实测：几何 + 项目解析 + cargo 子进程 + 路径判定）。
- **判据**：内聚性——"support" 是一个不承担信息的名字，四类关注点里两类（项目解析、cargo 子进程）与另两类（几何、路径）没有任何共享不变量。
- **动作**：见 §3 的 R-06 行：拆成 `geometry.rs`（纯几何/布局判定，保留原测试）与 `project_scan.rs`（项目解析 + `cargo metadata` 子进程）；`studio/studio/app/app.rs:34` 一条挂载行变两条。
- **身份**：安全。**回滚**：`git mv` 反向 + 还原两行挂载。
- **验收**：两个新文件各自 ≤300 行（体量只是证据，判据是"一个文件一件事"）；`cargo clippy -p xirang-studio --all-targets --offline -- -D warnings` 绿。

#### R-07 · `做` · `state/misc.rs` 是杂物抽屉
- **位置**：`studio/studio/app/state/misc.rs:1`（181 行，lane-studio S-13）。
- **判据**：内聚性——文件名为 `misc`，内容按"没地方放"落位；`state/state.rs:9-22` 已经按界面分组（forms/graft/pages/search），misc 是唯一没有分组依据的一组。
- **动作**：按内容归入同目录已有的分组页（`state.rs` / `navigation` 相关页 / `search.rs`），或在确有独立职责时新开一页并**用职责命名**；`studio/studio/app/state/state.rs:13-14` 的挂载行同步。
- **身份**：安全。**回滚**：内容还原 + 挂载行还原。
- **验收**：`grep -rn 'misc' studio/src` 为空；`cargo test -p xirang-studio --offline` 绿。

#### R-08 · `做` · `navigation.rs` 混了导航与文件系统时间戳
- **位置**：`studio/studio/app/navigation.rs:1`（173 行，lane-studio S-26 与 §5「FS 戳 + 导航」）。
- **判据**：内聚性 + 阅读顺序——导航（移动焦点）与 FS 戳（何时重扫）变化频率与测试方式都不同。
- **动作**：把 FS 戳/重扫判定移到 `lifecycle.rs`（它已经是"何时重扫"的归属），`navigation.rs` 只留导航；`studio/studio/app/app.rs:13` 挂载行不变（文件名不变）。
- **身份**：安全。**回滚**：函数搬回即可（无路径变化）。
- **验收**：`navigation.rs` 里不再出现 `metadata`/`elapsed`/时间戳调用（`grep -n 'metadata\|elapsed\|SystemTime'` 为空）；测试绿。

#### R-09 · `做` · `overlay/add.rs` 与 `overlay/edit.rs` 是两台同型状态机
- **位置**：`studio/studio/app/overlay/add.rs:12`（39 行）与 `studio/studio/app/overlay/edit.rs:17`（44 行），lane-studio S-20 实测两台的键位处理与字段推进逻辑同型。
- **判据**：同一规则两处实现（两台状态机各自维护同一套字段推进规则，改一处忘一处就是行为分叉）。
- **动作**：合成一台带 mode 的状态机（或把推进逻辑抽到共享页），`studio/studio/app/keyboard_overlay.rs:8`/`:13` 两条挂载行按结果调整。
- **身份**：安全。**回滚**：文件与挂载行还原。
- **验收**：`add`/`edit` 的字段推进只存在一份实现（`grep -c 'next_field\|advance' studio/studio/app/overlay/*.rs` 合计为 1 处）；两档测试绿。

#### R-10 · `做` · 四处表单页各写一遍按钮/按键区
- **位置**：`studio/studio/ui/forms/face.rs:105`、`ui/forms/project.rs:47`、`ui/forms/plugin.rs:46`、`ui/forms/graft.rs:214`（lane-studio S-21）。
- **判据**：同一规则两处实现（同一份按钮条渲染在四个页面里各写一遍）。
- **动作**：抽 `ui/forms/buttons.rs` 一页，`studio/studio/ui/forms.rs` 的挂载块加一条 `#[path = "forms/buttons.rs"] mod buttons;` 并由各页复用。
- **身份**：安全。**回滚**：删新页 + 还原四页。
- **验收**：四页里不再各写一份按钮条；`cargo test -p xirang-studio --offline` 绿。

#### R-11 · `做` · 测试占位：三份夹具助手重复，一页 562 行
- **位置**：`studio/studio/app/tests/call_tree.rs:1`（702 行）、`app/tests/evidence.rs`、`app/tests/graph.rs`（三份各自的夹具助手，lane-studio S-22）；`app/tests/project.rs:1`（562 行，lane-studio S-23）。
- **判据**：同一规则两处实现（三份助手）+ 内聚性（562 行的一页里同时装"向导流程"与"MIR target 解析"两组互不相关的用例）。
- **动作**：见 §3 的 R-11 行：助手并入已有的 `app/tests/fixtures.rs`（`studio/studio/app/tests.rs:71` 已挂载它）；`project.rs` 按用例组拆成 `project.rs`（向导/根解析）与 `mir_target.rs`（`cargo rustc` target 解析），在 `studio/studio/app/tests.rs` 补一条挂载。
- **注意（否决了 lane 的一个更强建议）**：lane-studio 建议"编译型验证移出单测二进制"。**不做**——`app/tests/project.rs` 通过 `super::*` 使用 `app` 的私有面（`select_project`、`App` 方法），移出 `#[cfg(test)]` 树就必须扩大 `xirang_studio` 的公开面（现在只有 `studio/lib.rs:14` 的 `launch`/`launch_with`）。进程级行为已由 `studio/tests/launch.rs` 用真实二进制覆盖，重复一遍不划算。
- **身份**：安全（测试页）。**回滚**：文件与挂载行还原。
- **验收**：`cargo test -p xirang-studio --offline` 与 `--features prototype-fixtures` 两档绿；测试文件名与它断言的组对得上。

#### R-12 · `做` · `keyboard_overlay.rs` 的文件名与它收纳子模块的目录不同名
- **位置**：`studio/studio/app/keyboard_overlay.rs:9` 起挂载的子模块住在 `studio/studio/app/overlay/`；t7 §2.2 的形态规则是"有子模块的模块，本体在 `<name>/<name>.rs`"。
- **判据**：门禁/文档边界 —— 全仓其余 57 个"有自己目录"的模块都满足 `<name>/<name>.rs`，这里是**唯一**的例外；它靠显式 `#[path = "overlay/…"]` 才能工作，读者按约定去找 `keyboard_overlay/keyboard_overlay.rs` 会扑空。
- **动作**：二选一：① `git mv studio/studio/app/overlay studio/studio/app/keyboard_overlay`，其中 `keyboard_overlay.rs` 移入 `keyboard_overlay/keyboard_overlay.rs` 并把父文件的 `#[path]` 由 `overlay/x.rs` 改为 `x.rs`；② 若认为 `overlay/` 才是好名字，则把父文件改名为 `overlay.rs` 并把它的子模块改成 `overlay/<name>.rs`（即 `<name>/<name>.rs` 形态），同时改 `studio/studio/app/app.rs:25` 的挂载。推荐 ②（目录名已表达职责，父文件跟着目录改名）。
- **身份**：安全。**回滚**：两次 `git mv` 反向 + 两行 `#[path]` 还原。
- **验收**：`find studio/src -name '*.rs'` 里每个"有子目录"的文件都满足 `<dir>/<name>.rs`（用 t7 §2.2 的脚本复算，例外数归零）。

### C 组：执行面文件级职责与阅读顺序

#### R-13 · `做` · 纯映射模块里藏着文件系统读取
- **位置**：`build_method/graft_view/matching.rs:74` 起的 `face_declares_plugin`（lane-surfaces 结构观察 3：`graft_view/` 五块里只有它混了纯映射与 IO）。
- **判据**：内聚性——同一模块里 `graft_expression_module`（纯函数）与 `face_declares_plugin`（读文件）属于两层；读者找"匹配规则在哪"要翻两个文件。
- **动作**：把 IO 那一半移到构建的 fs 层（`build_method/face_view.rs` 已有读面文件的职责，或新开 `build_method/face_file.rs`），`matching.rs` 只收纯映射；`build_method/graft_view/graft_view.rs` 的挂载行按结果调整。
- **身份**：安全。**回滚**：函数搬回。**验收**：`matching.rs` 内不再出现 `std::fs`（`grep -c 'std::fs' build_method/graft_view/matching.rs` = 0）；`cargo test -p xirang-build-method --offline` 绿。

#### R-14 · `做` · 最有价值的入口在文件最后一屏，110 行挂载在最前面
- **位置**：`build_method/lib.rs:24` 起是挂载块、`:137` 起才是 `run`/`run_for`/`check_for`；`run_method/lib.rs` 同型（crate 根只挂模块，入口宏在 `macros/`）——lane-surfaces 结构观察 5。
- **判据**：阅读顺序——读者按"先入口再细节"读时，入口在最末。
- **动作**：① 在 `build_method/lib.rs` 的挂载块**之上**加一段 crate 级文档导览（"先读 `run()`，再读 `pipeline.rs`，再读 `renderer/`"），`run_method/lib.rs` 同理（"先读 `host!()` 与 `runtime/trace/`"）；② `lib.rs` 内部块序改为"公开白名单 → 入口 → 内部再导出 → 挂载"（`#[path]` 顺序不影响编译，只影响阅读）。**不动任何路径**，因此不是公开 API 改动。
- **身份**：安全。**回滚**：块序与文档还原。
- **验收**：`build_method/lib.rs` 的入口函数出现在挂载块之前；两份 crate 文档各带一条导览；`cargo doc --workspace --no-deps --offline` 无新警告。

#### R-15 · `做` · 构建期两个文件与内核同名，且模块名被改过，从文件列表看不出谁是 shim
- **位置**：`build_method/identity.rs:1`（模块名 `registry_identity`，由 `build_method/lib.rs:60-61` 的 `#[path]` 重命名而来）、`build_method/node_id.rs:1`、`build_method/syntax.rs:1`（9 行，模块名 `registry_syntax`，见 `build_method/lib.rs:62-63`）——lane-surfaces 结构观察 4。
- **判据**：内聚性 + 阅读顺序——`build_method/identity.rs` 与 `node_id.rs` 同时存在，而内核也有 `identity/node_id`；`syntax.rs` 与内核 `syntax/` 同名却是两回事。读者会以为看到了"第二个实现"。
- **动作**：① 把三个文件改成承担信息的名字（如 `identity_kernel_shim.rs` / `node_id_shim.rs` / `registry_syntax_shim.rs`，具体命名由维护者定）；② `build_method/lib.rs:60-63` 的挂载块分成"crate 前缀：`registry_identity`/`registry_syntax`"与"本地实现"两段，各带一行说明。**模块名保持原样**（`registry_identity`/`registry_syntax` 已被下游书写？实测这两个模块不是 `pub`，见 §4 的可见性表，因此改模块名也安全——但为降低风险，本项只改文件名与挂载分段）。
- **身份**：安全。**回滚**：`git mv` 反向 + 挂载行还原。
- **验收**：`grep -rn 'registry_identity\|registry_syntax' --include='*.rs'` 的结果只剩 `build_method/lib.rs` 的两处显式重命名；`cargo test -p xirang-build-method --offline` 绿。

### D 组：桥与宿主（mcp / cli / plugin-host）

#### R-16 · `做` · `tree_delta.rs` 是 diff 与 search 共用的唯一规则，却没有自己的钉子
- **位置**：`mcp/tree_delta.rs`（120 行，无 `<name>_tests.rs`）；同型缺口还有 `mcp/nodes.rs`（76）、`mcp/preview.rs`（180）、`mcp/converge_trace.rs`（256）——lane-bridges 结构观察 1。
- **判据**：内聚性 + 阅读顺序——`mcp/src` 的其余 16 个模块都是"源码 + 同名测试页"1:1，这四个是缺口；而 `BR-6`（命名空间不一致）恰好落在 `tree_delta.rs` 的盲区。
- **动作**：为 `tree_delta.rs` 建 `mcp/tree_delta_tests.rs`，在 `mcp/lib.rs` 的挂载块补 `#[cfg(test)] #[path = "tree_delta_tests.rs"] mod tree_delta_tests;`（沿用其它 16 处的写法）；`preview.rs` 的 `diff_package` 至少加一条直接钉子。`nodes.rs`/`converge_trace.rs` 视内容是否值得单独立页再定。
- **身份**：安全。**回滚**：删新页 + 删挂载行。**验收**：`cargo test -p xirang-mcp --offline` 绿；新增钉子能在人为破坏 `tree_delta.rs` 的判定时变红。

#### R-17 · `做` · 三份 `tokens()`、两份 `blank()`、两份 `requires`
- **位置**：`mcp/src`（lane-bridges BR-10）。
- **判据**：同一规则两处实现——注释里已经写明"为什么不复制"的四处（`graft_plan_rows`/`face_views`/`build_output_is_current`/`OVERLAY_NOTE`）做到了单点，这三处漏网，方向已定只差收口。
- **动作**：把三组各收口到一份（同 crate 内的私有模块，或提为内核的公共词法工具——若提为内核公开项则属公开 API 变更，须与一次版本推进同批，见 §5 批 4）。
- **身份**：安全（若收口在内核公开面上则见 §4）。**回滚**：还原各文件。
- **验收**：`grep -rn 'fn tokens' mcp/src` 等各只剩 1 处；`cargo test -p xirang-mcp --offline` 绿。

#### R-18 · `做` · 同一分发入口，一半命令走 sink、一半直写 stdout
- **位置**：`cli/commands/build.rs:28`、`cli/commands/new.rs:81`、`cli/commands/snippets.rs:70` 等直接 `println!`；而 `check`/`explain`/`grafts`/`studio` 走 `run_to` 的 sink（lane-bridges 结构观察 4）。
- **判据**：同一规则两处实现（输出出口有两个）+ 阅读顺序（`cli/lib_tests.rs` 1015 行集中在抓 stdout，是这条分叉的直接代价）。
- **动作**：让 `build`/`new`/`snippets` 也接受 sink（`cli/lib.rs:136` 的 `run_to` 已是通用入口），`println!` 换成写 sink；测试改为对 sink 断言。
- **身份**：安全。**回滚**：还原各命令。
- **验收**：`grep -rn 'println!' cli/commands` 为空（或只剩 stderr 用途）；`cargo test -p xirang-cli --offline` 绿；`--json` 契约不变。

#### R-19 · `做` · 968 行的故障矩阵把两个特性门控的模块装在一页里
- **位置**：`plugin-host/tests/fault_matrix.rs`（968 行，lane-bridges 结构观察 5：`wasm` 与 `process-tools` 两个门控模块同处一文件）。
- **判据**：内聚性——改  wasm 限额时要在一块 968 行的文件里来回跳；两个后端各自独立门控。
- **动作**：拆成 `plugin-host/tests/wasm_faults.rs` 与 `plugin-host/tests/process_faults.rs`（各自保留 `#![cfg(feature = …)]`），CI 的 `cargo test -p xirang-plugin-host --features process-tools`（`.github/workflows/ci.yml:180`）保持不变即覆盖两者。
- **身份**：安全（测试）。**回滚**：合并回一页。
- **验收**：`cargo test -p xirang-plugin-host --offline` 与 `--features process-tools` 两档绿；两页各自 ≤600 行。

### E 组：同一规则两处实现（收口到唯一来源）

> 这一组的每条都与某条 lane 发现**同体**：结构动作与逻辑修复是同一件事。它们必须在同一批里批准（见 §5 批 4）。

#### R-20 · `条件做` · 函数符号扫描有第二份实现，且树内零读者
- **位置**：`build_method/manifests.rs:111` 起（第二套函数扫描器，lane-surfaces S4；内核已有 `function_symbols`）；产物被写进 `function_manifest.tsv`。
- **判据**：同一规则两处实现 + 体量 vs 职能（约 60 行只为生成一个**树内没有任何读者**的文件）。
- **动作**：删除该扫描器与 `function_manifest.tsv` 产物，或改用内核 `function_symbols`（若确有外部读者）。删产物属**格式变更**，需与一次版本推进同批并在 CHANGELOG 写明。
- **身份**：安全。**回滚**：恢复函数与产物行。
- **验收**：`grep -rn 'function_manifest' --include='*.rs' .` 为空；`build_method` 的 manifest 测试按新形状更新且绿。

#### R-21 · `条件做` · `FaceView.path` 与运行期 `Registry::path_for` 可能不同判
- **位置**：`build_method/face_view.rs:280` 起 vs `core/src/registry_core/tree/query/query.rs:86` 起（lane-surfaces S1；`build_method/graft_view/overlay_rows.rs:137` 消费它，`mcp` 的 overlay 直接受影响）。
- **判据**：同一规则两处实现——"一个面的逻辑路径"有两个算法；现有测试刻意没断言子面的 path（lane-surfaces 原话），说明分歧被测试回避掉了。
- **动作**：让内核的 `path_for` 成为唯一答案，`FaceView.path` 改为消费它（或把父级解析提进内核 query 面）。若因此改动内核公开项，与版本推进同批。
- **身份**：安全（不改模块路径）。**回滚**：还原两个实现。
- **验收**：新增一条钉子：对同一棵树，`FaceView` 的 path 与 `Registry::path_for` 逐面相等（含子面）；故意让两边分叉时该钉子变红。

#### R-22 · `条件做` · 字段词表在五处各说各话（数量从 20 到 26）
- **位置**：`core/src/registry_core/declaration/registration.rs:427`（`FACE_FIELD_ORDER`，24）、`run_method/macros/face.rs:59`（镜像，24）、`run_method/authoring/operations/face_values.rs:20`（23）、`run_method/authoring/operations/face_write.rs:32` 与 `:59`（22 / 20）、`core/src/registry_core/authoring/face_field.rs:107`（`FACE_FIELD_COUNT`，26）——lane-surfaces 结构观察 8。
- **判据**：同一规则两处实现 + 门禁/文档边界——五处**没有任何一处声明它们之间的关系**，这正是 lane-surfaces S6（"能编辑但不能写回"）与 lane-gates C6（同一集合被写成 27/28/29 三个数）得以共存的土壤。
- **动作**：定一份来源（内核的字段序，或宏侧的镜像由内核派生），其余各层只留"视图 + 显式映射"，并加一个门禁断言各表长度/顺序一致（`conventions` 或内核单测）。
- **身份**：**注意**——`FACE_FIELD_COUNT`/`face_field` 出现在 shim 承诺表里（`conventions/src/shims.rs` 的 `SHIMS`），因此改的是"派生方式"而不是名字时安全；一旦改名即公开 API 变更，须与版本推进同批。
- **回滚**：还原各表与派生代码。
- **验收**：新增一致性断言（改一处数字即红）；`cargo test --workspace --offline` 绿。

#### R-23 · `条件做` · 孪生类型的字段搬运清单手写在两处
- **位置**：`core/src/registry_core/declaration/registration.rs:343` 起（`into_snapshot`，约 40 行手写字段清单）与 `core/src/registry_core/declaration/owned.rs:280` 起（`merge_authored`）；lane-kernel 结构观察 2 指出 K-02 正是漏在这个清单上。
- **判据**：同一规则两处实现——7 对孪生类型（`RegistrationInfo`/`RegistrationSnapshot` 等）的字段搬移没有单一来源。
- **动作**：让字段清单只有一个来源（宏生成成对的 `into_owned`/`merge`，或让 `merge_authored` 复用 `into_snapshot` 的清单）。纯内核内部，公开类型形状不变。
- **身份**：安全（不改路径与类型名）。**回滚**：还原清单。
- **验收**：删掉清单里的一行字段 → 编译失败（证明它已是唯一来源）；`cargo test -p xirang-core --offline` 绿。

#### R-24 · `做` · `needs_registry` 被再解析一遍面文件
- **位置**：`build_method/renderer/aliases.rs:78` 起（为 `needs_registry` 再解析面文件，失败即当 `false`；lane-surfaces 结构观察 7）。
- **判据**：同一规则两处实现——构建期已经算过注册面的属性，这里又解析一次，且失败方向是静默的 `false`。
- **动作**：改为消费已算出的视图（`FaceView`/扫描结果）。
- **身份**：安全。**回滚**：还原该段。**验收**：`aliases.rs` 不再读面文件（`grep -c 'std::fs'` 相关调用归零）；`cargo test -p xirang-build-method --offline` 绿。

### F 组：内核模块粒度

#### R-25 · `不做` · `core/src/registry_core/tree/` 的 `Registry` 只读面分散在六页
- **位置**：`core/src/registry_core/tree/registry.rs`（98 行）、`tree/index/index.rs`（111）、`tree/query/query.rs`（191）、`tree/metadata/metadata.rs`（69）、`tree/inspection/inspection.rs`（199）、`tree/ports/ports.rs`（325），lane-kernel 结构观察 1 建议按"构造/读/写/统计"归并成两页。
- **判据**与**否决理由**：内聚性判据成立（读者要读完四页才知道 `Registry` 的公开面有多大），但**本轮不做**——① 这六页里有公开模块路径（`xirang::tree::query` 等，t7 的 §2.2 与 `core/src/lib.rs:33-37` 的模块层级重导出把它定成官方地址），归并即公开 API 破坏；② 归并只改善阅读，不消除任何重复实现；③ 成本（路径迁移 + 版本推进 + 文档锚点全量更新）远大于收益。若维护者仍要，见 §6 的条件与前置。

### G 组：门禁与文档的结构一致性

#### R-26 · `做` · 记录豁免只按文件名，不按目录
- **位置**：`conventions/src/doc_blocks.rs:69`（`RECORD_PREFIXES = ["audit", "design"]`）、`:133` 的 `is_record`、`:73` 的 `markdown_files`。
- **判据**：门禁/文档边界——把审计报告放进 `docs/audit-2026-09-28/` 这个"看起来就该豁免"的目录**并不豁免**；本轮 7 份报告全部踩到，实测门禁一度红（lane-gates G-06/G-23 双向实测）。队长的裁決是"文件名加前缀"，那是**绕开**而不是**修好**。
- **动作**：二选一：① 让 `is_record` 同时看目录（路径任一段以 `audit`/`design` 开头即豁免）；② 保持规则，把它写进 `AGENTS.md` 与 `conventions/src/mounting.rs:40` 的同类说明里，让"文件名前缀"成为一条被写下来的约定。推荐 ②（改动面小、与既有 `docs/audit-*.md` 惯例一致）。
- **身份**：安全。**回滚**：还原判定或文案。
- **验收**：按所选方案加一条夹具测试：目录名带 `audit` 但文件名不带时，行为与所选方案一致；`cargo test -p xirang-conventions --offline` 绿。

#### R-27 · `做` · 散文里的数字/清单与代码对不上（三处同类）
- **位置**：`conventions/src/mounting.rs:10`（"29 处裸声明"，实测 33）、`conventions/src/size.rs` 的模块文档（棘轮条目数写成 14，实际 1）、`AGENTS.md` 的门禁清单（比实际门禁宽、且缺 4 道）——lane-gates G-20/G-17/G-19。
- **判据**：门禁/文档边界——`size.rs` 的模块文档自己就写着"复述数字的散文终会与它漂移"，而 `mounting.rs` 仍在复述。
- **动作**：凡"清单/数字"要么删掉、要么改成从代码派生（`BASELINE.len()` 式），并在 `AGENTS.md` 里把门禁清单与实际门禁对齐（缺的 4 道补上，宽出来的删掉）。**不要**给这些数字加门禁去钉住散文（`AGENTS.md` 已记录不这么做的理由）。
- **身份**：安全。**回滚**：文本还原。
- **验收**：`grep -rn '29 such' conventions/src` 为空；`size.rs` 文档不再出现条目数；`AGENTS.md` 的门禁清单逐条对得上 `conventions/src/lib.rs:33` 起的挂载块。

#### R-28 · `做` · 两个示例宿主的版本要求没有任何门禁覆盖
- **位置**：`examples/control-button/Cargo.toml:14`、`examples/control-button-graft/Cargo.toml:13`（`version = "0.1.6"`）；`tools/xirang-publish` 的 `metadata_requirement_problems()` 只检查发布表里的 crate（lane-gates G-05）。
- **判据**：门禁/文档边界——一个成员清单里的版本要求写错，今天不会被任何门禁发现（而它们是**文档**：脚手架与示例是人抄格式的来源）。
- **动作**：二选一：① 把两个 example 也纳入内部要求检查（`shipping` 集合里加两个名字，或另开一段只查"路径依赖的 version 若写了就必须等于工作区版本"）；② 明确记录"examples 不参与版本线"并从它们的清单里删掉 `version =`（路径依赖不需要版本，只有发布才需要）。推荐 ②——它们 `publish = false`，版本字段是纯冗余。
- **身份**：安全。**回滚**：清单还原。
- **验收**：按所选方案，`--check-table` 覆盖到 examples（或 `grep -c 'version = "0' examples/*/Cargo.toml` = 0）；`cargo build --workspace --offline` 绿。

---

## 2. 目标结构

### 2.1 crate 清单：前后对照

| crate | 目录 | 前（HEAD cf0c378） | 后（本方案） | 变化 |
| --- | --- | --- | --- | --- |
| `xirang-core` | `core/` | 93 文件 / 21,787 行 | 不变 | 无 |
| `xirang-macro` | `macro/` | 3 / 917 | 不变 | 无 |
| `xirang-build-method` | `build_method/` | 48 / 10,110 | 文件重命名 3 处 + 两处内聚性搬移 | R-13/R-15 |
| `xirang-run-method` | `run_method/` | 61 / 9,584 | 不变（仅文档导览） | R-14 |
| `xirang-debug-method` | `debug_method/` | 5 / 481 | **不变**（合并案本轮否决，附录 B） | — |
| `xirang-plugin-host` | `plugin-host/` | 15 / 3,888 | 测试页 1 → 2 | R-19 |
| `xirang-studio` | `studio/` | 79 / 12,532 | 文件级搬移（8 项，全在私有模块内） | R-05…R-12 |
| `xirang-mcp` | `mcp/` | 39 / 8,682 | 新增 1–2 个测试页 + 三组收口 | R-16/R-17 |
| `xirang-cli` | `cli/` | 14 / 2,802 | 输出出口统一 | R-18 |
| `xirang-conventions` | `conventions/` | 20 / 6,399 | 新增 1 个门禁模块 | R-03 |
| `xirang-example-control-button` | `examples/control-button/` | 13 / 1,635 | **不变**（身份承载，§4） | — |
| `xirang-example-control-button-graft` | `examples/control-button-graft/` | 4 / 134 | **不变**（身份承载，§4） | — |

**成员数 12 → 12；发布面 9 → 9。** 本方案的收益来自**文件归属与"唯一来源"**，不来自挪 crate——这与维护者说的"文件的归属的科学性"是同一件事。

### 2.2 包管理：前后对照

| 项 | 前 | 后 |
| --- | --- | --- |
| `[workspace.dependencies]` | 不存在；`serde_json` ×4、`syn` ×5、`proc-macro2` ×3、`tempfile` ×2、内部要求 ×19 | 根清单一份；成员用 `workspace = true`（per-crate `features`/`optional` 局部覆盖） |
| 版本线政策 | `roadmap-1.0.md:74` 说"独立发版"，工具强制"整线同步" | 文档与工具一致（R-02 二选一） |
| crate 边界契约 | 只有 AGENTS.md 的一行职责 | crate 表加"为什么必须独立"列 + `conventions` 门禁（R-03） |
| 发布顺序文档 | 与活表矛盾的两处副本 | 唯一来源＝`tools/xirang-publish:86` 的 `levels`（R-04） |
| 门禁散文数字 | `mounting.rs:10` 说 29（实际 33）等三处 | 删数字或派生（R-27） |

### 2.3 目录树（标出改动点）

```text
core/                         不变（R-25 否决 tree/ 归并）
macro/                        不变
build_method/
  ├─ identity_kernel_shim.rs  ← R-15 重命名自 identity.rs（模块名 registry_identity 不变）
  ├─ node_id_shim.rs          ← R-15 重命名自 node_id.rs
  ├─ registry_syntax_shim.rs  ← R-15 重命名自 syntax.rs（9 行）
  ├─ face_file.rs             ← R-13 新增：从 graft_view/matching.rs 移出的 FS 读取
  ├─ graft_view/matching.rs   只留纯映射
  ├─ manifests.rs             去掉第二套函数扫描器（R-20，条件做）
  └─ lib.rs                   块序与导览（R-14）
run_method/
  └─ lib.rs                   导览（R-14）；其余不变
debug_method/                 不变
plugin-host/tests/
  ├─ wasm_faults.rs           ← R-19 拆自 fault_matrix.rs
  └─ process_faults.rs        ← R-19 拆自 fault_matrix.rs
studio/studio/app/
  ├─ mutations.rs             唯一写入方
  ├─ write_guard.rs           ← writers.rs（R-05：**改名，保留守卫缝**；不得并入 mutations.rs）
  ├─ geometry.rs              ← R-06 拆自 support.rs
  ├─ project_scan.rs          ← R-06 拆自 support.rs
  ├─ navigation.rs            只留导航（R-08）
  └─ overlay/ 或 keyboard_overlay/   改名为 <name>/<name>.rs 形态（R-12）
studio/studio/ui/forms/
  └─ buttons.rs               ← R-10 新增
mcp/
  └─ tree_delta_tests.rs      ← R-16 新增
conventions/src/
  └─ boundaries.rs            ← R-03 新增门禁
examples/                     两个宿主整体不变（身份承载）
```

---

## 3. 机械步骤总表

> 本仓库用 `#[path = "<dir>/<name>.rs"] pub mod <name>;`，**没有 `mod.rs`**；文件移动的机械动作永远是三件：`git mv` 源→目标、改**父文件**里的那一行 `#[path]`、改引用该文件路径的文档锚点。清单改动永远要跑 `tools/xirang-publish --check-table`。

### R-01 `[workspace.dependencies]`（清单）

```text
Cargo.toml                [workspace.dependencies] 新增 4 条外部依赖 + 9 条内部依赖（各带 path 与 version）
core/Cargo.toml:21,23      proc-macro2/syn → optional/physical features 保留，改用 workspace = true
build_method/Cargo.toml:16,17,25,26  xirang-core / proc-macro2 / serde_json / syn → workspace = true
macro/Cargo.toml:20,21,26  xirang-core / proc-macro2 → workspace = true（dev-dep syn 在 :26）
run_method/Cargo.toml:19,20,21     xirang-core / xirang-macro / syn(optional) → workspace = true
debug_method/Cargo.toml:16        xirang-run-method → workspace = true
plugin-host/Cargo.toml:33,34      xirang-run-method / tempfile → workspace = true
studio/Cargo.toml:72,73,74,84     xirang-run-method / -debug-method / -build-method / serde_json → workspace = true
mcp/Cargo.toml:27,33,39,40,41     xirang-* 四条 + serde_json → workspace = true
cli/Cargo.toml:23,24,25,34,38     xirang-* 四条 + serde_json → workspace = true
examples/*/Cargo.toml             R-28 选择 ② 时一并删 version
conventions/Cargo.toml:29         xirang-core（path-only，无 version）→ 保持 path 或 workspace = true
```
- **不要在 `[workspace.dependencies]` 里给外部依赖写死 features**：各 crate 需要的 features 不同（`syn` 的 `visit`、`proc-macro2` 的 `span-locations`），用成员侧 `features = [...]` 追加，避免把 `visit` 强加给 `macro` 的 dev-dep。
- 完成后：`cargo metadata --no-deps --format-version 1 --offline` 的依赖边与 feature 集合必须与改前**逐条相等**（脚本比对），`tools/xirang-publish --check-table` 绿。

### R-02 版本线政策与文档对齐（二选一）

```text
方案 (a)  改文档：
  docs/roadmap-1.0.md:74      删掉「独立发版 / 只有真正用到新行为时才抬下界」，改为「共享版本线：一次推进 = 9 个 crate 一起发」
  AGENTS.md                   在 crate 表附近加一句同样的说明
方案 (b)  改工具：
  tools/xirang-publish:462 的 metadata_requirement_problems()  接受「下界 ≤ 工作区版本」的 caret/范围写法
  tools/xirang-publish      补一组夹具覆盖 `>=0.1.0, <0.2` 与 `^0.1.6` 两种拼法
```
- 两方案都只需跑 `tools/xirang-publish --check-table`（方案 b 另跑它自己的夹具）。

### R-03 crate 边界表（文档 + 门禁）

```text
AGENTS.md                 crate 表加一列「为什么必须独立」；每个成员一句（取自 t7 §6.1 分类）
conventions/src/lib.rs    #[path = "boundaries.rs"] pub mod boundaries;（按现有 12 个门禁的挂载风格）
conventions/src/boundaries.rs   新门禁：读根清单 members + AGENTS.md 的表，双向断言
conventions/src/boundaries_tests.rs   夹具：表缺一行/多一个成员各必须红
```

### R-04 发布层级顺序的唯一来源（文档）

```text
docs/roadmap-1.0.md                      发布顺序段 → 改为「顺序的唯一来源是 tools/xirang-publish 的 levels」
tools/xirang-package-audit 头部注释     同上（它描述的是一张已不存在的表）
```
- 可选加固：让 `--check-table` 顺带断言"文档里没有第二份顺序表"。本轮只做文档，不加新门禁。

### R-05 `writers.rs` → `write_guard.rs`（改名，不并入）

```text
git mv studio/studio/app/writers.rs studio/studio/app/write_guard.rs
studio/studio/app/app.rs:35          mod writers;  →  mod write_guard;
```
- **不并入 `mutations.rs`**：该文件第二段写明的"写入必须走无回落根"是它存在的理由，合并会拆掉这道缝（NAM-06 的复核结论；t27 提出、t30 修订、t31 复判通过）。
- `writers` 这个名字要全仓消失（`grep -rn 'writers' studio/src` 为空），但**守卫缝本身保留**。

### R-06 `support.rs` 拆两页

```text
git mv studio/studio/app/support.rs  studio/studio/app/geometry.rs     （几何/布局判定 + 其测试）
新增    studio/studio/app/project_scan.rs                                   （项目解析 + cargo metadata 子进程）
studio/studio/app/app.rs:34          - mod support;
                                         + #[path = "geometry.rs"] mod geometry;
                                         + #[path = "project_scan.rs"] mod project_scan;
```
- `use super::*;` 的既有写法保持；两个新文件各自把需要的导入收紧（`clippy -D warnings` 会替我们检查未使用导入）。

### R-07 `state/misc.rs` 归位

```text
git mv studio/studio/app/state/misc.rs  <内容归入 state.rs / search.rs 等，无新文件时删除>
studio/studio/app/state/state.rs:13     - #[path = "misc.rs"] - mod misc;   （两行删除）
```
- 若拆分后有独立职责，新页必须有职责名，且 `state.rs` 的挂载块与 `pub use` 段同步。

### R-08 `navigation.rs` 只留导航

```text
（无文件移动）
studio/studio/app/navigation.rs   把 FS 戳/重扫判定移到 lifecycle.rs
studio/studio/app/lifecycle.rs    接收这两个函数（它已是"何时重扫"的归属）
```

### R-09 overlay add/edit 合一

```text
git mv studio/studio/app/overlay/edit.rs  <推进逻辑并入 add.rs 或共享页后删除>
studio/studio/app/keyboard_overlay.rs:13   - #[path = "overlay/edit.rs"] - mod edit;
```
- `Overlay::Edit(id, edit)` 的枚举载荷与 `keyboard_overlay.rs` 的 `match` 分支同步。

### R-10 `ui/forms/buttons.rs`

```text
新增    studio/studio/ui/forms/buttons.rs
studio/studio/ui/forms.rs           + #[path = "forms/buttons.rs"] mod buttons;（按字母顺序插在 graft 之前）
studio/studio/ui/forms/{face,project,plugin,graft}.rs   四处按钮条改为调用新页
```

### R-11 测试页整理

```text
git mv studio/studio/app/tests/project.rs  studio/studio/app/tests/mir_target.rs   （只搬 target 解析那组）
studio/studio/app/tests.rs:81              + #[path = "tests/mir_target.rs"] mod mir_target;
studio/studio/app/tests/call_tree.rs       <702 → 拆两页，第二页在 tests.rs 里加一条挂载>
studio/studio/app/tests/{evidence,graph}.rs  夹具助手并入 tests/fixtures.rs
```

### R-12 `keyboard_overlay` 与 `overlay/` 同名化（推荐方案 ②）

```text
git mv studio/studio/app/keyboard_overlay.rs  studio/studio/app/overlay.rs
git mv studio/studio/app/overlay/*.rs         studio/studio/app/overlay/  （原地，目录名不变）
studio/studio/app/overlay.rs                  #[path = "overlay/add.rs"] 等 7 行保持不变
studio/studio/app/app.rs:25                   - #[path = "keyboard_overlay.rs"] mod keyboard_overlay;
                                                  + #[path = "overlay/overlay.rs"] mod overlay;
```
- 该模块是 `mod`（非 `pub`），因此改名不触公开 API；但它被 `app.rs` 的 `use`/调用点引用（`handle_overlay_key`），要一并改。

### R-13 `matching.rs` 的 FS 读取外移

```text
新增    build_method/face_file.rs（或并入既有 face_view.rs）
build_method/graft_view/matching.rs:74   移出 face_declares_plugin
build_method/graft_view/graft_view.rs    如新开文件则加一条 #[path] 挂载
```

### R-14 阅读顺序

```text
（无文件移动）
build_method/lib.rs   块序：crate 文档导览 → 公开白名单 → run/run_for/check_for → 内部再导出 → #[path] 挂载
run_method/lib.rs     同上，导览指向 macros/ 与 runtime/trace/
```

### R-15 构建期 shim 页命名

```text
git mv build_method/identity.rs  build_method/identity_kernel_shim.rs
git mv build_method/node_id.rs   build_method/node_id_shim.rs
git mv build_method/syntax.rs    build_method/registry_syntax_shim.rs
build_method/lib.rs:60,62        #[path = "identity.rs"] → #[path = "identity_kernel_shim.rs"]（模块名 registry_identity 不变）
build_method/lib.rs:~62          #[path = "syntax.rs"]   → #[path = "registry_syntax_shim.rs"]（模块名 registry_syntax 不变）
build_method/lib.rs:52,60,62    三条 #[path] 行改为新文件名（`mod node_id` / `mod registry_identity` / `mod registry_syntax` 三行的模块名不变）
```
- 三处 `#[path]` 行号以实际编辑时的位置为准；改完 `grep -rn 'identity.rs\|node_id.rs\|syntax.rs' build_method/src` 必须只剩新名。

### R-16 mcp 测试页

```text
新增    mcp/tree_delta_tests.rs（+ 视情 preview_tests.rs）
mcp/lib.rs   在 16 处同类挂载的同一段里加：
                 #[cfg(test)]
                 #[path = "tree_delta_tests.rs"]
                 mod tree_delta_tests;
```

### R-17 mcp 三组收口

```text
mcp/*.rs     tokens()×3 → 1、blank()×2 → 1、requires×2 → 1
（若提到内核公开面，按 §5 批 4 与版本推进同批）
```

### R-18 cli 输出出口

```text
cli/commands/{build,new,snippets}.rs   println! → 写 run_to 的 sink
cli/lib_tests.rs                       对 stdout 的断言改为对 sink 断言
```

### R-19 plugin-host 测试拆分

```text
git mv plugin-host/tests/fault_matrix.rs  plugin-host/tests/wasm_faults.rs     （wasm 门控那一半）
新增                                      plugin-host/tests/process_faults.rs  （process-tools 门控那一半）
（无 mod 声明；两个文件都是 cargo 自动发现的测试 target）
```

### R-20…R-24 唯一来源收口

```text
build_method/manifests.rs:111-171   删第二套扫描器（R-20）
build_method/face_view.rs:280-298   改为消费 core 的 path_for（R-21）
core/src/registry_core/authoring/face_field.rs:107 等五处  单一来源 + 派生视图 + 一致性断言（R-22）
core/src/registry_core/declaration/{registration.rs:343-408, owned.rs:280-315}  字段清单单点化（R-23）
build_method/renderer/aliases.rs:78-94  改为消费已算出的视图（R-24）
```

### R-26…R-28 门禁与文档

```text
conventions/src/doc_blocks.rs:133-140    is_record 加目录判定（或把规则写进文档，R-26 二选一）
conventions/src/mounting.rs:10           - "29 such declarations" 数字
conventions/src/size.rs                  - 模块文档里的棘轮条目数
AGENTS.md                                门禁清单与实际门禁对齐（R-27）
examples/{control-button,control-button-graft}/Cargo.toml   删 version 或纳入检查（R-28）
```

**跨面同步清单（任何一项移动后都要过一遍）**：

| 面 | 何时要动 | 命令 |
| --- | --- | --- |
| `Cargo.toml` 的 `members` | 只在增删成员时（本方案不增删） | `grep -n 'members' Cargo.toml` |
| `path` 依赖 / `version` / `features` | R-01、R-28 | `tools/xirang-publish --check-table` |
| `tools/xirang-publish` 的层表与依赖表 | 只在增删 crate 或内部依赖边时（本方案不涉及） | 同上 |
| CI 的 `-p` 引用 | 本方案不点名任何被移动的文件；`-p xirang-plugin-host --features process-tools`（`.github/workflows/ci.yml:180`）在 R-19 后不变 | `grep -rno '\-p xirang[a-z-]*' .github` |
| 脚手架模板 | 不变（模板只提 `xirang-run-method`/`xirang-build-method` 两个 crate 名） | `grep -rn 'xirang-' build_method/scaffold/` |
| 文档锚点 | 每一项移动后：`docs/**/*.md` 与 crate README 里指向被移动文件的 `path.rs:NNN` | 见 §5 的门禁脚本 |

---

## 4. 身份风险专章

**机制**：注册面用 `file!()` 派生 `NodeId`（`AGENTS.md` 的挂载约定就是为保护它）；身份会写进落盘的 graft 记录与计划文件，因此**移动一个"声明了注册面"的文件 = 静默改身份 = 旧记录不再解析**。移动不带注册面的文件则只改报告里的路径字符串，不改身份。

**实测（本轮本人复算，命令在 §5）**：

| 集合 | 文件 | 身份 |
| --- | --- | --- |
| **身份承载（生产）** | `examples/control-button/src/control/control.rs`、`examples/control-button/src/control/object/button/button.rs`、`examples/control-button/src/control/object/slider/slider.rs`（各 1 处 `crate::*_object!`）；`examples/control-button-graft/src/{button_fast,control_fast,slider_fast}.rs`（各 1 处 `external_object!`） | **移动即改 NodeId**；且 `examples/control-button/src/lib.rs:50` 的类型化 `static_graft_plan!` 用 `NODE_ID` 常量引用其中两处，`examples/control-button-graft/src/lib.rs` 的 `external_registry()` 引用另三处 → 移动必须同批改这两处引用，并重新生成计划 |
| **身份承载（语料，不编译）** | `studio/tests/fixtures/node-editor/src/**`（3 处 `*_object!`） | 该包从不编译、也不写记录；移动只会改 Studio 报告里的 `source` 字符串与断言 → 视为安全，但要同步 `studio/studio/app/tests/` 的断言 |
| **身份中立（十条发布 crate 的 `src/`）** | 全部（含 `run_method/macros/` 的宏定义——定义不是声明） | 实测：`*_object!`/`*_face!` 的每一处命中都落在四类里——**文档注释、夹具字符串（含测试里的临时源码文本）、`#[cfg(test)]` 模块、宏定义自身**（`run_method/macros/*` 是定义不是声明）；`src/` 里没有一处生产注册面声明 |
| **身份中立（测试面）** | `run_method/tests/*`、`studio/studio/app/tests/*`、`build_method/*_tests.rs`、`cli/lib_tests.rs`、`mcp/*_tests.rs` | 只改测试自己的 NodeId；测试里对生成文本的断言（如 `studio/studio/app/tests/project.rs:261` 断 `crate::root_object!`）不依赖行号 |

**由此得到的三条规则**：

1. **本方案的全部移动都落在"身份中立"集合里**——十条发布 crate 的 `src/` 与测试页。因此 §5 的批 2/批 3 **不需要身份迁移脚本**，只需要跑既有门禁。
2. **examples 的两个宿主（以及夹具包）列进"不动"**（§6）。若维护者确要改它们的布局，动作不是 `git mv`，而是一次**记录迁移**：移动 → 重新 `cargo build` 生成计划 → 用新身份重写既有 graft 记录（`lexicon` 的 `.xirang` 路径下）→ 在 CHANGELOG 写明"身份已变，旧记录需重建"。这一步**不可回滚**（旧记录一旦按新身份重写就无法复原），必须先经批准。
3. **任何移动后必跑的两条**：`cargo test --workspace --offline`（含 `run_method/tests/graft_record.rs`、`examples/control-button/tests/*` 这两个盯身份/记录的测试）与 `tools/xirang-external-rehearsal`（只在动了 `examples/` 时才需要，本方案不需要）。

**可见性交叉表（决定"改名是否触公开 API"）**：

| 被移动/改名的文件 | 模块可见性 | 改名是否触公开 API |
| --- | --- | --- |
| `studio/studio/app/**`、`studio/studio/ui/**` | `mod`（私有） | 否 |
| `build_method/{identity,node_id,syntax}.rs` | `mod`（`build_method/lib.rs:60-63` 用 `#[path]` 重命名为 `registry_identity`/`registry_syntax`，非 `pub`） | 否（本方案只改文件名，模块名保持） |
| `mcp/*.rs`、`plugin-host/tests/*`、所有 `*_tests.rs` | 私有 / 测试 target | 否 |
| `core/src/registry_core/**`（R-20…R-24 涉及） | 多为 `pub mod` | **是**——R-22/R-23 只改派生方式与内部清单时安全；一旦改**名字**或模块路径，必须与一次版本推进同批 |
| `examples/control-button*/**` | 工作区成员但 `publish = false` | 不是 API 问题，是**身份**问题（上表） |

---

## 5. 执行分批与顺序

| 批 | 项 | 前置 | 身份迁移 | 与版本推进同批？ |
| --- | --- | --- | --- | --- |
| **0（已具备）** | — | 本文 | — | — |
| **批 1：包管理与治理** | R-01, R-02, R-03, R-04, R-26, R-27, R-28 | 无（可立即开始） | 不需要 | 否（清单/文档/门禁） |
| **批 2：studio 文件级** | R-05…R-12 | 批 1 的 R-01 先落（避免清单二次改动） | 不需要 | **是**（改的是已发布 crate 的产物） |
| **批 3：执行面与桥** | R-13, R-14, R-15, R-16, R-17, R-18, R-19 | 无（与批 2 并行） | 不需要 | **是** |
| **批 4：唯一来源收口** | R-20, R-21, R-22, R-23, R-24 | **必须与对应 lane 发现的逻辑修复同批**（同体） | 不需要 | **是**（部分触内核公开面） |
| **批 5：条件项** | R-25（已否决，仅在维护者坚持时做）、附录 B（debug_method 合并） | 维护者明确拍板 | 附录 B 需记录迁移 | 是，且**不可回滚** |

**批内顺序**：先做"无文件移动"的项（R-08/R-14/R-17/R-18），再做 `git mv` 类，最后做跨文件收口（R-20…R-24）——这样 `git status` 里移动与内容改动分开，评审时看得清。

**每批必跑的门禁（`--offline`）**：

```sh
cargo fmt --all -- --check                                  # 只检查，不写
cargo test --workspace --offline
cargo test --workspace --all-features --offline             # 覆盖 studio/prototype-fixtures、plugin-host/process-tools
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
cargo doc --workspace --no-deps --offline
tools/xirang-publish --check-table                         # 清单动了就必须跑
tools/xirang-package-audit                                 # 成员/清单动了就必须跑（内容那一半不需要网络）
cargo test -p xirang-conventions --offline                 # 门禁自身；被 workspace 覆盖，动了 conventions 时单跑
```

**批 2/3 的额外门禁**：文件移动会改文档锚点，跑一遍与 `conventions/src/doc_anchors.rs` 同规则的锚点自查（本轮各片区报告已用同一脚本，报告里零违规）：

```sh
# 抽取 docs/**.md 与各 README 里的 `path.rs:NNN`，断言文件存在且行号在界内
python3 <本轮使用的锚点脚本>            # 脚本口径见 conventions/src/doc_anchors.rs:246 的 anchors()
```

**每批的验收标准（可判定）**：

| 批 | 验收 |
| --- | --- |
| 1 | `--check-table` 绿；`cargo metadata` 的依赖边/feature 集合与改前逐条相等；crate 边界门禁在"删一行表项"时红、正常时绿；文档里不再有与工具矛盾的版本线句子 |
| 2 | `cargo test -p xirang-studio --offline` 与 `--features prototype-fixtures` 两档绿；`grep -rn 'writers\|misc\|support' studio/src` 里不再有"名不承担职责"的页；t7 §2.2 的"有子目录的文件必为 `<dir>/<name>.rs`"脚本例外数归零 |
| 3 | 三个 crate 的 `cargo test -p <crate> --offline`（plugin-host 另加 `--features process-tools`）绿；`mcp/src` 的模块与测试页 1:1（缺口数下降）；`cli` 里不再有命令直写 stdout；`build_method` 的 `graft_view/matching.rs` 无 `std::fs` |
| 4 | 每项配一条**能变红的钉子**（R-21 的逐面相等、R-22 的一致性断言、R-23 的"删一行即编译失败"）；`cargo test --workspace --offline` 绿 |
| 5 | 只在维护者拍板后启动；附录 B 另需"记录迁移"演练（在临时宿主上做一遍移动+重生成，确认旧记录会失败、新记录能解析） |

**并行性**：批 1 与批 2/3 无依赖，可并行；批 2 与批 3 之间无文件交叠（studio vs build_method/run_method/mcp/cli/plugin-host），可并行；批 4 依赖对应 lane 的逻辑修复排期，与批 2/3 亦可并行（文件交叠仅 `build_method`，同 crate 但不同文件）。

---

## 6. 「不做」清单

### 6.1 结构上明确不动（附理由）

| 不动的东西 | 理由 |
| --- | --- |
| **crate 数量（12 成员 / 9 发布面）** | t7 §6.1：11/12 有可复算的独立性理由；唯一候选 `debug_method` 的技术动机已被实测证伪（附录 B）。**代价真正在包管理（R-01…R-04），不在 crate 数。** |
| `debug_method` 的包边界（本轮） | 见附录 B 的三条依据；若维护者仍要减，按附录 B 执行，属批 5。 |
| `macro` 的包边界 | `[lib] proc-macro = true` 物理不可并入普通库，且 `core` 不能反向依赖它（成环）。 |
| `build_method` / `run_method` 的边界 | 一个是宿主的 build-dependency（host triple），一个是普通依赖（target triple）；合并让 `build.rs` 那一遍编译运行期全部代码。 |
| `plugin-host` 的包边界 | 唯一在 `cli` 闭包外的发布 crate，把 `wasmi`/`ed25519-dalek` 挡在其余 8 个之外；树内零消费者。 |
| `conventions` 的位置与 `publish = false` | 它遍历检出；放进已发布 crate 的 `tests/` 会让解包者失败。 |
| `examples/*` 的目录布局与成员身份 | **身份承载**（§4）：移动即改 NodeId，且类型化 `static_graft_plan!` 引用 `NODE_ID`；要动必须走记录迁移，不可回滚。 |
| `studio/tests/fixtures/node-editor/` 不被编译、不进成员 | 它是 Studio 当**文本**读的语料；编译它需要独立 lockfile/离线缓存 + 一条新 CI 作业，而现有 `prototype-fixtures` 测试已经会在语料腐化时变红（解析断言）。 |
| `core/` 的模块划分（含 R-25 的 `tree/` 归并） | `core` 的模块层级是官方地址（`core/src/lib.rs:33-37` 的重导出），归并即公开 API 破坏；收益只有阅读顺序。 |
| `studio/studio/{app,ui}` 的目录分层与 `ui → app` 单向依赖 | lane-studio §5 判为"本轮审计里最健康的一层"；R-05…R-12 只动文件内容归属，保住这条边。 |
| `plugin-host/src` 的五层 | lane-bridges 结构观察 6：`wasm` / `lazy_wasm`+`slot_state` / `process`+`child` / `admission` / `verifier` / `deployment` 边界清楚，且每层的"为什么直白写法是错的"都写在注释里。 |
| `debug_method/src` 的四个文件布局 | 它按"再导出 / MIR 合并 / 图适配 / 收集器"分页，已经一页一责。 |
| `tools/xirang-publish` 的两张手维护表 | 今天与清单一致（`--check-table` 绿）；改成派生表是另一件事，不要顺手做（本轮只同步 R-01/R-04 要求的部分）。 |
| 600 行棘轮与 4 个测试形态超标文件 | 按 `conventions/src/size.rs` 的边界豁免（测试按位置/挂载判定）；动它们买不到结构收益。 |
| `studio/studio/app/tests/project.rs` 移出 `#[cfg(test)]` 树 | 它用 `app` 的私有面；移出必须扩 `xirang_studio` 公开面（现仅 `launch`/`launch_with`），收益不抵成本（进程面已由 `studio/tests/launch.rs` 覆盖）。 |
| 任何 `pub mod` 的**改名** | 公开 API；只能与一次版本推进同批，且要有迁移说明。本方案一项都不要求（R-22/R-23 只改派生方式）。 |
| 28 个"目录里只有模块本体"的模块 | 目录是子模块未来的位置：把无子模块的页扁平化，等于让"日后长出一个子模块"变成一次文件移动。形状统一比少 28 个目录更值。（t7 §2.2 的实测清单） |

### 6.2 需补文档（不动结构）

| 位置 | 补什么 |
| --- | --- |
| `run_method/macros/face_registration.rs:164` 的 collector switch（`development`/`debug`/`linked` 三个 arm） | **实测**：没有任何文档说明三方语义；第三方照写 `collector: debug` 会在 debug 构建撞 `E0433`（找不到 `xirang_debug_method`），同一源码 `--release` 通过（logic-adversary 探针）。修法二选一：补一段文档（推荐：写清"`debug` 需要宿主自己依赖 `xirang-debug-method`"），或改由 `run_method` 的一个特性间接 re-export。这条与附录 B 的结论**无关**，独立成立。 |
| `docs/migration*.md` 与两份根 README | 若 R-21…R-24 触到内核公开项，同步写迁移段；本方案不新增公开项。 |
| lane 各片区的注释轴条目（kernel C-01…C-11、surfaces C1–C9、studio C-01…C-06、bridges BR-C1…C7、gates G-20…G-26） | 归各片区自己的修复批；本文不重复列入（它们不改文件归属）。 |

### 6.3 明确留给门禁那一路（不在本方案范围）

`conventions` 门禁自身的假阴性/假阳性（lane-gates **G-01**（`#![deny(warnings)]` 被当 `missing_docs`）、**G-02**（发布 tag 守卫的 `contains` 黑名单）、**G-03**、**G-04**、**G-07**、**G-08**、**G-13**（持有 token 的 job 的 action pin 无门禁）、**G-09…G-12**、**G-17**、**G-18**、**G-22…G-26**）属于门禁逻辑修复，排 t18 那一路；本文只吸收其中**影响文件/文档归属**的三条（R-26/G-06+G-23、R-27/G-20+G-17+G-19、R-04/G-16+G-21、R-28/G-05）。

---

## 附录 A：与既有决策的关系（roadmap 决策 1 的重开）

`docs/roadmap-1.0.md:74` 记着"包形状：九个 crate 保持不动，九个全部发布。合并方案否决"，理由是"选择保留编译隔离与独立发版"。本轮的实际结论是：

1. **结论不变的部分**：crate 数量不动（本轮），理由从"独立发版"换成**可由证据支撑的三类**（物理约束 / 打包约束 / 编译隔离 / 产品入口，t7 §6.1）。
2. **必须更正的部分**：那条理由里的"**独立发版**"在实现上并不成立（R-02）——版本线一份、内部要求必须等于当前线、发布脚本一次遍历 9 个 crate。保留 9 个 crate 的真实收益只剩**编译隔离**。
3. **因此维护者的"太多了"感受**是**有根据的，但落点不同**：贵的是"多 crate 的治理成本"（版本线连带、重复的依赖声明、分散的特性词汇、手维护的发布表），而这些可以**不动 crate 数**就大幅降低（R-01…R-04）。这正是本方案把批 1 排在最前面的原因。

## 附录 B：`debug_method → run_method` 合并案（结论：本轮不做）

### B.1 为什么撤销 t7 §6.3 的推荐

t7 §6.3 列了三条理由，其中**最硬的一条已被实测证伪**：

| t7 的理由 | 现状 | 证据 |
| --- | --- | --- |
| ③ 宏发射 `::xirang_debug_method::submit!` 造成"幻影依赖" | **已证伪** | 该路径只在 `collector: debug` 那一 arm 发出（`run_method/macros/face_registration.rs:164` 起，`:166` 即该行），且在 `#[cfg(debug_assertions)]` 之下；`development`（`:163`）与 `linked`（`:168`）两个 arm 是空的。生成计划调 `__xirang_object!{…}` 不带 collector → 默认走 `development`（`run_method/macros/face_objects.rs:13`）→ **示例宿主不需要 `debug_method`**。实测：只依赖 `run-method` 的最小宿主显式写 `collector: debug` 时，debug 构建报 `E0433`，`--release` 通过。 |
| ① 444 行的 crate + 消费者总是同时依赖 `run_method` | 成立，但**判据不足** | 维护者判据明说"行数只是证据之一，不能替内聚性判断做决定"；而"没有独立安装面"对 `core`/`build_method` 同样成立。 |
| ② 消除 1 份清单 / 2 份 README / 1 个 docs.rs 页 / 3 条内部版本要求 | 成立，但代价已变 | 九个包名 **0.1.0–0.1.5 已全部发布**（`CHANGELOG.md:14`、`docs/roadmap-1.0.md:21`）：合并只能在 0.2.0 做，且包名**永不回收**（Cargo 没有重定向）。把 `petgraph`/`tracing`/`inventory` 的可选隔离从"crate 边界"降级为"特性开关"，在工作区内还会被特性合并抵消。 |

**判定**：去掉证伪的那条板子后，合并只剩"少几份清单/文档"的记账收益，与"一次不可回滚的包名删除"的代价不成比例 → **本轮不做**。t7 §6.3 的推荐由本文取代。

### B.2 若维护者仍要减：可执行步骤（批 5，需拍板）

```text
git mv debug_method/src/lib.rs         run_method/evidence/evidence.rs
git mv debug_method/src/mir.rs         run_method/evidence/mir.rs
git mv debug_method/src/adapters.rs    run_method/evidence/adapters.rs
git mv debug_method/src/collector.rs   run_method/evidence/collector.rs
git mv debug_method/tests/collector_integration.rs  run_method/tests/collector_integration.rs
run_method/lib.rs    + #[path = "evidence/evidence.rs"] pub mod evidence;（#[cfg(feature = "evidence")]）
run_method/Cargo.toml    + evidence = ["dep:petgraph", "dep:tracing", "dep:inventory"]
                         + petgraph/tracing/inventory 三条 optional 依赖
run_method/macros/face_registration.rs:166   ::xirang_debug_method::submit! → $crate::evidence::submit!
Cargo.toml:2             members 删 "debug_method"
mcp/Cargo.toml、studio/Cargo.toml   删 xirang-debug-method 一行，features 加 "evidence"
tools/xirang-publish:86   层表删 xirang-debug-method
tools/xirang-publish:323  依赖表删该行；xirang-studio/xirang-mcp 两行各少一个名字
AGENTS.md / README.md / README.zh-CN.md / docs/migration*.md   删表行与目录段
CHANGELOG.md            0.2.0 段写明"并入 run_method（evidence 特性）"，历史段不动
```

**不可回滚标记**：`git mv` 与特性命名可回滚；**删 `members` 与发布表条目不可回滚**（包名永久占用），因此这一步必须与 0.2.0 的版本推进同批，且发布前用 `tools/xirang-publish --check-table` + `--verify-consumers` 复验。
