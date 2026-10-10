# S2 · 对照臂 · 逐栏处置表

审查对象：`/tmp/scen-runs/S2-cg/tree`（nichlink 宿主包 `control`：5 个 `.rs` + `Cargo.toml`/`Cargo.lock` + `.vscode/nichlink-face.code-snippets`）

纪律执行情况：
- 所有 codegraph 调用经 wrapper `bin/cg`，日志 `BENCH_CALLS=/tmp/scen-runs/S2-cg/calls.log`；**每次查询都带 `-p /tmp/scen-runs/S2-cg/tree`**，从未指向检出（`/home/nich/Moirai_N3/nichlink`）。
- 所有 cargo 命令带 `CARGO_TARGET_DIR=/tmp/scen-runs/S2-cg/cargo-target`。
- **树未被改动**：审查前快照 `tree-before.md5` 与审查后 `tree-after.md5` 逐文件 md5 一致（构建只写 OUT_DIR，未创建 `.nichlink/`，`.vscode` 快照 md5 未变）。

---

## 0. 一句话结论

这个项目的“裁决表”**不在树里**，而是它自己构建时写进 `OUT_DIR` 的一组决策表（4 张 TSV + `generated_lib.rs`）。以发布态裁决表 `pruning_manifest.tsv`（**7 栏** / 3 行）为主表逐栏处置后：**4 栏恒为空值 `-`**（symbol / path / registry_name / parent），其中 `parent` 栏与源码里确实存在的声明相矛盾；**3 个面里有 1 个（Slider）被构建丢弃**——它出现在 pruning 裁决表里，却不在 `generated_lib.rs` 的 `pub mod`、不在源码范围表、也不在 graft 计划里；`button::clamp` 无调用者、无测试、含一条不可达臂，而 clippy/编译器对此完全沉默。共 8 条发现，树的源文件一个字节没动。

---

## 1. ① 裁决表从哪来、它一共几栏

**来源链（不是文档，是构建产物）**：
`Cargo.toml:7 build = "build.rs"` → `build.rs:1-3 fn main() { nichlink_toolchain::build_time::run(); }` → 构建把这组表写进 `$CARGO_TARGET_DIR/debug/build/control-<hash>/out/`。

**证据命令**
```sh
cd /tmp/scen-runs/S2-cg/tree && CARGO_TARGET_DIR=/tmp/scen-runs/S2-cg/cargo-target cargo build --offline   # exit 0
ls -la /tmp/scen-runs/S2-cg/cargo-target/debug/build/control-4a2b5e438603350e/out/
cat .../out/{function_manifest,graft_plan,pruning_manifest,source_scope}.tsv
```

**这批表一共几张、各几栏**（栏数 = 表头字段数）

| 表 | 栏数 | 行数 | 性质（本仓词汇：发布/裁剪/接枝裁决） |
| --- | --- | --- | --- |
| `pruning_manifest.tsv` | **7** | 3 | 发布态保留/裁剪裁决：`node / source / symbol / path / kind / registry_name / parent` |
| `graft_plan.tsv` | **5** | 2 | graft 裁决：`cut / graft / full / line / column` |
| `function_manifest.tsv` | **3** | 2 | 进入构建的函数清单：`node / source / symbol` |
| `source_scope.tsv` | **3** | 2（+2 行前置键值） | 源码范围裁决：`mode=auto`、`selected=2`，再 `node / source / module` |
| 另：`generated_lib.rs` | 5383 B | — | 生成的注册代码（不是表，但含 41 行起的 `pub mod` 列表与 50 行的 `StaticFace` 清单） |

**我要处置的“裁决表”本体** = `pruning_manifest.tsv`（7 栏 / 3 行）：只有它逐条裁决了树里的**每一个**注册面（含被丢弃的那个）。但“裁决表”若指的是 graft 裁决，则指 `graft_plan.tsv`（5 栏 / 2 行）——因此我把同批另外三张表和 `.vscode` 快照表也逐栏点过（§2.2–§2.5），不把赌注押在单一解读上。

**反证：树内不存在成文的裁决表**
```sh
cg files -p /tmp/scen-runs/S2-cg/tree      # → 5 files，全是 rust（button/label/slider/lib/build）
find . -path ./.codegraph -prune -o \( -name '*.md' -o -name '*.json' -o -name '*.jsonl' \) -print   # → 空
```
输出摘要：项目结构只有 5 个 rust 文件；树里没有任何 `.md/.json/.jsonl`，因此“裁决表”只能来自构建产物，不是被检入的文档。

---

## 2. ② 逐栏处置表（栏名 / 处置 / 证据命令 / 输出摘要 / 未覆盖说明）

### 2.1 主表 `pruning_manifest.tsv` —— 7 栏 / 3 行

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
| --- | --- | --- | --- | --- |
| `node` | **通过**（3 行 3 个互异 hash，稳定 id） | `awk -F'\t' '!/^#/{print $1}' pruning_manifest.tsv` | `3e1f685c…`(button) / `68c941df…`(label) / `9174fc9f…`(slider)，3 distinct | 无法从 hash 反算源码位置（散列语义在 toolchain 实现里，属禁读区） |
| `source` | **不一致（缺陷 F1）**：这一栏比构建真正接受的面多一行 | `awk -F'\t' '!/^#/{print $2}' pruning_manifest.tsv` 与 `source_scope.tsv` 对比 | pruning = `button/button.rs`, `label/label.rs`, `slider/slider.rs`；source_scope = 只有前两个（`# selected 2`） | 该栏为何把“被发现但未启用”的面也列进来，需读构建实现（禁读区） |
| `symbol` | **空栏（反证）**：3 行全 `-`，而同两个 node 在另一张表里明明有符号名 | `awk -F'\t' '!/^#/{print $3}' … \| sort -u` 与 `function_manifest.tsv` 对比 | pruning `$3` distinct=1、值即 `-`；function_manifest 对同 node 给出 `button::clamp`、`label::the_label_face_is_still_there` | “为何留空”是构建内部选择，未读实现 |
| `path` | **空栏（反证）**：distinct=1，值 `-`；全仓找不到任何填值来源 | 同上取 `$4`；`find` 全树无同名数据文件 | 3 行全 `-` | 该列预期语义未定义（禁读区） |
| `kind` | **有值**：这一栏实际装的是面标记类型名（`kind:` 声明） | `awk -F'\t' '!/^#/{print $5}'` + `grep -n 'kind:' src/` | `Button`(button.rs:12) / `Label`(label.rs:12) / `Slider`(slider.rs:12)，3 distinct | 列名 `kind` 与下一栏 `registry_name` 的分工只在实现里（禁读区） |
| `registry_name` | **空栏（反证）**：3 行全 `-`，且本树**不可能**有值 | `grep -rn -e 'name:' -e 'stable_name:' src/` → 无输出；`awk` 取 `$6` | 三处面声明只用了 `kind:` 与 `parent:`，没有任何 `name:` 声明；`$6` 全 `-` | 无（源码侧已排尽） |
| `parent` | **空栏，且与源码矛盾（缺陷 F4）** | `grep -n 'parent:' src/*/*.rs` + `awk` 取 `$7` | 源码 3 处都有 `parent: crate::root_node_id(env!("CARGO_PKG_NAME"))`（button.rs:13 / label.rs:13 / slider.rs:13），表里 `$7` 3 行全 `-` | 无法排除“默认根即省略”的设计意图（判断点在构建实现，禁读区）；但**至少这一列对本次构建零信息** |

### 2.2 `graft_plan.tsv` —— 5 栏 / 2 行

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
| --- | --- | --- | --- | --- |
| `cut` | **有缺陷（F1 的另一面）**：只有 2 行，第三面缺席 | `awk -F'\t' '!/^#/{print $1}' graft_plan.tsv` | `crate::button::NODE_ID`、`crate::label::NODE_ID`；无 slider | slider 是否需要 graft 由宿主作者决定，未判断意图 |
| `graft` | **通过但信息重复**：与 `cut` 同值（自切自接） | 同上取 `$2` | 2 行与 `$1` 一一相同，无跨节点 graft | 跨节点 graft 的取舍需业务上下文 |
| `full` | **本次构建无区分度**：唯一值 `false` | `awk -F'\t' '!/^#/{print $3}' \| sort -u` | `false` | 未构造 `full=true` 的对照输入 |
| `line` | **有缺陷（F5）**：两行都是 `8`，指向宏调用行而非 cut 行 | `awk -F'\t' '!/^#/{print $4}'` + `cat -n src/lib.rs` | 两行均 `8` → `src/lib.rs:8` 是 `static_graft_plan!(`；真正两个 cut 在 `lib.rs:10` 与 `lib.rs:11` | `line` 的精度语义在构造端（禁读区） |
| `column` | **同 F5**：两行都是 `1` | 同上取 `$5` | 两行均 `1`（`lib.rs:8:1`） | 同上 |

### 2.3 `function_manifest.tsv` —— 3 栏 / 2 行

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
| --- | --- | --- | --- | --- |
| `node` | **通过** | `awk -F'\t' '!/^#/{print $1}'` | `3e1f685c…`(button)、`68c941df…`(label)；slider 的 `9174fc9f…` 不出现 | — |
| `source` | **通过** | 同上 `$2` | `button/button.rs`、`label/label.rs` | — |
| `symbol` | **有缺陷（F2/F9）**：唯一的生产函数是没人调用的 `button::clamp`，另一个是测试函数 | 同上 `$3` + `cg callers clamp` + `cargo test -- --list` | `button::clamp`（调用者 0、测试 0）、`label::the_label_face_is_still_there`（测试函数） | 未验证 `clamp` 是否被宿主外部（下游 crate）调用；本树内已排尽 |

### 2.4 `source_scope.tsv` —— 3 栏 / 2 行（+ `mode`/`selected` 两行前置键值）

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
| --- | --- | --- | --- | --- |
| `mode` | **通过** | `head -1 source_scope.tsv` | `# mode	auto`（唯一值，自动发现模式） | 未跑 `NICH_LINK_SCOPE`/`NICH_LINK_ENTRY` 手工模式对照 |
| `selected` | **与 pruning 差 1（F1）** | `cat source_scope.tsv` | `# selected	2`，而 pruning_manifest 有 3 个面 | — |
| `(node,source,module)` | **通过**：`module` 名与目录名一致 | `awk -F'\t' '!/^#/{print $3}'` | `button`、`label`，与 `src/button/`、`src/label/` 对齐 | `module` 的语义（是否等于 `mod` 名）未在实现侧确认 |

### 2.5 `.vscode/nichlink-face.code-snippets`（随树检入的唯一“表”）—— 24 栏 × 4 字段

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
| --- | --- | --- | --- | --- |
| 24 个字段快照（`source:`, `kind:`, …, `runtime_checks:`） | **有缺陷（F8）**：与生成的宏文档清单不一致 | `grep -c '^  "' .vscode/nichlink-face.code-snippets`；与 `generated_lib.rs` 字段清单 `comm` 对比 | 快照 24 条；`generated_lib.rs:12-18` 文档只列 23 条；差异恰为快照多一条 `source: ` | 谁对谁错需读宏解析器（禁读区）；内核元数据里确实有 `source` 字段名，**倾向**快照是对的、生成文档不全，但未证实 |
| `prefix` / `body` / `scope` / `description`（每条 4 字段） | **通过**：24 条 4 字段齐全、描述均为中英双语 | `head -40 .vscode/nichlink-face.code-snippets` | 每条含 `prefix`/`body`/`scope:"rust"`/`description`（英文 ` / ` 中文） | 未在编辑器里验证补全真的触发（需 VS Code） |
| 与树的实际用法对照 | **覆盖面偏大**：24 条里本树只用到 2 条 | `grep -rn '^\s*\(kind\|parent\)\s*:' src/` | 6 处全是 `kind:`(12 行) 与 `parent:`(13 行)，其余 22 条字段在本树无使用 | 其余字段是否在别处需要，取决于作者意图 |

### 2.6 清单/构建元数据侧（附带点过）

| 栏名 | 处置 | 证据命令 | 输出摘要 | 未覆盖说明 |
| --- | --- | --- | --- | --- |
| `Cargo.toml` 字段 | **有缺陷（F7）**：构建脚本为不存在的 feature 声明 check-cfg | `grep -n features Cargo.toml` → 无；`grep -n check-cfg .../control-*/output` | `output:6 cargo::rustc-check-cfg=cfg(feature, values("compile_error_demo"))`，但 `Cargo.toml` 没有 `[features]` → 该 cfg 在本树永远为假 | 模板意图（是否该补 feature）未判断 |
| `Cargo.toml:1` `[workspace]` | **通过但需知悉**：本树是自成一体的 workspace 根（无 `members`），检出只作为 path 依赖 | `cat -n Cargo.toml` | `[workspace]` 空，成员即自身；依赖是 `path = "/home/nich/Moirai_N3/nichlink/toolchain", version = "0.2.0"` | 未审计该 path 依赖的版本/发布面 |
| `Cargo.lock` | **通过**：15 个 package，名字与清单自洽 | `cargo build --offline`（exit 0，不改 lock） | `control`, `nichlink-kernel/macro/toolchain 0.2.0` + 11 个 registry 包 | 未做供应链/漏洞审计（`cargo audit` 需网络） |

### 2.7 发现清单（每条都能落到文件与行）

| 编号 | 严重度 | 结论 | 文件:行 |
| --- | --- | --- | --- |
| F1 | 高 | `Slider` 面被构建丢弃：树里有声明、pruning 裁决表里有它，但 `generated_lib.rs` 只有 `pub mod button;`/`pub mod label;`，源码范围表 `selected=2`，graft 计划无它 → 该面不参与任何注册/接枝 | `src/slider/slider.rs:9`、`src/lib.rs:8-12`、`generated_lib.rs:41,44,50-51,61-62`、`source_scope.tsv`、`pruning_manifest.tsv` |
| F2 | 高 | `button::clamp` 无调用者（构建内 0）、无测试、含**一条任何输入都到不了的臂** `if false { return LIMIT; }`，且被列进 function_manifest 随构建出厂 | `src/button/button.rs:20-25`（臂在 21-23） |
| F3 | 中 | 两个面各自定义一个 `1000` 常量：`LIMIT` 仅被那条不可达臂引用；`LIMIT_AGAIN` 全树零引用（只有声明） | `src/button/button.rs:16,22`、`src/label/label.rs:16` |
| F4 | 中 | pruning 裁决表 4/7 栏恒空（symbol/path/registry_name/parent）；其中 `parent` 与源码声明矛盾 | `pruning_manifest.tsv`；`src/button/button.rs:13`、`src/label/label.rs:13`、`src/slider/slider.rs:13` |
| F5 | 中 | graft 裁决的 `line`/`column` 两栏都指向 `src/lib.rs:8:1`（宏调用行），无法定位到 `lib.rs:10`/`lib.rs:11` 两个 cut | `graft_plan.tsv`、`src/lib.rs:8,10,11` |
| F6 | 低 | 三个面的中文文档不一致：button/label 有“不代表运行时 object 实现”，slider 没有（同一模板漂移） | `src/button/button.rs:8`、`src/label/label.rs:8` vs `src/slider/slider.rs:8` |
| F7 | 低 | 构建为 `feature = "compile_error_demo"` 声明 check-cfg，但清单无 `[features]` → 死脚手架 | `.../control-4a2b5e438603350e/output:6` vs `Cargo.toml`（无 `[features]`） |
| F8 | 低 | 编辑器快照表 24 栏 vs 生成文档清单 23 栏，差异是 `source: `；未证实哪边是权威 | `.vscode/nichlink-face.code-snippets` vs `generated_lib.rs:12-18` |
| F9 | 中 | 测试覆盖：全树 1 个测试（仅断言标记类型存在），doc-test 0；button/slider 两面 0 测试；clippy 与编译器对该树零告警（缺陷对静态工具不可见） | `src/label/label.rs:18-24`；`cargo test -- --list`；clippy exit 0 |

---

## 3. ③ 用反证收尾的栏（不止一栏）

| 反证命题 | 据以判断的那条命令 | 输出 |
| --- | --- | --- |
| `symbol` / `path` / `registry_name` 三栏是空的 | `awk -F'\t' '!/^#/{print $3"|"$4"|"$6}' pruning_manifest.tsv \| sort -u` | 唯一值 `-|-|-`（3 行） |
| `registry_name` 不仅空、在本树里**不可能**有值 | `grep -rn -e 'name:' -e 'stable_name:' src/` | 无输出（三个面只声明 `kind:`/`parent:`） |
| `clamp` 没有调用者（不是“只差一个测试”） | `cg callers clamp -p /tmp/scen-runs/S2-cg/tree` | `Callers of "clamp" (0)` / `ℹ No callers found for "clamp"` |
| `slider` 面没有任何依赖者（所以被丢弃是静默的） | `cg node src/slider/slider.rs -p /tmp/scen-runs/S2-cg/tree` | `— 15 lines, 1 symbol · no other indexed file depends on it` |
| 树里没有任何成文裁决表 / 文档 | `cg files -p …` + `find … -name '*.md' -o -name '*.json' -o -name '*.jsonl'` | 5 个 rust 文件；`find` 空 |
| 这一栏（Lint）没有问题 | `CARGO_TARGET_DIR=… cargo clippy --offline --all-targets --message-format short` | 只输出 `Compiling control`，无任何 warning/error；`clippy exit=0` |
| 特性矩阵这一栏是空的（没有第二个面可跑） | `cargo test --offline --all-features` 与 `--no-default-features` | 两次都 `1 passed; 0 failed`、doc-test `0`（本树无 features，两个面等价于默认面） |

---

## 4. ④ 结论锚点（文件 + 行）

- `src/lib.rs:1` `host!()`；`:6` `FRAMEWORK = "nichlink.scenario.s2"`；`:8-12` `static_graft_plan!`，其中 `:10` button cut、`:11` label cut（无 slider）。
- `src/button/button.rs:9` `Button`；`:11-14` `root_object!`（`:12 kind`、`:13 parent`）；`:16` `LIMIT`；`:20-25` `clamp`，`:21-23` 不可达臂。
- `src/label/label.rs:9` `Label`；`:13` parent；`:16` `LIMIT_AGAIN`；`:18-24` 唯一的测试。
- `src/slider/slider.rs:9` `Slider`；`:11-14` `root_object!`；`:8` 中文文档缺半句。
- 生成物：`…/out/generated_lib.rs:41` `pub mod button;`、`:44` `pub mod label;`（无 slider）、`:50-51` 两条 `StaticFace::new`、`:56-57` 两条注册断言、`:61-62` 两条 graft cut、`:78` `registrations()` 只含 button/label、`:12-18` 字段文档清单。
- 表：`…/out/pruning_manifest.tsv`（7 栏/3 行）、`graft_plan.tsv`（5 栏/2 行）、`function_manifest.tsv`（3 栏/2 行）、`source_scope.tsv`（3 栏/2 行 + 2 行键值）。
- 构建脚本输出日志：`…/control-4a2b5e438603350e/output:5-6`（check-cfg）。
- 清单：`Cargo.toml:1`（`[workspace]`）、`:7`（`build = "build.rs"`）、`:10,13`（path 依赖）；`build.rs:1-3`。

---

## 5. ⑤ 我没有覆盖到的范围

1. **toolchain / kernel / macro 的实现源码（禁读区）**：所有“为什么”级问题只能从产物反推——`symbol`/`path` 为何留空、`parent` 是“默认根省略”还是丢失、`graft_plan.line` 的精度语义、`source:` 是否被宏接受、`kind` 与 `registry_name` 的分工。我也**没有**把 `-p/--path/--project` 指向检出（检出上有索引）。
2. **运行时与追踪面**：本树是库宿主（`src/lib.rs:3-5` 注释自述没有 `main`），没有产生 trace artifact；`CallTrace::runtime` / `trace_call!` / `trace_artifact_path` / `write_trace_artifact` 这条链一次也没跑，MIR/JSONL 面亦未跑。
3. **其余执行面未执行**：`nichlink`/`cargo-nichlink` CLI、`nichlink-mcp`（AI 桥）、`nichlink-studio`、插件宿主（wasm / process）。运行时注册与拓扑校验实际行为未经验证——上面所有“注册/接枝”结论都来自**构建期产物**。
4. **特性矩阵的“假绿”判别只对本树成立**：`--all-features` / `--no-default-features` 都跑了，但这棵树自身没有 features，所以两个面等价于默认面；没有非默认特性可作对照。
5. **未跑的检查**：`cargo fmt --check`、release / `--release` 构建、跨平台、`cargo package` / 发布表检查、外部 rehearsal、`cargo audit`（需网络）、基准（无 bench target）。
6. **`.vscode` 快照的实际编辑器行为**（补全是否触发、`rust_analyzer` cfg 下的 `face_fields_mirror`）未验证。
7. **依赖与供应链**：`Cargo.lock` 15 个包只看名/版本与清单自洽，未审计内容与漏洞。
8. **意图层面**：slider 是否**本应**被 graft、`clamp`/`LIMIT`/`LIMIT_AGAIN` 是否有未提交的用途——树里没有 README/issue，只有源码可判，故 F1/F2/F3 按“与构建事实不一致”记录，不代作者裁定意图。

---

## 6. 我亲手跑过的命令（照录）

```sh
# 快照与完整性（树先存 md5，结束时对比）
find tree -type f -exec md5sum {} \; | sort -k2 > tree-before.md5        # 结束后同式生成 tree-after.md5；diff 为空
# codegraph（全部经 wrapper，BENCH_CALLS 指向 calls.log，全部 -p tree）
cg --version
cg init /tmp/scen-runs/S2-cg/tree
cg status -p /tmp/scen-runs/S2-cg/tree                                    # ← 被拒：unknown option '-p'（status 只接受位置参数）
cg files -p /tmp/scen-runs/S2-cg/tree
cg explore "button label slider registration face graft plan static_graft_plan clamp" -p /tmp/scen-runs/S2-cg/tree
cg callers clamp -p /tmp/scen-runs/S2-cg/tree
cg node src/slider/slider.rs -p /tmp/scen-runs/S2-cg/tree
# cargo（全部带 CARGO_TARGET_DIR=/tmp/scen-runs/S2-cg/cargo-target）
cargo build --offline                                                     # exit 0
cargo test --offline ; cargo test --offline -- --list                     # 1 passed；1 test, 0 benchmarks
cargo test --offline --all-features ; cargo test --offline --no-default-features
cargo clippy --offline --all-targets [--message-format short]             # exit 0，无告警
# shell 取证
cat -n src/lib.rs src/*/*.rs Cargo.toml ; cat .../out/*.tsv ; awk 逐栏 distinct/空值统计
grep -rn 'LIMIT' src/ ; grep -rn -e 'name:' -e 'stable_name:' -e 'source:' src/ ; grep -n 'kind:\|parent:' src/*/*.rs
grep -n '^pub mod\|StaticFace::new\|assert_static_registration\|StaticGraftCut' .../out/generated_lib.rs
grep -o '^  "[^"]*"' .vscode/nichlink-face.code-snippets | sed … | sort > snip.keys   # 24 栏
grep -o '`[a-z_][a-z_]*`' .../out/generated_lib.rs … | sort -u > macro.fields        # 23 栏
comm -23 snip.keys macro.fields ; comm -13 snip.keys macro.fields                    # 差异 = source:
grep -ao 'getting_from_other_registry.\{0,300\}' .../deps/libnichlink_kernel-*.rlib  # 内核字段名表里含 source
```

**codegraph 调用次数（本次 attempt 2，calls.log 记录）：5 次，其中 1 次被拒**（`cg status -p` → `unknown option '-p'`，改用位置参数即可；同一坑不要踩第二次）。另有 attempt 1 的 4 次（`--version`/`init`/`status`/`files`）——其日志被本轮开始时的重置脚本清掉了，本文件里的关键证据都用 attempt 2 的调用重新取过。
