# 定界三问：多语言 · 跨平台 · 分布式编译（2026-10-09 开工前）

**English (short).** A scoping document, not an implementation plan. It puts three questions in front of the
maintainer — what "cross-platform" means, where the first multi-language cut goes, and which of three
distributed-build routes to take — each with a recommended default, the numbers behind it, its ceiling, and how
to test it afterwards. Measured while writing this: a real example host **and** its partitioned shape both
cross-compile to `wasm32-unknown-unknown` today (exit 0, §2.2); a partition fragment's build script costs the
same as the whole host's (6.89 s vs 5.43 s at 1001 faces), so partitioning today **replicates** discovery and
splits only rendering — and rendering is 3% of it (§4.1); faces gated by anything but `feature` are refused by
name, so "a different tree per platform" is not expressible today (§2.4).

---

## 0. 怎么读这一份

不是实现计划，是**定界**。三问各给：**机制（用本仓的实例讲清）· 选项 · 我推荐哪个 + 为什么 · 可信的量（数字 + 它是怎么量的）· 天花板 · 改完怎么测**。

回我三个编号即可（例：`①a ②i ③A`）。每条结论都单独成行、可被推翻——哪一条你判它不成立，直接说，我不会拿它当既成事实继续做。

---

## 1. 今天树上的事实（三问的共同底座）

| 事实 | 读数 | 量器（可复跑） |
| --- | --- | --- |
| 三平台 CI **已存在** | `verify` 矩阵 `ubuntu-latest / macos-latest / windows-latest` × `{1.96.0, stable}`，跑 fmt / tests / release tests / clippy / doc | `.github/workflows/ci.yml:11` 起 |
| 重装置**只在 Linux** | 8 个步骤带 `if: runner.os == 'Linux'`（外部路径演练、分区演练、graft 矩阵、日常行为、真项目、MCP 会话、打包、打包审计） | 同上 |
| 交叉编译**今天就能跑** | 单 crate 真示例 → wasm32 exit 0（22.97 s）；分区真示例（graft + objects + host + facade 四个 crate）→ wasm32 exit 0（20.32 s） | `target/xirang-bench/build-cost/wasm-{build,partitioned}.log` |
| **按平台分叉的面**今天被按名拒绝 | 只有 `feature` 门控可跟随；`evaluate_cfg` 只认 `feature = "…"` 与 `not(…)`，其余一律 `face-cfg` 诊断 | `toolchain/src/build_method/src/static_plan.rs:94`、`:159` |
| 身份**与平台/target 无关** | `NodeId = hash(命名空间, 包内相对源码路径, 名字)`；命名空间＝包名；路径来自 `file!()` | `kernel/src/registry_core/identity`；AGENTS.md「身份红线」 |
| **记录落点没有配置分量** | `<包>/target/xirang/out`（写死；只认 `CARGO_TARGET_DIR` 的是发现缓存） | `toolchain/src/mcp/src/build_evidence.rs:36` · `build_method/src/partition_view.rs:36` · `discovery_cache.rs:86` |
| 内核**没有 C ABI 面** | 全仓 `extern "C"` 只有 1 处、与内核无关；没有 cdylib/staticlib target | `grep -rn 'extern "C"'` ⇒ `toolchain/src/bin/xirang-dev.rs:185` |
| 证据（MIR）**很少人读** | 只有 6 个文件命中 `mir::`（含实现文件本身）；`check`/`registry`/`diff`/`affected` 一个都不读 | `grep -rln 'mir::' toolchain/src/` |
| 10k 面规模：**墙在构建脚本** | 构建脚本 103.885 s / 总 141.635 s = **73.3%**；rustc 37.750 s = 26.7%（debug、warm、本机、同一检出） | `scale-logs/compile-wall.json`（2026-10-04） |

---

## 2. ① 跨平台：你要哪一种口径

### 2.1 三个口径，"变"的是什么

- **P1 宿主平台一致**：同一棵树，在 Linux / macOS / Windows 上各构建一次。变的是**宿主 OS**。
- **P2 交叉编译目标**：一台机器，产物面向别的 triple（`--target`）。变的是**目标 triple**；构建脚本仍在宿主上跑。
- **P3 非 Rust 宿主**：变的是**宿主语言**——它和 ② 是同一件事，见 §3。

### 2.2 实测：P2 今天已经能跑（两个形状都跑了）

```
$ cargo build -p xirang-example-control-button --target wasm32-unknown-unknown --offline
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.97s          # exit 0
    → target/wasm32-unknown-unknown/debug/libcontrol_button.rlib (398 KB)
    → target/wasm32-unknown-unknown/debug/build/<pkg>-<hash>/out/generated_lib.rs

$ cd examples/partitioned-button && cargo build --target wasm32-unknown-unknown --offline
    Compiling xirang-example-partitioned-button-graft / partitioned-button-objects
    Compiling xirang-example-partitioned-button / partitioned-button-facade
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.32s          # exit 0
```

两条附带事实：分区的 remap 写在 `.cargo/config.toml` 的 `[build] rustflags` 里，**对任何 target 都生效**（`crate_plan.rs:399`）；本机 `wasm32-unknown-unknown` 的 std 已装（`$(rustc --print sysroot)/lib/rustlib/` 里第二个 triple），所以这条**不需要网络、不需要 CI** 就能复跑。

### 2.3 P1 今天有多少证据

三平台 CI 跑的是 `fmt / test / release test / clippy / doc` 那五步；**那批重装置一个都没在第二个平台跑过**——CI 里 8 个带 `if: runner.os == 'Linux'` 的步骤，加上只在 `ubuntu-latest` 上跑的 `features` / `release-audit` / `process-plugin` 三个作业。所以"跨平台"当前的真实状态是：**编译与测试三平台绿，端到端装置只有 Linux 有证据**。

真要搬，先知道会碎在哪：那批 Linux-only 脚本里，真正用 GNU 专有工具的目前查到一处——`tools/xirang-partition-rehearsal` 用 `sha256sum`（macOS 上是 `shasum -a 256`）；`xirang-scale-audit` 的 `/usr/bin/time` 已经自带降级（`if [ -x /usr/bin/time ]`）✓；其余几个的命中在注释里（是历史说明，不是调用）。⇒ 搬上去大概率红在**装置**（脚本、路径分隔符、`sed -i` 一类），而不是红在产品——**这正是要分辨的东西**。

### 2.4 推荐 ①a：同平台口径 + 任意 target 同一批 id

推荐 **(a)**：你要的是"同一棵树在三平台、任意 target 上都成立，且得到同一批 `NodeId`"，而不是"同一棵树按平台有不同的面"。三条理由：

1. **身份红线决定它**：`NodeId` 不含平台。一旦同一个 `file!()` + 名字在不同平台上是"有面 / 没面"，同一个 id 的含义就随平台变——而 graft 记录、采信台账（租约）、剪枝清单今天都只带 id。选 (b) 等于选一次**记录格式**的破坏性改动。
2. **(b) 今天不可表达**（§1 的 `face-cfg` 一条），所以选它等于选一个新特性，而不是修一个缺陷。
3. **(a) 是 ③ 的前提**：分布式下不同机器产出的产物要靠 id 装配；id 随平台变，装配就得靠配置分量。

**天花板**：全做完 = "三平台 + 任意 target 上，`check` / `registry` / `diff` / `affected` 与同一份 `graft_plan.tsv` 逐字一致"。**不是我们能动的**：平台特有的依赖与链接（那是宿主自己的事）、macOS/Windows runner 的额度。
**改完怎么测**：把 `xirang-partition-rehearsal`、`xirang-external-rehearsal`、`xirang-real-usage` 逐条搬上 macOS/Windows 矩阵（先只加 `stable` 单元）；判据＝三平台输出**同一份 `graft_plan.tsv` + 同一批 NodeId**——注意后者今天**没有**跨平台清单，得先造一个（把每棵树的 id 集合按排序取哈希，逐平台比）。

---

## 3. ② 多语言：第一刀切在哪

### 3.1 一句话机制（你能一句话复述的那种）

**内核不知道语言：它只吃三样东西——「谁是谁的孩子」（声明）·「这一行是谁」（身份＝命名空间 + 包内相对路径 + 名字）·「谁能调用谁」（证据）。给一门新语言写前端，就是为这三样各写一个来源，喂给同一个内核。**

本仓的两个现成实例：

- **声明今天不是 rustc 插件读的**：构建脚本用自己的词法解析器读源码里的 `root_object! { … }` 一类声明（`build_method` 的 discovery + 内核的 `source`/`syntax`）；宏展开只负责**编译期**那一半（写进 `static REGISTRATION`）。⇒ 新语言的"声明"只要能写成**同样可被词法读到的一段文本**（例如一个 C 头文件里的表），就不需要接编译器插件。
- **身份今天来自 `file!()` + 包名**：换成 C 就是 `__FILE__`（C++ 是 `std::source_location::file_name()`）+ 同一个包名口径 ⇒ **必须做同样的相对路径归一化**（`file!()` 是宏注入、grep 不到，也不是绝对路径），否则同一棵树在两种宿主语言下不是同一批 id。

### 3.2 三个选项

- **L-i 宿主语言先切（推荐）**：一个手工 C 宿主 + 一个 C 头文件形状的声明 + 一个**窄 C ABI**（只三件事：喂声明 / 算身份 / 取结论），产出**同一套 `.xirang/` 记录**。验收＝`check` / `registry` / `diff` / `affected` 对 C 宿主原样作答，且**同一棵树两种宿主语言得到同一批 NodeId**。**这一刀先不做 MIR**：证据那半只有 6 个文件消费（§1），`affected` 走的是词法源索引（`toolchain/src/mcp/src/affected.rs:20`），所以竖切可以先不碰它。
- **L-ii 证据来源先切**：宿主仍是 Rust，把 MIR 换成 Clang IR / DWARF，回答"跨语言调用链"。它**不**解决"非 Rust 项目也能有注册树"，只让已有 Rust 项目的调用证据伸到 C 侧。
- **L-iii 两者都要（先 i 后 ii）**：成本是 i 的两倍量级，但也是唯一能回答"跨语言调用链证据"的路。

### 3.3 推荐 L-i，并写明它**不**做什么

推荐 **L-i**。今天内核与执行面之间已经有一条干净的缝（内核＝协议词汇 + 纯方法、不做 I/O，`conventions::purity` 守着），C ABI 表面就该落在这条缝上：**窄**（三件事，不是"把内核暴露成 C"）、**不重写内核**（不为 C 把内核改成 C）、**不做运行时互操作**（不做"C 调用 Rust 对象"那一层）。你早先定过 **C++ 先不做**（风险大、群体不友好）⇒ Step 0 只做 C。

**天花板**：做完 = "非 Rust 项目也能声明注册树，并被同一套工具回答"。它**不**解决跨语言调用链证据（那是 L-ii），也**不**降低内核的 Rust 依赖。
**改完怎么测**：一个 C 宿主（放在检出之外）产出 `.xirang/` 记录 ⇒ `check: ok` 且面数与等价 Rust 宿主一致 ⇒ `registry` / `diff` / `affected` 的答案逐字可比。**反证**：把 C 侧 `__FILE__` 的相对化故意写错，`check` 必须红——不红就说明身份那半根本没接上。

---

## 4. ③ 分布式：先动哪一段

### 4.1 实测：分区今天**复制**发现，只拆渲染

**量器**（已收在 `target/xirang-bench/build-cost/`）：`gen_scale_host.py` 造一棵 1003 个 `.rs`、7.9 MB 源码、**1001 个面**的库宿主；`run-rig4.sh` **直接跑构建脚本二进制**（给它 `CARGO_MANIFEST_DIR` / `OUT_DIR` / `CARGO_PKG_NAME` 三个变量，与 W6-1 同一套协议），cold＝先删 `target/xirang/cache`，warm＝同树重建，每种形状 3 次。

| 形状（同一棵树、同一台机器） | cold | warm |
| --- | --- | --- |
| 单 crate 宿主（无声明，渲染 1001 面、写 1.47 MB） | 5.400 / 5.470 / 5.426 s | 3.377 / 3.410 / 3.362 s |
| 宿主（声明就位，**渲染 0 面**、写 5.8 KB） | 5.269 / 5.240 / 5.245 s | 3.422 / 3.343 / 3.396 s |
| **碎片**（幽灵，认领 `control`） | 6.886 / 6.894 / 6.888 s | 3.663 / 3.606 / 3.625 s |
| facade | 5.681 / 5.719 / 5.671 s | 3.794 / 3.796 / 3.809 s |

四条读数：

1. **一个碎片 ≈ 一个整宿主**：6.89 s vs 5.43 s = **1.27×**。原因逐字可见——碎片自己的 `build.rs` 把**宿主根**交给同一条管线：
   `run_for_partition("/tmp/xirang-lab/scale1000", &out, "scale1000", Some("control"), false)` ⇒ 走树、逐文件内容指纹、解析、校验**全量重跑**，只有"渲染哪一半"是它自己的。
2. **渲染那一份只值 3%**：宿主在分区形状里渲染 **0 个面**仍要 5.245 s；单 crate 渲染 1001 个面 + 写 1.47 MB 是 5.432 s ⇒ 差 **0.19 s（3.4%）**。
3. ⇒ 分区把构建脚本的**总 CPU 抬到 3.28×**（5.245 + 6.888 + 5.681 = 17.814 s vs 5.432 s）。墙时间的下限是三者中最大的那个 ≈ 6.9 s，仍高于单 crate 的 5.43 s——**这一条是推的，不是量的**：本次三个包是分开构建的，没有一次 `cargo` 调用同时跑它们。
4. **发现缓存只值三分之一**：cold−warm 的差是"逐文件解析"那一段（1.9–3.3 s）；warm 剩下的 3.4–3.8 s（≈单 crate 的 63%）是走树 + 逐文件内容指纹 + 校验 + 渲染 + 落盘，**都不在缓存里**。

⚠️ **跨量器可比性**：W6-1 的日志在 ~1003 面上记的是 **9.694 s**，我这次同量级面数是 **5.43 s**（1.8×）。两次的宿主形状、机器负载都不同 ⇒ **绝对值只在同一量器内可比，跨量器只比比例**；要合成一条曲线，得在同一台机器上把两种形状都重跑一遍（列进 ③ 落地后的第一件事）。

复跑方式（约 6 分钟，全程离线；脚本与逐次日志都在 `target/xirang-bench/build-cost/`，路径写死在 `/tmp/xirang-lab`）：

```sh
cargo build --offline --features cli --bin xirang      # 不带 --features cli 会得到旧二进制
mkdir -p /tmp/xirang-lab && cd /tmp/xirang-lab
<检出>/target/debug/xirang new scale1000 --lib
cp <检出>/target/xirang-bench/build-cost/gen_scale_host.py /tmp/xirang-lab/
sh <检出>/target/xirang-bench/build-cost/run-rig.sh     # ① 造 1000 个叶子面 ② 构建一次（专用 CARGO_TARGET_DIR）
sh <检出>/target/xirang-bench/build-cost/run-rig2.sh    # ③ 写 add_crates.rs → check → crates --write（末尾的 ls 会报错，那是脚本自己的行）
cd /tmp/xirang-lab && for p in scale1000-objects scale1000-facade; do \
  cargo build --offline --manifest-path crates/$p/Cargo.toml; done   # ④ 两个生成包各构建一次
sh <检出>/target/xirang-bench/build-cost/run-rig4.sh    # ⑤ 四种形状 × 3 次，cold/warm
```

两个装置事实值得记住：宿主自带 `[workspace]` 时，生成包落在**它旁边**的 `crates/`（`xirang crates --write` 会自己说清为什么、以及该怎么构建它们）；构建必须 `cd` 到分区根再做，remap 靠 cwd 找 `.cargo/config.toml`。

### 4.2 三条路各自的天花板

- **(A) 把构建脚本本身分片。** 今天分区给的是**产出单位**，不是**发现单位**（§4.1）。要做的是把管线切成"发现 → 可共享记录 → 渲染"，让 N 个碎片只付 **1 份发现 + 各自那份渲染**。**天花板**：把 10k 面那 103.9 s（占墙 73%）里今天被乘了 N 倍的那 97% 收成 1 份；**动不了的**是渲染与落盘（每包必付，实测 0.19 s @1000 面）与 rustc 那一段。
- **(B) 远程缓存 / 执行（sccache 那一路）。** 按构造只覆盖 rustc 那一段：10k 面 **26.7%**（37.75 s）＋ 冷启动依赖编译（~80 s，一次性）。而 rustc 是**超线性**的（103 / 1003 / 4644 / 10003 面 = 0.41 / 1.96 / 14.25 / 37.75 s），生成物又每轮都变（10k 面 `generated_lib.rs` 12 MB）⇒ 命中率有结构性上限。**结论**：它是一条**正交**、随时可加的缓存层，**不该占设计预算**。
- **(C) 多机并行产出分区产物。** 分区真正能兑现的一条，但被 (A) 的缺口卡住：每台机器都要有宿主源码树（否则发现跑不了），而每台又要付全量发现 ⇒ 多机收益被"每台都做一遍 73%"吃掉。

### 4.3 推荐 ③A→C：(A) 的前半句，然后 (C)；(B) 不投入设计

**先把"发现"变成一份可共享的记录**（谁发现了什么、每个面的身份、校验结论），碎片只读记录 + 渲染自己那份 + 写自己的产物；这份记录本身可以随分发走到别的机器——**那正是 (C) 需要的东西**。

注意：`target/xirang/cache` **已经是**"可共享记录"的形状（内容寻址的 unit 文件 + 原子写，`discovery_cache.rs:47`），但它只缓存"解析"，不缓存走树与校验，也没有"谁认领了哪棵子树"的全局视图 ⇒ 要加的是后两样，不是从零造。

**改完怎么测**：同一棵 1000 面树、同一台机器，三个包的构建脚本**总 CPU 从 17.8 s 降到 ≈6–7 s**（1 份发现 + 3 份渲染），且 `graft_plan.tsv` 与逐包面归属**逐字不变**（用既有的 `tools/xirang-real-usage` 那四条归属断言守着）。**否定断言**：不许靠"碎片少渲染东西"取胜——那是改语义，会把归属门禁弄红。
**天花板**：10k 面时构建脚本墙从 103.9 s 降到"1 份发现 + 若干并行渲染"；配合 (C) 才能让 N 台机器真的各干一份。rustc 那 27%（超线性）仍要靠 (B) 那类缓存或更强的并行。

---

## 5. 我建议现在不做的

- 不做"同一棵树按平台 / 按 target 分叉内容"（①b）：与身份红线冲突，且是**记录格式**级设计，不是修缺陷。
- 不为 C 把内核改成 C，也不做运行时互操作（②的边界）；**C++ 不先做**（你早先定过）。
- 不在 MCP 层做缓存 / 索引（既有纪律）；(B) 不占设计预算。

---

## 6. 顺手发现（只记，没改）

1. **`tools/gen_scale_tree.py --host` 生成的宿主今天编译不过**：模板里还写着改名前的 `xirang_toolchain::build_time::run()` 与 `runtime::host!()`。实测：

   ```
   $ python3 tools/gen_scale_tree.py /tmp/xirang-lab/gen-probe 3 --host && cargo build
   error[E0433]: cannot find `build_time` in `xirang_toolchain`
    --> build.rs:2:23
   ```

   ⇒ 产生 W6-1 那张 10k 面表的**装置现在不可复现**（两行改名即修）。同一批：`examples/control-suite/*/add_crates.rs` 里还写着 `xirang_toolchain::runtime::{Crate, Shape}`（声明文件不参与编译，所以没红）。
2. 我第一版量器把 `pub mod control;` 写进 `lib.rs` ⇒ `E0428`（`host!()` 已经生成模块树）——记忆里记过的夹具坑，我又踩一次，记在此处给下一个人。

---

## 7. 请你回的三行

```
① 跨平台口径：a（三平台成立 + 任意 target 同一批 id —— 推荐） / b（按平台分叉内容）
② 多语言第一刀：i（C 宿主经 C ABI 用同一内核 —— 推荐） / ii（先换证据来源 MIR→Clang/DWARF） / iii（都要）
③ 分布式先动哪段：A（先把发现做成可共享记录 —— 推荐） / B（远程缓存） / C（多机分区产物） / A→C
```

## 8. 另外两件还等你一句话的（与本批无关，顺手问）

1. **新线的 tag 命名**：远程 `v0.1.0`–`v0.1.5` 是旧线（NichLink）占着的，已推的 tag 不许改写 ⇒ 建议起 `xirang-v0.1.0` 并把 `release.yml` 触发扩成 `["v*", "xirang-v*"]`（版本号仍从 tag 里去掉前缀）；或者暂不打标（crates 已在 index，只少一个 GitHub Release 页）。
2. **旧 crate 处置**（`nichlink-*` 0.2.2）：不动 / **yank（推荐）** / 删除（条件：发布 <72 h + 单一 owner + 无反向依赖 + 月下载 <1000；删后名字 24 h 后对所有人开放、不可逆）。
