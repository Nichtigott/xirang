# 审计 B（lane-surfaces）：执行面 build_method / run_method / macro / debug_method

- 审计对象：工作树（`HEAD=cf0c378`，交付时 122 项未提交改动 + 新增未跟踪文件）。
- 只读声明：本轮**源码一行未改**；只新建了本文件。未跑 `git checkout/restore/stash`，未跑 `cargo fmt`，`cargo` 一律 `--offline`。
- 本路复核的规模（我自己实测，和任务书给的数字并存一行说明）：`build_method/src` 46 文件 / 9928 行；`run_method/src` 52 文件 / 8277 行；`macro/src` 3 文件 / 917 行；`debug_method/src` 4 文件 / 444 行；四个测试/示例目录合计 1526 行。任务书里的 `run_method 9584` / `debug_method 481` 与我在 HEAD 上按 crate 全量（含 `tests/`、`examples/`）数出的结果一致，`build_method 10110` 与我在 HEAD 上数出的 9613 不一致，差异应是统计口径（HEAD 全量 vs 工作树 `src/`），以 executor 的清单为准。
- 每条发现的 file:line 都是我在工作树上 `read` 过的行；文末附锚点自检说明。
- 本报告不重复门禁基线（fmt/test/clippy/check-table）的红绿结论，那属于 verify 路与 executor 的账。

---

## 一、发现（按严重度）

### S1 · [MAJOR] · 大逻辑／边界（同一规则的第二个实现） · `FaceView.path` 与运行期 `path_for` 可能不同判

**file:line**
- `build_method/face_view.rs:280`（`logical_path` 只走一层，靠 `paths` 记忆化）
- `build_method/face_view.rs:288`～`build_method/face_view.rs:295`（父路径查不到就回落 `"root"`，然后把结果写进缓存）
- `core/src/registry_core/tree/query/query.rs:86`～`core/src/registry_core/tree/query/query.rs:96`（运行期 `Registry::path_for` 是**递归**求值）
- `build_method/discovery.rs:124`（同级节点按 name 排序，决定 `collect` 的访问顺序）
- `build_method/graft_view/overlay_rows.rs:137`（用 `face.path` 判"哪个槽位被替换"）

**现象**
`logical_path` 计算某个面的逻辑路径时，先查 `paths` 缓存里有没有**父面**的路径；查不到就用 `"root"` 当父路径，并把 `"root/<自己的槽位名>"` 写进缓存。父面是否已经在缓存里取决于 `collect` 的遍历顺序（DFS 前序 + 同级按名字排序）。只要某面的父面在同级里排在它**后面**（或父面在另一条分支上更晚被访问），这个面的骨架路径就少一段，并被永久冻结。运行期的 `path_for` 是从活树递归求值，永远给出完整链。

**判据**
- 构造最小反例：`src/alpha/alpha.rs` 声明 `parent: crate::zeta::NODE_ID`，`src/zeta/zeta.rs` 是 `needs_registry: true` 的面。DFS 先访问 `alpha`，于是 `paths` 里还没有 `zeta`，`alpha.path` 得到 `root/alpha`；运行期树给出的是 `root/zeta/alpha`。`parent_resolved` 仍为 `true`，没有任何东西标记这处不一致（`build_method/face_view.rs:148`）。
- 现有测试正好回避了这一格：`build_method/face_view.rs:350` 断言了父面的 `control.path == "root/object"`，但对子面（`build_method/face_view.rs:353` 起）只断言了 `registry_name` 与 `parent_resolved`，**没有断言 `button.path`**——所以这处顺序依赖没有被钉子覆盖。
- 该字段的承诺写在 `build_method/face_view.rs:82`～`build_method/face_view.rs:84`：“`path` 是运行期树会报告的逻辑注册路径”。`overlay_rows` 又用它做切口匹配（`build_method/graft_view/overlay_rows.rs:137`），而 `mcp/overlay.rs:55`～`mcp/overlay.rs:58` 直接消费这条投影，所以"哪个槽位被已声明切口替换"这个答案会跟着错。

**最小修复方向**
把 `logical_path` 改成真正的递归（先解析父面路径，再拼自己；带环检测），或直接复用内核的路径规则（`Registry::path_for` 那一条），不要保留"靠访问顺序预热缓存"的一级近似。

**复核手段**
在 `build_method/face_view.rs` 的既有夹具里，把子面造在父面**之前**（名字排序在前即可），断言子面 `path == "root/<父槽位>/<子槽位>"`；或对同一棵源码树分别跑 `face_views` 与（注册这两个快照后的）`Registry::path_for`，逐面比较。

---

### S2 · [MAJOR] · 大逻辑／构建期 IO 错误路径 · 货币凭据先于载荷发布，且载荷不是一次事务

**file:line**
- `build_method/pipeline.rs:155`～`build_method/pipeline.rs:164`（先写 `discovery.fingerprint`；失败时删指纹用 `let _ =` 吞掉错误）
- `build_method/pipeline.rs:174`～`build_method/pipeline.rs:184`（五份载荷依次写，逐个收集错误，彼此没有事务关系）
- `build_method/cache.rs:36`～`build_method/cache.rs:39`（`write_if_changed` 用 `fs::write` 就地截断重写，不是临时文件 + rename）
- 读取方：`build_method/scope_view.rs:39`～`build_method/scope_view.rs:49`（`build_output_is_current` 只读指纹）、`cli/explain.rs:122`、`mcp/evidence.rs:51`～`mcp/evidence.rs:53`、`mcp/tree_delta.rs:73`

**现象**
"这次构建的产物是否描述当前源码"这件事，由 `discovery.fingerprint` 一枚凭据回答；而这枚凭据是在载荷之前写的。任何一份载荷写失败（只读文件、配额、中断）时：构建脚本路径会 panic（构建失败，安全），但**结构化调用方**（`check --json`、MCP/Studio 的 `check_for`）拿到的是"一条 `out-dir` 诊断 + 退出"；`OUT_DIR` 里留下的却是**混代状态**——指纹是新的，而失败那一份清单还是旧的或缺失，其余几份已经换成新的。下一次由另一个进程读 `build_output_is_current` 会得到 `true`，于是把旧行当成当前事实（`pruning_manifest.tsv` 那一列正是维护者在发布剪枝前读的东西）。失败时删指纹那一支还带 `let _ =`：删不掉就继续带着一枚旧指纹往下走，方向同样是"把旧产物当新产物"。

**判据**
- `write_if_changed` 用 `fs::write`，在写入过程中失败会留下**截断的目标文件**（`build_method/cache.rs:38`），`generated_lib.rs` 也在这一支里（`build_method/pipeline.rs:179`）。
- 载荷数组与指纹写入之间没有任何"全部成功才提交"的顺序保证：`build_method/pipeline.rs:155` 的指纹写发生在这五份载荷之前。
- 反证这处不是有意设计：`build_method/pipeline.rs:147`～`build_method/pipeline.rs:154` 的注释明确说"只有干净的一次运行才发布这枚凭据……没有那枚凭据，没有任何东西会把它们读作描述当前源码"——注释描述的语义要求"载荷全部落地"才算干净，而代码只检查了 `compile_errors`（校验错误），没有检查 `write_errors`。

**最小修复方向**
把载荷写在前、指纹写在最后，且只在 `write_errors.is_empty()` 时发布；删指纹失败上报为诊断而不是 `let _ =`；`write_if_changed` 改成写唯一临时文件后 rename（与 `run_method/authoring/filesystem/filesystem.rs:18` 同一模式），这样"文件存在"就等于"文件完整"。

**复核手段**
把 `out_dir/pruning_manifest.tsv` 的父目录或文件权限改成不可写，跑 `check_for` 或 `xirang check`，断言：指纹不存在、且 `build_output_is_current` 返回 `false`；再另跑一次，断言 `pruning_manifest.tsv` 不出现半截内容。

---

### S3 · [MAJOR] · 假实现／机械清点 · 剪枝清单的符号列由硬编码文本扫描产生，其中一支把符号写死成 `Button::`

**file:line** `build_method/manifests.rs:200`～`build_method/manifests.rs:215`（`parse_pruning_item`），关键行 `build_method/manifests.rs:208`、`build_method/manifests.rs:210`

**现象**
`pruning_manifest.tsv` 的第三列（symbol）不是从已解析的注册面/静态计划里读出来的，而是逐行匹配三个魔法标识符得到的：`PRUNING_TABLE`、`pruning_probe`、`optional_pruning_probe`。第三个分支**无条件**返回字符串 `"Button::optional_pruning_probe"`——与当前是哪个面无关；第一个分支只要某行包含 `PRUNING_TABLE` 且以 `static `/`pub static ` 开头（注释、字符串里也算）就算命中。

**判据**
- 这三个标识符在整个仓库里只出现在这一处（`grep -rn 'PRUNING_TABLE\|pruning_probe' build_method run_method cli mcp studio core docs` 只命中 `build_method/manifests.rs:200` 起的这段函数），没有任何注册面声明它们，也没有测试覆盖它们。
- 这份产物确实被人当作事实读：`build_method/graft_view/plan_rows.rs:118`～`build_method/graft_view/plan_rows.rs:119` 的注释说"这个数字正是维护者在发布剪枝前读的东西"；`cli/explain_report.rs:131` 与 `mcp/evidence.rs:53` 会把 `PruningRow.symbol` 渲染出来。
- 一旦某个不叫 `Button` 的面定义了 `fn optional_pruning_probe`，产物给出的符号就是别的面的名字——一条**错误证据**，而且没有任何东西能发现它错了。

**最小修复方向**
符号应当来自已解析/已注册的事实：至少把前缀换成该面的 module 或 kind（`format!("{}::optional_pruning_probe", kind)`），并把"哪些符号会被发布期修剪"的约定写进文档与测试；更彻底的做法是直接从 kernel 的静态计划/注册快照读被跟踪符号，删掉这条行扫描。

**复核手段**
加一个不叫 `Button` 的面定义 `fn optional_pruning_probe`，断言 `pruning_manifest.tsv` 里的符号带该面的 kind；再断言注释里出现 `PRUNING_TABLE` 字样不会产生行。

---

### S4 · [MAJOR] · 边界（同一规则两处实现） · `function_manifest.tsv` 用第二套函数扫描器，内核已有词法扫描器

**file:line**
- 第二套实现：`build_method/manifests.rs:111`～`build_method/manifests.rs:162`（`collect_function_symbols`）与 `build_method/manifests.rs:164`～`build_method/manifests.rs:171`（`function_name`，`line.find("fn ")`）
- 内核实现：`core/src/registry_core/source/source.rs:120`（`function_symbols`），已被 `mcp/index.rs:199` 与 `studio/studio/app/source_index.rs:90` 消费
- 写出方：`build_method/pipeline.rs:176`

**现象**
"这个文件声明了哪些函数"这一个问题有两份答案。构建期这份是按行文本扫描：`fn ` 出现在注释、文档示例、字符串里都会命中；impl 区块的归属靠"上一行以 `impl ` 开头且不以 `}` 结尾"来猜（`build_method/manifests.rs:127`～`build_method/manifests.rs:137`），块内嵌套的 `impl`/闭包会污染归属。内核那份是词法扫描，两个工具面都在用。

**判据**
- 两份实现的存在可以直接对照：`core/src/registry_core/source/source.rs:120` 是 `pub fn function_symbols(source: &str)`，而 `build_method/manifests.rs:164` 自己又写了一个 `function_name(line)`。
- 这份产物的**树内读者为零**：`grep -rn function_manifest` 只命中写入侧（`build_method/pipeline.rs:6`、`build_method/pipeline.rs:176`、`build_method/lib.rs:121`、`build_method/manifests.rs:24`、`build_method/manifests.rs:31`），CLI/MCP/Studio 都不读它。也就是说分叉不会马上显现，只会等到树外读者信任它时暴露。
- 该产物的内容是会被别的面报告的（既有的 3p 审计 KN5 就是拿 `function_manifest.tsv` 举例说明错误符号会流进构建产物），所以它不是可以放着不管的死产物。

**最小修复方向**
调用 `xirang::source::function_symbols`（它已经处理了标识符边界与 impl 归属），删掉本地扫描；如果确认无人读这份产物，就在同一轮里删掉写入，别留一个会分叉的第二答案。

**复核手段**
对一个含 `// fn fake()` 注释、`let s = "fn fake";`、以及 `impl A { fn b() {} }` 的文件，同时跑内核扫描器与构建期扫描器，断言两者输出一致。

---

### S5 · [MAJOR] · 大逻辑／数据损坏 · authoring 迁移用无边界的文本替换改写子树内每个 `.rs`

**file:line** `run_method/authoring/operations/migration.rs:179`～`run_method/authoring/operations/migration.rs:181`

**现象**
重命名模块时，迁移器对子树里**每个** `.rs` 文件做两次 `String::replace`：`crate::<旧模块路径>` → `crate::<新模块路径>`，以及 `src/<旧目录>/` → `src/<新目录>/`。两者都是**字符串前缀**匹配，没有 `::` / `/` 边界，也不区分注释与字符串字面量。

**判据**
- 同一仓库的其它位置明确为这个边界写过规则与注释：`build_method/scope_view.rs:109`～`build_method/scope_view.rs:116`（"边界是 `::` 段，不是文本前缀：`control` 保留 `control::object::button`，绝不保留 `control_extra`"），scope 的子树选择器同理。迁移路径没有这层保护，属于同一工程里的两种口径。
- 可复现：树里有兄弟模块 `control_extra`，把 `control` 迁成 `widget`。子树里任何 `crate::control_extra::NODE_ID`（`parent:` 字段、类型化 graft 切口）会被改写成 `crate::widget_extra::NODE_ID`；任何 `src/control_extra/...`（例如 `registry_rule_path:` 的取值）会被改写成 `src/widget_extra/...`。改写是静默的（只比较"是否与原文本不同"才写盘，`run_method/authoring/operations/migration.rs:188`），不校验替换结果是否仍是同一个东西。
- 触发条件是"名字互为前缀"，这在真实注册树里很常见（`control` / `control_v2`、`panel` / `panel_extra`）。

**最小修复方向**
不要对整棵树做文本替换：迁移只应改本面自己渲染出来的内容（父级引用与规则路径都在 `FaceManifest` 的字段里），后代面的 `parent:` 指向的是 `crate::<module>::NODE_ID` 这类**常量路径**，模块目录改名本来就不需要动它们。若确实要改，也必须在 `::` / `/` 边界上匹配（或走 token 级重写）。

**复核手段**
建一个含 `control_extra` 兄弟模块的夹具，在 `control` 子树里放一处 `crate::control_extra::NODE_ID` 引用，迁移 `control` 后断言那处引用**未变**；再断言 `crate::control::NODE_ID` 的引用变成了新模块。

---

### S6 · [MAJOR] · 边界／假实现 · `FaceManifest.values` 的"声明字段集合"与实际写出集合不一致

**file:line**
- 写入方只写固定子集，唯一的"无法复现则拒绝"保护只覆盖 `plugin`：`run_method/authoring/manifest/face/render.rs:22`～`run_method/authoring/manifest/face/render.rs:28`，模板在 `run_method/authoring/manifest/face/render.rs:243`～`run_method/authoring/manifest/face/render.rs:245`
- `edit()` 的白名单里有三个模板从不写出的字段：`run_method/authoring/manifest/face/face.rs:100`（`registry_name`）、`run_method/authoring/manifest/face/face.rs:92`（`handle`）、`run_method/authoring/manifest/face/face.rs:97`（`params`）
- 解析侧塞入 `handle`：`run_method/authoring/manifest/parse/parse.rs:108`～`run_method/authoring/manifest/parse/parse.rs:115`
- 新建面时预置更多不写出的键：`run_method/authoring/face_manifest.rs:27`～`run_method/authoring/face_manifest.rs:65`（`registry_name`、`handle`、`params`、`provided_parts`、`required_parts`、`namespace`、`parent_node`、`parent_kind`）
- 幂等性钉子只打在手工构造的字段表上：`run_method/authoring/operations/authored.rs:184`～`run_method/authoring/operations/authored.rs:227`

**现象**
`FaceManifest.values` 同时装着三类东西：(a) 渲染器会写回文件的字段，(b) 派生元数据（`namespace`/`parent_node`/`parent_kind`/`provided_parts`/`required_parts`），(c) 编辑器声称可改、但模板从不发射的字段（`registry_name`/`handle`/`params`）。三者没有任何标记区分，维护者无法从代码看出"哪些字段真的会落盘"。

**判据**
- `plugin` 这一支确立了本仓库自己的标准：**重写无法复现的字段要响亮拒绝，不许静默删掉**（`run_method/authoring/manifest/face/render.rs:13`～`run_method/authoring/manifest/face/render.rs:20` 的注释正是这么写的）。同一个模板对 `registry_name`/`handle`/`params` 没有任何对应保护。
- `handle` 这一支可以确定是死代码：字段词表由 `core/src/registry_core/syntax/tokens.rs:125` 的 `FACE_FIELD_ORDER` 把关，`handle` 不在词表里，所以 `face.field("handle")` 永远拿不到值，`run_method/authoring/manifest/parse/parse.rs:109` 插进去的必定等于 `kind`；而 `registry_name`/`params` 在任何 `edit()` 调用点都不可达（两个字段顺序表 `run_method/authoring/operations/face_write.rs:32`～`run_method/authoring/operations/face_write.rs:80` 都不含它们）。结论：这三个白名单项是**永远无法生效的契约**，而不是被使用的能力。
- 幂等性测试覆盖不到这一点：它用 `BTreeMap::from([...])` 手搓字段表（`run_method/authoring/operations/authored.rs:186`～`run_method/authoring/operations/authored.rs:213`），走的是 `from_values → as_patch` 的字段搬运，**没有解析文件、没有应用、没有渲染**，所以"解析得到的字段能否活过重写"这件事根本没被验过。

**最小修复方向**
二选一并写清：要么让 `render_source` 发射 `edit()`/解析接受的每个字段（并给解析→应用→渲染加一条真实文件往返测试），要么把 `registry_name`/`handle`/`params` 从 `edit()` 白名单、`parse` 与 `FaceManifest::new` 里删掉，并把"派生元数据"键和"可落盘字段"键用不同命名（例如统一前缀）区分开。

**复核手段**
给 `authored.rs` 加一条真正的文件往返测试：写一个带全部可选字段的面文件 → `FaceManifest::parse_source` → 应用一个 identity 补丁 → `render_source` → 重新解析，断言字段集合不变。

---

### S7 · [MAJOR] · 命名／边界 · `compile_error_demo` 这个魔法目录名会悄悄改写任意宿主的编译结果

**file:line** `build_method/renderer/tree.rs:68`～`build_method/renderer/tree.rs:71`（顶层面叫这个名字就套 `#[cfg(feature = "compile_error_demo")]`），以及同名的四处特判：`build_method/registration_check.rs:87`、`build_method/contracts.rs:78`、`build_method/static_plan.rs:74`、`build_method/renderer/pass.rs:168`

**现象**
凡是顶层目录名正好叫 `compile_error_demo` 的子树，构建期会把它整棵挂到 `#[cfg(feature = "compile_error_demo")]` 之下，同时在注册检查/合同检查/静态计划里默认跳过它。这是本仓库早期演示用的目录名，被写死进了构建规则。

**判据**
- 该名字在本工作树里没有任何声明者：既没有 `compile_error_demo/` 目录，也没有任何 `Cargo.toml` 声明 `compile_error_demo` 特性（`grep -rn compile_error_demo` 只命中上述四处构建代码与它们的注释）。
- 对第三方宿主：如果它恰好有一个叫这个名字的注册目录，而它没有声明该特性，那些面会被**静默门控掉**（不编译、不进静态计划），而构建不会因此报错；`cargo::rustc-check-cfg` 只声明了 `cfg(rust_analyzer)`（`build_method/pipeline.rs:210`），所以这处特性名还是未声明的 cfg。
- 与 S8 同型：执行面里带着本仓库夹具/演示的名字当规则。

**最小修复方向**
把这个特例收进配置（例如由脚手架模板显式写入一个 `demo` 标记，或干脆移到 examples 一侧），至少要在 `AGENTS.md` 与生成物注释里说明"这个目录名是保留字"，并在遇到时给一条诊断而不是静默门控。

**复核手段**
在一个宿主里建 `src/compile_error_demo/compile_error_demo.rs`，不声明同名特性，断言生成的 `generated_lib.rs` 里该模块不带 `#[cfg(feature = ...)]`（或断言构建给出明确诊断）。

---

### S8 · [MAJOR] · 假实现／硬编码 · authoring 的 `source` 回落链里写死了本仓库夹具目录名

**file:line** `run_method/authoring/parse/parse.rs:31`（`let roots = ["compile_error_demo", "control", "engine", "trimmed_core"];`），使用点在 `run_method/authoring/parse/parse.rs:32`～`run_method/authoring/parse/parse.rs:39`

**现象**
`source_path_from_file` 优先相对 `source_root()` 求路径；求不出来时，它会在**绝对路径**里寻找名为这四个之一的路径分量，并把该分量之后的全部内容当成相对路径返回。

**判据**
- 这四个名字是仓库自己的夹具/演示模块名（`grep -rn '"engine"' run_method/src` 只命中这里；`trimmed_core` 同样只在这里）。库代码里的这个启发式对任何其它宿主都是错的：`/data/control/myrepo/src/foo/foo.rs` 会被读成 `myrepo/src/foo/foo.rs`（正确应是 `foo/foo.rs`），而这个值就是 `FaceManifest.values["source"]`，直接决定身份输入与清单里的 `source` 列（`core/src/registry_core/identity/path_text.rs:123` 的 `manifest_relative_source` 是为这件事准备的、不依赖硬编码的规则）。
- 触发条件是"文件不在 `source_root()` 之下"，而这正是 Studio 打开一个非标准布局工程、或 `XIRANG_PACKAGE_ROOT` 指向别处时的情形，不是测试专属路径。

**最小修复方向**
删掉这份名字清单：路径不在 `source_root()` 下时应当报错（或按 `manifest_relative_source` 的规则折叠 `src/` 前缀），而不是猜一个分量。若确有历史夹具需要兼容，把它作为参数/配置传入，不要写进库的通用路径。

**复核手段**
对 `/tmp/control/proj/src/foo/foo.rs` 这类路径调用 `source_path_from_file`，断言结果不是 `proj/src/foo/foo.rs`。

---

### S9 · [MAJOR] · 大逻辑／边界 · `linked` collector 分支不提交任何东西，模块文档却说 collector 决定进哪个链接段

**file:line**
- 文档承诺：`run_method/macros/face_registration.rs:9`～`run_method/macros/face_registration.rs:10`（"collector 标识决定注册信息进入哪个链接器段"）
- 实际三个分支：`run_method/macros/face_registration.rs:163`（`development` 空）、`run_method/macros/face_registration.rs:168`～`run_method/macros/face_registration.rs:171`（`linked` 空）、`run_method/macros/face_registration.rs:164`～`run_method/macros/face_registration.rs:167`（`debug` 才真的 `submit!`）
- `linked` 还是外部声明形式的默认值：`run_method/macros/face_external.rs:51`

**现象**
三个分支里只有 `debug` 会真的向链接段提交；`linked`（外部面省略 `collector:` 时的默认）展开为空，`development` 也展开为空。于是"collector 决定注册进哪个链接段"这句话对三分之二的取值不成立，而外部面的默认取值正是那个什么都不做的 `linked`。作者看到 `collector: linked` 这个名字、再读模块文档，会得出"已经进了链接段"的错误结论。

**判据**
- `debug_method/src/collector.rs:8`～`debug_method/src/collector.rs:21` 的 `registrations!` 在非 debug 构建下返回空迭代器，与"`debug` 才收集"这条一致；但没有任何地方说明 `linked` 并不提交。
- 文档与实现的矛盾在同一份文件里：`run_method/macros/face_registration.rs:9` 的承诺 vs `run_method/macros/face_registration.rs:168` 的空展开，中间没有"仅 `debug` 会提交"的限定。
- 该分支的注释只写了"收集归 xirang-debug 所有"（`run_method/macros/face_registration.rs:169`），既没解释为什么 `linked` 什么都不做，也没有把注意力引到"这个名字表达的是'由别处收集'"。

**最小修复方向**
二选一：(a) 若 `linked` 只是"不收集"的别名，改名（例如 `off` / `none`）并在模块文档里列出每个取值的真实效果；(b) 若 `linked` 本应提交到某个链接段，就在这一支里实现它。无论哪种，`collector: $collector:ident` 的取值集合都应当是编译期可枚举、可拒绝非法名的（现在是任意 ident，写错名字会掉进"没有匹配 arm"的裸 marco 错误）。

**复核手段**
对 `collector: linked` / `collector: development` / `collector: bogus` 各写一个声明，断言：前两者不产生 inventory 项（`registrations!` 为空），第三者给出明确的编译错误而不是 `no rules expected`。

---

### S10 · [MINOR] · 一致性／宏卫生 · 前端回退臂用绝对 crate 路径而不是 `$crate`

**file:line** `run_method/macros/face_objects.rs:169`（`::xirang_run_method::face_fields! { … }`），对比同层其它臂的 `$crate::` 写法（`run_method/macros/face_objects.rs:10`、`run_method/macros/face_external.rs:48`）

**现象** 只有在"字段顺序不同/用 `;`/漏分隔符/拼错字段"时才会走到的容错回退臂，硬写了 `::xirang_run_method`。

**判据** 宿主把依赖改名（`run = { package = "xirang-run-method", … }`）时，`$crate::` 仍然解析，而绝对路径解析不到；因为这条臂**只**在宽容路径上出现，同一份声明在合法顺序下能编译、换个顺序就报"use of undeclared crate or module `xirang_run_method`"，错误信息指向宏内部而不是作者的行。仓库其它所有展开点都用 `$crate::`（`run_method/macros/face_external.rs:48`、`run_method/macros/face_registration.rs:156`），这里是不一致的那一个。

**最小修复方向** 改成 `$crate::face_fields!`。**复核手段** 在 `run_method/tests/face_fields.rs` 的既有宽容用例里，把依赖以别名方式接入（或直接断言生成文本里没有绝对 crate 名）。

---

### S11 · [MINOR] · 假实现／诊断质量 · `rule_method_strings` 只读第一处调用，且对注释与转义引号不设防

**file:line** `build_method/contracts.rs:193`～`build_method/contracts.rs:206`（`source.find(marker)`、`args.find(')')`、`split('"')` 的奇偶取项），消费点 `build_method/contracts.rs:131`、`build_method/contracts.rs:146`、`build_method/contracts.rs:170`

**现象** 这是一段手写的 Rust 源文本扫描器，用来读父子规则里的 `require_preset` / `require_exports` / `require_handle_traits` / `require_part_traits` 参数。它只取**第一次**出现的 `.方法名(`；参数区间以第一个 `)` 结束；字符串用 `split('"')` 后取奇数下标。

**判据**
- 同一方法调用两次（例如两段 `.require_exports(&[…])`）时，第二处的需求被静默丢弃；参数里出现 `)`（函数调用、元组、闭包）时区间被截断；字符串里出现转义引号 `\"` 时奇偶会整体错位，此后读到的"需求"是源码里的另一段文本；注释里的 `.require_exports("x")` 会被当成真实需求，从而让构建报出一条不存在的违约。
- 影响面被"真钉子"限制住了：五类需求都有编译期检查兜底（`core/src/registry_core/release/release.rs:291` 起 `assert_static_registration`，由 `build_method/renderer/pass.rs:73` 发射成 `const _`），所以**漏读**只会让错误信息变差；但**误读**（注释/转义引号那一类）会让构建失败并指向一个作者没写过的需求，属于假实现型诊断。

**最小修复方向** 用 `syn` 解析规则源（本 crate 已依赖 `syn`：`build_method/static_plan.rs:158` 就在用 `syn::parse_str`），或至少遍历所有出现处、跳过注释与字符串字面量。**复核手段** 一条规则里对同一方法调用两次、并在上方放一行注释示例，断言两处需求都被检查且注释不产生需求。

---

### S12 · [MINOR] · 契约／静默降级 · trace artifact 的解析顺序依赖只降级不报错

**file:line** `run_method/runtime/trace/artifact/parse.rs:21`～`run_method/runtime/trace/artifact/parse.rs:26`（文档："记录可以任意顺序出现"）、`run_method/runtime/trace/artifact/parse.rs:122`～`run_method/runtime/trace/artifact/parse.rs:124`（局部值的 `function` 从已读入的帧取，取不到写 `"<local>"`）、`run_method/runtime/trace/artifact/parse.rs:147`（边的 `function` 从已读入的局部值取，取不到写 `"<runtime>"`）

**现象** 文档说记录顺序自由，实现却让两条引用解析依赖前文：局部值先于其帧出现时，`source.function` 变成 `"<local>"`；边先于其局部值出现时变成 `"<runtime>"`。文档后半句确实补了这层依赖，但**行为是静默降级**：文件解析成功、`into_trace` 的引用完整性检查（`run_method/runtime/trace/artifact/artifact.rs:263`～`run_method/runtime/trace/artifact/artifact.rs:290`）也不看 `function`，于是错掉的函数名会一直留在证据里，直到有人拿它做归属判断。

**判据** 规范渲染（`run_method/runtime/trace/artifact/artifact.rs:198`）总是"帧→局部值→边"，所以自产文件不受影响；受影响的只有手写/第三方产出的 artifact，而格式是带版本的公开契约（`TRACE_ARTIFACT_VERSION`），手工编辑是被鼓励的（`parse.rs` 的注释明确说顺序自由）。**最小修复方向** 要么在引用未就绪时报 `Malformed`（诚实），要么两遍解析先建帧表，再用帧表补全局部值与边。**复核手段** 把规范文档的行顺序打乱成"局部值在前、帧在后"，断言解析结果与规范顺序逐字段相等（或明确报错）。

---

### S13 · [MINOR] · 命名／可读性 · `render_call_report` 的名字与文档承诺"调用树"，实现恒为空树

**file:line** `run_method/call_report/call_report.rs:20`（函数签名与文档首句）、`run_method/call_report/call_report.rs:24`（内部 `CallTrace::new()`）

**现象** 文档首句是"渲染完整的逻辑调用树"，但函数体第一件事就是造一个**空** trace，因此 `matching_call_paths`（`run_method/call_report/call_report.rs:183`）永远拿不到帧，函数只能走"注册面清单"那一支。实现注释解释了这个取舍（`run_method/call_report/call_report.rs:21`～`run_method/call_report/call_report.rs:23`），但文档首句没有跟着改，调用方按名字理解会拿不到它以为的东西；而且 `CallTrace::new()` 顺带读了进程环境（`run_method/runtime/trace/call_trace.rs:100`～`run_method/runtime/trace/call_trace.rs:102`），一个报告函数读环境也是没必要的副作用。

**最小修复方向** 改名（例如 `render_registration_report`）或把文档首句改成事实：这个入口只给"没有执行证据时"的注册面报告，调用树必须由调用方传 trace（`render_call_report_for_trace`）。**复核手段** 一条断言：`render_call_report` 的输出永不包含树形前缀 `|--` / `` `-- ``（今天的实现成立，改名/改文档后这条钉子仍在）。

---

### S14 · [MINOR] · 静默吞错／回滚可靠性 · 失败回滚链里的每一个 `let _ =`

**file:line** `run_method/authoring/operations/create.rs:101`～`run_method/authoring/operations/create.rs:106`（写源文件失败时只 `let _ = fs::remove_file(&rule)`，且**已建的模块目录不删**）、`run_method/authoring/operations/operations.rs:374`～`run_method/authoring/operations/operations.rs:393`（两处 `let _ = atomic_write(…)` 回滚）、`run_method/authoring/operations/migration.rs:218`～`run_method/authoring/operations/migration.rs:225`（`rollback_migration` 全用 `let _ =`）

**现象** 这些回滚点全部丢弃自己的错误。回滚失败时，调用方只会看到最初那条错误，并相信"已经回退干净了"；实际磁盘上可能是"新面已写入、规则文件没删掉"或"目录已改名、内容已恢复一半"的状态。`create_module` 还会在 `fs::create_dir_all(source.parent()…)` 之后、写文件失败时留下一个空目录（`run_method/authoring/operations/create.rs:87`）。

**判据** 同一仓库对"半成品"的立场是明确的：脚手架路径把回滚失败明确写进错误文本（`build_method/scaffold/project.rs:252`～`build_method/scaffold/project.rs:261` 会说 `a partial project was left in …` / `the partial project was removed`，`build_method/scaffold/project.rs:259` 也是 `let _ =`，但它把结果写进了给用户的消息）。authoring 这边没有对应的措辞，回滚是否成功是不可知的。**最小修复方向** 回滚失败时把原因拼进返回的错误（"回滚未完成：…"），并清理自己创建的空目录。**复核手段** 把规则文件设为不可删（或只读目录），触发回滚分支，断言错误文本里出现回滚未完成的信息。

---

### S15 · [MINOR] · 死逻辑／可读性 · `edit()` 里一个恒等分支

**file:line** `run_method/authoring/manifest/face/face.rs:144`～`run_method/authoring/manifest/face/face.rs:148`

**现象** `let key = if field == "admission" { "admission" } else { field };` 与 `field` 完全等价，没有任何注释解释为什么要有这一步。**判据** `field` 就是 `&str`，两个分支值相同；这个分支只增加读者停顿（读到这里会去找 admission 与其它字段的差异，而差异不存在——真正的差异在上面的校验分支里）。**最小修复方向** 删掉，直接用 `field`；如果它曾是"白名单键归一化"的遗留，把那件事写成一条测试。**复核手段** 删掉后跑 `cargo test -p xirang-run-method --offline`（authoring 面现有测试覆盖两条入口的字段应用）。

---

### S16 · [MINOR] · 库内 panic 面清点（生产代码路径）

**file:line**（全部为 `#[cfg(test)]` 之外的生产代码）
- `run_method/runtime/trace/locals/call_trace.rs:76`（`.expect("local was just pushed")`）
- `macro/src/front_end.rs:69`（`.expect("a part is always open")`）
- `macro/src/lib.rs:137`（`.expect("the derived rule template is static")`）
- `build_method/build_input.rs:30`～`build_method/build_input.rs:31`（`CARGO_MANIFEST_DIR` / `OUT_DIR` 的 `expect`）
- `build_method/static_plan.rs:88`、`build_method/static_plan.rs:116`（"活跃注册面一定有源文件 / 一定有身份"）
- 渲染器与清单写入方里大量 `writeln!(…).unwrap()`（例：`build_method/renderer/tree.rs:73`、`build_method/manifests.rs:106`、`run_method/call_report/call_report.rs:87`）

**现象/判据** 逐条看过，结论是**都可接受但要记账**：`writeln!` 的目标是 `String`（`std::fmt::Write` 不可失败），`macro` 的两处只作用在自己造出的常量 token/模板上，`build_input` 的两处只在 Cargo 真的没给环境变量时触发（那种情况下构建脚本本来也没法继续），`static_plan` 的两处由前面的 `face_source_is_active` 与 `parsed_face` 谓词保证。真正的风险点是"谓词与 `expect` 之间的距离"：`build_method/static_plan.rs:85` 的谓词和 `build_method/static_plan.rs:88` 的 `expect` 之间隔着一次 `node.file` 的读取，任何人以后放宽谓词都会把它变成 panic 面。**最小修复方向** 把这两处 `expect` 换成 `continue`（跳过并让别处的诊断说话），或在注释里显式声明这条不变量由哪一行保证。**复核手段** `grep -n 'expect(' build_method/static_plan.rs` 只剩注释里的引用。

---

### S17 · [MINOR] · 诊断质量 · 一条 `compile_error` 消息里带 18 个连续空格

**file:line** `macro/src/lib.rs:283`

**现象** 字符串字面量在源码里被换了行（缩进进了字面量），于是作者看到的编译错误是：`a generated host alias                  records the file itself`。**判据** 该字符串是给作者看的（`compile_error!` 的文本），而同一函数的其它消息都由 `format!` 拼出、没有这种空洞。**最小修复方向** 用 `\` 续行或 `concat!`，把消息压成一行。**复核手段** 对 `@control` 目标写一条 `source:` 字段，断言错误文本里不含连续两个以上空格。

---

### S18 · [MINOR] · 一致性 · `build_output_is_current` 用 `root/src` 复算指纹，而管线用布局解析出的身份基准

**file:line** `build_method/scope_view.rs:43`～`build_method/scope_view.rs:48`（`root.join("src")` + `discover_root`）对比 `build_method/pipeline.rs:31`～`build_method/pipeline.rs:32`、`build_method/pipeline.rs:56`（`layout.identity_base` + `discover_root_reporting`）

**现象** 指纹是以"身份基准"为参数算出来的；读取方却固定按 `root/src` 复算。对库目标不在 `src/` 下的包（`build_method/source_layout.rs` 支持的布局，`build_method/tests/outside_src_layout.rs` 有夹具），两侧基准不同，读取方永远得到"产物不是当前的"。**判据** 方向是保守的（退回"再问构建一次"，不会把旧产物当新产物），所以不是错答；但同一个包会永远拿不到 `known: true` 的作用域/修剪证据，`explain` 与 MCP 的 evidence 会长期显示 unknown。**最小修复方向** 读取方也走 `source_layout` 解析基准，或用写入方记录的基准确认。**复核手段** 对 `build_method/tests/outside_src_layout.rs` 的夹具跑一次 `check_for` 再断言 `build_output_is_current` 为 `true`。

---

### S19 · [MAJOR] · 边界／安全可靠性（内核已有守卫，执行面这一份没有） · `flow_provider` 编辑入口缺 `guard_nesting`，深层类型路径让进程 abort

> 本条由 kernel-auditor 转达，我按自己的口径**独立核实**：结论成立，且失败模式我跑出了实测。

**file:line**
- 缺守卫的一侧：`run_method/authoring/manifest/face/face.rs:140`～`run_method/authoring/manifest/face/face.rs:143`（`syn::parse_str::<syn::Path>(value)`，错误文本是 `flow_provider must be a Rust type path`）
- 有守卫的一侧（同一个值、同一句话、同一个解析调用）：`core/src/registry_core/authoring/parse/flow.rs:107`（`guard_nesting(value)`）→ `core/src/registry_core/authoring/parse/flow.rs:108`～`core/src/registry_core/authoring/parse/flow.rs:109`
- 守卫的动机就写在内核那一侧：`core/src/registry_core/authoring/parse/flow.rs:99`～`core/src/registry_core/authoring/parse/flow.rs:106`（"没有这道守卫时，一条 542 字节的类型路径会以栈溢出 abort 进程，而不是返回错误"）；度量本身在 `core/src/registry_core/syntax/nesting.rs:342`（深度上限 128、线性串上限 1024，见 `core/src/registry_core/syntax/nesting.rs:57` 与 `core/src/registry_core/syntax/nesting.rs:80`）。
- 可达链（代码阅读）：`run_method/authoring/operations/operations.rs:226`（`add_module_from_face`）与 `run_method/authoring/operations/operations.rs:318`（`edit_module_face`）→ `apply_module_face_values` → `run_method/authoring/operations/face_write.rs:131` → `run_method/authoring/operations/face_write.rs:168`（`face.edit(field, value)`）→ `run_method/authoring/manifest/face/face.rs:140`；值是公开结构体字段，作者可写：`run_method/authoring/operations/operations.rs:191`（`ModuleFacePatch.flow_provider`）与 `NewModuleFace` 的同名字段，取值入口在 `run_method/authoring/operations/face_values.rs:131`。

**现象**
同一个校验存在两份：内核那份先量嵌套再 `syn::parse_str`，执行面这份直接 `syn::parse_str`。`syn` 是递归下降且没有自己的深度守卫，栈溢出**不是可捕获的 panic**，因此这一支的失败不是"少一条诊断"，而是进程 abort。而且顺序上执行面的这一支先跑：`edit()` 在 `render_source()` 之前执行，而带守卫的 `render_flow_provider` 只在渲染时被调用（`run_method/authoring/manifest/face/render.rs:110`），所以守卫没有机会说话。

**判据（实测）**
我用工作区自己的 `syn` rlib（debug 构建，即编辑器/MCP 调试会话实际链接的那份）构造了最小复现，测的是 `run_method/authoring/manifest/face/face.rs:141` 这一行的调用本身（探针代码与二进制在 `/tmp/xirang-probe-surfaces/`，只读工作区、未改工作区任何文件）：
- 输入形如 `A<A<…>>`（嵌套泛型路径）。8 MiB 栈：深度 200（601 字节）→ 正常返回；深度 300（901 字节）→ `fatal runtime error: stack overflow, aborting`，退出码 134。
- 256 KiB 栈：深度 100（301 字节）即 abort。
- 结论：阈值是"几百字节"量级，与内核注释里记的 542 字节同阶；一份手写或代理生成的畸形 `flow_provider` 就能让进程死掉，而不是拿到一条 `Err`。
- 反面（说明这不是"整条链都没守"）：同一条值在渲染侧是安全的，`run_method/authoring/manifest/face/render.rs:110` 调的是内核那个带守卫的入口；`flow` 字段也安全，它走内核的 `parse_flow_value`（`core/src/registry_core/authoring/parse/flow.rs:83`），全程不调 `syn`。

**最小修复方向**
把这一支改成调用内核的 `render_flow_provider`（或先 `guard_nesting` 再 `syn::parse_str`），并让执行面不再保留这份平行的校验；更彻底的做法是把"字段值 → 校验"整族收到内核一处，执行面只调用（这正是 `face_write.rs` 那条"新增字段只接一次线"的注释想达到的状态）。

**复核手段**
给 `run_method` 加一条测试（`FaceManifest::edit("flow_provider", &deep)`，或走 `edit_module_face` 一个补丁），断言深度 300 的输入返回 `Err` 且进程存活；把 `/tmp/xirang-probe-surfaces/probe_syn 300 8388608` 作为失败模式的复现命令留在报告里。

---

## 二、注释清晰度与可读性

（本节的条目按维护者给的判据排序：过时/与实现不符者一律 MAJOR。）

### C1 · [MAJOR] · 文档块与声明错位：`split_semicolons` 的文档挂到了 `splice` 上

**file:line** `macro/src/front_end.rs:18`～`macro/src/front_end.rs:25`（`split_semicolons` 的说明）、`macro/src/front_end.rs:26`～`macro/src/front_end.rs:36`（`splice` 的说明）、`macro/src/front_end.rs:57`（真正的 `split_semicolons`）

**现象** 第一个文档块与第二个文档块之间**没有任何条目**，两个块在同一个 `pub(crate) fn splice` 上合并成一段文档：`splice` 的文档里前半段讲"在顶层 `;` 处切分 token 流"，而 `split_semicolons` 本身没有任何文档（它是 `pub(crate)`，`missing_docs` 看不见）。

**判据** 直接读文件即可验证：`macro/src/front_end.rs:26` 起紧接 `macro/src/front_end.rs:37` 的 `pub(crate) fn splice(`，中间没有第三个条目；`macro/src/front_end.rs:57` 的 `pub(crate) fn split_semicolons` 上方是空行，没有 `///`。**最小修复方向** 把第一个文档块移回 `split_semicolons` 正上方。**复核手段** `cargo doc -p xirang-macro --offline` 或直接 `read` 这一段；若加上 `#![warn(missing_docs)]` 之外的内部文档门禁（conventions 已能扫 doc 注释），这类错位会被自动抓到。

### C2 · [MAJOR] · 同型错位：`collect_rust_sources` 的文档挂在 `StdSourceTree` 上

**file:line** `build_method/discovery.rs:234`～`build_method/discovery.rs:238`（两段文档 + `struct StdSourceTree;` 紧接其后）、`build_method/discovery.rs:261`（真正没有文档的 `collect_rust_sources`）

**现象** "Every `.rs` file under `directory`, following the crate's own layout." 与"`directory` 下的每个 `.rs` 文件…"这两行描述的是 `collect_rust_sources`，却位于 `struct StdSourceTree;` 之前；`StdSourceTree` 自己的说明（`build_method/discovery.rs:236`～`build_method/discovery.rs:237`）紧跟其后，于是结构体带着一段讲函数的文档，而函数裸着。**判据** 与 C1 同一机制，两处并存说明这是**习惯性**问题而非偶发。**最小修复方向** 把第一组文档移到 `collect_rust_sources` 前。**复核手段** 同上。

### C3 · [MAJOR] · 注释描述了实现没有的行为：collector 与链接段

**file:line** `run_method/macros/face_registration.rs:9`～`run_method/macros/face_registration.rs:10`（承诺 collector 决定链接段）对比 `run_method/macros/face_registration.rs:163`、`run_method/macros/face_registration.rs:168`～`run_method/macros/face_registration.rs:171`（两个分支什么都不提交）

**现象/判据/修复/复核** 见 S9（那条是行为面，这条是同一处的注释面）。注释还写着"收集由 `xirang-debug` 持有"（真实的 crate 名是 `xirang-debug-method`，见 C5）。

### C4 · [MAJOR] · 编辑器悬停文本与宏契约不符：`parent` 被标成 Required

**file:line** `run_method/macros/face.rs:139`～`run_method/macros/face.rs:140`（"Where this face hangs. Required." / "本面挂在谁下面。必填。"），实现见 `run_method/macros/face_objects.rs:111` 与 `run_method/macros/face_external.rs:132`

**现象** 唯一一条匹配臂让 `parent:` 变成可选，省略时解析到包根（`__face_expr_or!($crate::root_node_id(env!("CARGO_PKG_NAME")); $($parent)?)`）。`FaceFields` 的字段文档正是编辑器的补全说明（`run_method/macros/face.rs:16`～`run_method/macros/face.rs:19` 自己说明了这一点），所以作者会被告知一个不存在的必填项。**判据** 该结构体的每个字段都要求"含义、默认值、一个示例"，`parent` 这一项既没写默认值，又把"必填"写进了含义。**最小修复方向** 改成"省略时取包根"并给出示例。**复核手段** 对照 `run_method/tests/face_arm_defaults.rs` 与 `run_method/tests/kind_only_registry_name.rs` 已钉住的默认值行为。

### C5 · [MAJOR] · 术语不一致：注释里出现两个不存在的 crate 名

**file:line** `run_method/macros/entry.rs:18`、`run_method/macros/entry.rs:24`、`run_method/macros/entry.rs:63`（`xirang-build`）；`run_method/macros/face_registration.rs:169`～`run_method/macros/face_registration.rs:170`（`xirang-debug`）

**现象/判据** 真实 crate 名是 `xirang-build-method` 与 `xirang-debug-method`（`build_method/Cargo.toml:2`、`debug_method/Cargo.toml:2`），lib 名分别是 `xirang_build_method`、`xirang_debug_method`；`AGENTS.md` 的改动规则 4 明确要求 crate 名/目录名/lib 名一致。读注释的人会在仓库里搜不到这两个名字。**最小修复方向** 改成带 `-method` 的全名。**复核手段** `grep -rn 'xirang-build\b\|xirang-debug\b' build_method run_method macro debug_method` 归零。

### C6 · [MINOR] · 同一个字段集合被三处注释写成三个数

**file:line** `run_method/authoring/operations/face_write.rs:85`（"全部 27 个共用字段"）、`run_method/authoring/operations/face_values.rs:6`（"The 28 registration-face fields"）、`macro/src/mirror.rs:286`（"The 29 type arguments of the mirror's literal"）

**现象/判据** 实际数字各不相同且都与注释不符：`ModuleFaceValues` 有 23 个字段（`run_method/authoring/operations/face_values.rs:20` 起），其中真正被写进文件的由两个字段顺序表决定（22 / 20 项，`run_method/authoring/operations/face_write.rs:32`～`run_method/authoring/operations/face_write.rs:80`）；`FaceFields` 有 24 个类型参数（`run_method/macros/face.rs:59` 起）；内核还有第四个词表 `FACE_FIELD_COUNT = 26`（`core/src/registry_core/authoring/face_field.rs:107`）。这些数字是维护者"新增字段要接几处线"的清单，写错会直接误导改动。**最小修复方向** 只保留可被测试钉住的表述（例如"与 `FACE_FIELD_ORDER` 同序"），或干脆删掉数字。**复核手段** 给字段顺序表加一条"表长 == 视图字段数"的测试，注释里的数字随之不再需要。

### C7 · [MINOR] · 复述与残句：`face_objects.rs` 里三段讲"已删除的 arm"的注释互相重复

**file:line** `run_method/macros/face_objects.rs:62`～`run_method/macros/face_objects.rs:87`（中段出现断句："…写另一个、或都不写），因此" 后直接接 "`handle`，所以不带 handle 的自定义 preset/parts 声明会落到这里。"）、`run_method/macros/face_objects.rs:120`～`run_method/macros/face_objects.rs:130`（同一句"作者写下的表达式优先…"连写两遍）、`run_method/macros/face_objects.rs:150`～`run_method/macros/face_objects.rs:156`（"只写 kind 的注册面不需要单独的 arm"三种说法）

**现象/判据** 这三处都在讲"以前有哪些 arm、现在删了"的历史，读者要先建立起一个已经不存在的结构才能读懂；其中中文那句已经断成病句（英文侧的对应句是完整的，说明是编辑过程中掉了一句）。这类注释违反"讲为什么/不变量"的取向，变成历史叙述。**最小修复方向** 每处只留一句"为什么现在只有一条 arm"，把"曾经有过哪几条"移到 crate 级文档或 CHANGELOG。**复核手段** 纯阅读面，改后跑 `cargo test -p xirang-run-method --offline` 保证 `run_method/tests/face_arm_defaults.rs`、`run_method/tests/face_preset_parts.rs` 仍绿。

### C8 · [MINOR] · 该由名字承担的信息被塞进注释：`write_if_changed` 的文档没说它不原子

**file:line** `build_method/cache.rs:34`～`build_method/cache.rs:35`

**现象** 文档只说"内容变化时才写"，没提它是就地写（非原子）。读者据此会在需要事务性的地方放心使用它——`build_method/pipeline.rs:179` 正是这样用的。**判据** 仓库对这件事的标准写法就在对面：authoring 的同名函数（`run_method/authoring/filesystem/filesystem.rs:18`～`run_method/authoring/filesystem/filesystem.rs:47`）用了整整一段注释解释"为什么必须是唯一的临时文件 + rename"。**最小修复方向** 让名字或一行注释把"就地覆盖、可能留下半截"讲出来（`write_in_place_if_changed`），或改成原子写。**复核手段** 见 S2 的复核手段。

### C9 · [MINOR] · 注释密度：反例与正例并存（供维护者判断，不作为缺陷）

- 正例（值得保留的"为什么"）：`build_method/pipeline.rs:105`～`build_method/pipeline.rs:117`（为什么计划文件的未声明槽位是构建错误）、`build_method/graft_view/declared.rs:99`～`build_method/graft_view/declared.rs:118`（为什么写法分类不能从文本推断）、`run_method/runtime/trace/call_trace.rs:175`～`run_method/runtime/trace/call_trace.rs:187`（为什么 id 计数器不能回绕、为什么 `error_scope_depth` 不能清零）、`run_method/runtime/trace/artifact/io.rs:54`～`run_method/runtime/trace/artifact/io.rs:70`（为什么 namespace 是参数而不是读环境）。
- 反例（复述代码）在渲染器里最集中：`build_method/renderer/pass.rs:140`～`build_method/renderer/pass.rs:146` 与 `build_method/renderer/tree.rs:78`～`build_method/renderer/tree.rs:96` 这类块是"把下面 5 行代码翻译成 5 行中英说明"，而真正非直觉的取舍（为什么 `include!` 禁用、为什么 IDE 影子必须存在）反而讲得更长——建议只保留后者。

---

## 三、查了、干净（我逐条核实过、未发现问题的面）

- **core shim 重导出仍在，历史公开路径未断**：`build_method/syntax.rs:6`、`build_method/identity.rs:7`、`run_method/registry/registry.rs:6`～`run_method/registry/registry.rs:7`、`run_method/plugin/plugin.rs:6`～`run_method/plugin/plugin.rs:7`、`run_method/runtime/runtime.rs:11`、`run_method/authoring/parse/parse.rs:15`、`run_method/authoring/validation/validation.rs:15`、`run_method/authoring/face_manifest.rs:89`。棘轮门禁在 `conventions/src/shims.rs:120`；`run_method/tests/path_compat.rs:25` 起把"一个身份经每条历史路径都能解析"钉住了。
- **`host!()` 与 `include!` 边界**：全仓执行面只有一处 `include!`（`run_method/macros/entry.rs:55`），它引入的是 `OUT_DIR/generated_lib.rs`；文件名与 `lexicon::GENERATED_LIB_FILE` 用 `const` 断言钉在一起（`run_method/macros/entry.rs:37`～`run_method/macros/entry.rs:40`），漂移会变成编译错误而不是静默 include 别的文件。生成侧在 `build_method/renderer/pass.rs:37` 起组装。
- **构建期 IO 失败的"两条调用方"分工**：构建脚本路径 panic 并带原因（`build_method/pipeline.rs:196`），结构化路径转成诊断（`build_method/pipeline.rs:198`～`build_method/pipeline.rs:200`）；布局不可读时也走同一条分工（`build_method/pipeline.rs:262`～`build_method/pipeline.rs:269`），替代了过去的裸 `expect`（历史在 `build_method/pipeline.rs:36`～`build_method/pipeline.rs:42` 的注释里）。——除 S2 的"顺序"问题外，错误面本身是完整的。
- **不可读的注册面文件不会被静默丢掉**：`record_unplaced` 对读不了的文件直接返回（`build_method/discovery.rs:156`～`build_method/discovery.rs:161`），但发现树的每个节点文件都会在入口解析阶段被读一次，失败会产出 `entry` 诊断（`build_method/entry.rs:87`～`build_method/entry.rs:96`），所以我最初怀疑的"静默少一个面"不成立。
- **执行面没有重做内核的纯逻辑（除 S1/S4/S8 点名的三处外）**：布局解析委托给 `source_layout`（`build_method/source_layout.rs`）、身份经 `registry_identity` 与 `lexicon`（`build_method/identity.rs:7`）、源码遍历把文件系统事实经 `xirang::source::SourceTree` 注入（`run_method/authoring/operations/operations.rs:257`～`run_method/authoring/operations/operations.rs:280`）、`mir`/`tree`/`plugin` 全是内核类型的 shim。`run_method/lib.rs:29`～`run_method/lib.rs:38` 的 glob 与 `authoring` shim 的重叠是**有意保留**并写在注释里的（`run_method/lib.rs:30`～`run_method/lib.rs:37`）。
- **trace artifact 的写入/解析契约**：转义与反转义对称（`run_method/runtime/trace/artifact/artifact.rs:352`～`run_method/runtime/trace/artifact/artifact.rs:364` 对 `run_method/runtime/trace/artifact/parse.rs:266`～`run_method/runtime/trace/artifact/parse.rs:284`）；未知键与不支持的版本一律拒绝（`run_method/runtime/trace/artifact/parse.rs:157`、`run_method/runtime/trace/artifact/parse.rs:54`）；重建前检查重复 id、缺失父帧、悬空边（`run_method/runtime/trace/artifact/artifact.rs:263`～`run_method/runtime/trace/artifact/artifact.rs:290`）；写入是唯一临时文件 + rename（`run_method/runtime/trace/artifact/io.rs:117`～`run_method/runtime/trace/artifact/io.rs:143`），并会创建自己的输出目录（`run_method/runtime/trace/artifact/io.rs:83`～`run_method/runtime/trace/artifact/io.rs:89`）；"外来快照拒绝盖章"由读取方把关（`mcp/trace.rs:93`、`studio/studio/app/trace.rs:108` 在比对 namespace 后拒绝），写入方的 namespace 由宿主编译期身份传入而不是读环境（`run_method/runtime/trace/artifact/io.rs:57`～`run_method/runtime/trace/artifact/io.rs:70`）。
- **graft 计划与声明状态的交叉判定**：切口↔注册面的匹配规则只有一份（`build_method/graft_view/declared.rs:138` 的 `names_face`），区间性作为数据携带而不是从文本推断（`build_method/graft_view/declared.rs:37`、`build_method/graft_view/declared.rs:163`）；构建期"没有任何声明能命名这个槽位"用的是计划自己记录的 `target_path` 与目标身份反查出的 module（`build_method/graft_plan_check.rs:141`），不依赖 `FaceView.path`（这是 S1 的影响范围没有扩大到"构建拒绝/通过"的原因）。计划目录读不了的条目会被计数成一行而不是消失（`build_method/graft_view/plan_rows.rs:120`～`build_method/graft_view/plan_rows.rs:133`），这条正是本轮工作树改动钉住的行为（新增 `build_method/graft_view/plan_rows_tests.rs:19`）。
- **authoring 执行器的事务写入模式**：新增面先暂存校验再落盘（`run_method/authoring/operations/create.rs:77`～`run_method/authoring/operations/create.rs:85`）、`atomic_write` 用唯一临时文件（`run_method/authoring/filesystem/filesystem.rs:18`～`run_method/authoring/filesystem/filesystem.rs:47`，并有"不删别人的兄弟文件"的测试 `run_method/authoring/filesystem/filesystem.rs:72`）、重写前把旧文本留进 trash（`run_method/authoring/operations/operations.rs:369`～`run_method/authoring/operations/operations.rs:373`、`run_method/authoring/operations/trash.rs:48`～`run_method/authoring/operations/trash.rs:67`）。脚手架也有明确回滚与措辞（`build_method/scaffold/project.rs:245`～`build_method/scaffold/project.rs:263`）。
- **只读视图不碰进程级状态**：`face_view` 明确不查被固定的命名空间，`package` 由调用方传入（`build_method/face_view.rs:16`～`build_method/face_view.rs:20`）；而 `check_for` 用 `run_as_package` 把命名空间作用域化（`build_method/lib.rs:192`～`build_method/lib.rs:205`）。
- **trace 收集器的回滚语义**：id 只发放一次、回滚重建索引而不回绕计数器（`run_method/runtime/trace/call_trace.rs:198`～`run_method/runtime/trace/call_trace.rs:213`）；`clear()` 刻意不清 `error_scope_depth`（`run_method/runtime/trace/call_trace.rs:175`～`run_method/runtime/trace/call_trace.rs:187`）。我把"绕过作用域逃出的 id"当成嫌疑点查了，结论是它**解析不到任何东西**而不是指向错误证据，属于有意设计。
- **debug_method 的收集面**：只在 `debug_assertions` 下 `collect!`（`debug_method/src/lib.rs:34`～`debug_method/src/lib.rs:35`），`registrations!` 在非 debug 构建下返回空迭代器而不是缺失（`debug_method/src/collector.rs:8`～`debug_method/src/collector.rs:21`）；`submit!` 在 `collect!` 缺席时也**能编译**（inventory 0.3.24 的 `__do_submit` 走类型擦除的 `ErasedNode::submit`，见 `inventory-0.3.24/src/lib.rs` 中 `__do_submit` 的定义，本机路径 `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/inventory-0.3.24/src/lib.rs` 的 477 行起），所以 release 构建不会因未收集而失败——这一点我专门查过，不是假实现。
- **macro 前端的容错能力**：`split_semicolons` 只保留三段且组内 `;` 不算分隔符（`macro/src/front_end.rs:57`～`macro/src/front_end.rs:74`）、`splice` 进组替换并保留 `$crate` 卫生（`macro/src/front_end.rs:37`～`macro/src/front_end.rs:55`）、错误都带作者 token 的 span（`macro/src/front_end.rs:122`～`macro/src/front_end.rs:135`）；"重排后仍不匹配"会给出专门的递归错误而不是无限展开（`macro/src/lib.rs:349`～`macro/src/lib.rs:361`）。除了 S17 的空格与 C1 的文档错位，诊断质量整体是好的。
- **嵌套守卫的覆盖面（除 S19 点名的 `flow_provider` 编辑支外）**：内核的解析入口都带守卫（`core/src/registry_core/authoring/parse/parse.rs:217`、`core/src/registry_core/authoring/parse/flow.rs:107`、`core/src/registry_core/authoring/parse/flow.rs:123`），连文档门禁也用它（`conventions/src/doc_blocks.rs:343`）；`flow` 字段的编辑走内核的 `parse_flow_value`（`core/src/registry_core/authoring/parse/flow.rs:83`），全程不调 `syn`，因此不受影响。缺口只有 S19 那一处。

---

## 四、结构观察（给结构架构师汇总用）

1. **深目录的成因**：`run_method/runtime/trace/artifact/` 三分（`artifact.rs` / `io.rs` / `parse.rs`）的注释直接写明"只是为了把每个文件保持在仓库尺寸棘轮之下"（`run_method/runtime/trace/artifact/artifact.rs:9`～`run_method/runtime/trace/artifact/artifact.rs:14`）。也就是说 `x/x.rs` + `x/` 的深层结构有一部分是 600 行棘轮的产物，而不是内聚度的产物——评价目录切分时要先扣掉这一层。
2. **`x/x.rs` 挂载壳**：`authoring/authoring.rs`(38)、`authoring/manifest/manifest.rs`(13)、`authoring/external_graft/external_graft.rs`(10)、`authoring/snapshot/snapshot.rs`(18)、`authoring/filesystem/filesystem.rs`(95)、`operations/operations.rs`(423)、`manifest/face/face.rs`、`manifest/parse/parse.rs`、`parse/parse.rs`、`validation/validation.rs`、`locals/locals.rs`、`edges/edges.rs`、`frames/frames.rs`、`trace/trace.rs` 等，名字与所在目录同名。代价是**同名文件在不同深度重复**：`authoring/parse/parse.rs` 与 `authoring/manifest/parse/parse.rs`；`authoring/face_manifest.rs` 与 `authoring/manifest/face_manifest.rs`；`authoring/manifest/face/face.rs` 与 `macros/face.rs`；`runtime/trace/call_trace.rs` 与 `runtime/trace/locals/call_trace.rs`。搜索/跳转时这四个碰撞点最费时，建议按职责重命名（而不是按目录名）。
3. **`graft_view/` 的内聚**：`declared.rs`（视图类型 + 一条匹配规则）、`matching.rs`（切口→模块映射，但 `build_method/graft_view/matching.rs:74`～`build_method/graft_view/matching.rs:81` 的 `face_declares_plugin` 是**文件系统读取**）、`plan_rows.rs`（计划目录）、`overlay_rows.rs`（投影）、`query.rs`（读宿主入口，412 行）五块里，只有 `matching.rs` 混了纯映射与 IO：纯的那半（`graft_expression_module`）与 `declared.rs` 的匹配规则是同一族，IO 的那半属于构建的 fs 层。读者找"规则在哪"需要知道要翻两个文件。
4. **同名不同功能的两条"边界"**：`build_method/identity.rs`（模块名 `registry_identity`）与 `build_method/node_id.rs`、`build_method/syntax.rs`（模块名 `registry_syntax`）与 `build_method/static_plan.rs::source_module_path`——从文件列表看不出哪些是内核 shim、哪些是本地实现（只有读 `build_method/lib.rs:60`～`build_method/lib.rs:63` 的重命名才知道）。建议在 `lib.rs` 的挂载区把"shim"与"本地"分两段。
5. **公开面与内部面的阅读顺序**：`build_method/lib.rs:24`～`build_method/lib.rs:75` 按模块名排序挂载，`build_method/lib.rs:79`～`build_method/lib.rs:94` 是公开白名单，`build_method/lib.rs:100`～`build_method/lib.rs:132` 是内部再导出，真正的两个入口 `run`/`run_for`/`check_for` 在 `build_method/lib.rs:137`～`build_method/lib.rs:205`（即文件末尾）。对一个"先读入口再往下钻"的读者，最有价值的 45 行在最后一屏，而 110 行挂载在最前面；`run_method/lib.rs` 同样如此（入口宏在 `macros/` 而 crate 根只挂模块）。这是无 `mod.rs` 约定的代价，可在 crate 文档里给一条"先读 `run()`/`host!()`"的导览来补偿。
6. **包边界候补**：`debug_method` 只有 444 行，却独立成包并拉入 `petgraph` 与 `tracing`（`debug_method/src/adapters.rs:6`～`debug_method/src/adapters.rs:8`）——而它的实质是"运行期证据模型 + 两个适配器"，与 `run_method` 的 trace 面强耦合（`debug_method/src/mir.rs:12` 直接从 `xirang_run_method` 再导出）。可考虑并入 `run_method` 的一个特性，或把 petgraph/tracing 变成可选依赖；`inventory` 已经用 `#[doc(hidden)] pub use` 透传（`debug_method/src/lib.rs:25`～`debug_method/src/lib.rs:26`），说明这个包已经有一部分是"转发面"。
7. **同一事实的第二个答案清单（架构层要给结论的地方）**：`FaceView` 的父级解析与逻辑路径（S1）、函数符号扫描（S4）、`collect_object_aliases` 为 `needs_registry` 再解析一遍面文件（`build_method/renderer/aliases.rs:78`～`build_method/renderer/aliases.rs:94`，失败即当 `false`）、`rule_method_strings` 的手写扫描（S11）、迁移期的文本替换（S5）、authoring 的 `source` 回落硬编码（S8）。它们的共同点是"某个已经在内核/构建里算出过的事实，被另一个执行面重新算了一次"。结构上的收敛方向是：**内核只暴露查询，执行面只消费**，每个事实一个出口。
8. **字段词表有四份**（供命名/边界决策）：`FACE_FIELD_ORDER`（24，宏与语法，`core/src/registry_core/declaration/registration.rs:427`）、`FaceFields` 镜像（24，`run_method/macros/face.rs:59`）、authoring 的 `ModuleFaceValues`/两个字段顺序表（23 / 22 / 20，`run_method/authoring/operations/face_values.rs:20`、`run_method/authoring/operations/face_write.rs:32`、`run_method/authoring/operations/face_write.rs:59`）、Studio 表单的 `FACE_FIELD_COUNT`（26，`core/src/registry_core/authoring/face_field.rs:107`）。它们服务的是不同层（宏 / 编辑器 / 表单 / 文件写回），但**没有任何一处声明它们之间的关系**，这正是 C6 的数字各说各话与 S6 的"能编辑但不能写回"得以共存的土壤。

---

## 五、方法与锚点自检

- 阅读方式：对 `build_method/src`、`run_method/src`、`macro/src`、`debug_method/src` 的全部文件按目录逐个 `read`（`renderer/`、`graft_view/`、`scaffold/`、`authoring/`、`runtime/trace/`、`macros/` 为逐行读；`tests/`、`examples/` 为逐文件扫过函数与断言）；另读了四个 counter-part（`core/src/registry_core/tree/query/query.rs`、`core/src/registry_core/source/source.rs`、`core/src/registry_core/declaration/registration.rs`、`core/src/registry_core/authoring/face_field.rs`）以确认"第二份实现"。
- 探针：`grep -rn` 用于"某个名字在仓库里只出现一次/多份实现"这类清点；`git ls-tree`/`git show` 用于 HEAD 与工作树的行数对照；`~/.cargo/registry` 里的 `inventory-0.3.24/src/lib.rs` 用于核实 `submit!` 与 `collect!` 的边界（结论见第三节）。S19 另跑了一个**实测探针**：用工作区自己的 `syn` rlib 编译一个最小程序，直接复现 `run_method/authoring/manifest/face/face.rs:141` 那一行的调用（探针源码与二进制放在 `/tmp/xirang-probe-surfaces/`，属工作区之外；工作区内除本报告外未新建/改动任何文件）。全部命令只读，`cargo` 仅带 `--offline`。
- 锚点自检：本报告每条 `x.rs:NNN` 在写下的同一轮里用 `sed -n`/`grep -n` 打印过该行内容，共提取 326 条，全部文件存在且行号在界内；正文**没有**使用 `` `token`（`x.rs:line`） `` 这种把 token 与行号成对的写法（按 `conventions/src/doc_anchors.rs` 的配对规则实测 0 处），因此不存在"token 与所点行不符"的风险；各条发现里的 token（例如 `parse_pruning_item`、`logical_path`）都单独列在 file:line 旁边而不是塞进括号。行号对应工作树当前内容（`HEAD=cf0c378` + 未提交改动）。
- 未覆盖/未决：`build_method/scaffold/install.rs`（275 行）只做了接口级阅读（脚手架安装路径的回滚由 `project.rs` 那一套覆盖，未见第二套写盘逻辑）；`run_method/examples/scale_audit.rs` 属基准示例，`ceiling` 对环境变量解析失败会静默回落默认值（`run_method/examples/scale_audit.rs:29`～`run_method/examples/scale_audit.rs:34`）——示例允许 `expect`，这条只作为"我看到了"的记账，不计为发现。
