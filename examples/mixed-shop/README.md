# mixed-shop

A complete, runnable mixed-language example: a registration tree whose `Area` face is declared **in
C++** (`shop/native/area/area.cpp`) and hangs under the Rust `Panel` face — and `Panel`'s subtree is
split into a crate of its own, so the crate that **owns the parent** is the crate that compiles the
C++ source. Three tests assemble the tree across the split and call the C++ function through the face.

一个完整可跑的混合语言示例：注册树里的 `Area` 面**声明在 C++ 里**（`shop/native/area/area.cpp`），
挂在 Rust 的 `Panel` 面下；而 `Panel` 子树被拆成独立的 crate——于是**拥有父面的那个 crate**就是编译
那份 C++ 源码的 crate。三条测试跨拆分把树装起来，并通过面真的调到那个 C++ 函数。

```
examples/mixed-shop
├── Cargo.toml              # the example's own workspace / 示例自己的工作区
├── shop/                   # the host: faces + add_crates.rs + the declared C++ source
│   ├── add_crates.rs       # Shape::of("shop", &[Crate::named("panel").at(&[…SUBTREE])])
│   ├── build.rs            # xirang_toolchain::build_method::run() — one call
│   ├── native/area/area.cpp # 声明自己的面：XIRANG_FACE(r_area, soil<Area>().under(::xirang::faces::shop__panel_Panel)…)
│   └── native/area/gauge.cpp # 挂在 Area 下：area.cpp 声明了 .owns_registry()，Gauge 也是 C++
│   └── src/shop/…          # Shop → Panel → Button（Area 不在这边）
├── crates/shop-panel/      # generated: the faces the host handed away (compiles area.cpp)
├── crates/shop-facade/     # generated: the cross-crate half
└── verify/                 # hand-written: assembles both crates' registrations and asserts
```

The declaration stays where the author wrote it, next to the host's sources; who *compiles* it is
derived from the tree, not written down anywhere. `area.cpp` names its parent through the generated
`xirang_faces.hpp`, so the parent is a compile-time constant and a wrong name is a compile error.
声明留在作者写它的地方（宿主源码旁边）；**谁编译它**是从树上推出来的，不写在任何地方。`area.cpp`
经生成的 `xirang_faces.hpp` 点名自己的父面，因此父是一个编译期常量，写错名字就是编译错误。

The tree it produces / 它产生的树：

```
root/shop               kind=Shop
root/shop/panel         kind=Panel
root/shop/panel/Area    kind=Area     ← declared by native/area/area.cpp, compiled by shop-panel
root/shop/panel/button  kind=Button
root/shop/panel/Area/Gauge  kind=Gauge   ← declared and owned in C++, also compiled by shop-panel
```

## Run it / 怎么跑

```sh
# from the repository root / 在仓库根目录
cargo build -p xirang-toolchain --features cli --bin xirang
cd examples/mixed-shop/shop
../../../target/debug/xirang check        # 3 Rust faces + a note naming the native one
../../../target/debug/xirang crates --write
cd ..
cargo test                                # three tests / 三条测试
cargo run -p verify --bin tree            # print the assembled tree / 打印装好的树
```

`xirang check` counts the faces the **source scan** sees (Rust only) and says what that count leaves
out — with the split in place: `note: 1 native face(s) in this project are compiled by the crate that
owns their parent, not by this one: native/area/area.cpp → crate `shop-panel` (claim `shop::panel`)`.
The same line reaches a plain `cargo build` as a `cargo:warning=`.
`xirang check` 数的是**源码扫描**看得见的面（只有 Rust），并说出这个计数漏掉了什么——带拆分时是上面
那句 note；普通 `cargo build` 也会以 `cargo:warning=` 看到同一行。

`xirang crates --write` rewrites the workspace `members` list (it drops hand-added members such as
`verify`), so run it last — or put `verify` back before committing.
`xirang crates --write` 会重写工作区的 `members`（手工加的 `verify` 会被丢掉），所以把它放在最后跑，
或者在提交前把 `verify` 加回去。

`crates/` is committed, because "declare once, split on submit" is the deal: re-running
`xirang crates --write` must reproduce it byte for byte, and `tools/xirang-mixed-shop` is the gate
that checks exactly that.

`crates/` 入库了，因为"顶层声明一次、提交时生成"就是约定：重跑 `xirang crates --write` 必须逐字节复现它，
`tools/xirang-mixed-shop` 就是把这条钉住的门禁。

## What it proves, and what it does not / 它证明了什么、没证明什么

Proves / 证明了：① **a face declared in C++ lands in the same tree** next to the Rust ones, and it
may hang under a Rust face that belongs to a **subtree this host handed away** — the crate that owns
the parent compiles the source, links the archive and registers the face, with the host compiling and
registering neither (one declaration, one compile owner); ② a wrong or missing parent name is a build
failure (`error: 'app__panel_Pannel' is not a member of 'xirang::faces'`), never a silent drop;
③ splitting by subtree keeps the two crates' faces in one tree (no source merge); ④ the call really
reaches the C++ body (`area(3, 4) == 12`, `gauge(21) == 42`, and the gate's red control changes the body
and watches the test fail); ⑤ the split needs one declaration — `add_crates.rs` — and no face file
moves; ⑥ an external face can be a **parent**: `Gauge` hangs under `Area`, ownership resolves *through*
it, both are compiled by `shop-panel`, and a code error in either one fails the build.

证明了：① **声明在 C++ 里的面落进同一棵树**，并且可以挂在**宿主已经交出去的子树**里的 Rust 面下——
拥有父的那个 crate 编译源码、链接归档并登记这个面，宿主两样都不做（一次声明、一个编译归属）；
② 父名写错或不存在是构建失败（`error: 'app__panel_Pannel' is not a member of 'xirang::faces'`），
绝不是静默丢弃；③ 按子树拆分让两个 crate 的面留在同一棵树里（不合并源码）；④ 调用真的走到 C++ 函数体
（`area(3, 4) == 12`、`gauge(21) == 42`，门禁的红控制改函数体并看测试变红）；⑤ 拆分只需要一份声明
——`add_crates.rs`——没有任何面文件搬家；⑥ **外部面可以当父面**：`Gauge` 挂在 `Area` 下，归属**穿过**
外部父面解析，两者都由 `shop-panel` 编译，任一份源码写错都会让构建失败。

Does not prove / 没证明：① a **Rust** face cannot hang under an external parent yet: the Rust-side
parent resolver does not accept the generated `crate::xirang_external::…` spelling, and the facade crate
does not render external identities, so a Rust child of a C++ face fails to build (the C++→C++ case above
does work);
② the record carries the identity subset only: `registry_rule` / `admission` / `handle_*` /
`part_*` / `runtime_checks` are still Rust-only, so a C++ face cannot declare contracts yet; ③ nothing
here promises a stable ABI for C++ types beyond the C ABI the export table generates; ④ the example
builds against this checkout by relative path, so it is not evidence about building outside it.

没证明：① **Rust** 面今天还不能挂在外部父面下——Rust 侧的父解析器还不接受生成的
`crate::xirang_external::…` 拼写，facade 也不会渲染外部身份，所以 C++ 面的 Rust 子面构建失败
（上面 C++→C++ 那条是成立的）；② 记录只携带身份子集：
`registry_rule` / `admission` / `handle_*` / `part_*` / `runtime_checks` 仍只在 Rust 面能写，C++ 面
还不能声明合同；③ 除了导出表生成的 C ABI 之外，这里不承诺任何 C++ 类型的稳定 ABI；④ 示例按相对路径
对着本检出构建，因此它不构成"在检出之外构建"的证据。
