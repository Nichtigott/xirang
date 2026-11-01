# xirang-multilang

Language adapters for XiRang registration authoring: the protocol every language implementation
implements, the SDK each language ships, and the generated-source plans a surface applies through
its own transaction. It also owns the multilingual production pipeline (project resolution,
per-language planning and execution, package sources, remote workers, delivery) that the toolchain's
execution surfaces call.

XiRang 注册面创作的语言适配器：每个语言实现都要实现的协议、各语言随包发布的 SDK，以及执行面通过
自己的事务落盘的生成源码计划。多语言生产管线（项目解析、按语言的计划与执行、包源、远程 worker、
交付）也住在这里，由工具链的执行面调用。

The crate depends on `xirang-kernel` only. It never depends on `xirang-toolchain`: the toolchain
composes this crate's adapters with its own execution context (package roots, registry loading,
transactions), which is what keeps the dependency arrow one-way.

本 crate 只依赖 `xirang-kernel`，绝不依赖 `xirang-toolchain`：工具链把本 crate 的适配器与自己的
执行上下文（包根、注册树加载、事务）组合起来，依赖箭头因此只有一个方向。

Features:

- `semantics` — the language-neutral semantic facts plus the Rust `syn` facts. `lang-cpp` and
  `lang-python` imply it because their AST readers return the same facts; the MCP query layer maps
  them onto its own function view. No process or filesystem work lives here.
- `lang-cpp` — the C++ adapter and `sdk/cpp/xirang/soil.hpp`:
  `XIRANG_SOIL` + `::xirang::soil<T>()` generation, version-2 record reading, precise rewrite.
- `lang-python` — the Python adapter and `sdk/python/xirang/__init__.py`:
  `@soil.face(...)` generation, reading and precise rewrite.
- `production` — the multilingual production pipeline plus the cached file facts it shares with the
  MCP check path. It carries `serde_json`; a host that only authors registration faces does not
  compile it.

- `semantics` —— 语言无关语义事实与 Rust `syn` 事实。`lang-cpp`/`lang-python` 隐含它，因为它们的
  AST 读取器返回同一批事实；MCP 查询层再映射到自己的函数视图。进程与文件系统工作不在这里。
- `lang-cpp` —— C++ 适配器与 `sdk/cpp/xirang/soil.hpp`：`XIRANG_SOIL` + `::xirang::soil<T>()`
  的生成、版本 2 记录读取与精确改写。
- `lang-python` —— Python 适配器与 `sdk/python/xirang/__init__.py`：`@soil.face(...)` 的生成、
  读取与精确改写。
- `production` —— 多语言生产管线，以及与 MCP 检查路径共享的文件事实缓存。它带 `serde_json`；
  只创作注册面的宿主不会编译它。

Features control which adapter code and dependencies compile. They do not remove source files from
the published package: the crate's `include` list ships `sdk/` and `src/` regardless.

特性控制编译哪些适配器代码与依赖，但**不**从发布包里删除源码：本 crate 的 `include` 清单无论
特性如何都会带上 `sdk/` 与 `src/`。

See `docs/multilang-registration-implementation-2026-10-22.md` for what is implemented, verified,
and still open.
已实现、已验证与仍未闭合的项见 `docs/multilang-registration-implementation-2026-10-22.md`。
