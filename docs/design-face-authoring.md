# 注册面作者契约

**状态:已实施,契约。** 本文件曾是一份讨论稿;现在它记录注册面**作者侧**的最终形状:作者要
写什么、机器推导什么、Studio 表单展示什么。每条结论都能在仓库里指出实现位置。与早期讨论稿冲突
之处以本文件为准,为什么改记在 §7。

> `conventions/src/doc_blocks.rs` 的 `RECORD_PREFIXES` 把 `design` 前缀当"记录",因此本文件里的
> Rust 块不在围栏解析门禁内。但本文件维护的是契约:下面的片段取自仓库真实文件,改动请同步。

---

## 1. 一句话

注册面宏接受 24 个键,而**作者必须写的是一个**:`kind`。

```rust
crate::root_object! { kind: Control, needs_registry: true }
```

`handle`、`params`、`registry_name`、`source`、`registry_rule_path` 全部由 `kind` 与文件自身
位置推导;`name` 默认是 kind 的拼写;`summary` 默认为空。可达的最小形态是
`run_method/tests/kind_only_registry_name.rs` 里钉住的那一条(前端宏会补 `collector`):

```rust
xirang_toolchain::run_method::__control_object! { collector: development, kind: KindOnlyFace }
```

它同时钉住 `registry_name` 的来源:该面记录的 `registry_name` 是 `"kind_only"`——声明所在
**模块**的末段,而不是 kind(见 §6 第 2 条)。`preset:`/`parts:` 的各种省略形态由
`run_method/tests/face_arm_defaults.rs` 钉住。

## 2. 推导规则:机器知道什么

| 字段 | 省略时的值 | 实现位置 |
| --- | --- | --- |
| `source` | `manifest_relative_source(CARGO_MANIFEST_DIR, file!())` | `face_objects.rs`、`face_external.rs` |
| `handle` | `$kind`(同一个类型) | 同上 |
| `params` | `stringify!($kind)` | 同上 |
| `registry_name` | `last_path_segment(module_path!())` | `core/src/registry_core/identity/path_text.rs` |
| `registry_rule_path` | 与 `source` 同一条路径 | `face_objects.rs` |
| `preset` / `parts` | `NoPreset` / `NoParts`,名字由类型取 | `face_helpers.rs::__face_ty_name_or!` |
| `name.zh` / `name.en` | `stringify!($kind)` | `face_objects.rs` |
| `summary.zh` / `summary.en` | `""` | 同上 |
| `exports` / `requires` / `provides` / `runtime_checks` | `[]` | 同上 |
| `stable_name` | 空:身份按源码位置算 | `face_helpers.rs::__stable_name!` |
| `needs_registry` | `false` | 同上 |
| `parent` | `root_node_id(env!("CARGO_PKG_NAME"))` | `face_objects.rs` |
| `getting_from_other_registry` | `None` | 同上 |
| `admission` | 无,全放行 | `face_helpers.rs::__admission!` |
| `flow` / `flow_provider` / `plugin` | 无 | `face_helpers.rs::__flow_select!` |
| `handle_traits` / `part_traits` | 由 `handle_contracts` / `part_contracts` 推导 | §3 |
| `expected_output` / `actual_output` | 字段已删除,类型由断言证明 | §4 |

`__control_object!` 与 `__external_object!` 各只有**一条 arm**:一条 arm 就是一个"我必须写什么"
的答案。此前"严格 arm / 带 handle 的 arm"的差别表达不出这条 arm 表达不了的东西。

## 3. 契约路径是作者输入,标签是它的读数

**作者写路径,机器推标签。** 路径是承重的:

- `handle_contracts` / `part_contracts` 里的每个 `path` 会进
  `run_method/macros/face_helpers.rs::__assert_impls!`,展开为
  `fn assert_impl<T: Trait>()` 的编译期断言——路径写错,编译失败。
- `handle_traits` / `part_traits` 是**搜索用标签**,由路径按同一条规则取末段。这条规则有三个
  读取方,共用一份实现:
  1. 文件视图:`core/src/registry_core/syntax/fields.rs::FaceSyntax::string_list`
     (委托 `authoring::parse::trait_names_from_paths`);
  2. 过程宏前端:`xirang_toolchain::run_method::__face_trait_labels_or!`(`macro/src/lib.rs`);
  3. 写入方:`run_method/authoring/operations/face_write.rs::apply_trait_contract`。

因此编译进的注册信息、构建期契约检查、创作解析器与 Studio 不可能对标签各说一套。**只写标签、
不写路径**仍然合法,但那是一个未经编译器校验的声明;三个外置替换面
(`examples/control-button-graft/src/*_fast.rs`)就是这种形状,有意保持不动。

## 4. 输出对已删除,改成跨面类型断言

作者过去写的 `expected_output` / `actual_output` 只在两处与**自己**比较
(`core/src/registry_core/release/release.rs`、`build_method/contracts.rs::check_output`),
从未描述真实类型:button 面写着 `"ControlFrame"`,而它的 `NoPreset`/`NoParts` 输出是 `()`。
今天这件事由类型证明,分两层:

- 每个面:`__registration_face!` 发射 `assert_contract::<$preset, $parts>()`;编译进
  `RegistrationInfo` 的 `contract` 只有 `required_parts`(来自 preset 的 trait)与
  `provided_parts`(来自 parts 的 trait)两个由类型读出的值。
- 每个**有类型的嫁接切口**:`build_method/renderer/pass.rs` 在 `BUILTIN_GRAFT_CUTS` 旁发出
  `assert_contract::<{cut}::__Preset, {graft}::__Parts>()`,因此替换面的 parts 不提供被替换
  preset 要求的 parts 时,构建失败。

边界(有意,不是退路):字符串切口与选择器切口不发断言——它们不命名类型。这一点写在填充断言的
地方。

## 5. Studio 表单的实测布局

槽位由 `core/src/registry_core/authoring/face_field.rs` 具名,`FACE_FIELD_COUNT = 26`。
26 = **22 行作者输入 + 4 行只读机器取值**,后者收尾成一整条并暗色打印(列表边框带图例)。

| # | 槽位 | 宏键 | 表单角色 |
| --- | --- | --- | --- |
| 0 | `PARENT` | `parent` | 作者输入(树选中,也可写路径/节点身份) |
| 1 | `MODULE` | 文件/目录名 | 作者输入(必填) |
| 2 | `NEEDS_REGISTRY` | `needs_registry` | 作者输入,默认 false |
| 3 | `TREE_SLOT` | `registry_name`(推导) | **机器取值** |
| 4 | `REGISTRY_RULE` | `registry_rule` | 作者输入,默认 ANY |
| 5 | `ADMISSION` | `admission` | 作者输入,默认全放行 |
| 6 | `PARTS` | `parts` | 作者输入,默认 NoParts |
| 7 | `EXPORTS` | `exports` | 作者输入 |
| 8 | `KIND` | `kind` | 作者输入,默认 PascalCase(module) |
| 9/10 | `NAME_ZH` / `NAME_EN` | `name` | 作者输入,默认 kind |
| 11/12 | `SUMMARY_ZH` / `SUMMARY_EN` | `summary` | 作者输入 |
| 13 | `PRESET` | `preset` | 作者输入,默认 NoPreset |
| 14 | `STABLE_NAME` | `stable_name` | 作者输入 |
| 15 | `GETTING_FROM_OTHER_REGISTRY` | 同名字段 | 作者输入 |
| 16 | `REGISTRY_RULE_PATH` | `registry_rule_path`(推导) | **机器取值** |
| 17 | `HANDLE_TRAITS` | `handle_traits`(推导) | **机器取值** |
| 18 | `HANDLE_CONTRACTS` | `handle_contracts` | 作者输入(§3) |
| 19 | `PART_TRAITS` | `part_traits`(推导) | **机器取值** |
| 20/21 | `REQUIRES` / `PROVIDES` | 同名 | 作者输入 |
| 22 | `RUNTIME_CHECKS` | `runtime_checks` | 作者输入 |
| 23/24 | `FLOW` / `FLOW_PROVIDER` | `flow` / `flow_provider` | 作者输入 |
| 25 | `PART_CONTRACTS` | `part_contracts` | 作者输入(§3) |

计划中的"12 行"是**把相关字段并成一行**的估算(名字两行并一行、摘要并一行、preset/parts 并
一行……)。合并意味着键盘表单要多一层"行内选格",因此按"作者面的行 = 作者真能决定的字段"做完,
实测是 22+4。裸下标已全部清零:注册面在 `authoring::face_field`,两个向导表单在
`app::state::new_project_field` / `app::state::plugin_field`,每行一个常量并挨着表单打印的
标签,因此渲染器原先各自携带的标签表也一并消失。

## 6. 六项决定与落地位置

| 决定 | 落地 |
| --- | --- |
| `handle` 可省 | `__control_object!` / `__external_object!` 各一条 arm,`handle: $kind` |
| `registry_name` 必须等于声明模块 | 宏推导 `last_path_segment(module_path!())`;编辑器 tree slot 行变只读。`kind_only_registry_name.rs` 钉住"是模块名而不是 kind" |
| `params` 永远等于 kind | 宏推导 `stringify!($kind)`;作者面与表单槽位一并删除 |
| trait 合成一处 | §3:路径承重、标签由路径推导,三个读取方共用一条规则 |
| 只读输出字段说不清,删掉 | §4:字段、两处比较、缓存键、创作管线、Studio 槽位全删;改为类型断言 |
| name/summary 建议有但不强制 | 省略时取 kind 的拼写 / 空串;表单仍留两行 |

## 7. 记录在案的更正

1. **第 3 项的方向被反转。** 计划写的是"保留标签、删除 `handle_contracts`/`part_contracts`
   路径(含 Studio 从文件解析路径回填表单的那趟往返)"。实施时发现路径正是
   `__assert_impls!` 的输入——那是真正的编译期检查;删掉路径等于删掉检查,而标签什么也不校验。
   因此保留了路径、把标签改为推导,并保留 Studio 回填路径的那趟往返(它读的也是路径)。
2. **讨论稿说 `expected_output`/`actual_output` 要"留"** (§5 原文写作"它们是一对校验")。实际
   是两处把这一对与彼此比较,从未与真实类型比较;见 §4。讨论稿的结论已作废。
3. **讨论稿说 `registry_rule_path` 那行"删掉"**。今天它是只读机器取值:它默认与 `source` 同
   路径,但作者可以显式指向别处,所以它显示的是"这台面的规则实际在哪",保留但不可编辑。
4. **`params`/`handle` 的说明** 原标 `Derived` 却可编辑;`tree slot` 的 help 说"必要时可覆盖"
   而规则不允许;`external source note` 说"仅来源元数据"而它参与注册机解析。三条都已改正,后者
   的标签改为 `dependency registry`。

## 8. 显式记录:有意保留与仍可收口的部分

- **`RegistrationInfo.params` / `handle` 字段保留**(公开 API:宿主可读,`owned.rs` 的重载
  路径复制它们,`declaration.rs` 的合同失败消息点名 handle)。它们按规则等于 `kind`,因此它们的
  **重复展示与重复搜索键**已清掉:call report 曾把 `handle=` 与 `kind=` 并排打印,搜索曾把
  `params` 当作同一字符串的第二个键。删字段本身是 breaking change,留给版本策略决定。
- **示例声明仍逐条写出默认值**(`getting_from_other_registry: None`、`requires: []`、
  `registry_rule_path: "…"` 等)。宏允许省略(§1 的测试钉住),删这些行是风格选择;其中
  `examples/control-button-graft/src/control_fast.rs` 是用户的手写文件,只做过本轮要求的键删除,
  因此不为了统一而重排它。
- **`docs/` 里的历史文档**(`audit-*.md`、`migration*.md`)仍提到 `registry_name: slot`、
  `expected_output` 之类的旧字段。它们是记录,不改写。

## 9. 明确不删的

| 字段 | 为什么留 |
| --- | --- |
| `kind` | 身份;注册面因它而存在 |
| `parent` | 挂载点;顶层与外部面必须能改 |
| `registry_rule` / `admission` | 拥有注册机时的两条门禁,`release` 与 `overlay` 都读 |
| `flow` / `flow_provider` | 嫁接兼容性判据(`FlowContract::semantically_compatible_with`) |
| `stable_name` | 源码移动后身份不变的唯一手段 |
| `requires` / `provides` | 能力解析;`missing_capabilities` 与 release 检查都读 |
| `handle_contracts` / `part_contracts` | §3:唯一的编译器校验入口 |
| `handle_traits` / `part_traits` | 搜索与 release 的子规则校验读标签 |
| `getting_from_other_registry` | 跨注册机取依赖的解析依据(不是来源说明) |
| `plugin` | 插件策略与信任判定 |
| `runtime_checks` | 宿主实际执行的校验 |
| `name` / `summary` | 对行为无影响,但"给人看"正是它们的职责 |
