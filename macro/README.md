# xirang-macro

The compile-time face-field front end for XiRang.
XiRang 的编译期注册面字段前端。

`xirang-macro` is the compile-time front end for XiRang's face-field
declarations. A `macro_rules!` matcher can only fail with "no rules expected
`...`": it cannot reorder fields, cannot compare field names, and cannot attach
a span to a message of its own. This crate receives the author's tokens with
their spans instead, so it can accept every reasonable spelling and still point
at the exact token that is wrong.

It is a build detail of `xirang-toolchain`: host code writes `root_object!` /
`<parent>_object!` / `external_object!`, and the runtime macro ladder calls this
front end only when its strict arms have already declined the declaration.

`xirang-macro` 是 XiRang 注册面字段声明的编译期前端。`macro_rules!` 匹配失败
只会说 "no rules expected `...`"：既不能重排字段、不能比较字段名，也无法把自己的
消息挂到具体 token 上。本 crate 拿到的则是带 span 的作者 token，因此既能接受各种
合理写法，又能把错误精确指到出问题的那个 token。

它是 `xirang-toolchain` 的构建细节：宿主代码写 `root_object!` /
`<parent>_object!` / `external_object!`，只有当运行期宏阶梯的严格 arm 都不接受时，
才会走到本前端。

## What the front end accepts / 前端接受什么

- Any field order. The front end reorders the fields into the kernel's
  `FACE_FIELD_ORDER` before dispatching, so `kind:` need not come first.
- `,` or `;` as the separator, including a forgotten separator between two
  fields. A separator inside `{}`, `()` or `[]` belongs to that group and is
  never treated as a field boundary.
- Every field has a declared shape. A missing `:` after a field name, or a
  repeated field name, is reported on the offending token rather than as a
  generic macro mismatch.
- An unknown field is rejected with the full list of accepted names. The
  `collector:` argument is split off first and must appear exactly once; it is
  not a `FaceFields` member, so it never reaches the mirror.

- 任意字段顺序。前端在派发前把字段重排为内核的 `FACE_FIELD_ORDER`，`kind:` 不必写在
  最前；
- 分隔符可以是 `,` 或 `;`，两个字段之间漏写分隔符也可以。`{}`、`()`、`[]` 组内的
  分隔符属于该组，绝不算字段边界；
- 每个字段都有约定的写法：字段名后漏写 `:`、或字段名重复，都会报在出问题的 token 上，
  而不是笼统的宏不匹配；
- 未知字段会被拒绝，并列出全部可接受字段名。`collector:` 参数先被单独取出，必须有且
  仅有一次；它不是 `FaceFields` 成员，因此永远不会进入镜像。

The declaration is re-emitted as
`::xirang_toolchain::run_method::__xirang_object! { … }` (or `__external_object!` for
the external target), so the collector mode the caller chose survives the round
trip. If reordering produces exactly the tokens it was given, the front end
reports that a field's *shape* is wrong instead of recursing.

归一化后的声明会重新发出为 `::xirang_toolchain::run_method::__xirang_object! { … }`
（外部目标则为 `__external_object!`），调用方选择的 collector 模式因此得以保留。
如果重排得到的正是收到的 token，前端会报告"某个字段的写法不对"，而不是无限递归。
The IDE-only field mirror that used to live here was **retired** in step ⑤ of A2e: the new
face spelling is plain Rust, so an editor completes it natively and the macro no longer needs
a `cfg(rust_analyzer)` splice. The reason it existed, and what replaced it, are in
`docs/native-completion-design-2026-10-10.md`.
这里原来的『仅供编辑器的字段镜像』已在 A2e 第⑤步**退役**：新的面拼法是普通 Rust，编辑器原生
即可补全，宏不再需要 `cfg(rust_analyzer)` 拼接。它当初为何存在、如今由什么取代，见
`docs/native-completion-design-2026-10-10.md`。
| --- | --- | --- |
| `Value` | `parent`, `flow`, `admission`, `source`, `plugin`, … | the author's expression, typed by `_` inference / 作者表达式，由 `_` 推导类型 |
| `Type` | `kind`, `preset`, `parts`, `handle`, `flow_provider` | the author's tokens move into the annotation; the value is `loop {}` / 作者的 token 进入类型注解，值为 `loop {}` |
| `Replace` | `name`, `summary`, `exports`, `requires`, `runtime_checks`, … | the field name is kept and the value replaced / 保留字段名、替换值 |

Unwritten fields take a rigid `__Any` parameter, so the literal is fully typed:
no `E0282`, no `expected (), found …`. `face_fields_mirror!` serves generated
aliases whose token tree an editor cannot split; unlike the full front end it
tolerates a field the author has not finished yet and never re-dispatches to
the runtime.

未写或不能单独作为表达式的字段取刚性参数 `__Any`，字面量因此完全定型：既没有
`E0282`，也没有 `expected (), found …`。`face_fields_mirror!` 服务于编辑器无法切分
token 树的生成别名；与完整前端不同，它容忍作者尚未写完的字段，也绝不回派到运行期。

## Entry points / 入口

- `face_fields!` — normalise a declaration for the generated host aliases.
  / 为生成的宿主别名归一化声明。
- `face_fields_mirror!` — emit only the editor mirror.
  / 只发射编辑器镜像。
- `face_rule_or!` — pick a face's `registry_rule`: the author's expression
  first, then the canonical sibling rule for a face that owns a registry, then
  the permissive fallback.
  / 选择注册面的 `registry_rule`：作者表达式最优先，其次是有注册机的面所用的同目录
  规范规则，最后是宽松默认值。

## License / 许可证

MIT. See [LICENSE](https://github.com/Nichtigott/xirang/blob/main/macro/LICENSE).
MIT，见 [LICENSE](https://github.com/Nichtigott/xirang/blob/main/macro/LICENSE)。
