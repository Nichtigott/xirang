# Graft 记录与运行期覆盖

[English](graft.md) | 简体中文

本页记录第三层 graft：`.xirang/external-grafts/` 记录，以及运行期如何把它变成
有效树，连同由此产生的优先级规则与报告。README 描述的是声明与应用，本文件是记录的
参考。

## 三个层次

一次 graft 会同时出现三种不同的产物，而且都不复制源码：

| 层次 | 产物 | 归属 | 作用 |
| --- | --- | --- | --- |
| **声明** | 宿主入口的 `static_graft_plan!` | 构建步骤 | 命名槽位，让剪枝保住它并填充发布态 `StaticPlan`。不删除任何代码。 |
| **记录** | `.xirang/external-grafts/<selector>/graft.plan` | Studio（创作）与运行期宿主 | 运行期可以叠加在声明之上的创作输入。不参与编译。 |
| **应用** | `Registry::overlay` / `overlay_static` / `overlay_recorded` | 运行期宿主 | 校验并返回一棵新的有效树，不改动原树与外部树。 |

声明宏由 `xirang-toolchain` 导出（`xirang_toolchain::run_method::static_graft_plan!`）；
kernel 只解析宏 stringify 出来的文本。应用方法位于 kernel（`xirang_kernel::Registry`）。

## 哪个文件是宿主入口

一次 graft 的两半由同一个入口决定：剪枝保住它命名的槽位，生成的
`BUILTIN_GRAFT_CUTS` 表（以及 `graft_plan.tsv` 审计文本）描述同一批槽位。构建只解析
一次，顺序如下：

1. 设置了 `XIRANG_ENTRY` 时以它为准——相对路径相对包根解析，绝对路径原样使用。
   指不到文件时**构建失败**（`XIRANG_ENTRY names <path>, which is not a file`）。
   回退才是本条规则要消除的 bug：剪枝会跟随变量，而切口表仍在描述 `main.rs`，于是
   运行期可能拿着发布态并未保留的表，或静默丢掉只在被指定文件里声明的槽位。
2. 声明 `application!(entry = …)` 的文件；
3. 调用 `host!()` 的文件——库+二进制宿主会把空壳留在 `main.rs`，因此真实调用胜过
   Cargo 的顺序；
4. Cargo 约定的 `main.rs`、然后 `lib.rs`。这一步可能指不到任何文件，那只是没有入口的
   宿主，不是错误。

创作界面（`xirang grafts`、Studio）用同一套顺序解析入口，但把问题作为消息上报，
而不是让构建失败。

## 记录如何到达覆盖层

记录的加载与应用由 `xirang-toolchain` 中不受门控的运行期 API 完成（它刻意**不**放在
`authoring` 特性之后，宿主读取计划文件不需要 `syn` 依赖）：

```rust
use xirang_toolchain::run_method::{apply_recorded_grafts, Registry, StaticGraftCut};

// `declared` 是构建捕获的静态计划，也是哪些槽位存活的仲裁者；
// `external` 是已链接的外部注册机。
let overlay = apply_recorded_grafts(
    base,
    external,
    builtin_static_plan().grafts(),
    package_root,
)?;
let effective: Registry = overlay.effective;
```

- `graft_record_root(package_root)` 就是 `.xirang/external-grafts/`；
- `load_graft_records(package_root)` 返回每个计划目录，无论可否读取（`LoadedGraft`），
  按选择器排序；目录不存在时返回空列表而不是错误；
- `load_graft_record(package_root, selector)` 按选择器读回一条计划，拼接前先校验选择器，
  因此记录无法逃出该目录；
- `apply_recorded_grafts(...)` 读取记录并调用 kernel 的 `Registry::overlay_recorded`，
  返回 `GraftOverlay { effective, reports }`；不可读的计划或内核拒绝的记录是 `Err`，
  并一次报出所有有问题的计划；
- `Registry::overlay_recorded(records, declared, external)` 是纯 kernel 方法：对账逻辑在
  kernel 内，文件系统访问不在。

没有记录时，它是 `overlay_static` 的等价替代：未被记录触及的声明按原来的类型化切口
原样传入，报告列表为空（由 `no_records_reproduces_overlay_static` 钉住）。

## 记录到覆盖层的优先级

记录是最新、最具体的产物，而构建从不应用它。但这不意味着它无条件最终。
策略按声明的形式区分：

| 声明形式 | 记录的效果 | 报告 |
| --- | --- | --- |
| 字符串形式 `cut "root/control/button" graft "button_fast"`（`CutTarget::Path`） | 记录获胜：它的 `graft` **与** `full` 取代声明 | `DeclarationOverridden`，`full` 不同时再加 `GranularityOverridden` |
| 类型化形式 `cut(crate::…::NODE_ID) graft(…)`（`CutTarget::Id`） | 声明保持最终，记录被忽略 | `TypedDeclarationKept` |
| 记录的 `graft` 选择器在外部注册机里无法唯一解析 | 保留声明 | `RecordSelectorUnresolved` |
| 记录指向没有任何声明保活的槽位 | 跳过记录 | `UnkeptSlot` |

为什么这样区分：“总让记录赢”会让未经审查、机器本地的 `.xirang/` 文件重新路由已发布
行为，并击败已链接、编译器解析过的注册面；“总让声明赢”则让记录的 `graft` 完全没有生产
读者。按声明形式区分，既保留了字符串形式本就授予的“按名字动态解析”的信任，又拒绝击败
类型化形式。

另外两条边界：

- `graft` 与 `full` 一起移动。绝不会出现“记录的实现配声明的 `full`”；整条记录是原子的
  （`granularity_is_overridden_as_one_atomic_record`）。
- 只被记录部分覆盖的区间声明按节点拆分：被覆盖的节点取记录，其余节点保留声明。计划格式
  没有区间字段，因此记录永远无法创建区间。

## 身份漂移

记录同时存有 `NodeId`（`target`）和逻辑路径（`target_path`）。运行期施加在槽位的**当前**
路径上——该路径从存储的身份重新推导，而不是用存储的文本。存储的路径是创建计划时写下的，
而切口选择器必须精确匹配，直接使用它会让记录在注册面改名后静默失效。

| 存储的身份可解析 | 存储的路径可解析 | 结果 |
| --- | --- | --- |
| 是 | 指向同一个面 | 干净；施加在当前路径上 |
| 是 | 否（或指向不同的面） | 按身份施加，报告 `IdentityDrifted` |
| 否 | 是 | 按路径施加，报告 `IdentityDrifted` |
| 是 | 是，但指向**不同的**面 | **致命**：记录自相矛盾 |
| 否 | 否 | `UnkeptSlot`；记录被跳过 |

唯一的致命记录条件是身份与路径指向不同的面：任选其一都会静默改变作者的本意。

## 报告，以及哪些失败是致命的

`RecordReport` 有六个提示性变体。`apply_recorded_grafts` 会在返回前把每一条都打印出来：
没有生效的嫁接为 `warning:`，优先级裁决为 `note:`。同样的条目仍留在 `GraftOverlay` 中，
供把证据转往自己日志的宿主使用。因此忽略返回值的宿主依然能看到被跳过的嫁接——否则这种
失败在构造上就是不可见的：源码树看起来正常，运行中的二进制只是从未应用它。

| 变体 | 含义 |
| --- | --- |
| `DeclarationOverridden` | 字符串形式声明被记录取代。 |
| `TypedDeclarationKept` | 类型化形式声明保持最终；记录被忽略。 |
| `RecordSelectorUnresolved` | 记录的 `graft` 在外部注册机里解析不出；保留声明。 |
| `GranularityOverridden` | 记录相对声明改变了 `full`。 |
| `UnkeptSlot` | 没有声明保活该槽位，或槽位已消失；记录被跳过。 |
| `IdentityDrifted` | 身份与存储路径不一致；记录施加在现存注册面上。 |

以上全部是**报告而不是致命错误**：覆盖照常进行，其余记录照常应用。`UnkeptSlot` 与
`RecordSelectorUnresolved` 留在这里而不是升级为错误，是因为运行期无法区分"合法配置"与
"失误"——声明可能被 `#[cfg]` 在本次构建里关掉，记录也可能写在其声明那次构建之前。这一类
里无歧义的那一半改由构建期拒绝（见下）。

三件事**是**致命的，因为它们都没有合法解读：

- 解析不了的计划文件——应用失败并报出每一个不可读计划，因此不会应用任何记录
  （`an_unparseable_plan_fails_the_apply`）；
- 目录选择器与计划里的 `graft` 不一致的记录——两个候选实现且无从判断作者指的是哪个
  （`record_tests::a_directory_that_disagrees_with_its_plan_is_refused`）；
- 身份与路径指向不同面的记录
  （`record_tests::contradictory_identity_and_path_are_refused`）。

底层覆盖的硬失败与普通 `overlay_static` 逐字节相同：选择器歧义或未知、数据流合同不匹配、
目标注册规范、准入与连接器拒绝、重复切口。

## 构建期拒绝

构建从不应用计划，因此它不能等运行期去发现被剪掉的槽位。它改为拒绝该构建：当存在一条
计划，而**没有**任何声明能命名它的目标槽位时，`xirang-toolchain` 报出一条构建错误
（`phase=static-plan`，指向计划文件），消息里带着可直接粘贴的子句：

```text
external graft plan `<selector>` targets `<path>`, which no declaration in the host entry names; the release-time plan keeps no such slot alive, so the record could never take effect. Declare it in static_graft_plan!: cut "<path>" graft "<implementation>",
```

带特性门控的声明即使在本次构建里求值为假也算数，因为门控是作者的事：当前特性组合把槽位
编译掉的记录是配置，不是失误。正因如此，这项检查读入口的完整声明清单，而生成的切口表只读
启用的那一部分。构建不应用该计划；`xirang grafts` 让同一判断可按需检视（见下）。

## 记录目录是运行期输入

`.xirang/external-grafts/` 是**运行期输入**而不是生成物。因为记录可以重新路由字符串
形式声明，不审查该目录的包可能被机器本地文件改变已发布行为。请像审查源码一样审查它，
并把它挡在不可信检出之外。也正因如此，`apply_recorded_grafts` 与 `overlay_static` 对相同
输入可能产出不同的树；示例测试 `a_record_moves_the_effective_tree_but_not_the_static_plan`
把这处有意为之的偏离钉住，而不是当作缺陷。

槽位没有任何声明命名时，写入记录仍然允许——Studio 与运行期 API 都是如此：“先写计划、
再声明槽位”是合法的顺序。后果容易被忽略，因此三处都会说明：Studio 在撰写时警告（见下），
构建会拒绝"没有任何声明能命名的计划"（见下），运行期在应用记录时打印
`warning: … record skipped`。
记录仍是被跳过而不是致命错误——一条陈旧记录不该让其余记录失效——但没有任何一条路径会把
这次跳过留在沉默里。`xirang grafts` 是事后看到同一事实的只读方式。

## 从命令行读取各层

| 命令 | 报告内容 |
| --- | --- |
| `xirang grafts [path] [--json]` | `.xirang/external-grafts/*/graft.plan` 下的每条计划：选择器、目标路径、graft、`full`，以及宿主入口是否声明该槽位。只读。 |
| `xirang explain <node-id\|logical/path> [--path <dir>] [--json]` | 单个节点的身份、构建作用域、剪枝状态，以及命名它的声明切口。 |
| `xirang explain --overlay [--path <dir>] [--json]` | 每个槽位的保留/剪枝状态与替换，外加计划行。 |

`explain --overlay` 明确是一次**静态投影**（`"kind": "static-projection"`）：CLI 无法链接
任意宿主的 `base_registry()` 与 `external_registry()`，因此它报告构建自己的作用域与声明
切口——覆盖正是由它们算出的——而不会从解析出的源码伪造一棵 `Registry`。真正的有效树是
宿主侧的 `Registry::dump_effective(cuts, external)`（它调用 `overlay_static` 再 `dump`）；
持有两棵真实注册树的宿主用 `builtin_static_plan().grafts()` 调用它。

## 兄弟区间

切口可以命名两个端点而不是一个：字符串写法是
`cut "root/a1 to root/a3" graft "replacement"`，类型化写法是 `cut(a to b)`。覆盖层
随后替换跨度内的每个节点。

- 跨度是两个端点按 **`registry_name` 排序后**位于两者之间、连续的那一段兄弟——与
  兄弟列表和大纲使用的是同一种排序。它不是文件顺序、注册顺序或树渲染顺序：一个列成
  `a3, a1, a2` 的目录，`a1 to a3` 仍然恰好覆盖这三个面。
- 两个端点必须同属一个父级。跨父级的区间会被拒绝，不会隐式放宽到最近的共同祖先。
- 从靠后的兄弟写到靠前的兄弟会被拒绝，错误会报出两个端点与该规则，而不是静默交换。
  交换会选中同一个集合，却掩盖了真正决定的顺序是 `registry_name`，于是按另一种心智
  模型写下的下一个区间会悄悄覆盖错误的面。
- 构建把两个端点都作为 graft 槽位强制保活，因此区间的目标能穿过剪枝并到达运行期覆盖层。

## 类型化切口与字符串切口

两种形式可以在同一声明里混用，但一条切口的两侧必须用同一种方式命名（都是 Rust 表达式
或都是字符串）：

```rust
// 字符串形式：用逻辑路径命名槽位，用选择器名命名实现。
// 在覆盖时动态解析；不需要链接。
xirang_toolchain::run_method::static_graft_plan!(FRAMEWORK,
    cut "root/control/button" graft "button_fast",
);

// 类型化形式：编译器解析目标面的 `NODE_ID` 与外部实现的 `NODE_ID`；
// 外部 crate 必须已链接，因此类型化声明对记录保持最终。区间写成 `cut(a to b)`，
// 见上文“兄弟区间”一节。
xirang_toolchain::run_method::static_graft_plan!(FRAMEWORK,
    cut(crate::control::object::button::NODE_ID)
        graft(control_button_graft::button_fast::NODE_ID),
);
```

实现是已链接的 crate、希望编译器和编辑器解析目标、或希望声明不被本地记录覆盖时，用
类型化形式。实现按名字绑定（运行期插件）或没有链接、或本就希望记录能重新路由该切口时，
用字符串形式。真实例子见 `examples/control-button/src/lib.rs`：类型化声明旁边就是它所
取代的字符串形式。
