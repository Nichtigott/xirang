# 独立验证：`K-10` 家族（严格注册规则读取器 + 生产调用点）

- 验证者：**bridge-auditor**（t75，**非作者**：`K-10` 的两笔交付 `t66`/`t72` 由 run-engineer 完成，我一条未参与）
- 对象：`K-10` 家族——`t66` 在 `core/src/registry_core/authoring/parse/rules.rs` 新增的 `try_rule_syntax_from_text`（只读 `REGISTRATION_RULE` 初始器的严格读法），与 `t72` 把生产调用点 `run_method/authoring/parse/parse.rs` 的 `rule_syntax_for_source` 切到该入口
- 日期：2026-09-28（本检出，`--offline`）
- 源码：**真实树零改动**（本轮我只写本文件；六份被验证源码/夹具文件前后 `sha256sum -c` 全部 `OK`，见 §7）
- 手段：**自写探针**（`/tmp/t75-copy/run_method/tests/k10_probe.rs`，6 个用例）驱动**公开生产入口** `generated_snapshots_from` 与公开的 `try_rule_syntax_from_text` / `rule_syntax_from_text` 一对；夹具由探针自建（含一份出厂面文件 + 自写规则文件）；**3 组变异**回旧行为；出厂三份 `registry_rule.rs` 反过火核对；`examples/control-button --test registry` 独立复跑
- 副本：`/tmp/t75-copy`（`tar` 副本）+ 自带 `CARGO_TARGET_DIR=/tmp/t75-target`；变异后逐文件 `sha256sum -c` 还原

## 0. 判定一览

| 条目 | 判定 | 一句话 |
| --- | --- | --- |
| `K-10`-`t66` 严格读取器 | **证实** | 只读 `REGISTRATION_RULE` 初始器：注释里的伪子句不参与（①）；认不出的形状被拒绝（②）；缺常量/非构建器初始器/未知子句/非内联参数都有专门理由 |
| `K-10`-`t72` 生产调用点 | **证实** | `rule_syntax_for_source` 走严格入口，拒绝时错误含**面文件名 + 规则文件名**（②）；规则文件缺失仍 `Ok("ANY")`（③）；不再使用宽容入口 |
| 形状① 的口径（“应读出、不该报错”） | **同意（作者的更正口径正确）** | 注释不是子句、初始器本身毫无歧义 ⇒ 报错等于拒绝合法输入（§3.1） |
| 形状② 的口径（常量指名的清单应 `Err`） | **同意** | 读取器**看不见**清单内容，静默降级成 `ANY` 会丢掉作者声明的结构要求，而 `ANY` 是有意义的值（§3.2） |
| 形状③ 的口径（缺文件仍 `Ok("ANY")`） | **同意** | 缺席不是畸形；`ANY` 正是“没有结构要求”（§3.3） |
| 变异反证 | **3/3 红** | 只读初始器→读整份文本：2 例红；静默降级：1 例红；生产路径改回宽容入口：2 例红；还原后 `sha256` 零残留（§4） |
| 反过火 | **证实** | 出厂 3 份规则文件两入口逐字节一致；其中 2 个面经生产入口成功；渲染器→严格读法往返成立；`control-button --test registry` 25 passed（§5） |
| 五条门禁 | **全绿** | 见 §7 |

## 1. 装置：测什么、怎么测

生产入口的选取是本节的关键。`rule_syntax_for_source` 本身是 `pub(super)`，因此我不去调它、也不读作者的钉子；我调到的是**同一个公开生产路径**：

```
xirang_run_method::generated_snapshots_from(root)        # MCP `xirang.apply` 预览与 Studio 刷新所用
  → FaceManifest::parse_source(path)
    → manifest::parse::source(path)                        # run_method/authoring/manifest/parse/parse.rs:261
      → rule_syntax_for_source(path)                       # run_method/authoring/parse/parse.rs:88
        → xirang::authoring::parse::try_rule_syntax_from_text(text)
```

三条形状各有一个自建夹具：一个一次性源码根，里面放一份**出厂面文件**（`examples/control-button/src/control/control.rs` 或 studio fixture 的同名文件）作为“真面”，规则文件由探针写入；形状③干脆不写规则文件。这样生产遍历看到的面与它在检出里看到的面同形。

## 2. 三条形状：逐字实测值

探针里有一条只打印取值、供本文件逐字引用的用例（`report_the_measured_values`）。下面全部是它的原样输出。

### 形状① 注释里的伪子句

夹具（注释落在常量 `=` **之后**，正是宽容读取器“先找第一个 `=`、再找第一个标记”会先撞到注释的位置）：

```rust
const REGISTRATION_RULE: RegistrationRule = // a comment carries: .require_exports(&["wrong"])
    RegistrationRule::new().require_exports(&["control.render"]);
```

```
① strict   = Ok("exports:control.render")
① tolerant = "exports:wrong"
```

生产入口在同一个夹具上读出 `exports:control.render`（探针断言 `compact_registration_rule(快照.registry_rule) == "exports:control.render"`）。**即：严格读法读真规则，宽容入口读注释里的伪子句**——这正是这一族的红侧。

### 形状② 清单由常量指名

```rust
const EXPORTS: &[&str] = &["control.render"];
pub const REGISTRATION_RULE: RegistrationRule =
    RegistrationRule::new().require_exports(EXPORTS);
```

```
② tolerant   = "ANY"
② strict     = Err(FaceParseError { message: "`.require_exports(…)` needs a list of string literals" })
② production = Err("/tmp/…/control/control.rs: face `…/control/control.rs`: registration rule source \
                    `…/control/registry_rule/registry_rule.rs` was refused: `.require_exports(…)` needs \
                    a list of string literals")
```

三条要点都在原文里：宽容入口**静默**给出 `ANY`（作者声明的结构要求消失）；严格入口拒绝；生产入口拒绝且错误**同时点名面文件与规则文件**（`face \`…control.rs\`` 与 `registration rule source \`…registry_rule.rs\``）。

### 形状③ 规则文件缺失

```
③ production ok, rule = "ANY", registry_rule_path = "src/control/registry_rule/registry_rule.rs"
```

生产入口成功、规则为 `ANY` —— **缺席不是畸形** ✓。`registry_rule_path` 的取值是本节 §6 的一条观察。

## 3. 对作者更正口径的独立判断

### 3.1 形状①为什么**应当读出结果**，而不是报错

**我同意作者的口径（读出真规则）。理由是可证的，不是口味：**

1. **注释在语法上不是子句。** 严格读法经由 `syn::parse_file` 拿到的 AST 里根本没有那条注释；初始器只有一条 `RegistrationRule::new().require_exports(&["control.render"])`，语义唯一。对它报错，等于宣布“文件里存在一条注释”是畸形——那会拒绝**任何**带注释的规则文件，包括本仓出厂的三份（它们都有 `//!` 文档注释）。
2. **报错会制造更坏的失败模式。** 若把①判为 Err，作者为了加一句注释就必须改写规则；而错误信息会说“认不出这条规则”，把读者引向不存在的语法问题。真正的问题（宽容读法把注释当真）反而被掩盖。
3. **两入口的分工因此才成立。** 严格读法的价值恰在于“读得出的读对、读不出的拒绝”：①属于“读得出”，②属于“读不出”。把①也拒绝，严格与宽容的差别就退化成“一个拒绝得多、一个拒绝得少”，而不是“一个读对、一个读错”。
4. **红侧已按断言固定。** 探针与作者钉子都把宽容读法的 `exports:wrong` 钉在严格读法的 `exports:control.render` 旁边；若反过来让严格入口报错，探针会红——也就是说，这不是“我这么觉得”，而是可被下次变异推翻的断言。

（若有人主张①应 `Err`，其唯一站得住的理由是“不接受任何可能被误读的写法”——但那与形状③的口径直接冲突：缺文件都能 `Ok("ANY")`，说明本设计选择的是“按 AST 判定语义”，而不是“按文本保守拒绝”。）

### 3.2 形状②为什么必须 `Err`（而不是尽力而为）

`EXPORTS` 的取值不在规则文件里；读取器做不到“读得对”，只能二选一：**静默当没看见**（`ANY`）或**拒绝**。`ANY` 是“本注册机对进入的注册面无结构要求”——一个有意义、会被下游当真的结论。工具链把“我读不懂你的要求”冒充成“你没有要求”，正是这一族缺陷的形态。拒绝是对的。

### 3.3 形状③为什么“缺席”不是畸形

没有规则文件的面本来就没有结构要求，`ANY` 是它的**正确**读数，而不是“降级”。把它也判 Err 会让 `t72` 的生产路径拒绝所有不带规则文件的面——那才是过火。探针实测：不写规则文件 → `Ok`、规则 `ANY`。

## 4. 变异反证（副本内，自带 `CARGO_TARGET_DIR`）

三处都改回旧行为，各自单独跑探针：

| 变异 | 改法 | 探针结果 |
| --- | --- | --- |
| M1 “只读初始器” → 读整份文本 | `try_rule_syntax_from_text` 直接 `return Ok(rule_syntax_from_text(text))`（AST 路径成为不可达） | `FAILED. 4 passed; 2 failed` —— 形状①（严格读法读到 `exports:wrong`）、形状②（不再拒绝）双红 |
| M2 “遍历识别子句” → 静默降级 | `require_exports` 分支 `string_list_argument(…)?` 改成 `.unwrap_or_default()` | `FAILED. 5 passed; 1 failed` —— 形状②红（严格读法不再拒绝，生产入口跟着不报错） |
| M3 生产路径改回宽容入口 | `rule_syntax_for_source` 改回 `Ok(rule_syntax_from_text(&text))` | `FAILED. 4 passed; 2 failed` —— 形状①（生产读到注释里的 `wrong`）、形状②（生产不再报错）双红 |

**零残留**：三组变异后 `sha256sum -c` 对 `core/…/parse/rules.rs` 与 `run_method/…/parse/parse.rs` 均 `OK`，副本哈希与检出哈希一致；还原后探针 6/6 绿。

## 5. 反过火：出厂物、往返、示例

1. **出厂 3 份 `registry_rule.rs` 两入口逐字节一致**（探针实测原文）：
   - `studio/…/node-editor/src/control/registry_rule/registry_rule.rs`：两入口都 `"exports:control.render;handle:ControlHandle"`
   - `studio/…/node-editor/src/control/object/node_editor/registry_rule/registry_rule.rs`：两入口都 `"exports:node_editor.render;handle:NodeEditorHandle"`
   - `examples/control-button/src/control/registry_rule/registry_rule.rs`：两入口都 `"exports:control.render;handle:ControlHandle"`
2. **经生产入口**：示例的 Control 面与 studio fixture 的 Control 面**各自单独成根**时都成功，且规则取值与宽容入口逐字节相同。
   - **限制（如实记录）**：第三份（`node_editor`）的**面**过不了生产入口，原因是它把父级写成模块路径 `parent: crate::control::NODE_ID`，遍历在读任何规则之前就带 `generated face has an invalid parent path` 停下。这是**与规则读取器无关**的既有形状（示例与 studio fixture 的嵌套面同样如此，整根遍历对两棵出厂树都在此停下）；探针对此断言的是“拒绝是关于父级的、不是关于规则的”，并把它记为观察（§6）。因此“3 份经生产入口成功”这一条**只对其中 2 份成立**，第 3 份只能到库入口级证实。
3. **渲染器 → 严格读法往返**：对 `ANY`、`exports:control.render`、`exports:control.render;handle:ControlHandle`、`preset:P;parts:T;exports:a.render;handle:H;part_traits:PT` 四个取值，`render_registration_rule` 的输出放进 `const REGISTRATION_RULE` 后，严格读法与宽容读法**都**等于 `compact_registration_rule(parse_registration_rule_owned(value))`。工具链里唯一的写入者写出的规则，绝不可能被严格入口拒绝。
4. **`examples/control-button --test registry`**：独立复跑 `running 25 tests` → `test result: ok. 25 passed; 0 failed` ✓（与作者的 25 passed 一致）。

## 6. 观察（不是 K-10 缺陷，也不改）

1. **`registry_rule_path` 在规则文件不存在时仍被填成“本应在的位置”**（实测 `"src/control/registry_rule/registry_rule.rs"`），而它的字段文档写的是“提供该注册规范的声明所在源码路径”。规则值与生产行为都不受影响；但它会让读该字段的调用方以为存在这样一份文件。低严重度，建议下一轮在字段文档或取值上择一对齐。
2. **宽容入口在“整段文本没有 `=`”时静默给 `ANY`。** 我最初的探针把表达式本身（而非文件）交给宽容入口，实测 `rule_syntax_from_text("crate::RegistrationRule::new().require_exports(&[\"control.render\"])")` 得到 `"ANY"`。这与作者文档所写的“第一个 `=` 不是初始器时就静默降级”同源，是**已发布 API 的既知损失性**，不是新洞；记在这里因为它正好是同一形状的第二个可复现点。
3. **整根遍历对两棵出厂树都在“模块路径父级”上停下**（`examples/control-button/src`、`studio/tests/fixtures/node-editor/src` 都是 `invalid parent path`）。与规则读取器无关，但意味着 `generated_snapshots_from` 不能直接用于这两棵树；作者的钉子 `every_shipped_rule_source_still_reads_through_the_production_entry` 与我的探针都因此改为逐面测量。
4. **单 crate 验证需要点名特性**（本仓准则）：`cargo test -p xirang-run-method --offline` 在没有 `authoring` 时报 `0 passed` —— 规则/生产路径的钉子一条都不会跑。本文件的门禁因此都带了 `--features authoring`（或走工作区/`--all-features`）。

## 7. 门禁与零改动

2026-09-28 23:28:29，五条全部 `exit 0`：

| 命令 | 结果 |
| --- | --- |
| `cargo test -p xirang-core --offline` | `ok`，lib 目标 `187 passed; 0 failed`（`--features syntax` 时 `246 passed`；规则钉子 6 条在其中） |
| `cargo test -p xirang-run-method --offline --features authoring` | `ok`，`parse_tests` 10 条全过（含 `a_malformed_rule_source_is_refused_with_its_context`、`every_shipped_rule_source_still_reads_through_the_production_entry`） |
| `cargo test --workspace --offline` | `ok`，无 `FAILED`/`error` 行 |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | `ok`，无告警 |
| `cargo fmt --all -- --check` | `ok`，无 Diff |

**零改动**：以下六份被验证文件在本轮前后 `sha256sum -c` 全部 `OK`，我没有改动任何源码或他人产物：

```
core/src/registry_core/authoring/parse/rules.rs
run_method/authoring/parse/parse.rs
run_method/authoring/manifest/parse/parse.rs
studio/tests/fixtures/node-editor/src/control/registry_rule/registry_rule.rs
studio/tests/fixtures/node-editor/src/control/object/node_editor/registry_rule/registry_rule.rs
examples/control-button/src/control/registry_rule/registry_rule.rs
```

**ENV-1**：本轮五条门禁均未出现环境相关失败，因此没有需要排除的 `ENV-1` 项；若他人树上的 `ENV-1` 指的是某条门禁的环境敏感形态，本轮的实测结果是五条同在 23:28:29 全绿。

## 8. 结论与边界

**结论：`K-10` 家族证实。** 严格入口只按 AST 读 `REGISTRATION_RULE` 初始器，三种形状都按“读得出就读对、读不出就拒绝、缺席不是畸形”工作；生产调用点确实切到了它，拒绝时点名两个文件；三条形状的红侧我都用自己的探针复现，三处变异都让探针变红，出厂物与渲染器往返都没有被误伤。作者更正的验收口径（形状①应读出、不该报错）**成立**。

**边界（未验证/不可断言的部分）：**

- 我只驱动了 `generated_snapshots_from` 这一条生产路径；`cli/`、`studio/` 是否还有别的读者直接调用宽容入口，本轮没有逐调用点普查。`grep -rn`（排除 `target/`）显示：**严格入口的生产调用点只有 `run_method/authoring/parse/parse.rs:98`**；其余命中全部是测试与文档——`core/tests/registration_rule_entry.rs`（集成测试，5 处严格 / 14 处宽容）、`core/…/parse/rules_tests.rs`、`run_method/…/parse/parse_tests.rs`，以及 `core/…/parse/parse.rs` 与 `run_method/…/parse/parse.rs` 的文档引用。
- “3 份出厂 `registry_rule.rs` 经生产入口成功”只对 2 份成立，第 3 份卡在与本研究无关的父级写法上（§5.2、§6.3）。
- §6.1 的 `registry_rule_path` 取值与字段文档的口径差我没有判定“应以哪边为准”，只如实记录。
