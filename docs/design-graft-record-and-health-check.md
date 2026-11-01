# Design: graft records reach `overlay`; `runtime_checks` becomes a host API
# 设计：graft 记录接入 `overlay`；`runtime_checks` 成为宿主 API

Design only; no `.rs` edited. Argues settled decisions 2 and 3 of `docs/roadmap-1.0.md:24-29`. Cite form is `path:line`.
仅设计，未改任何 `.rs`。为 `docs/roadmap-1.0.md:24-29` 的已定决策 2、3 提供论证。

## Part 1 — wire `.xirang/external-grafts/<selector>/graft.plan` into `Registry::overlay`
## 第一部分 — 把计划文件接进 `Registry::overlay`

### 1.1 Three layers today, and the gap

| Layer | Owner | Read by |
| --- | --- | --- |
| Declaration | `static_graft_plan!` host entry (`run_method/macros/entry.rs:61`) | build → `GraftSyntax` → `BUILTIN_GRAFT_CUTS` (`build_method/renderer/pass.rs:63-99`) → `builtin_static_plan().grafts()` (`core/.../release/release.rs:188`) |
| Application | `Registry::overlay` / `overlay_static` (`core/.../tree/graft_ops/overlay.rs:95,111`) | hosts; production `plugin-host/src/deployment.rs:45-60`; example `examples/control-button/tests/registry.rs:356-366` |
| Record | `.xirang/external-grafts/<selector>/graft.plan` (`run_method/authoring/external_graft/plan.rs:26,96,131,148`) | Studio only (`studio/studio/app/graft.rs:7-18`) |

`GraftPlanDocument::cut()` (`core/.../plugin/graft/document.rs:153`) and `plan()` (`:163`) have **no production caller**: grep finds only `document.rs` tests, and Studio calls `declaration()` (`studio/.../graft.rs:117`), never `cut`/`plan`. The build only warns about a plan no declaration keeps alive (`build_method/graft_plan_check.rs:99`, emitted `build_method/pipeline.rs:37-44,68-72`) and never opens a plan (`graft_plan_check.rs:4-8`). Net: the record is inert — what Studio writes changes the screen and nothing else.

### 1.2 Where the loading entry point belongs

Split by the kernel's no-I/O rule (AGENTS.md rule 3):
- **Pure reconciliation → `core`.** Which base `NodeId` a record's `target`/`target_path` addresses, comparing that slot against the declaration's cuts, and composing the cut list are pure tree operations; they sit beside `resolve_cut_targets` (`resolution.rs:42`) and `overlay_cuts` (`overlay.rs:123`) and test without a filesystem.
- **Reading `.xirang/...` → `run_method`.** The only reader today is feature-gated behind `authoring` (`run_method/Cargo.toml:11`, pulls `syn`). A runtime host must not need `authoring`, so the loader goes in the **non-gated runtime module**, not `authoring::external_graft`.

New files, mounted the uniform way: `core/src/registry_core/tree/graft_ops/record.rs` (declared from `graft_ops.rs:31` beside `overlay`/`resolution`) and `run_method/runtime/graft_record.rs` (declared from `run_method/runtime/runtime.rs:4-7` beside `evidence`).

### 1.3 Kernel seam (pure): exact signatures

```rust
// core/src/registry_core/tree/graft_ops/record.rs
pub struct RecordedGraft { pub selector: String, pub document: GraftPlanDocument }

pub enum RecordReport {
    IdentityDrifted { selector: String, target: NodeId, target_path: String },
    UnkeptSlot { selector: String, target_path: String },
    SelectorDirectoryMismatch { directory: String, document_graft: String },
    GranularityOverridden { slot: NodeId, declared_full: bool, recorded_full: bool },
    DeclarationOverridden { slot: NodeId, declared: String, recorded: String },
    TypedDeclarationKept { slot: NodeId, declared: String, recorded: String },
    RecordSelectorUnresolved { selector: String, graft: String },
}

pub struct RecordedOverlay { pub effective: Registry, pub reports: Vec<RecordReport> }

impl Registry {
    /// Slot, the slot's CURRENT logical path, and the granularity to apply.
    pub fn resolve_record(&self, record: &RecordedGraft) -> RegistryResult<ResolvedRecord>;
    /// Apply records over the declarations that keep slots alive.
    pub fn overlay_recorded(&self, records: &[RecordedGraft], declared: &[StaticGraftCut],
                            external: &Registry) -> RegistryResult<RecordedOverlay>;
}
```

`resolve_record` returns the slot's **current** path (`self.path_for(id)`, `query.rs:80`), never the stored `target_path`: `GraftCut.cut` is a string `resolve_path` must match exactly (`resolution.rs:22-28`), so a record written before a rename would otherwise miss. `overlay_recorded` builds one `GraftPlan` with `framework = self.framework()` (`tree/registry.rs:96`) and delegates to the private `overlay_cuts` (`overlay.rs:123`), so hard failures (ambiguous selector, contract mismatch, duplicate cut) stay byte-identical to today.

`GraftPlanDocument::plan(framework)` (`document.rs:163`) should be **deleted**: `overlay_cuts` rejects a framework differing from the base (`overlay.rs:129`), so a caller-supplied `FrameworkId` is a mismatch waiting to happen. `cut()` (`:153`) folds into the resolver; keep it `#[doc(hidden)]` only if the CLI wants a tree-free preview.

### 1.4 Execution surface: exact signatures

```rust
// run_method/runtime/graft_record.rs  (NOT feature-gated)
pub fn graft_record_root(package_root: &Path) -> PathBuf; // root/.xirang/external-grafts
pub enum LoadedGraft { Record(RecordedGraft), Unreadable { selector: String, reason: String } }
pub fn load_graft_records(package_root: &Path) -> Result<Vec<LoadedGraft>, String>;
pub fn load_graft_record(package_root: &Path, selector: &str) -> Result<GraftPlanDocument, String>;
pub fn apply_recorded_grafts(base: &Registry, external: &Registry,
    declared: &[StaticGraftCut], package_root: &Path) -> Result<GraftOverlay, String>;
pub struct GraftOverlay { pub effective: Registry, pub reports: Vec<RecordReport>,
                          pub unreadable: Vec<(String, String)> }
```

`package_root` is explicit so the module needs neither the `authoring` feature nor the `AuthoringContext` thread-local (`authoring/validation/validation.rs:88-110`). The loader must reuse `GraftPlanDocument::parse` (`document.rs:68`) and the `lexicon` constants (`core/.../lexicon/lexicon.rs:74-83`); refactor `authoring::external_graft` to delegate to it so one parser and one layout serve Studio and hosts.

### 1.5 How a host and the CLI call it

Host: `apply_recorded_grafts(&base_registry(), &external, builtin_static_plan().grafts(), Path::new(env!("CARGO_MANIFEST_DIR")))`; print `outcome.reports` and `outcome.unreadable`, then use `outcome.effective`. CLI (roadmap B4 item 2, `xirang grafts [--json]`) calls only `load_graft_records(root)` and prints selector/`target`/`target_path`/`graft`/`full` or the parse reason. Cost: `cli/Cargo.toml` has no `xirang-toolchain` dependency, so this needs one direct dep (its `studio` dep is transitive and unusable).

### 1.6 Reconciliation: `target` NodeId vs `target_path` vs the tree

Compute `by_id = base.find(document.target)` (`query.rs:76`) and `by_path = base.resolve_path(&document.target_path)` (`resolution.rs:22`):

| `by_id` | `by_path` | Result |
| --- | --- | --- |
| `Some(a)` | `Some(b)`, `a==b` | clean; apply at `a` |
| `Some(a)` | `None` | apply at `a`; report `IdentityDrifted` (stale human text) |
| `Some(a)` | `Some(b)`, `a!=b` | **refuse** the record (error names both paths); contradictory is not a guess |
| `None` | `Some(b)` | apply at `b`; report `IdentityDrifted` (face edited; id derives from namespace/path/kind) |
| `None` | `None` | report `UnkeptSlot`, skip; do not fail the whole overlay |

Only `a!=b` is fatal, because either selector silently changes which face the author meant. Skipping `UnkeptSlot` matches the build, which warns and prunes (`graft_plan_check.rs:99`) rather than failing.

### 1.7 Decision: record vs declaration naming the same slot differently

**A — record wins + warning.** Prevents the record being inert for the case it exists to serve: choosing a plugin the binary could not name. The string declaration form exists because "the implementation may arrive later as a plugin, and the compiler is not asked to resolve a crate the author has not written yet" (`document.rs:173-179`). Allows an unreviewed, machine-local `.xirang/` file to re-route shipped behavior, and makes the runtime overlay diverge from `BUILTIN_GRAFT_CUTS` (`overlay_static`, pinned at `examples/control-button/tests/registry.rs:356`, can yield a different tree).

**B — declaration wins + warning.** Prevents source/review bypass and keeps build scope, static plan, and runtime overlay aligned. Allows the record's `graft`/`full` to have no runtime meaning — the feature is wired but unobservable, and there is no way to opt into a runtime override at all.

**C — refuse + error.** Prevents any silent divergence. Allows one stale authoring file to abort overlay/startup, promoting a non-compiled artifact above the compiled declaration while the build treats the same file as advisory (`graft_plan_check.rs:4-8`).

**Recommendation: A, scoped by declaration form.** Apply the record over a **string-form** declaration (`StaticGraftCut::cut()` is `CutTarget::Path`, `release.rs:58`) and report `DeclarationOverridden`. Treat a **typed-form** declaration (`CutTarget::Id`, emitted by `pass.rs:81-84` from `cut(...)`) as final: report `TypedDeclarationKept` and ignore the record. If the record's selector does not resolve in `external`, report `RecordSelectorUnresolved` and fall back to the declaration (B for that one case, so a bad record can never break a graft the declaration could satisfy).

Reasoning: (1) the record is the newest, most specific artifact and the build is explicitly written never to open it (roadmap decision 3); under B the `graft` field has no production reader and the feature is empty in behavior. (2) A typed declaration is the host saying "this exact linked crate face"; the compiler resolved it and the binary linked it, so a text file must not defeat it. A string declaration is already resolved dynamically by name (`resolution.rs:103`), so a record selector adds no new class of trust. (3) The residual A risk is bounded: the record cannot resurrect a pruned slot (§1.6 `UnkeptSlot`), cannot select a face the loaded `external` registry lacks, and is reported on every overlay; making it fatal (C) would be disproportionate.

### 1.8 The three edge cases the brief names

- **Plan's slot no declaration keeps alive** (today only the build warning `graft_plan_check.rs:99`): in `overlay_recorded`, expand `declared` to its `NodeId`s with `resolve_cut_targets` (`resolution.rs:42`, handles ranges) and test membership; report `UnkeptSlot`, skip that record, keep the rest. Runtime and build then say the same thing.
- **Directory selector vs `document.graft` vs the declaration's `graft`** are two independent checks. (i) `create_external_graft` sets directory and `document.graft` equal (`plan.rs:114`), so a mismatch means hand-edit/rename. **Later decision (S1+), which supersedes the original "report and use the file's `graft`": this is now a hard `Err`.** Two candidate implementation names with no way to tell which one the author meant is a state the source tree cannot show, so the record is refused rather than silently resolved; `RecordReport::SelectorDirectoryMismatch` no longer exists (`record_tests::a_directory_that_disagrees_with_its_plan_is_refused`). (ii) `document.graft` vs the declaration's `graft`: §1.7.
- **`full` vs node-only.** `document.full` maps to `GraftCut::subtree`/`new` (`document.rs:153-159`), consumed by `apply_overlay_face` (`overlay.rs:239-287`: full discards base children and adopts the external subtree; non-full keeps base children and re-validates the replacement's rule). `graft` and `full` are one atomic record — override both or neither; a differing `full` reports `GranularityOverridden`. The plan format has no range field, so a record can never create a range; when the declaration is a range and the record resolves to one node inside it, only that node is overridden (per-node report) and the rest keep the declaration.

### 1.9 Tests that pin the design

| # | Crate | File | Assertion |
| --- | --- | --- | --- |
| 1 | `xirang` | `core/src/registry_core/tree/graft_ops/record.rs` | id and path agree → resolves to `target` |
| 2 | core | `.../record.rs` | id gone, path present → resolves by path + `IdentityDrifted` |
| 3 | core | `.../record.rs` | id and path differ → `Err` naming both |
| 4 | core | `.../record.rs` | both gone → `UnkeptSlot`, never fatal |
| 5 | core | `.../record.rs` | over a string declaration → record's kind survives + `DeclarationOverridden` |
| 6 | core | `.../record.rs` | over a typed declaration → declaration's kind survives + `TypedDeclarationKept` |
| 7 | core | `.../record.rs` | record selector absent from `external` → declaration applied + `RecordSelectorUnresolved` |
| 8 | core | `.../record.rs` | `full=true` discards base children, `full=false` keeps them (mirror `resolution.rs:266-314`) |
| 9 | core | `.../record.rs` | differing `full` reports `GranularityOverridden`, never mixes graft/full |
| 10 | core | `.../record.rs` | after the stored `target_path` goes stale, the applied cut uses the current `path_for` |
| 11 | `xirang-toolchain` | `run_method/tests/graft_record.rs` | **`a_record_on_disk_reaches_overlay`**: write a `graft.plan` under a temp root, call `apply_recorded_grafts`, assert the effective kind at the slot is the record's implementation — the test that would have caught "record never reaches overlay" |
| 12 | run_method | `run_method/tests/graft_record.rs` | directory ≠ `document.graft` → `Err` naming both names (S1+ superseded the report) |
| 13 | run_method | `run_method/tests/graft_record.rs` | unparseable plan → `Err` naming every unreadable plan; no record is applied (S1+ superseded "land in `unreadable`") |
| 14 | run_method | `run_method/tests/graft_record.rs` | loader/overlay works with `default-features` (no `authoring`) |
| 15 | example | `examples/control-button/tests/registry.rs` | a record overriding `root/control/button` changes the effective tree but not `builtin_static_plan()` |
| 16 | `xirang-toolchain` | `build_method/graft_plan_check.rs` | `an_undeclared_plan_target_is_an_error_with_the_clause`: a plan no declaration can name is a build error carrying the paste-ready clause; `a_gated_declaration_still_counts` keeps a `#[cfg]`-off declaration sufficient |

Tests 11–15 are new. Test 11 alone closes the gap.

### 1.10 Open questions / risks (Part 1)

1. **Typed-is-final exception.** The owner may prefer plain A or plain B; the split in §1.7 is my recommendation, not a settled decision.
2. **Directory vs `document.graft`.** Report-and-use-document is a choice; refusing is defensible if the directory is meant as an immutable key.
3. **`cut()`/`plan()` removal** is a public-API change pre-1.0; keep `cut()` if the CLI wants a tree-free preview.
4. **Fatal vs skip** for contradictory records: §1.6 refuses `a!=b` but skips `UnkeptSlot`; whether one broken record fails the whole overlay is an owner call.
5. **Risk not resolvable by reading:** whether real hosts keep `.xirang/` out of VCS. If ignored, A is machine-local behavior drift; the doc must state that `.xirang/external-grafts/` is runtime input.
6. **Risk:** once A applies, `overlay_recorded` and `overlay_static` can produce different trees for the same inputs. Nothing compares them today; test 15 pins the intended divergence, but a reviewer may read it as a bug.

---

## Part 2 — make `runtime_checks` real as a documented host API
## 第二部分 — 把 `runtime_checks` 做成有文档的宿主 API

### 2.1 Today

`RuntimeCheckSpec` and its five variants (`core/.../declaration/runtime_checks.rs:127-134`), `run` (`:240`), `RuntimeValue` (`:37`), `Coordinates` (`:80`), `Provenance` (`:14-35`), and `COORDINATES_IN_VIEWPORT`/`FINITE_NUMBER`/`NON_EMPTY_TEXT` (`:251-255`) are public and re-exported (`core/src/lib.rs:36-39`, `run_method/runtime/runtime.rs:9-12`). `Registry::health_check` (`core/.../tree/inspection/inspection.rs:27`) loops a face's `runtime_checks` (`registration.rs:227`) and aggregates failures — with **zero callers and zero tests**. `RuntimeValue` has **no producer anywhere** (grep: definition, re-export, and the `PhantomData` path check `run_method/tests/path_compat.rs:123`). `plugin-host/src/deployment.rs:28` calls `PluginInstance::health_check`, a different trait method, not this one. The field is parsed/stored by every declaration surface (`run_method/macros/face_registration.rs:39`, `snapshot/snapshot.rs:129`, `authoring/manifest/parse/parse.rs:186`, `macro/src/mirror.rs:82`) and read only as search text (`call_report.rs:53,170`) and counts (`metadata.rs:50`), so `runtime_checks:` is inert metadata.

### 2.2 Decision and boundary

Per roadmap decision 2 (`:24-26`): document as a host API, do not delete. The kernel cannot observe a value, so **the host owns the boundary** and calls `health_check` where a host-side value crosses into a plugin/consumer, for the face whose declared checks apply. It is voluntary validation, not an automatic hook.

### 2.3 Exact host-facing call pattern

Who: the host application, at its value boundary, right after obtaining/deriving a value it will hand to a consumer. Arguments: (1) `node`, the face's `NodeId` — the host already has it (`crate::control::object::button::NODE_ID`) or finds it via `Registry::find` (`query.rs:76`), which returns the snapshot (including `runtime_checks`, so the host can see the expected value kind); (2) `value`, a `RuntimeValue` the host builds; (3) `call_path`, `Vec::new()` when not tracing; (4) result `RegistryResult<()>` = `Result<(), Box<RegistryError>>` (`diagnostic/error.rs:28`), rendered with `Display` (`error.rs:242`).

### 2.4 Constructing a `RuntimeValue` per check

`Provenance::default().push(node, "Button", "paint", "<observed>")` attaches the trail (`runtime_checks.rs:20-34`); object/operation are `&'static str`.

| Check | Value the host builds |
| --- | --- |
| `FiniteNumber` | `RuntimeValue::number(v, prov)` (`:51`) |
| `NumberInRange { min, max }` | `RuntimeValue::number(v, prov)` |
| `NonEmptyText` | `RuntimeValue::text(s, prov)` (`:55`) |
| `TextLength { min, max }` | `RuntimeValue::text(s, prov)` |
| `CoordinatesInViewport` | `RuntimeValue::Coordinates(Coordinates::new(x, y, w, h, vw, vh, "logical", "logical", prov))` (`:93-117`); the two space strings must be equal (`:305`) |

### 2.5 `call_path` without tracing

Pass `Vec::new()`. `CallTrace::current_path()` (`frames.rs:223`) is the traced value; `CallTrace::disabled()` (`call_trace.rs:98`) also yields an empty `current`, so `disabled().current_path()` is `Vec::new()`. The kernel never synthesizes a path; an empty path renders as no call evidence.

### 2.6 What the host does with the `RegistryResult`

`Ok(())` → every declared check passed. `Err(error)` → render with `format!("{error}")`, then apply host policy (reject the value / fail startup / log and continue). The error aggregates one child per failed check, each with the face's declaration source (`inspection.rs:53-63`) and the value's provenance as `source_chain` (`:59`). **Honest limitation:** B3a removed every `RegistryError` read accessor (`error.rs:108-117`), so a host can only `Display` the aggregate and cannot branch on `failure.check` or read `children`. For 1.0, document `health_check` as validate-and-render; re-adding `children()` is defensible now that `health_check` is a real caller, but it is a public addition (§2.12).

### 2.7 Bilingual doc text a host reads (rustdoc on `health_check`)

```rust
/// Validate one observed runtime value against the checks a registry face declares.
/// 用注册面声明的检查校验一个观测到的运行期取值。
///
/// This is a host API: the kernel never observes a value, so the caller owns the boundary.
/// Call it where a host-side value crosses into a plugin or consumer, with the `NodeId` of
/// the face whose `runtime_checks:` list applies, the `RuntimeValue` the host built, and the
/// current call path.
/// 这是宿主 API：内核从不观测取值，边界由调用方拥有。请在宿主侧取值跨入插件或消费者的
/// 那一点调用它，传入适用该取值的注册面 `NodeId`、宿主构造的 `RuntimeValue` 与当前调用路径。
///
/// `call_path` is `CallTrace::current_path()` when a trace is active; a host that does not
/// trace passes `Vec::new()`. The kernel never synthesizes a call path.
/// 有活动 trace 时传 `CallTrace::current_path()`；不接 trace 的宿主传 `Vec::new()`。
/// 内核从不合成调用路径。
///
/// An empty `runtime_checks:` list passes for any value. On failure the error aggregates one
/// child per failed check, each carrying the face's declaration source and the value's
/// provenance; render it with `Display`. Presence or absence is the host's decision signal:
/// the registry does not choose whether a failed check is fatal.
/// `runtime_checks:` 为空的面对任何取值都通过。失败时按每条失败的检查聚合子错误，各自携带
/// 声明源与来源链；用 `Display` 渲染。存在与否就是宿主的决策信号：注册机不替宿主决定是否致命。
///
/// ```no_run
/// # use xirang_toolchain::run_method::{Provenance, Registry, RuntimeValue};
/// # fn demo(registry: &Registry, node: xirang::NodeId) {
/// let value = RuntimeValue::number(0.5, Provenance::default().push(node, "Slider", "measure", "0.5"));
/// if let Err(error) = registry.health_check(node, &value, Vec::new()) { eprintln!("{error}"); }
/// # }
/// ```
```

### 2.8 The example to add

Crate `xirang-example-control-button`, file `examples/control-button/examples/health_check.rs` (sibling of `examples/tree.rs`), run as `cargo run -p xirang-example-control-button --example health_check`. To make it real, the Button face (`examples/control-button/src/control/object/button/button.rs`) gains `runtime_checks: [NON_EMPTY_TEXT]`; the example validates a label, passing one good `RuntimeValue::text` and one whitespace-only and printing both results. Without a declared check it would prove nothing.

### 2.9 Tests that pin the design

Five per-check unit tests in `core/src/registry_core/declaration/runtime_checks.rs` (add a `#[cfg(test)]` module; none exists today):

| # | Test | Assertion |
| --- | --- | --- |
| 1 | `coordinates_in_viewport_accepts_inside_and_rejects_overflow_and_space_mismatch` | inside → `Ok`; `x+width > viewport_width` → `Err`; `"logical" != "screen"` → `Err` (`:305`) |
| 2 | `finite_number_accepts_finite_and_rejects_nan_and_infinity` | `1.0` → `Ok`; `f64::NAN`, `f64::INFINITY` → `Err` (`:340`) |
| 3 | `number_in_range_is_inclusive_and_reports_inverted_bounds` | `0.0`,`10.0` → `Ok`; `10.5` → `Err`; `min>max` → message `invalid check bounds` (`:359`) |
| 4 | `non_empty_text_rejects_empty_and_whitespace_only` | `"x"` → `Ok`; `""`, `"   "` → `Err` (`:377`) |
| 5 | `text_length_counts_characters_not_bytes` | `{min:1,max:3}`: `"abc"`, `"日本語"` → `Ok`; `"abcd"` → `Err` (`:400`) |

One end-to-end test through `Registry::health_check` with a real registered face: `examples/control-button/tests/health_check.rs`, using `control_button::base_registry()` and the Button face after §2.8 declares a check. Assert `health_check(button_id, &RuntimeValue::text("ok", prov), Vec::new())` is `Ok`, a whitespace value is `Err`, and `format!("{error}")` contains `non_empty_text` and the face's source file. A core twin in `inspection.rs` (register a snapshot with `runtime_checks: vec![FiniteNumber]` via `register_snapshot_batch`, NaN → `Err`) is worth keeping but is not the "real registered face" test.

### 2.10 Can all five checks be exercised without new API?

**Yes — none is impossible with today's public surface.** `Number`/`Text` have constructors (`runtime_checks.rs:51,55`); `CoordinatesInViewport` needs `RuntimeValue::Coordinates(Coordinates::new(...))` — the tuple variant and `Coordinates::new` are public even though there is no `RuntimeValue::coordinates` helper (`:37-48,93`). Friction, not a blocker: the 9-argument constructor is the only awkward call; a convenience helper plus a doc example would remove it for one public item. The real gap is on the **result** side, not the value side: no `RegistryError` read accessor exists (§2.6), so a host cannot branch on which check failed. Re-adding `RegistryError::children()`/`node()` (removed in B3a, `error.rs:108-117`) would cost ~20 lines plus tests and make `health_check` programmatically consumable; if the owner wants only "render and decide", no new API is needed.

### 2.11 Rejected alternative: delete the vocabulary

Deleting `RuntimeValue`/`RuntimeCheckSpec`/`Provenance`/`Coordinates` and `Registry::health_check` was the other B0 option (`roadmap:42-43`). Rejected because: (1) it is a public-API removal touching the crate-root whitelist (`core/src/lib.rs:36-39`) and the `run_method` re-export (`runtime/runtime.rs:9-12`), which the roadmap already decided against; (2) `runtime_checks:` is a declared face field on every surface (`registration.rs:227`, `face_registration.rs:39`, `snapshot.rs:129`, `parse.rs:186`), so deleting the runtime half leaves a parsed, stored field with no consumer or forces deleting the field too, breaking every face source that writes `runtime_checks: [...]` before 1.0; (3) the five checks are ~300 lines of pure boundary logic (inclusive ranges, NaN, char-counting, coordinate-space equality) that nothing else expresses; (4) wiring costs only doc text plus an example and makes a documented field honest, the roadmap's stated goal.

### 2.12 Open questions / risks (Part 2)

1. **Result readability.** Should `RegistryError::children()`/`node()` return (undoing part of B3a) so `health_check` failures are machine-consumable, or is `Display`-only acceptable for 1.0? I recommend `Display`-only now, and a read accessor when a real host asks.
2. **Doc ownership.** The §2.7 text must land as rustdoc on `health_check`; someone must decide whether it is mirrored into `README`/docs.rs (roadmap B4 item 5 adds per-crate READMEs).
3. **Example face change.** Adding `runtime_checks: [NON_EMPTY_TEXT]` to the Button face touches an example other tests use; confirm no snapshot/identity assertion shifts (`NodeId` derives from namespace/path/kind, not `runtime_checks`, so it should be inert — unverified here).
4. **Boundary ownership.** Nothing enforces that a host calls `health_check` at the right boundary; this stays a documented convention, not a checked invariant.
5. **Risk not resolvable by reading:** whether a real host value maps cleanly onto `RuntimeValue`. A struct-typed host value must be flattened into coordinates/number/text, and there is no documented guidance yet.
