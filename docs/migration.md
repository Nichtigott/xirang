# Migration Notes

[简体中文](migration.zh-CN.md) | English

## From the pre-five-crate layout

The registration protocol stays in `xirang-kernel`. Build-time discovery,
identity caching, scope calculation, and `StaticPlan` generation are now in
`xirang-toolchain`; MIR and data-flow evidence are in `xirang-toolchain`; the TUI
is in `xirang-toolchain`; isolated plugin execution is in `xirang-toolchain`.
The unified command-line entry (`xirang new/check/build/snippets/explain/grafts/studio/mcp`) is in `xirang-toolchain`.

Applications provide their own root registry through
`Registry::root_for_namespace`. Release builds consume the host application's
generated `StaticPlan`; this workspace does not ship a concrete `root_registry`.

## External declarations

Use `xirang_toolchain::run_method::external_object!` for declarations owned by another
crate. After B3b only `kind` is required; every other field is optional and
defaults the way the generated compact form defaults it: `source` to the
declaring file, `registry_name` to the module's last segment, `parent` to the
package root, `handle` to `kind`, `preset`/`parts` to `NoPreset`/`NoParts`,
`plugin` to `None`, and `exports`/`requires`/`provides`/`runtime_checks` to
empty. There is no version or target-framework field to fill: the framework
belongs to the registry, not to the face. `registry_rule` defaults to
`RegistrationRule::ANY` rather than the host's sibling-rule resolver, because
that resolver (`super::registry_rule::REGISTRATION_RULE`) is a relative path the
build generates only beside a face inside the host's own tree; an external
crate has no such sibling, so an external face that owns a registry gets the
permissive `ANY` and must name its rule explicitly to narrow it. Development
builds discover these declarations when the host explicitly opts into
`collector: debug`; the core crate itself remains free of inventory. Release
applications must retain external faces through a verified plugin artifact or
another explicit application-owned input; release builds do not create
inventory linker sections.

## Registration faces load in place

Build-time rendering used to copy every face into
`OUT_DIR/registration_sources` and `include!` that copy, so the module the
compiler and editor tooling resolved was a build artefact rather than the file
being edited. Faces are now loaded as real modules:

- a **leaf face** (it owns no child registries) is declared as
  `#[path = "..."] pub mod <name>;`, so its public module path and its
  `type_name` are unchanged;
- a **face that owns child registries** must also be their container, so it
  loads into a same-named child module and re-exports
  (`mod <name>; pub use <name>::*;`). Only that case gains a module-path
  segment, for example `app::control::control::Control`.

Two consequences:

- A face file stays an ordinary module file, so it may keep opening with `//!`
  or any other inner attribute. `include!` expansion cannot introduce them,
  which is why the previous design had to rewrite `//!` into `//` in its copy.
- Declaration macros now derive `source` from `file!()` and the registry name
  from `module_path!()`, so the generated tree no longer injects
  `__REGISTRATION_SOURCE` / `__REGISTRATION_MODULE_NAME`. A declaration outside
  `src/` (an integration test, for example) keeps the path cargo recorded
  instead of failing the manifest strip.

`#[path]` alone is not enough for the IDE. rust-analyzer applies the attribute
only when the `mod` declaration sits at the top level of a file or an expansion,
so a face declared inside one of the generated inline modules — every face below
the root — would never join the crate: no completion, no go-to-definition, no
hover, even though `cargo` and `rustc` see it. Each such face therefore also
gets an IDE-only view:

- the real declaration carries `cfg(not(rust_analyzer))`;
- a crate-root `#[cfg(rust_analyzer)] #[path = "..."] mod __xirang_ra_<path>;`
  loads the same file where rust-analyzer does apply `#[path]`;
- a `#[cfg(rust_analyzer)] use crate::__xirang_ra_<path> as <name>;` in the
  face's real position rebuilds its module path, with the same visibility the
  real declaration has (`pub` for a leaf face, `pub(crate)` for a container).

Exactly one of the two declarations exists for either tool, so `rustc` reads the
same tree it read before and nothing user-visible changes. `build_method` emits
`cargo::rustc-check-cfg=cfg(rust_analyzer)` so the extra cfg stays quiet.

## Face fields may be written in any order

Hand-written declarations used to fail on a separator: `kind: Tool;` produced
`no rules expected ';'`, and a field written out of order failed the same way.
Both readers are now tolerant, and they share one splitter
(`split_face_fields`), so the builder and the compiler always read the same
fields:

- `,` and `;` both separate fields, a forgotten separator ends a field at the
  next `name:`, a trailing separator is ignored, and the order is free;
- a field the vocabulary does not know, or one given twice, is reported on its
  own token with the accepted field list, by the `xirang-macro` front end;
- a well-formed declaration never reaches that front end: it is the last arm of
  the macro ladder, so its expansion is byte-for-byte what it always was.

One limit is worth stating: the front end's diagnostic is attributed to the
generated alias that forwarded the tokens, with the face file shown as the
macro invocation. A `macro_rules!` hop loses the author's token spans, so a
hand-written face cannot get the caret on the offending token itself.

## The IDE completes face fields

A macro's token tree is opaque to an editor, so `crate::control_object! { … }`
used to offer nothing inside its braces — not the field names, not even path
completion in value position. Two pieces fix that without changing the
authoring syntax:

- each generated `*_object!` alias hands the author's tokens to
  `face_fields_mirror!`, and `external_object!` reaches the same emitter through
  `face_fields!`; both build a real field list;
- that call carries `#[cfg(rust_analyzer)]`, and the mirror it expands to is
  valid, type-correct Rust, so an editor that compiles the mirror reports no
  errors on the author's lines: a field written as an expression keeps its
  tokens and infers its type, a field that names a type (`kind`, `preset`,
  `parts`, `handle`, `flow_provider`) moves into the annotation, and a field the
  mirror cannot type (`name: { zh: "…", en: "…" }`, `requires: [a => b]`,
  `registry_name: slot`, `getting_from_other_registry: None`, an empty list)
  keeps its name while its value becomes `loop {}`.

`FaceFields` is a hidden mirror of the authoring vocabulary, declared in the
order the macros accept, with one type parameter per field. No host code
constructs it: the mirror is a `fn` that an editor compiles under
`cfg(rust_analyzer)` and `rustc` never expands, and it is what lists the
remaining field names and answers the value position.

The cfg belongs to the crate being edited. A host declares it from its build
script — `build_method` emits `rustc-check-cfg` — and an external
implementation, which has no build script, declares it in its own manifest:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(rust_analyzer)'] }
```

Note that the cfg cannot be moved into `run_method` instead: an editor applies
`cfg(rust_analyzer)` to the crates it analyzes directly, not to a dependency
resolved from git, so a macro defined in the dependency would take the
compiler branch and offer nothing.

Identities are unchanged: `source`, `registry_name`, and therefore `NodeId`,
registry paths, and every graft selector keep the same values.

A graft plan declared with `static_graft_plan!` is now captured from the host
entry whether or not the host also declares `application!(entry = ...)`,
matching how `SourceScope` already resolved the entry. Previously the plan was
silently dropped without the `application!` declaration.

See `examples/control-button/` for the README Control/Button tree as a real
host, together with `examples/control-button-graft/` for an out-of-project
implementation that grafts over it.

## Studio

Run the standalone package with `cargo run --manifest-path studio/Cargo.toml`.
Use `1` through `3` to select Search, Inspect, or Data. `watch` is
provided by `xirang-dev`, a workspace-only binary behind the non-default
`dev-supervisor` feature: it rebuilds the child Studio process after source,
Cargo, or plugin catalog changes, so it needs this checkout
(`cargo run -p xirang-toolchain --features dev-supervisor --bin xirang-dev -- watch`)
and is not installed by `cargo install xirang-toolchain --features cli` either.

## Plugin host

Install only `VerifiedPluginArtifact` values. Declare Wasm slots at compile
time; enable `process-tools` only for isolated process adapters. Existing
`PluginManifest` flow contracts and lock records remain compatible with the
new host APIs.
# Graft overlay migration

The current graft model is an immutable overlay. A host keeps its original
source tree and declares the external implementation at its entry point:

```rust
let plan = xirang_toolchain::run_method::graft_plan!(framework,
    cut ["root/canvas"] graft "canvas_fast",
    cut ["root/layout"] full graft "layout_v2",
);
let effective = base.overlay(&plan, &external)?;
```

`cut A graft X` replaces one logical slot and inherits A's children. `cut A
full graft X` replaces the entire subtree rooted at A. A range cut addresses
contiguous siblings. The base and external registries remain unchanged; the
effective view is published only after contract, registration-rule, admission,
and connector validation succeeds.

The pre-overlay `Registry::graft` API and Studio source-copy workflow have been
removed. The public execution path is `GraftPlan` followed by `Registry::overlay`.

## `graft.plan` records

A `.xirang/external-grafts/<selector>/graft.plan` file is an authoring record,
not a declaration the compiler sees and not an `overlay` call. Its layout is
unchanged (`version=1`, `target`, `target_path`, `graft`, `full`), but the
format now has a reader: `GraftPlanDocument` in the kernel parses and renders it,
refuses an unknown version or key instead of guessing, and is the only place the
layout is defined. The record is also now the **input** to an overlay:
`xirang_toolchain::run_method::apply_recorded_grafts` loads `.xirang/external-grafts/`
and `Registry::overlay_recorded` reconciles each record against the static
declarations, applies it, and reports every adjustment; see [`graft.md`](graft.md)
for the precedence policy and which reports are fatal.
`xirang-toolchain` gained `declared_grafts`/`host_entry_source`
for authoring surfaces that need to know which slots the build ships.

`xirang_toolchain::run_method::ExternalGraftPlanFile` no longer exposes `target`,
`graft`, and `full` as public fields; it carries the parsed `GraftPlanDocument`
and the selector, and answers through `target()`, `target_path()`, `graft()`,
`full()`, and `plan_path()`. `root` is still a public `PathBuf` field — there is
no `root()` method — alongside the public `selector` and `document` fields. New
sibling functions read, list, re-scope, and trash plans:
`read_external_graft`, `list_external_grafts`, `rewrite_external_graft`, and
`remove_external_graft`. `create_external_graft` keeps its signature and now
writes through the kernel document, so everything it writes reads back.
