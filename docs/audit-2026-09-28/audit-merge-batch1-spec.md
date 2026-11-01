# 合并批次 1 开工前现算与可执行清单（内核改名 `core/` → `kernel/`）

- 作者：bridge-auditor（t108，**只读任务**：本次唯一写入就是本文件）
- 树：HEAD `4d288ba`（"审计产物:命名批次账本 fix_batch_5 + 锚点刷新"）。**本单开工时工作树是干净的**：命名批次（B7+B8）与 A1 収平的成果已在该 HEAD 里（`git status --porcelain` 除本文件外为 0 行），因此下文的"现算"都是对**已提交**的树读数。
- 只读引用：`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`（下称**方案**，587 行，未改）
- 口径：方案是审计早期写的；命名批次已改了 24 处路径与大量符号 ⇒ 下表把方案的每张表按**当前树**重算，逐处给「旧值 → 新值」。每条计数附可复核命令，并标明 **判别性**（批 1 后应 `>0 → 0`）或**仅计数**（信息性）。
- 计数命令的排除口径：`git` 对象与 `target/` 一律排除；`*.md` 的「活文档集」= 根 `*.md` + `docs/**`（排除 `audit*`/`design*` 分量）+ `*/README*.md`，当前 **33 份**（与 `conventions` 的 `doc_blocks::markdown_files` 同口径）。

## 0. 漂移总表（方案读数 → 当前树现算）

| 方案的表 | 方案值 | **现算值** | 命令（本仓实测） | 性质 |
| --- | --- | --- | --- | --- |
| §3.1 `xirang::` | 237 次 / 88 文件 | **295 次 / 102 文件** | 词边界 `(?<![A-Za-z0-9_])xirang::` 全 `.rs` 计数 | **判别性**（批 1 后 0） |
| §3.1 `xirang_run_method::` | 275 / 93 | **303 / 100** | 同上 | 仅计数（批 2 才改） |
| §3.1 `xirang_build_method::` | 83 / 45 | **83 / 44** | 同上 | 仅计数（批 2） |
| §3.1 `xirang_debug_method::` | 8 / 7 | **10 / 7** | 同上 | 仅计数（批 2） |
| §3.1 `xirang_plugin_host::` | 15 / 5 | **18 / 5** | 同上 | 仅计数（批 2） |
| §3.1 `xirang_studio::` / `_mcp::` / `_cli::` / `_macro::` | 2/2、2/2、5/2、4/1 | **2/2、2/2、5/2、4/1**（未漂移） | 同上 | 仅计数（批 2） |
| §3.2 按目录分布（`xirang::`） | run_method 73 / build_method 49 / mcp 44 / conventions 32 / core 25 / cli 8 / macro 6 | **run_method 81 / build_method 62 / core 52 / mcp 50 / conventions 36 / cli 8 / macro 6**；`examples/`、`studio/`、`plugin-host/`、`debug_method/` **均为 0** | 按顶层目录分组计数 | 仅计数（但「0」很关键：**批 1 不触身份红线区**） |
| §3.5 `#[path]` 声明 | 339 条、非相对 0 | **432 条、非相对 0**（源文件，排除 `target/`） | `grep -rn '#\[path' --include=*.rs . \| grep -v target/` | **判别性**（非相对应恒为 0） |
| §3.5 `#[path]` 非相对 | 0 | **源文件 0**；另有 **8 条落在生成产物** `examples/control-button/target/xirang/out/generated_lib.rs`（绝对路径，构建产物，非源码） | 同上的 `grep -E '#\[path\s*=\s*"(\.\.\|/)'` | 仅计数（`rm -rf examples/*/target/xirang` 后归零） |
| §5.1 `shims` 条目 | 16 条 | **17 条** | 见 §1.6 的解析脚本 | 仅计数 |
| §5.1「针串」字面量 | `"pub use xirang::"` | 文中出现 **20 处**（14 条 `run_method` + 3 条 `build_method` 的 pin 语句写法各异） | `grep -c 'pub use xirang::' conventions/src/shims.rs` | **判别性**（批 1 后应为 0） |
| §5.1 `size` 棘轮基线 | 1 条（`core/src/.../contracts.rs`, 639） | **`BASELINE: &[(&str, usize)] = &[]`（0 条，`conventions/src/size.rs:82`）** | `grep -n 'pub const BASELINE' -A 1 conventions/src/size.rs` | 仅计数（批 1 无路径要改） |
| §5.1 `mounting` 行号 | `mounting.rs:252` | **`mounting.rs:258`** | `grep -n 'core/src' conventions/src/mounting.rs` | 仅计数 |
| §5.2 发布工具层表 | `:86` 起 7 层 9 名 | **`:91`–`:97`（7 行 9 名）** | `grep -n '^xirang-' tools/xirang-publish` | 仅计数 |
| §5.2 发布工具依赖表 | `:323` 起 9 行 | **`:354` 起**（首行 `xirang-macro:xirang-core`） | `grep -n 'xirang-macro:' tools/xirang-publish` | 仅计数 |
| §5.3 CI 包名行 | `ci.yml:55`（`cargo package -p xirang-core`） | **`ci.yml:56`** | `grep -n 'cargo package -p' .github/workflows/ci.yml` | **判别性**（批 1 后该行写 `xirang-kernel`） |
| §5.3 CI `plugin-host` 行 | `ci.yml:180` | **`ci.yml:181`** | `grep -n 'plugin-host --features' .github/workflows/ci.yml` | 仅计数（批 2） |
| §5.3 CI 示例行 | `ci.yml:153` | **`ci.yml:154`** | `grep -n 'ide_mirror' .github/workflows/ci.yml` | 仅计数（不改） |
| §4.4 `RUN_METHOD_CRATE` 定义处 | `lexicon.rs:28` | **`lexicon.rs:63`**（值仍是 `xirang_run_method`；钉子在 `lexicon_tests.rs:15`） | `grep -rn RUN_METHOD_CRATE core/src/registry_core/lexicon/` | 仅计数（批 2） |
| 「带连字符包名出现」（§3.4） | `xirang-core` 80 次 / 38 文件 | **52 次 / 17 文件**（可编辑文件，排除 `.git`、`target/`、记录类 md） | `grep -rc xirang-core` 逐文件 | 仅计数 |

**方案没写到、但批 1 必须处理的三处新事实**（现算发现）：

1. **6 个成员清单各有一行 `xirang-core = { path = "../core" }`**（`build_method`、`cli`、`conventions`、`macro`、`mcp`、`run_method`）⇒ 批 1 的 manifest 面不止方案写的「根 + core 自己」。
2. **活文档里有 25 处 `core/*.rs:line` 锚点**（`run_method/README.md` 12、`run_method/README.zh-CN.md` 12、`docs/roadmap-1.0.md` 1）⇒ 不改就会被 `doc_anchors` 门禁判红（判别性，详见 §1.7）。
3. **根 `Cargo.toml` 没有 `[workspace.dependencies]`**（t14 R-01 未做）⇒ 那 6 行依赖的改名在批 1 必须逐行做；若 R-01 先落地，则只改一处（方案批 0 的建议仍成立）。

## 1. 批 1 可照单执行清单

### 1.1 目录与 manifest

```text
git mv core kernel                                   # 目录改名（方案 §4.1 原样成立）
kernel/Cargo.toml
  package.name  "xirang-core" → "xirang-kernel"   # :2
  [lib] name    "xirang"      → "xirang_kernel"   # :32
  readme / documentation 字段里的 "xirang-core" → "xirang-kernel"（该文件 2 处）
Cargo.toml:2   members  "core" → "kernel"
```

- **6 行依赖**（逐文件，全部形如 `xirang-core = { path = "../core", … }` → `xirang-kernel = { path = "../kernel", … }`，冒号后为行号）：
  `build_method/Cargo.toml:16`、`cli/Cargo.toml:34`、`conventions/Cargo.toml:29`、`macro/Cargo.toml:20`、`mcp/Cargo.toml:40`、`run_method/Cargo.toml:19`。
  复核命令：`grep -rn '^xirang-core' */Cargo.toml`（**判别性**：批 1 后 0）
- **不要动**：`examples/**` 与 `studio/tests/fixtures/**` 的清单（它们只依赖 `xirang-run-method`/`xirang-build-method`，实测 0 处 `xirang-core`；身份红线区因此零改动）。
- 批 1 的 lib 名谓词（方案的 `grep 'lib name.*"xirang"'` 写法在当前树不命中，改为）：`grep -rn -A2 '^\[lib\]' */Cargo.toml | grep -c 'name = "xirang"'` ⇒ 现 **1**（core），**批 1 后应为 0**（判别性）。

### 1.2 代码替换（`xirang::` → `xirang_kernel::`）

- **规模：295 次 / 102 文件**（§0 表）。口径：词边界 `(?<![A-Za-z0-9_])xirang::`，因此 `xirang_run_method::` 等不会被误伤。
- 词边界也决定了**本批唯一一次全局替换**是安全的：目标串 `xirang_kernel::` 与其余 8 个 lib 名互不为前缀（方案 §3.6 第 1 步）。
- 形态清单（都要替换）：
  - `use xirang::…` / `xirang::foo::Bar` 常规路径；
  - **`::xirang::…` 前缀写法**：`run_method/macros/entry.rs:37-38`（`::xirang::lexicon::…`）；
  - 文档注释里的 `xirang::`（同一次 sed 覆盖）。
- 复核命令：`grep -rnE '(^|[^A-Za-z0-9_])xirang::' --include=*.rs .`（**判别性**：295 → 0）。
- **反向后果**：替换后 `registry_core::`/`xirang_*::` 不受影响；`examples/`、`studio/`、`plugin-host/`、`debug_method/` 的 `xirang::` 计数为 **0** ⇒ 批 1 不动身份红线区与 studio 夹具。

### 1.3 `conventions` 门禁落点（批 1 只改内核相关的那几条）

| 门禁 | 现状（文件:行） | 批 1 动作 |
| --- | --- | --- |
| `purity` | `conventions/src/purity.rs:291` `root.join("core").join("src")` | → `join("kernel").join("src")` |
| `mounting` | `conventions/src/mounting.rs:258` `root.join("core/src")` | → `root.join("kernel/src")` |
| `naming` | `conventions/src/naming.rs:39` `LIB_NAME_EXCEPTIONS = &[("core", "xirang")]` | **删掉该例外表**（`kernel/` + `xirang-kernel` + `xirang_kernel` 三件套自动成立）；若想保守，可先改成 `("kernel", "xirang_kernel")` 再删 |
| `size` | `conventions/src/size.rs:82` `BASELINE = &[]` | **无需改**（0 条基线；方案里的那条 `contracts.rs` 基线已在命名批次中清掉） |
| `shims` | 见 §1.6 | 表内路径**不改**（17 条全在 `run_method/**` 与 `build_method/**`），**pin 字符串与检测器字面量要改** |
| `release_version` / `release_workflow` / `bilingual` / `doc_blocks` / `doc_anchors` | — | 自动跟随新名字 / 只扫 md；批 1 只受 §1.7 的锚点影响 |
| 夹具里的 `"core"` | `conventions/src/lib.rs:507`（合成 members）、`:553/:555`（合成 `core/src`） | **可选**：它们是**合成夹具**，不指向真实树；建议与本批同改以免读者误会，但不改也不会红 |

### 1.4 替换顺序（批 1 内，必须一次提交内完成）

1. `git mv core kernel` + 根 members + `kernel/Cargo.toml` 的包名/lib 名 + 6 行依赖（**同一提交**，否则 `cargo metadata` 先失败）。
2. 全树 `xirang::` → `xirang_kernel::`（295 处）。
3. `conventions` 的 `purity`/`mounting`/`naming` 三处 + §1.6 的 shims 字符串。
4. 工具/CI/脚手架里的 `xirang-core`、`core/`、`core/src` 文本（§1.5）。
5. 活文档锚点与 README/AGENTS（§1.7）。
6. 一键复核：§2 的判别性谓词全为 0。

### 1.5 工具 / CI / 脚手架

| 位置 | 现状 | 批 1 动作 |
| --- | --- | --- |
| `tools/xirang-publish` 层表 | `:91`–`:97`（7 行 9 名，首行 `xirang-core`） | `xirang-core` → `xirang-kernel`（1 行）；**3 行化是批 2** |
| `tools/xirang-publish` 依赖表 | `:354` 起（`xirang-macro:xirang-core` 等） | 把 `xirang-core` 两处改成 `xirang-kernel`（`xirang-macro:` 行与 `xirang-run-method:` 行） |
| `tools/xirang-publish` 头部散文 | 多处点名 `xirang-core`（共 13 处） | 同步文（不改逻辑） |
| `.github/workflows/ci.yml:56` | `cargo package -p xirang-core --locked --allow-dirty` | → `-p xirang-kernel`（**判别性**） |
| `.github/workflows/ci.yml` 注释 | `:53`、`:129`、`:133` 点名 `xirang-core`/`xirang-cli` | 同步文 |
| `build_method/scaffold/project.rs:76` | `workspace.join("core").is_dir()` | → `join("kernel")`（`:77` 的 `build_method` 属批 2） |
| `build_method/scaffold/project.rs:111-112/:154-…` | `workspace.join("run_method"/"build_method")`、生成文本里 `xirang_run_method::…` | **批 2**（不在本批） |
| `cli/lib.rs`、`cli/commands/new.rs` | 各 2 处 `xirang-core` 文本 | 同步文（若指向真实包名则必改） |
| `conventions/src/{release_version.rs,release_version_tests.rs,naming.rs}` | 2 / 11 / 4 处 `xirang-core` | 同步文（`release_version` 的表格与钉子要跟着改） |
| `core/tests/ungated_authoring_data.rs` | 2 处 `xirang-core` | 同步文（批 1 后该文件在 `kernel/tests/`） |

### 1.6 SHIMS 表（批 1 的精确落点）

- **条目数与路径**（现算 17 条，方案写 16）：`run_method/runtime/trace/trace.rs` 3、`run_method/lib.rs` 2、`run_method/registry.rs` 2、`run_method/plugin.rs` 2、`run_method/authoring/face_file.rs` 1、`run_method/authoring/parse/parse.rs` 1、`run_method/authoring/context.rs` 1、`run_method/runtime/runtime.rs` 1、`run_method/runtime/evidence.rs` 1（合计 14）；`build_method/identity.rs` 2、`build_method/syntax.rs` 1（合计 3）。
- **批 1 动作**：条目的**文件路径不改**（没有一条指向 `core/`）；要改的是**每条 pin 的语句字符串**里的 `pub use xirang::` → `pub use xirang_kernel::`（`conventions/src/shims.rs` 全文共 **20** 处该字面量，含检测器 `xirang_reexports()` 的口径串）。
- **判别性**：`grep -c 'pub use xirang::' conventions/src/shims.rs` **20 → 0**；`cargo test -p xirang-conventions --offline` 必须绿（`missing_shims` 对缺失文件是 **panic**，所以 pin 与真实语句必须同批改）。
- 方案 §5.1 说「路径 → `toolchain/src/{run,build}/**`」是**批 2** 的事。

### 1.7 活文档锚点与 README/AGENTS（批 1 必须同步的部分）

| 文档面 | 现算 | 说明 |
| --- | --- | --- |
| **`core/*.rs:line` 锚点** | **25 处**：`run_method/README.md` 12、`run_method/README.zh-CN.md` 12、`docs/roadmap-1.0.md` 1 | **判别性**：`doc_anchors` 会解析它们；批 1 后路径须写 `kernel/…`（行号与所引 token 不变），谓词 `grep -rnE 'core/(\.\./)*[A-Za-z0-9_/.-]*\.rs:[0-9]+' <活文档集>` **25 → 0** |
| `core/src` 文本 | 7 文件 / 41 次：`run_method/README{,.zh-CN}.md` 各 13、`AGENTS.md` 7、`docs/roadmap-1.0.md` 4、`CHANGELOG.md` 2、`cli/README{,.zh-CN}.md` 各 1 | 除 `CHANGELOG.md`（历史段不改）外全部同步 |
| `core/` 目录名文本 | 5 文件 / 17 次：`CHANGELOG.md`、`AGENTS.md`、`README{,.zh-CN}.md`、`docs/roadmap-1.0.md` | `AGENTS.md` 的 crate 表与「变更规则 4」按新名字/路径重写（方案 §5.5）；README 的内核段把 lib 名改成 `xirang_kernel` |
| `xirang-core` 文本 | 19 文件 / 32 次 | **可编辑的 18 文件 / 25 次**要改；`CHANGELOG.md` 的 7 次属历史段（方案 §5.5：只**新增** `[0.2.0] — unreleased` 段） |
| 其余 crate README | `core/README{,.zh-CN}.md` 随目录改名；`build_method`/`debug_method`/`run_method` 的 README 里各 1 处 `xirang-core` 文本 | 文本同步；**12 个 README 的删除是批 2** |
| 裸名 `xirang`（无 `::`/连字符） | 活文档集 33 文件 / 839 次 | **仅计数**：绝大多数是产品名或协议名词（`xirang.trace` 等），不是 lib 路径；**不要**全局替换 |

## 2. 批 1 验收（判别性谓词）

```sh
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo test -p xirang-conventions --offline
cargo fmt --all -- --check
tools/xirang-publish --check-table          # 仍报 9 个包（只换了内核的名字）
tools/xirang-external-rehearsal             # 两个示例宿主在检出外重建（判别性：身份不变）
# 判别性谓词（批 1 后全部应为 0）
grep -rn '^xirang-core' */Cargo.toml                       # 6 → 0
grep -rnE '(^|[^A-Za-z0-9_])xirang::' --include=*.rs .     # 295 → 0
grep -c 'pub use xirang::' conventions/src/shims.rs        # 20 → 0
grep -rnE 'core/(\.\./)*[A-Za-z0-9_/.-]*\.rs:[0-9]+' <活文档集>  # 25 → 0
grep -rn -A2 '^\[lib\]' */Cargo.toml | grep -c 'name = "xirang"'   # 1 → 0
ls -d core 2>/dev/null | wc -l                               # 1 → 0（目录已改名）
# 仅计数（记录当前规模，不作为红绿判据）
grep -rn 'xirang-core' --include=*.md --include=*.toml . | wc -l
```

## 3. 批 1 回滚

批 1 是「目录改名 + 文本替换」，**完全可回滚**（方案 §8）。三种粒度：

1. **推荐（一个提交内完成）**：`git revert <batch-1-commit>`。批 1 不产出任何生成物、未发布、无身份后果，revert 后工作树与批 1 前逐字节等价。
2. **手工（未提交时）**：
   ```sh
   git mv kernel core
   # 还原 6 行依赖与根 members（把 xirang-kernel/path="../kernel" 改回 xirang-core/path="../core"）
   # 还原 core/Cargo.toml 的包名与 [lib] name
   grep -rl 'xirang_kernel::' --include=*.rs . | xargs sed -i 's/xirang_kernel::/xirang::/g'
   # 还原 conventions 的 purity/mounting/naming 与 shims 的 20 处 pin 串
   git checkout -- .    # 注意：本批的团队规则禁在工作树里用 checkout；这里是"未来批次回滚"的说明
   ```
3. **必须一起回滚的东西**：`kernel/Cargo.toml`、根 `Cargo.toml:2`、6 行依赖、`conventions/src/{purity,mounting,naming,shims}.rs`、`.github/workflows/ci.yml:56`、`tools/xirang-publish` 的层表/依赖表/散文、`build_method/scaffold/project.rs:76`、§1.7 的文档面。**生成物**（`examples/*/target/xirang`）无需回滚：批 1 结束后本来就该清掉（方案 §4.4）。
4. **回滚后自证**：§2 的判别性谓词全部回到批 1 前读数（295 / 20 / 25 / 6 / 1 / 1）。

## 4. 批 2 / 批 3 的前置与不可回滚点

### 4.1 批 2（七个 crate → `xirang-toolchain`）前置清单

1. **批 1 已合入**（方案 §7：批 1 与批 2 之间不允许并行——批 2 的所有替换都以内核已叫 `xirang_kernel` 为前提）。
2. **方案 §4.2 的 `git mv` 表必须按当前树重算**：命名批次已把被搬运子树内部改了形状（A1 収平 24 处、mcp 三个模块改名、run_method 七个站点、`evidence.rs`→`build_evidence.rs`、`locals/call_trace.rs`→`recording.rs`、`trace/artifact/`→`trace/snapshot/`、`face_manifest.rs`→`face_file.rs`、`validation/validation.rs`→`context.rs`、`cache.rs`→`discovery_cache.rs`）。批 2 开工前应再跑一次同口径的重算（尤其 §4.2 的每行 `git mv` 目标路径）。
3. **`#[path]` 结论前提已变**：方案 §3.5「0 处 `#[path]` 需要改」建立在「子树内部形状不变」上；命名批次已经动过形状，但**全部仍是相对路径**（432 条源声明、非相对 0）⇒ 结论本身仍成立（合并只搬整棵子树，相对解析不变）。**不要**照抄方案的 339。
4. **同一批必须落的清单**（方案 §4.4，节选）：`RUN_METHOD_CRATE`（现 `lexicon.rs:63` + 钉子 `lexicon_tests.rs:15`）、宏里的绝对路径（`run_method/macros/entry.rs:37-38`、`face_registration.rs:263`）、`shims` 17 条的**路径**（`toolchain/src/{runtime,build_time}/**`）、`features`/`lint`/`bilingual` 的门禁路径、`tools` 三处、CI 两处、`AGENTS.md` 与 README（含 12 个 README 删除）、脚手架模板。
5. **前置的事实性检查**（现算）：5 个 bin（`xirang`、`cargo-xirang`、`xirang-studio`、`xirang-dev`(required-features `dev-supervisor`)、`xirang-mcp`）与 4 组 feature（`core::syntax`、`run_method::authoring`、`studio::{default,node-graph,prototype-fixtures,dev-supervisor}`、`plugin-host::{default,wasm,process-tools}`）——批 2 要保持这些名字不变。
6. **清缓存**：批 2 验收前 `rm -rf examples/*/target/xirang`（`build_output_is_current` 的指纹不含渲染器输出，旧产物可能被判"仍最新"）。

### 4.2 批 3（发布面切换）前置与**不可回滚点**

- 前置（方案 §7/§8）：① §2 的六条命令全绿；② `tools/xirang-publish --check-table` 报 **3 个包**；③ `tools/xirang-package-audit` 的内容检查对 3 个包全绿；④ `tools/xirang-external-rehearsal` 在检出外用新依赖重建两个示例宿主；⑤ feature 名与 bin 名已冻结（发布后再改是新的破坏）。
- **不可回滚点（显式列出）**：
  1. **每个包名下的那一个版本号的内容**——crates.io 上不可替换，只能 yank；因此 `kernel → macro → toolchain` 的发布顺序不能试错重来。
  2. **旧 9 名冻结在 0.1.x**：yank 可用 `cargo yank --undo` 撤销，但"9 → 3"的事实是"旧 9 名 + 新 3 名并存"，不是删除（方案 §8）。
  3. **身份数据不可回滚**：批 3 之前的任何批次若移动了 `examples/**` 的 6 个声明注册面文件，`NodeId` 会变、落盘 graft 记录会失效（方案 §6）——这是"9 → 3"全过程里**唯一**不可靠回滚的语义后果，因此两处宿主保持不动是本方案的红线。
- 回滚：批 3 的**上传**不可回滚；`yank` 可 `--undo`；文档与工具表可 revert。

## 5. 未覆盖 / 风险（如实）

- 本单是**只读现算**：没有执行批 1 的任何改动，因此 §1 的清单**未逐条跑红**；§0/§1 的每个数字都可用文中命令复现。
- 方案里若干路径/行号在本树已漂移（§0 表已逐处给出）；**凡本文件与方案冲突处，以本文件的现算值为准**，并请把漂移回写进方案或记为下一轮的事务。
- 「`core/` 目录名字面量」我只统计了**活文档集**与可编辑源文件；`docs/audit-*.md`、`.dsh-meow/**`、`docs/design-*.md` 里的旧名按记录性质**不动**（方案 §5.5/§9 同口径）。
- 批 2 的 `git mv` 表**必须重算**（§4.1 第 2 条）：本文件只给出"必须重算"与漂移清单，没有替批 2 出表。
- `tools/xirang-publish` 的 `--verify-consumers`/`--publish` 都从同一张表枚举，因此批 1 只改表即可；但**发布工具的层表在批 1 后仍是 9 名**（只换内核名字），这一点容易与"批 2 才 3 名化"混淆。

## 校正（t112）

§2 的宽松谓词 `core/…rs:NN` **需要词边界**：应写作 `(^|[^A-Za-z0-9_])core/…rs:NN`（或等价的正则 `\bcore/…rs:NN`）。不加词边界时那 **25 条**命中是 `kernel/src/registry_core/…` 路径里的 **子串假阳性**；加词边界后**真锚点是 24 条 → 0**（已实测）。本条校正由 t112 追加，只改本行，未动 §2 其它内容。
