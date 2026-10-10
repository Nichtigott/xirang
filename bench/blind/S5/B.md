# S5-cg · 同族对象核查（button / slider / label）

结论一句话：**同族三对象里只有 `label` 有差异 —— 它多了一条 `exports: ["control.render"]`，而这条能力声明在本树没有任何接收者（同一声明里 `needs_registry: false`，全树无人 `requires`/`provides` 它）；它是低危的“哑声明”，不是构建级缺陷。`button` 与 `slider` 没查出问题。**

---

## 0. 纪律与调用账目

- 只读/写 `/tmp/scen-runs/S5-cg/` 下的 `tree/` 与 `cargo-target/`；未读检出 `/home/nich/Moirai_N3/nichlink` 下的任何路径。唯一被读的两处例外是任务明确许可的：任务书本体 `bench/briefs/S5-cg.md` 与 wrapper `bench/bin/cg`。
- 已 `export BENCH_CALLS=/tmp/scen-runs/S5-cg/calls.log`；所有 codegraph 调用走 wrapper `cg`，**每一次查询都带 `-p /tmp/scen-runs/S5-cg/tree`**，从未指向检出。
- 所有 cargo 命令都带 `CARGO_TARGET_DIR=/tmp/scen-runs/S5-cg/cargo-target`。
- **没有改这棵树**：唯一写入是 `cg init` 建出的 `tree/.codegraph/`（索引目录，任务书工具清单里明确要求 init）。build / test / clippy / 宏展开后，除 `.codegraph/` 外树里所有文件的 mtime 未变，也没有生成 `.nichlink/`。
- **codegraph 调用账目：3 次 wrapper 调用，全部 EXIT 0；拒绝 0 次、重复预览 0 次。**

| # | 调用 | 性质 |
| --- | --- | --- |
| 1 | `cg --help` | 非查询（看命令面），1 次 |
| 2 | `cg init /tmp/scen-runs/S5-cg/tree` | 建索引（任务书要求），1 次 |
| 3 | `cg explore -p /tmp/scen-runs/S5-cg/tree "Button Slider Label root_object root_node_id env CARGO_PKG_NAME exports control.render"` | **唯一一次查询 = 本次同族比较** |

即：**3 次调用 / 1 次实质查询 / 0 拒绝 / 0 重复**。`--help` 那一发是纯开销，可以省掉。

---

## 1. ① 一次同族比较（就是上面第 3 发 `cg explore`）

命令：

```sh
export BENCH_CALLS=/tmp/scen-runs/S5-cg/calls.log
/home/nich/Moirai_N3/nichlink/bench/bin/cg explore \
  -p /tmp/scen-runs/S5-cg/tree \
  "Button Slider Label root_object root_node_id env CARGO_PKG_NAME exports control.render"
```

输出摘要（`calls.log` 里有全文）：一次调用返回 **3 symbols / 3 files**，并把三份源码**并排**给出（不是逐个查、不是逐个读）：

- `src/slider/slider.rs` — Slider(struct)：`root_object!{ kind: Slider, parent: crate::root_node_id(env!("CARGO_PKG_NAME")), }`
- `src/button/button.rs` — Button(struct)：`root_object!{ kind: Button, parent: crate::root_node_id(env!("CARGO_PKG_NAME")), }`
- `src/label/label.rs` — Label(struct)：**多一行** `exports: ["control.render"],`

三份源码的第 1–10 行（头部注释、`use crate::{NoParts, NoPreset};`、marker 类型、doc 注释）逐字同形；差异只在 label 的第 13 行。**同族比较一步完成，没有对任何对象做第二次从头阅读。**

### 1b. 对同一次比较的机器化加固（不是第二轮逐对象阅读）

为把“目测一行差异”钉成可复现的判据，我用一次宏展开把三份声明变成可 diff 的数据：

```sh
cd /tmp/scen-runs/S5-cg/tree
CARGO_TARGET_DIR=/tmp/scen-runs/S5-cg/cargo-target RUSTC_BOOTSTRAP=1 \
  cargo rustc --offline --lib -- -Zunpretty=expanded > /tmp/scen-runs/S5-cg/cargo-target/expanded.rs
```

再把 `button` / `slider` / `label` 三段 `pub const REGISTRATION` 归一化（把 kind 名、`control::<mod>`、树内路径归一）后互相 diff：

```
### diff button(normalized) vs label  -> 2 changed lines
    -            exports: &[],
    +            exports: &["control.render"],
### diff button(normalized) vs slider -> 0 changed lines
```

即：**归一化之后 button 与 slider 完全一致；button 与 label 只差 `exports` 这一行。** 同族只有这一个差异点，它落在 `label` 上。

---

## 2. ② 逐对象裁定

| 对象 | 裁定 | 依据 | 反证（削弱项 / 我为什么不过度断言） |
| --- | --- | --- | --- |
| `button` | **没问题** | ①归一化 diff：与 `slider` 0 行差异。②展开后的声明全字段（`expanded.rs:84-132`）：`namespace: "control"`、`parent: crate::root_node_id("control")`、`registry_rule: ANY`、`admission: ANY`、`needs_registry: false`、`requires/provides` 空、contract = `NoPreset/NoParts`、`source.file/line` 记录到声明行 11 —— 全是“紧凑形态”的文档缺省值。③`generated_lib.rs` 里它有一条 `StaticFace`（父 id `[71,204,…,218]`，与另两条**完全相同**＝同一个包根）＋一条编译期 `assert_static_registration`。④`cargo build` / `cargo clippy --all-targets` 零诊断。 | 生成的 `mod button;` 上挂着 `#[allow(dead_code, unused_imports, …)]`，所以“零警告”**不能**证明文件顶部的 `use crate::{NoParts, NoPreset};` 真被用到（这一项三个文件等量，不影响同族差异判定）。另外树内 0 个 `#[test]`，`cargo test` 跑 0 测试，所以我拿不出测试层证据。 |
| `slider` | **没问题** | 同 `button`，逐条相同：与 button 归一化 0 行差异（`expanded.rs:262-…`），同一父 id，同一条 `StaticFace` + 同一条编译期断言，build/clippy 零诊断。 | 同 `button`：`#[allow(unused_imports)]` 使编译器对“导入是否被用”沉默；无测试可依。 |
| `label` | **有问题（低危：能力声明没有接收者 / 与本树结构自相矛盾）** | ①同族唯一差异就是它（见 §1、§1b）。②同一声明里 `exports: &["control.render"]`（`expanded.rs:192`）与 `needs_registry: false`（`expanded.rs:193`）并存；而工具链元数据文档对这两槽的定义是：`exports` = *“Capabilities this face exports for its children.”*、`needs_registry` = *“Whether this face provides a registry its children may register into.”*（生成的 `StaticFace::new(…, false)` 第三字段即后者）——**子女不可能存在，这份导出就没有接收者**。③全树没有任何消费方：三个面的 `requires: &[]` / `provides: &[]`、`registry_rule: ANY`，`grep -rn "control.render" tree/` 只命中 `src/label/label.rs:13` 一处。 | `exports` 是**文档化的可选字段**（缺省为空），且 `["control.render"]` 逐字就是工具链元数据里的文档示例值；label 的 rule 是 `RegistrationRule::ANY`，编译期 `assert_static_registration` 通过，`cargo build` / `clippy` / `test` 零诊断。**所以这不是编译或注册失败，只是“声明了但当前无作用”的低危项**，不是“编译不过”或“注册被拒”。修法二选一：给 label 配子注册面并让消费方对齐（`needs_registry` + 某个面的 `requires`/`provides`），或删掉这一行。另：`control.render` 的前缀与包命名空间 `"control"` 自洽，我**没有**依据说这个字符串拼错了。 |

---

## 3. ③ 同族共有观察（等量、不构成差异，故不作为裁定）

- `src/lib.rs` 的 `static_graft_plan!` 对三个对象各写了一次 `cut(X) graft(X)`（同一 `NODE_ID` 自切自接）：生成的 `BUILTIN_GRAFT_CUTS` 三条都是 `from_ids(X, X, false)`。三个对象**等量**，不区分同族；且它写在 `lib.rs`（不在三个对象的文件里），我把它记为观察项而非裁定。
- 三个面都没有 `name` / `summary`（展开里 `summary {zh:"",en:""}`）/ `handle_traits` / `part_traits` / `runtime_checks` —— 等量的文档缺省。
- build 的唯一输出是 `cargo:warning=nichlink discovery cache hit (units=3)`（发现缓存命中提示，不是注册面诊断）。

---

## 4. ④ 我没核查到的范围

1. **测试层**：树内 0 个 `#[test]`/`#[cfg(test)]`，`cargo test --offline` 报 `0 passed; 0 failed` —— 没有测试证据可用。
2. **运行时**：本树是库宿主（无 `main`、无 trace 写入），我没有构造运行时注册树，因此 `exports` 在运行期的实际效果（谁消费、是否被规则拒绝）**未验证**。
3. **覆盖层**：树内没有 `.nichlink/` 目录（`build.rs` 的 `rerun-if-changed` 指向 `.nichlink/external-grafts`，缺失＝没有外部 graft 记录）——我据此认为没有覆盖记录，但没有构造一个覆盖来验证这条推理。
4. **不读检出**：`/home/nich/Moirai_N3/nichlink` 下除 wrapper 与任务书外一律没读，所以我不掌握 `exports`/`needs_registry` 的**源码级/规范级**定义。我的语义依据只有两处：我自己 `cargo-target/` 里的构建产物（宏展开 `expanded.rs`、`generated_lib.rs`）与 `libnichlink_toolchain-*.rmeta` / `libnichlink_kernel-*.rmeta` 里的**元数据文档字符串**（`strings` 提取，文件都在我的 target 目录内）。这是本裁定最大的不确定性来源：`label` 那条裁定若与工具链源码里的更细规则冲突，取决于我读不到的那份源码。
5. **未跑工具链 CLI 的 explain / authoring / topology 子命令**，也没有在 authoring 面（非默认 feature）下复核声明。
6. **只看 S5-cg 自己的树**：没有、也不允许对照其它 arm / 其它 run 目录或 `target/bench-*`。
7. **没有做命名裁定**：`control.render` 这个名字该不该长这样、该属于哪个命名空间，我没有依据判定。

---

## 5. 附录：我亲手跑过的命令（按时间顺序）

```sh
export BENCH_CALLS=/tmp/scen-runs/S5-cg/calls.log
cat /home/nich/Moirai_N3/nichlink/bench/bin/cg            # 只读 wrapper 本身（任务书许可）
cg --help                                                                  # 1 次非查询调用
cat tree/Cargo.toml tree/build.rs tree/src/lib.rs                          # shell 读树（许可）
cat -n tree/src/button/button.rs tree/src/slider/slider.rs tree/src/label/label.rs
cg init /tmp/scen-runs/S5-cg/tree                                          # 建索引
cg explore -p /tmp/scen-runs/S5-cg/tree "Button Slider Label root_object root_node_id env CARGO_PKG_NAME exports control.render"   # ★同族比较
cat tree/.vscode/nichlink-face.code-snippets                               # 面字段定式（生成器遗留）
cd tree && CARGO_TARGET_DIR=/tmp/scen-runs/S5-cg/cargo-target cargo build --offline
cd tree && rm -rf cargo-target/debug/build/control-* && CARGO_TARGET_DIR=... cargo build --offline -vv    # 抓 build script 输出
cd tree && CARGO_TARGET_DIR=... NICH_LINK_BUILD_VERBOSE=1 cargo build --offline -vv
cat tree/../cargo-target/debug/build/control-*/out/generated_lib.rs        # 生成的 StaticFace / 断言
cd tree && CARGO_TARGET_DIR=... RUSTC_BOOTSTRAP=1 cargo rustc --offline --lib -- -Zunpretty=expanded > cargo-target/expanded.rs
strings cargo-target/debug/deps/libnichlink_toolchain-*.rmeta > cargo-target/tc.strings.txt   # 元数据文档字符串
strings cargo-target/debug/deps/libnichlink_kernel-*.rmeta    > cargo-target/kernel.strings.txt
grep -rn "render\|exports" tree/ --exclude-dir=.codegraph
grep -rn "#\[test\]" tree/src tree/build.rs
cd tree && CARGO_TARGET_DIR=... cargo test --offline                        # 0 tests
cd tree && CARGO_TARGET_DIR=... cargo clippy --offline --all-targets        # 零告警
python3 <归一化 diff 三段 REGISTRATION>                                     # 只读 expanded.rs，输出到 stdout
grep -c '^=== CMD: cg ' calls.log                                          # 调用账目
```

（所有 cargo 都带 `CARGO_TARGET_DIR=/tmp/scen-runs/S5-cg/cargo-target`；`rm -rf` 只删了我自己的 target 目录里 `control-*` 的 build 子目录，用来强制重跑 build script。）
