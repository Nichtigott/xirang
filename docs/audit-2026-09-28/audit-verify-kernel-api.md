# t11 独立验证 — 内核公开紧凑渲染入口与 Studio 改调

- **验证对象**：t7（内核公开 `compact_admission` + `:48/52` 历史分支并入 + X-1/LG-40）与 t8（Studio 删除第二份实现）。
- **身份**：我是这两条的作者之外的独立验证者；本文件之外的写入只发生在 `/tmp`（装置见第 5 节），**没有改动工作树里的任何源码**，没有 `git checkout/restore/stash`，没有 commit。
- **工作树状态**：`8c646c4c780dad9d`（`git status --porcelain` 排序 + `git diff` 的 sha256 前 16 位）。五条门禁那场跑**跑前=跑后=同一个 hash**（详见第 8 节）。
- **判定汇总**：acceptance 4/4 通过；**0 条阻断项**；3 条"表述边界"写在第 6 节（都不是实现缺陷）。

## 1. 入口真的公开可用（crate 外调用）

**装置**：`/tmp/t11-ext`，一个与工作区无关的 cargo 工程：

```toml
[dependencies]
xirang-core = { path = "/home/nich/Moirai_N3/nichlink/core", features = ["syntax"] }
xirang-run-method = { path = "/home/nich/Moirai_N3/nichlink/run_method" }
```

`CARGO_TARGET_DIR=/tmp/t11-ext-target cargo run --offline`，输出原文：

```
kernel: ANY OK
shim:   ANY OK
kernel: allow:a,b OK
shim:   allow:a,b OK
kernel: deny:c OK
shim:   deny:c OK
kernel: allow:ui;deny:ui/experimental OK
shim:   allow:ui;deny:ui/experimental OK
EXTERNAL-CALL-OK
```

它做的三件事，每一件都是 crate 外身份：① `xirang::authoring::parse::compact_admission(&OwnedAdmission{…})`
（注意类型必须写模块路径 `xirang::declaration::OwnedAdmission`：根上**没有**裸名 `OwnedAdmission`——
第一版探针就是这样拿到 `E0432: no `OwnedAdmission` in the root`，说明这条调用确实是从 crate 外按公开
路径走的）；② 用 `parse_admission_owned` 把渲染结果读回并与原策略 `assert_eq!`（渲染器与解析器互钉）；
③ 走 `xirang_run_method::authoring::parse::compact_admission`——Studio 实际调用的那条 shim 路径——
断言同一份字节。

**必要特性**（负向实测，不是猜）：把 `features = ["syntax"]` 去掉后，同一个探针：

```
error[E0432]: unresolved import `xirang::authoring::parse`
note: found an item that was configured out
  --> core/src/registry_core/authoring/authoring.rs:20
   | #[cfg(feature = "syntax")]
   |       ------------------ the item is gated behind the `syntax` feature
   | pub mod parse;
```

⇒ 外部消费方**必须**开 `syntax`（`core/Cargo.toml` 的注释与 `[package.metadata.docs.rs] all-features = true`
都覆盖它）。

**修前不可公开调用**（我的独立证据，不是复述作者）：`git show HEAD:core/src/registry_core/authoring/parse/admission.rs`
里只有 `fn compact_admission(allow: &[String], deny: &[String]) -> String`——**没有 `pub`**，签名也不是
`&OwnedAdmission`。因此 HEAD 上任何 crate 外代码都调不到它。

**判定：通过。**

## 2. `:48/52` 并入：字节不变由构造证明

我把 HEAD 版（`git show HEAD:…/admission.rs`）的四个分支与现在的实现逐条对照：

| 分支 | HEAD（作者动手前） | 现在 | 字节 |
| --- | --- | --- | --- |
| 空 / `Admission::ANY` | `return Ok("ANY".to_owned());` | `compact_admission(&{[], []})` → arm `(true,true) => "ANY".to_owned()` | 相同 |
| `allow_paths(` | `return Ok(format!("allow:{}", paths.join(",")));` | `compact_admission(&{paths, []})` → `(false,true) => format!("allow:{}", allow.join(","))` | 相同 |
| `deny_paths(` | `return Ok(format!("deny:{}", paths.join(",")));` | `compact_admission(&{[], paths})` → `(true,false) => format!("deny:{}", deny.join(","))` | 相同 |
| `Admission::new(` | `compact_admission(&allow, &deny)`（同一四臂体） | 同一四臂体，只是参数换成 `&OwnedAdmission` | 相同 |

四臂体的两个文本我逐字比对过：公开渲染器的 `match` 与 HEAD 的私有函数**逐字节相同**（含 `;`
分隔符、`,` 连接符、`ANY` 字面量）。因此"历史拼法逐字节不变"**由构造成立**，不依赖作者的
"跑前跑后各一次"记录——那条记录我也复跑了（`core/tests/compact_admission_entry.rs`，见第 8 节全绿）。

函数开头（`expression.trim().trim_end_matches(',')`）与末尾的拒绝文本也与 HEAD 一致。

**判定：通过。**

## 3. Studio 侧不再有第二份实现

### 装置 A：我自写的结构检查器（不是看作者的 grep）

`/tmp/t11-check-second-implementation.py`：遍历工作区全部 `*.rs`，用**字符串感知**的剥注释器
（处理 `"…"`、`r#"…"#`、`//`、`/* */`）留下代码，然后问三个问题：

- **H1** admission 语法**专属**的字面量 `"allow:` / `"deny:` 出现在哪些代码里（有意**不**把 `"ANY"`
  当判据：注册规则族也用 `ANY` 表示空规则，它区分不了两套语法——第一版把它当判据时，立刻在
  `face_field_presentation`、`registration_rule_text`、`face_field_value` 等处产生假阳性）。
- **H2** 哪些函数同时"读 `allowed_paths`/`denied_paths`"且"拼字符串"（`format!`/`.join(`/`.push(`）——
  这是**再现一份副本**必然留下的形状；位于 `core/`、带断言（`assert*`/`panic!`）或不在出厂表面
  （`tests/`、`_tests.rs`、`examples/`）的命中记为 justified。
- **H3** 用花括号配对自行切出 Studio 的 `admission_text` 函数体，断言它调用 `compact_admission`
  且体内没有任何拼装针。

**绿（出厂树）原文摘要**：

```
production hits outside the kernel: 0
H3 studio admission_text body:
   calls the kernel entry: True
   assembly needles inside: []
VERDICT second-renderer-outside-kernel = NONE      (exit 0)
```

**变异反证（证明检查器本身有牙）**：在 `/tmp/t11-mut`（我自己的树外副本）把 `admission_text` 换回
手拼副本（`format!("allow:{}", …join(","))` / `format!("deny:{}", …)` / `clauses.join(";")` / `"ANY"`），
同一个检查器：

```
production hits outside the kernel: 3
   FINDING studio/studio/app/source_index.rs::admission_text  "allow:
   FINDING studio/studio/app/source_index.rs::admission_text  "deny:
   FINDING studio/studio/app/source_index.rs::admission_text  renderer shape
VERDICT second-renderer-outside-kernel = FOUND      (exit 1)
```

⇒ 检查器不是"恒绿的看客"：副本一回来它就红。

### 装置 B：作者的 pin 我复跑 + 变异

- 出厂树：`cargo test -p xirang-studio --offline --all-features --lib admission_text_tests` →
  `3 passed; 0 failed`（EXIT=0）。
- 在副本变异（把 `admission_text` 换成手拼副本）后：**只有结构性那条红**——
  `2 passed; 1 failed`，失败者是 `the_call_site_carries_no_second_compact_renderer`。
  ⇒ 两条行为测试（与内核逐字节比较、与解析器互钉）**看不出副本**，因为副本的字节恰好相同；这正是
  作者那条结构 pin 必须存在、且**确实有牙**的理由。这一条与我的装置 A 互为独立复核。

### 装置 C：改坏内核，Studio 必须变红

在 `/tmp/t11-mut` 把公开渲染器的两列表 arm 改成丢掉 deny 子句（**FIXR-01 的放宽形状**）：

```rust
// 变异前：(false, false) => format!("allow:{};deny:{}", allow.join(","), deny.join(",")),
// 变异后：
(false, false) => format!("allow:{}", allow.join(",")),
```

- **对照组**（同一副本、未变异、同一 target 目录）：`cargo test -p xirang-studio --offline --all-features --lib admission_text_tests`
  → `3 passed`，EXIT=0。
- **变异组**：`cargo test -p xirang-studio --offline --all-features --lib` → `95 passed; 5 failed`，
  失败清单原文：

```
studio::app::keyboard::edit_form_tests::saving_an_untouched_edit_form_keeps_the_source_deny_list
studio::app::keyboard::edit_form_tests::the_edit_form_prefill_keeps_both_admission_lists
studio::app::source_index::admission_text_tests::both_lists_survive_the_compact_rendering
studio::app::source_index::admission_text_tests::the_call_site_renders_what_the_kernel_renders
studio::app::tests::project::new_project_and_explicit_root_face_compile
```

其中 `the_edit_form_prefill_keeps_both_admission_lists`、`saving_an_untouched_edit_form_keeps_the_source_deny_list`
是**用户可见面**（Edit 表单预填、未改动表单保存时保住 deny 列表）的端到端测试 ⇒ Studio 不只是
"看着像调用了"，它的用户可见行为**经内核**。对照组的绿说明失败不是变异环境的噪声。

**判定：通过。** 另外：全树只有**一个** `fn compact_admission`（`grep -rn "fn compact_admission"` →
仅 `core/src/registry_core/authoring/parse/admission.rs:73`），Studio 侧只有一个 `fn admission_text`
（`studio/studio/app/source_index.rs:69`）。

## 4. X-1 / LGC-LG-40 的语义

**装置**：`/tmp/t11-x1`（crate 外工程，path 依赖 `core`）。夹具是我自造的十字段锁行，三列来源字段
**为空**：

```
official|framework-x|package-x|1.0.0|crate-x|checksum-x|extension|||
```

候选记录由我自建（`PluginRecord` 全字段公开，`PluginSource::parse("official")` / `PluginMode::parse("extension")`）。
规则本体是 `contains_record` —— 我读码确认 `contains_manifest` 只是把一个 manifest 搬进
`PluginRecord` 再调它（第 6 节第 1 条说明了这一替代的边界）。

**修后（现状）输出原文**：

```
parsed empty columns: signature=Some("") fingerprint=Some("") revocation=Some("")
ten-field(empty) accounts for signed        = false
ten-field(empty) accounts for unsigned      = true
ten-field(empty) accounts for signature-only= false
seven-field accounts for signed             = true
```

即：空列被解析成 `Some("")`（**声明"此处没有值"**，不是 `None`）；带签名的候选被拒（不再放行）；
同样没点值的候选仍被覆盖；七字段形式（从未提到）仍覆盖带签名的候选——**区分是真实的，不是一刀切拒绝**。

**红侧（"修前会被静默当成没提到"）**：在 `/tmp` 副本把作者删掉的过滤器加回
（`fields.get(7).filter(|value| !value.is_empty()).map(ToString::to_string)`，其余不动），同一探针：

```
parsed empty columns: signature=None fingerprint=None revocation=None
ten-field(empty) accounts for signed        = true
ten-field(empty) accounts for unsigned      = true
ten-field(empty) accounts for signature-only= true
seven-field accounts for signed             = true
```

两次运行**只差那一处过滤**，而 `accounts for signed` 从 `false` 翻成 `true`：一份**写明"没有签名"**
的十字段锁记录会被读成"没提到"，从而给带签名的 manifest 放行——X-1 / LGC-LG-40 的洞就是这一行。

**判定：通过**（语义＝明确标注缺失，且"缺失"与"没提到"可区分）。

### 4.1 取舍本身可接受吗？——"改成拒绝会打红谁"的实测与我的判断

作者选的是"明确标注缺失"而不是"拒绝"，给出的理由是 Studio 把 `…|extension|||` 当合法种子。
我按队长要求独立判这条，并**实测**了另一条路的代价。

**实测（"拒绝"语义）**：在 `/tmp/t11-mut2`（当前树的**新副本**，只改一处）把拒绝写进
`PluginCatalog::parse`——`fields.len() == 10 && fields[7..10].iter().any(|f| f.is_empty())`
即 `Err(PluginLockError::new(line_number + 1, "has an empty provenance column"))`，其余一字不动：

```
cargo test --workspace --offline --no-fail-fast     # 变异组
787 passed; 4 failed; EXIT=101
```
红的四条（原文）：

```
an_empty_provenance_column_is_not_the_seven_field_form          (xirang-core --test plugin_lock_provenance)
an_explicitly_empty_provenance_column_pins_absence              (同上)
studio::app::mutations::lock_writes::an_official_append_that_would_duplicate_an_identity_is_refused_not_written
studio::app::tests::project::new_project_and_explicit_root_face_compile
```

**对照组**（`/tmp/t11-ctrl`，同一份树的**未变异**副本，各自**独立 target 目录**）：

| 命令 | 对照组 | 变异组 | 归因 |
| --- | --- | --- | --- |
| `-p xirang-core --test plugin_lock_provenance`（独立 target） | `3 passed; 0 failed` | `1 passed; 2 failed`，报错 `the ten-field lock parses: PluginLockError { line: 1, message: "has an empty provenance column" }` | **归因于"拒绝"** |
| `-p xirang-studio --all-features --lib lock_writes`（独立 target） | `4 passed; 0 failed` | `an_official_append_…_refused_not_written` 红 | **归因于"拒绝"**（该测试断言 `PluginCatalog::parse(&text).expect("the seeded lock still parses")`，种子正是 `…|extension|||`） |
| `-p xirang-studio --all-features --lib new_project_and_explicit_root_face_compile`（独立 target） | **也红** | 红 | **不归因**：两边的报错都是离线环境里生成的样例工程解析 `xirang-run-method = "^0.1.6"` 失败（`location searched: Git repository … Nichtigott/xirang?branch=main / candidate versions found: 0.1.5`）；真实工作树里这条**是绿的**（`1 passed`），因此它是 `/tmp` 副本的离线解析产物，与本次变异无关 |

⇒ "拒绝"归因明确的爆炸半径 = **3 条测试**：core 自己的 X-1 钉子两条 + Studio 写入路径一条
（外加一条与它无关、在任何副本里都红的离线解析测试）。也就是说，改成拒绝会先把**仓库今天明确当合法
输入**的那种拼法判死，并且是在 core 的 X-1 钉子本身上判死。

**装置教训（写给下一轮用同一手法的复核者）**：我第一次的对照组是**错的**——两份 `/tmp` 副本共用工作区
`target/`，而 cargo 复用了一个测试二进制（两份日志里 `Running … plugin_lock_provenance-9efa651de66d2c42`
是同一个 hash，只在 19:54 构建过一次），于是"对照"跑的是变异组的二进制、报出变异组的错误文本。
发现方式正是"对照组与变异组出现同一句只有变异代码里才有的错误文本"，再由
`grep -rn "has an empty provenance column" .` 在真实树里 0 命中坐实。**结论：copy-based A/B
必须给每份副本一个自己的 `CARGO_TARGET_DIR`**（或把被测包强制重建）；本文件 §3-C、§4 的其余
变异都在**同一路径**里做内容改动（强制重建），因此不受这条影响。

**我的判断：可接受，而且比"拒绝"更好。** 四条理由：

1. **安全方向已经堵住**：§4 的 `false → true` 翻转就是 X-1/LG-40 的要害——写明"没有签名"的记录不再给带
   签名的 manifest 放行。选"明确标注缺失"就足以修好这个洞；"拒绝"在**效果上并不更强**，它只是更早、
   更粗暴地拒绝**整份锁**而不是拒绝那一条候选。
2. **"拒绝"的爆炸半径打在可用性上，而且不可局部修复**：它会让我们**今天接受**的合法锁记录变成解析
   失败；宿主的失败模式是整体性的（`plugin-host` 的准入读两份锁，失败即
   `HostError::Policy("invalid plugin lock under …")`，插件面**完全不可用**），运维只能手改锁文件才能
   恢复。为一个"记录比它写的更宽"的洞引入"升级后插件全停"，代价明显更大。
3. **本仓自己已经把这种拼法当合法输入**：`studio/studio/app/tests/lock_writes.rs` 用它做种子并断言
   `PluginCatalog::parse(&text).expect("the seeded lock still parses")`；把它改成拒绝，等于同一次改动里
   先宣布那份种子非法。
4. **残留风险（应写进文档而不是忽略）**：`Some("")` 与 `None` 的区别**在写入方 API 上看不出来**——今天
   树里没有任何写入方写空列（Studio 写七字段），所以不存在"想表达 unknown、却被读成 declares-none"的
   现实场景；这层语义只由 `PluginRecord` 三个字段的文档承载（t7 已补）。未来若出现写空列的写入方，
   它必须知道**空列＝声明缺失**，而不是"未知"。

**顺带发现（不在我 t11 的 inScope，故只报告不修改）**：`studio/studio/app/tests/lock_writes.rs` 那条
测试的文档注释仍写着 "A ten-field official record with empty provenance **parses as the seven-field
form**, so the trust rule admits a duplicate identity…"。t7 之后这句不再成立——空列现在解析成 `Some("")`，
与七字段（`None`）是**两种**形态（这正是 `core/tests/plugin_lock_provenance.rs::an_empty_provenance_column_is_not_the_seven_field_form`
钉住的事）。该测试的**结局**（那次追加被可见地拒绝）不变、套件全绿，因此这是**注释漂移**而非行为缺陷；
文件属 t7/Studio 侧的 inScope，请 owner 顺手改一句。

## 5. 装置、保真性与复现命令

| 装置 | 路径 | 用途 |
| --- | --- | --- |
| crate 外探针 | `/tmp/t11-ext`（target `/tmp/t11-ext-target`） | §1 公开调用、（负向）特性要求 |
| 结构检查器 | `/tmp/t11-check-second-implementation.py` | §3 H1/H2/H3，且经变异反证 |
| 变异树 | `/tmp/t11-mut`（target = 工作区 `target`，只复用依赖编译产物，不改工作区源码） | §3-C 内核变异、§3-B 副本变异、§4 红侧 |
| "拒绝"变异树 | `/tmp/t11-mut2`（该实验的测量用共享目标目录，见 §4.1 的装置教训） | §4.1 "改成拒绝会打红谁" |
| "拒绝"对照组 | `/tmp/t11-ctrl` + **独立 target `/tmp/t11-ctrl-target`** | §4.1 对照（同路径内 A/B 之外的副本对照） |
| X-1 探针 | `/tmp/t11-x1`（target `/tmp/t11-x1-target`） | §4 夹具 |
| 证据留档 | `/tmp/t11-check-green.txt`、`/tmp/t11-check-mutant.txt`、`/tmp/t11-x1-green.txt`、`/tmp/t11-studio-control.log`、`/tmp/t11-reject3.log`（变异组）、`/tmp/t11-gates2.log`、`/tmp/g-*.log` | 上面引用的原文 |

复现：

```sh
# §1
cd /tmp/t11-ext && CARGO_TARGET_DIR=/tmp/t11-ext-target cargo run --offline
# §3-A
python3 /tmp/t11-check-second-implementation.py /home/nich/Moirai_N3/nichlink
python3 /tmp/t11-check-second-implementation.py /tmp/t11-mut
# §3-C（/tmp/t11-mut 的 core 里把两列表 arm 改坏后）
cd /tmp/t11-mut && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target cargo test -p xirang-studio --offline --all-features --lib
# §4
cd /tmp/t11-x1 && CARGO_TARGET_DIR=/tmp/t11-x1-target cargo run --offline
# §4.1（拒绝语义；每份副本各自的 target 目录——见该节的装置教训）
cd /tmp/t11-mut2 && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target cargo test --workspace --offline --no-fail-fast
cd /tmp/t11-ctrl && CARGO_TARGET_DIR=/tmp/t11-ctrl-target cargo test -p xirang-core --offline --test plugin_lock_provenance
cd /tmp/t11-ctrl && CARGO_TARGET_DIR=/tmp/t11-ctrl-target cargo test -p xirang-studio --offline --all-features --lib lock_writes
```

**保真性**：所有源码修改都在 `/tmp`；工作树里我只新增本文件。五条门禁那场跑在 hash 钉住的树上
（见第 8 节）。另需说明一次**被作废的瞬时红**：`8c646c4c` 状态的第一次门禁尝试里，
`xirang-core --test registration_rule_entry` 与 `xirang-studio (lib)` 各出现一次编译错误
（E0432 / E0425），而**紧接着在同一 hash 下重跑全部命令即全绿**（第 8 节）——那是别人在飞文件的
瞬时状态，按队规作废重跑，不计入判定。

**本文件是两段**：第 1–8 节是 t11 首轮交付（验证状态 `8c646c4c`）；第 4.1 节是队长追加要的
"X-1 语义取舍的可接受性判断 + 改成拒绝会打红谁的实测"，它在**该轮之后**的树上做（副本
`/tmp/t11-ctrl`、`/tmp/t11-mut2`，即当前树 + 只一处变异），因此 4.1 的数字不与第 8 节的 hash 同源，
两者各自标明了自己的基准。

## 6. 未覆盖范围与表述边界

1. **`contains_manifest` 本体我没直接调用**：它的入参是 `PluginManifest`（字段多、含 `FrameworkId`
   等 newtype），crate 外没有现成的文本构造器。我改走 `contains_record`＝同一规则本体
   （`accounts_for` 三条链），并读码确认包装只是字段搬运；若评审要求"逐字 API 覆盖"，这一条是缺口。
2. **§2 的字节不变性是读码 + 对照 git 历史**，不是"改前改后各跑一次"的实测（后者是作者的记录）。
   两种手段结论一致，但口径不同，故写明。
3. **"内核唯一渲染器"的"唯一"指紧凑拼法**。`render_admission` 仍渲染
   `crate::Admission::new(&[…], &[…])` 这一 **Rust 构造拼法**（另一种拼法，不是第二份紧凑渲染器）；
   把这句话读成"内核只有一处渲染 admission"会与事实不符，t7 的文档措辞已限定在"紧凑拼法"上。
4. 我未跑 `cargo test --workspace --all-features --doc`（CI 的额外一条）与
   `tools/xirang-package-audit` / `--verify-consumers`（前者与本批新增符号无关、后者需要 index，
   见第 7 节）；`tools/xirang-release-audit` 未实跑（需构建 artifact）。
5. §4.1 的"打红谁"名单里，`studio::app::tests::project::new_project_and_explicit_root_face_compile`
   在**对照组里也红**（离线解析 `xirang-run-method = "^0.1.6"` 时本地 git 缓存只有 0.1.5），
   而真实工作树里同一条是绿的 ⇒ 它是 `/tmp` 副本的环境产物，**不计入**"拒绝会打红谁"。
   若要一条不依赖副本环境的名单，应在真实树里对同一处做同样的变异（需要工作树写权限，本轮没有）。

## 7. 本批新增公开符号对版本线与 `tools/xirang-package-audit` 的含义

- **新增公开符号 = 1 个**：`xirang::authoring::parse::compact_admission`（crate `xirang-core`，lib 名
  `xirang`），位于 `#[cfg(feature = "syntax")]` 之后（§1 负向实测）。它同时经 run_method 的
  glob shim 出现在 `xirang_run_method::authoring::parse::compact_admission`——Studio 走的就是这条，
  因此**发布面**（`xirang-core` 与 `xirang-run-method` 两个 crate）各多一个可用路径。
- **版本线**：这是**向后兼容的公开新增**（不破坏任何调用方；`Admission::new` 等既有拼法不变）。
  但在本仓 `0.1.x` 线上，`release_version` 与 `tools/xirang-publish --check-table` 都要求内部
  `xirang-*` 要求 **==** 工作区版本，所以它不会"只发 core"：下一次抬线时 19 处要求行与两个
  `publish = false` 宿主的同一行一起移动（后者正是本队 B2-1/`G-05` 刚扩进门禁的那类行）。
  换句话说：**它是 0.1.z 线上的兼容新增，但发布动作是一条线，不是一个 crate**。
- **`tools/xirang-package-audit`**：它检查两件不同的事。①**内容**半段（每个 `src/**/*.rs` 模块与
  声明的 README 必须在包里）——新符号落在**既有模块** `authoring/parse/admission.rs` 内，**不新增文件**，
  因此对该半段没有影响；②**打包**半段（从 tarball 隔离构建）只与"带版本号内部依赖是否已上 index"有关，
  与本符号无关。⇒ 本批新增符号对 package-audit **无行为影响**，不需要新的清单/表动作。
- 旁证：`tools/xirang-release-audit` 的 `defined_symbols` 会因新符号变化，但该脚本**只报告、不设基线
  断言**（读码：`:44` 计数、`:158` 打印），因此不构成任何门禁口径的变更。

## 8. 五条门禁（hash 钉住的同一棵树）

`8c646c4c780dad9d`（跑前）→ 同一状态（跑后）：

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | EXIT=0，零输出 |
| `cargo test -p xirang-core --offline --features syntax` | EXIT=0；8 个 test 二进制，**241 passed / 0 failed** |
| `cargo test -p xirang-studio --offline --all-features` | EXIT=0；5 个二进制，**114 passed / 0 failed** |
| `cargo test -p xirang-conventions --offline` | EXIT=0；2 个二进制，**113 passed / 0 failed** |
| `cargo test --workspace --offline` | EXIT=0；55 个二进制，**791 passed / 0 failed**（FAILED 计数 0） |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | EXIT=0（`Finished dev profile … in 1.00s`，无 warning） |
| `tools/xirang-publish --check-table` | EXIT=0：`dependency table matches the manifests (9 crates)` |

（作废的第一次尝试见第 5 节末；它没有产出任何判定。）

---

**结论**：t7 与 t8 的 acceptance 全部成立，且每一条都用了与作者不同的装置：crate 外调用、
自写并变异反证过的结构检查器、改坏内核的端到端变异、以及自造夹具的前后对照。没有发现阻断项；
第 6 节的三条边界与第 7 节的发布面含义请评审与队长按各自口径取用。
