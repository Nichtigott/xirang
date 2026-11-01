# Adversarial audit — B1 (bug fixes) / B2 (dedup) / B3a (API deletions)

Scope: falsify the three batches' success claims against `docs/roadmap-1.0.md`.
Read-only. No git, no `.rs` edit. Test runs: `cargo test -p xirang-core`,
`-p xirang-run-method` (also `--features authoring`), `-p xirang-example-control-button`,
`-p xirang-debug-method`, `-p xirang-plugin-host`, `-p xirang-studio` — all
green during the audit (one concurrent editor was active in `core/run_method/build_method`;
no recurring failure was observed).
所有测试命令在审计期间一次通过，无稳定失败。

Legend: **Falsified** = a claim in the roadmap is wrong or an equivalence test does not
test what it says. **Confirmed under attack** = survived specific probes.

---

## Falsified

### F1. The macro ladder still records the wrong `preset`/`parts` names for the most common face shape (custom `handle`, default preset/parts)

**Claims attacked.** B1 item 1: "`face_objects` arm 绑定 `$preset`/`$parts` 却展开成
`NoPreset`/`NoParts` … 先写一条**失败**测试钉住'自定义 preset/parts 不被吞'，再修".
The B1 fix comment (`run_method/macros/face_objects.rs:278-289`) states the
boundary: an omitted `preset`/`parts` "keeps `NoPreset`/`NoParts`" /
"省略者保持 `NoPreset`/`NoParts`". That is only true for the **no-`handle`** arm
(lines 239-341). One arm earlier it is false.

**Mechanism.** The "compact form with a custom handle but default preset and parts"
arm (`face_objects.rs:175-238`) has no `preset`/`parts` bindings, so it re-dispatches
to `__control_object!` with literal tokens:

```rust
// face_objects.rs:207-211
$crate::__control_object! {
    collector: $collector,
    kind: $kind,
    preset: $crate::NoPreset,
    parts: $crate::NoParts,
```

The full `__control_object!` arm then names them by `stringify!` of the captured type
(`face_objects.rs:55-58`):

```rust
preset: ($preset),
preset_name: stringify!($preset),
parts: ($parts),
parts_name: stringify!($parts),
```

`stringify!($crate::NoPreset)` is **not** `"NoPreset"`. Verified with `rustc` from
stdin (no source file written):

```
$ printf 'macro_rules! m { () => { stringify!($crate::NoPreset) }; } struct NoPreset; fn main(){ println!("[{}]", m!()); }' | rustc -o /tmp/p - && /tmp/p
[$crate :: NoPreset]
```

and the exact two-macro indirection used here (`compact!{ preset: $crate::NoPreset }`
forwarded to a `$preset:ty` capture and stringified) also prints `[$crate :: NoPreset]`.
So `REGISTRATION.preset == "$crate :: NoPreset"` and `REGISTRATION.parts == "$crate :: NoParts"`.

**It ships.** All three example faces write `handle:` and no `preset:`/`parts:`
(`examples/control-button/src/control/control.rs:19-21`,
`.../object/button/button.rs:15-18`, `.../object/slider/slider.rs:15-18`), so all take
this arm. The literal is embedded in the compiled example:

```
$ grep -a -o '.\{0,10\}\$crate :: NoPreset.\{0,40\}' target/debug/libcontrol_button.rlib
Control$crate :: NoPreset$crate :: NoPartscontrol_button::control
$ grep -a -o '\$crate :: NoPreset' target/debug/xirang-example-control-button | head -1
Control$crate :: NoPreset
```

**Why no gate caught it.** The new B1 test
`run_method/tests/face_preset_parts.rs` only exercises the no-`handle` arm (it writes
`preset`+`parts` explicitly); no test writes `handle:` with an omitted `preset`. All
six test suites above pass.

**Consequences (not cosmetic).**
- `RegistrationRule::validate` (`core/.../declaration/declaration.rs:97-144`) compares
  `check.preset != expected`, and `assert_static_registration`
  (`core/.../release/release.rs:224-250`) compares `str_eq(required, info.preset)`.
  A parent rule using `.require_preset("NoPreset")` now *rejects / build-panics* a child
  that genuinely uses the default.
- The compiled value and the authored-snapshot value disagree for the same face:
  `RegistrationSnapshot::merge_authored` (`core/.../declaration/owned.rs:189-226`) and
  the manifest parser default to the literal `"NoPreset"`
  (`run_method/authoring/manifest/parse/parse.rs:111`), while the compiled
  `into_snapshot` copies `"$crate :: NoPreset"` (`registration.rs:287`). This is exactly
  the compiled/owned divergence B2 claims to have eliminated; the B2 equivalence test
  cannot see it because it builds both twins by hand with identical data
  (`declaration.rs:188-289`).

Minimal counterexample is in **Proposed tests** (`face_arm_defaults.rs`).

---

### F2. `path_is_under` silently loosened the connector's external-branch test (B2 item 4)

**Claim attacked.** B2 item 4 and its acceptance: merge the four prefix checks into one
and keep "既有行为测试不变" (existing behavior tests unchanged). The completion note even
records removing the connector's per-comparison allocation:
"连接器里 `format!("{owner_path}/")` 的每次比较分配已消".

**The two old spellings were not equivalent at equality.** `lexicon.rs:82-104` documents
the four old sites:

> They differed in how they spelled the check — `== prefix || strip_prefix(prefix).starts_with('/')`
> against `starts_with(&format!("{prefix}/"))` against a bare `strip_prefix(prefix)`
> 它们的写法互不相同——`== prefix || strip_prefix(prefix).starts_with('/')`、
> `starts_with(&format!("{prefix}/"))`、光秃秃的 `strip_prefix(prefix)`

`Admission`/`OwnedAdmission` used the equality-inclusive form; the connector used
`starts_with(&format!("{owner_path}/"))`, which is **false** when
`provider_path == owner_path`. The merged predicate makes equality true
(`lexicon.rs:105-110`, and the test asserts it at `:247`).

**Equality is reachable — it is the normal ancestor-provider case.** In
`connector_errors_from` the owner is the registry `entry.info.parent`. `ancestor_providers`
(`connector.rs:66-89`) finds the face `E` whose child registry **is** that owner and can
return `E.info` as the provider. A child registry's path is exactly the owning face's path
(`transaction.rs:245-252`: `format!("{target_path}/{segment}")`; `path_for` builds
`format!("{header.path}/{registry_name}")`, `query.rs:80-88`). Therefore

```
provider_path == owner_path          // the provider is the registry's own parent face
```

- **Old** (`starts_with("{owner_path}/")`): `false` → `is_external = true` → admission gate applied.
- **New** (`path_is_under`): `true` → `is_external = false` → gate skipped.

Both connector sites are affected: `connector.rs:160` and `connector.rs:265`.

**Concrete counterexample.**
- Face `P` (`registry_name = "p"`, `needs_registry = true`, `provides = ["cap"]`,
  `admission = Admission::new(&["allowed"], &[])`) registered at root (`path = "root"`).
- Its child registry `RP` has `path = "root/p"` and `admission` copied from `P`
  (same struct, `transaction.rs:253-255`).
- Leaf `L` under `RP` with `requires = ["cap" => P.kind]`.

Then `root.provider_for(RP.id, "cap", …)` returns `P`; `provider_path = "root/p"`;
`owner_path = RP.path() = "root/p"`.
- Old: `is_external = true`, `admission.accepts("root/p")` is `false` (allow-list
  non-empty and `"root/p"` is not under `"allowed"`) → connector error
  "admission gate rejects that external branch".
- New: `is_external = false` → the tree registers without error.

`core/src/registry_core/requirements/requirements.rs:186-194`
(`satisfied_requirement_produces_no_diagnostic`) confirms this ancestor-provider shape is
the intended one. There is **no connector test at all** in `connector.rs`, and
`admission_twins_accept_and_reject_identical_paths` tests only `Admission`, so B2's
"两边结果相同" tests do not cover this. Whether the new classification is the *desired*
behavior (the owner's parent face is arguably not an external dependency) is a product
decision — but it is an unacknowledged behavior change, and the direction is a silent
loosening of a trust gate.

---

### F3. B2's flow equivalence test does not compare the two entry points its own doc names; `compatible_with` is literal-only

**Claim attacked.** `core/src/registry_core/plugin/contracts/contracts.rs:178-190` says
the two bodies that "could drift apart" are `compatible_with` (compiled) and
`semantically_compatible_with` (owned), and that
`static_and_owned_flow_contracts_compare_identically` pins them:

> so `compatible_with` on the compiled side and `semantically_compatible_with` on the
> owned side could drift apart on, say, whether the version participates. Borrowing the
> fields once keeps both twins on one comparison. … pins the two entry points to each other.

and at `:250-256`:

> The compiled twin used to reach this through `compatible_with` while the owned twin
> repeated the whole conjunction inline
> 编译期孪生过去经 `compatible_with` 走到这里，而 owned 孪生把整个合取式内联重写了一遍

But the code is:

```rust
// :154-159  — literal only, no labels_compatible
pub fn compatible_with(self, expected: Self) -> bool {
    flow_fields_equal(FlowFields::from_declared(self), FlowFields::from_declared(expected))
}
// :171-175  — literal OR semantic
pub fn semantically_compatible_with(self, expected: Self) -> bool {
    flow_fields_equal(left, right) || flow_fields_semantically_compatible(left, right)
}
```

and the test pairs `compatible_with` with `owned_left == owned_right`, i.e. plain
`PartialEq` (`:495-499`), **not** with `owned_left.semantically_compatible_with(...)`.

**Counterexample.** Take id `"render.v1"`, version `1`, output `"CanvasFrame"`, and inputs
`"LocalCoordinates"` vs `"local_coordinates"`:

- `FlowContract::compatible_with` → `false` (literal field comparison).
- `OwnedFlowContract::semantically_compatible_with` → `true` (`labels_compatible` knows
  both spellings; `contracts.rs:282-294`).
- The test's first assertion compares the former to `owned_left == owned_right` (also
  `false`) and passes, so the divergence is invisible.

`compatible_with` has no production caller (only
`plugin_policy.rs:142-143`, which uses identical/`AbsoluteInput` cases that both a literal
and a semantic comparison reject), so nothing else catches it. Either the doc is stale or
`compatible_with` lost the semantic path during the merge; the equivalence test cannot
tell the two apart.

---

### F4. The macro ladder cannot express `preset`-only or `parts`-only together with a `handle`, and one arm-order comment is false

**Claim attacked.** B1 fix 1's premise is per-binding presence
(`face_objects.rs:278-281`: "presence is decided per binding … an omitted one keeps
`NoPreset`/`NoParts`, a written one is forwarded verbatim"). That is true only for the
arm at 239, whose matcher has no `handle` slot.

Probed shapes (generated alias / `__xirang_object!` path always injects `collector`
first):

| shape | arm that matches | result |
| --- | --- | --- |
| `preset`+`parts` + `handle` | 91 | ok, names forwarded |
| neither + `handle` | 175 | compiles, but F1 garbage names |
| `preset` only + `handle` | **none** | catch-all → front end → recursion guard error |
| `parts` only + `handle` | **none** | same |
| `preset`+`parts`, no `handle` | 239 | ok (B1 fix) |
| kind-only (no `handle`) | 239 | ok |

`normalise` (`macro/src/lib.rs:113-260`) only reorders/renames existing fields; it never
injects a missing `preset`/`parts`, and the recursion guard fires at `:228-240`
("these face fields are in the accepted order but the declaration still did not match").
So the front end's tolerant promise does not extend to these two shapes.

**False comment.** `face_objects.rs:88-90` (directly above the arm at 91):

> Fields are intentionally ordered like the generated source, but **every field after
> `kind` is optional** and has a safe default.
> 精简写法按生成源码顺序排列；除 kind 外均可省略，并使用安全默认值。

For the arm it annotates, `preset: $preset:ty`, `parts: $parts:ty` and
`handle: $handle:ident` are all mandatory (lines 94-100). It is also false for arm 175
(`handle` mandatory). Only arm 239 makes every post-`kind` field optional.

(The comment at `:343-349`, "the previous arm makes every field after `kind` optional, so
`{ collector, kind }` already matches there and a later smallest-face arm can never
fire", is **true**; the catch-all comment at `:351-357` also holds for the supported
entry points because both `__xirang_object!` arms inject `collector:` first. The caveat
is only a direct `__control_object! { kind: …, collector: … }`, which no author-facing
macro produces.)

---

### F5. B3a removed every host-readable path to `RegistryError`'s `node`, `path`, and `source`

**Claim attacked.** B3a completion: "`RegistryError` 八个读访问器(字段一个没删, `render_at`
仍读它们; `*_mut` 与 `with_children` 保留)". The zero-*in-workspace*-caller fact is true,
but the conclusion "safe deletion" is not.

`RegistryError`'s fields are private (`diagnostic/error.rs:15-24`) and the comment at
`:108-117` concedes this:

```rust
// The read accessors (`node`, `path`, `source`, `message`, `source_chain`,
// `call_path`, `registration_chain`, `children`) … were removed in B3a.
```

The surviving public surface (`:119-154`) is `with_children`, `children_mut`,
`message_mut`, `source_chain_mut`, `call_path_mut`, `registration_chain_mut`, plus
`Display`/`Error`. There is **no** reader for `node`, `path`, or `source` — not even a
`*_mut` (those three have no setter at all). A host that receives a `RegistryError` can
only scrape the rendered `Display` text; it can no longer map an error back to a `NodeId`,
a logical path, or a `SourceLocation`, which are exactly the three facts
`RegistryError::new(node, path, source, message)` requires a host to supply. This is a
plausible host-facing entry point and the strongest of the B3a removals.

---

## Confirmed under attack

### B2 merged cores

**`validate_registration_requirements` / `validate_object_contract`
(`declaration/declaration.rs:36-178`).**
Tried: empty lists, duplicate entries, `""` entries, whitespace, case, and cross-checked
each exact message string against the tests.
- Both callers pass the *same* source fields
  (`registration.rs:133-148` vs `owned.rs:90-105`; `contract.rs:85-93` vs `owned.rs:111-119`),
  so there is no field-swap divergence.
- `provided_contains` (`:50-54`) is exact `==` on `AsRef<str>`; for `S = &str` the
  `as_ref()` could resolve to `[u8]`, but both operands resolve identically, so the
  comparison is still text equality. Whitespace and case are not folded, `""` matches only
  `""`, duplicates are harmless. No divergence found.
- The five messages asserted in `registration_rule_twins_report_identical_failures`
  (`:310-321`) match `:104-141` byte for byte; the two in
  `object_contract_twins_report_identical_failures` (`:345-351`) match `:166-176`.
  Check order is preset → parts → exports → handle traits → part traits, and the
  aggregation test `parent_rule_aggregates_every_missing_structural_requirement`
  (`transaction.rs:376-404`) depends on it.
- One duplication survives the "single source" claim:
  `assert_static_registration` (`release.rs:224-250`) is a third, const copy of the same
  five conditions (plus output equality). It is currently *consistent* with the core, but
  a rule change must be made in both places. See Unresolved.

**`FlowFields` / `flow_fields_*` (`contracts/contracts.rs:191-294`).**
Tried: the 6×6 matrix over declared/owned, spelling variants, different ids/versions,
empty contract. `flow_is_declared` is one const fn; `labels_compatible` correctly refuses
to equate two *different* unknown labels (`:274-280`, pinned by
`unknown_output_labels_must_agree_literally`) and accepts known-domain respellings.
`FlowContract::semantically_compatible_with` and
`OwnedFlowContract::semantically_compatible_with` share both predicates and agree on the
whole matrix. The only issue is the *naming/pairing* of `compatible_with` (F3), not the
shared core itself.

**`path_is_under` (`lexicon.rs:105-110`).** Requested probes, reasoned (and proposed as
executable text in **Proposed tests**):

| probe | result | reasoning |
| --- | --- | --- |
| `path_is_under("", "")` | `true` | `path == prefix` short-circuits before `strip_prefix`. Callers: only reachable from `Admission` with an empty entry; identical to the old `Admission` spelling, so no regression. `is_registration_path("")` is still `false` because the prefix is `"registry_core"`. |
| `path_is_under("a", "")` | `false` | `"a" != ""`; `strip_prefix("") = "a"`, `starts_with('/')` false. An empty allowed/denied prefix is *not* a universal prefix. Old spellings agreed (the connector's `starts_with("/")` is also false). |
| `path_is_under("ab", "a")` | `false` | segment boundary holds; same as both old segment-correct spellings. |
| `is_registration_path("registry_core")` | `false` | `path_is_under` is `true` but the `&& relative != SCOPE_REGISTRATION_MODULE` subtraction (`:125-127`) removes exactly the name case the scope exemption list matches by name (`build_method/scope.rs:286`). Preserved as documented. |

Trailing slash: `path_is_under("a/", "a") == true`; `path_is_under("a", "a/") == false`.
Unicode: byte-prefix based, no panic, correct segment boundary. `\\`: returned `false`,
consistent with the old spellings, and **all current callers feed `/`-normalized paths**
(`build_method/node.rs:21-26` replaces `\` with `/`; registry paths are built with
`format!("{}/{}")` and `transaction.rs:251`), so the backslash case is latent, not live.
The equality delta is F2; the fixed-prefix `.rs`/`_extra` delta is in Unresolved.

### B1 fix 1 (arm 239) and the hidden helpers

- `run_method/tests/face_preset_parts.rs` passes: a custom `ProbePreset`/`ProbeParts`
  pair survives the no-`handle` arm, and `contract.required_parts`/`provided_parts` read
  the same types, so it is not just a label.
- `__face_ty_or!` and `__face_ty_name_or!` (`face_helpers.rs:139-167`) are valid in every
  position used: type position for `preset:`/`parts:` and expression position for
  `preset_name:`/`parts_name:` (only 8 use sites, all inside `__registration_face!`
  invocations: `face_objects.rs:294-297`, `face_external.rs:108-111`).
- The string forms are preserved. A `rustc` probe comparing the helper path against the
  direct `stringify!($preset)` path printed identical output for a generic
  (`some::Path<A, B>` → `some::Path<A, B>`) and a path-qualified type
  (`&'static str` → `&'static str`). This is the arm-239/re-dispatch question and it is
  clean; F1 is the different, un-repaired arm 175.
- The external compact path is also clean: `__external_object!` arm 2 re-dispatches with
  `handle: $kind` into arm 1, which uses the literal helpers
  (`face_external.rs:108-111`), and `external_compact_face.rs` asserts
  `REGISTRATION.preset == "NoPreset"`.

### Macro arm comments (target 5)

- `face_external.rs:61-67` (arm 1 must precede arm 2 because arm 2 has no `handle` slot):
  true — tested by attempting a `handle:` shape against both matchers; arm 2 cannot consume
  the tokens.
- `face_external.rs:163-175` ("a `macro_rules!` matcher cannot default an `ident` slot
  through a helper macro"): true; `handle: __face_ident_or!(…)` would reach
  `__registration_face!` as a macro call and no `$handle:ident` matches it.
- `face_objects.rs:343-349` (kind-only already matches the previous arm; a later
  smallest-face arm can never fire): true; `{collector, kind}` is consumed by arm 239
  because arms 91/175 require `preset`/`parts`/`handle`.
- `face_helpers.rs:148-157` (`__face_ty_name_or` exists so the default and the author's
  spelling stay distinguishable): true and pinned by `face_preset_parts.rs`.
- The `__face_ty_or` rationale (`face_helpers.rs:132-133`, "`$fallback` must be a type,
  not an expression, because a custom preset may be generic") states a true rule with a
  non-sequitur reason: the reason it must be a `ty` is that it is spliced into `preset:`
  type position; genericness is irrelevant. Cosmetic.

### The `registry_name` question (target 3): identity does **not** change

`registry_name` never reaches `NodeId`:

- `run_method/macros/face_registration.rs:45-49` builds the face identity from
  package, source, and **kind** only:
  ```rust
  pub const NODE_ID: $crate::NodeId = $crate::NodeId::from_namespaced_path(
      env!("CARGO_PKG_NAME"),
      $source,
      stringify!($kind),
  );
  ```
  `registry_name` is a *different* field two dozen lines later (`:65`).
- `NodeId::from_namespaced_path` hashes `(namespace, relative_path, declared_name)`
  (`core/.../identity/node_id.rs:45-57`); no `registry_name` input.
- The build side and the cache agree: `package_node_id(relative, declared_name)` /
  `package_node_id(&relative, &kind)` (`build_method/identity.rs:62-64`,
  `faces.rs:47-51`, `cache.rs:127-132`), and `StableFaceId` uses `stable_name` else `kind`
  (`registration.rs:234-258`).

So B1's kind-only `registry_name` change cannot move a `NodeId`, a cache key, a
`StaticPlan` face, or a pruning probe. The real blast radius is **logical path strings and
ordering**, which is exactly what `registry_name` feeds:
- `Registry::path_for` and every diagnostic path (`query.rs:80-88`, `connector.rs:43,214`,
  `inspection.rs:109`);
- `resolve_cut_targets` sorts siblings by `registry_name`
  (`core/.../tree/graft_ops/resolution.rs:78-84`) — the roadmap already lists this as an
  open uncertainty;
- Studio display and graft-selector naming (`studio/.../app/graft.rs:36`:
  `format!("{}_graft", info.registry_name)`), search, and the authoring patch fields.
For the record: the deleted arm's `stringify!($kind)` (`KindOnlyFace`) vs the surviving
arm's `last_path_segment(module_path!())` (`kind_only`) is a path-spelling change, not an
identity change. `run_method/tests/kind_only_registry_name.rs` pins the surviving value.

### B3a deletion re-verification (beyond grep)

Method: name-grep (method names catch `.method()` call sites; only `get`/`new`/`path`/
`is_empty` were ambiguous, so those were resolved by receiver type), read of the defining
modules to confirm private fields / `pub(super)` types, and green `cargo test -p` on every
crate that could touch them.

| Deleted item | Zero-caller? | Equivalent retained? | Host-facing? |
| --- | --- | --- | --- |
| `RegistryIndex::id_for_path` | yes (only `index.rs` defines it) | **no** public by-path→id lookup | plausible (unique capability) |
| `RegistryIndex::path_for_id` | yes | yes, `Registry::path_for` | low |
| `RegistryIndex::ids_for_kind` | yes | `Registry::find_kind` (snapshots, not ids) | low |
| `RegistryIndex::ids_for_stable` | yes | **no** public stable-name→id lookup | plausible (unique capability) |
| `RegistryIndex::is_empty` | yes (only `scale_audit.rs:108` uses `len`) | `len()` kept | low |
| 8 `RegistryError` readers | yes | **no** reader for `node`/`path`/`source` | **high** (F5) |
| `StaticPlan::new` | yes | `with_grafts(faces, &[])` | medium |
| `CutTarget::path` | yes (`.id()` is the one used, in `examples/control-button/tests/registry.rs:250,261`; `describe()` retained) | `describe()` | medium |
| `accepts_with_catalog` | yes | `decision(plugin, Some(catalog))` retained (note: no in-workspace caller of the `Some` branch remains) | medium |
| `requires_isolation` | yes (no textual trace left) | none obvious (match on `PluginSource`/`PluginMode`) | plausible, semantics unknown |
| `ProductionPolicyNotStrict` | yes (no textual trace) | `PluginTrustPolicy::open()/official()` | plausible, semantics unknown |
| 3 `PluginArtifact` verify wrappers | yes (`verify_artifact` is the only impl method left at `artifact.rs:55-73`) | `verify_artifact` | medium |
| `Registry::get` | yes (`RegisteredEntry`/`entry_at` are `pub(super)`, so `get` could not have returned the entry; `find`/`registry` remain) | `find` (snapshot), `registry` | **plausible, name is the natural one** |
| 5 metadata readers (`namespace`/`name`/`registration_rule`/`dependency_admission_accepts`/`allowed_dependency_paths`) | yes | `framework()`, `path()`, `id()` only; `RegistryHeader` is `pub(super)` with `pub(super)` fields (`header.rs:9-21`) | **high for `namespace`/`name`** |
| `tree_outline` | yes | `dump()` | medium |
| `edit_module` | yes; `edit_module_face` retained; `compile_fail,E0433` doctest passes under `--features authoring` | `edit_module_face` | **high (documented authoring verb)** |
| `debug_method` `collect!` + duplicate `pub use inventory` | yes (`inventory::collect!(CollectedRegistration)` at `lib.rs:27` is the crate macro and must stay; only one `pub use inventory` remains, `lib.rs:19`) | `submit!`/`registrations!` | low |
| `OwnedFlowContract::matches` (B2) | yes (only `str::matches` survives) | `semantically_compatible_with` | medium |

`edit_module` deserves a note: it is the only deletion justified by a *behavioral* argument
(it never wrote/removed the rule source), and the roadmap explicitly weighs whether it is
1.0 public API (B0 #3 / B3a). It remains a plausible host-facing verb. Same for
`Registry::get` and `Registry::namespace()`/`name()`, whose natural names a downstream
author would reach for first.

---

## Proposed tests

All are proposals, as text only (no file was created).

### 1. `run_method/tests/face_arm_defaults.rs` — pins F1 and F4

```rust
//! The default `preset`/`parts` *names* must not depend on whether a `handle:` was written.
//! 默认 preset/parts 记录名不得因是否写了 `handle:` 而不同。

mod with_handle {
    // No `preset:` / `parts:`: arm 175 (custom handle, default preset/parts).
    xirang_run_method::__control_object! {
        collector: development,
        kind: DefaultsWithHandle,
        handle: DefaultsWithHandle,
    }
}

mod without_handle {
    // Same omission, but arm 239 (no handle).
    xirang_run_method::__control_object! {
        collector: development,
        kind: DefaultsWithoutHandle,
    }
}

#[test]
fn omitted_preset_and_parts_record_the_same_defaults_in_both_arms() {
    assert_eq!(without_handle::REGISTRATION.preset, "NoPreset");
    assert_eq!(without_handle::REGISTRATION.parts, "NoParts");
    // Fails today: both read "$crate :: NoPreset" / "$crate :: NoParts".
    assert_eq!(with_handle::REGISTRATION.preset, "NoPreset");
    assert_eq!(with_handle::REGISTRATION.parts, "NoParts");
}

// F4: `preset`-only and `parts`-only with a `handle` currently fail to compile.
// Uncomment one at a time; each should compile after the ladder is fixed.
// mod preset_only_with_handle {
//     xirang_run_method::__control_object! {
//         collector: development,
//         kind: PresetOnly,
//         preset: super::ProbePreset,
//         handle: PresetOnly,
//     }
// }
```

### 2. `core/src/registry_core/lexicon/lexicon.rs` (`mod tests`) — executes the requested `path_is_under` probes

```rust
#[test]
fn the_requested_prefix_probes() {
    // Empty path against empty prefix is the equality short-circuit.
    assert!(path_is_under("", ""));
    // An empty prefix is NOT a universal prefix: no '/' boundary follows "a".
    assert!(!path_is_under("a", ""));
    // Segment boundary, not starts_with.
    assert!(!path_is_under("ab", "a"));
    // The registration-module name itself is subtracted by the wrapper.
    assert!(!is_registration_path("registry_core"));
    // Trailing slashes and unicode stay segment-correct.
    assert!(path_is_under("a/", "a"));
    assert!(!path_is_under("a", "a/"));
    assert!(path_is_under("模块/子", "模块"));
    assert!(!path_is_under("模块x", "模块"));
    // Windows separator is not a boundary (documented limitation, consistent with the old spellings).
    assert!(!path_is_under("a\\b", "a"));
}
```

### 3. `core/src/registry_core/tree/connector/connector.rs` (`mod tests`) — pins F2

Build the ancestor-provider shape with a restrictive owner admission, then assert the
classification with and without equality. Sketch (uses the in-module
`pub(super) connector_error` and the transaction test helpers' pattern):

```rust
#[test]
fn an_ancestor_provider_is_not_reclassified_as_internal() {
    // namespace/root -> P (registry_name "p", provides "cap", admission allows only "allowed"),
    // P needs_registry -> RP (path "root/p"), L under RP requires ("cap" => P.kind).
    // Old connector: is_external = true  -> admission rejects "root/p" (connector error).
    // New path_is_under: is_external = false -> no error.
    let error = registry.connector_error().map(|e| e.to_string());
    assert!(
        error.as_deref().is_some_and(|e| e.contains("admission gate rejects")),
        "an ancestor provider used to be gated; got {error:?}"
    );
}
```

If the new behavior is the intended one, this test should instead assert `None` and the
change must be written into the changelog; either way the delta must be acknowledged.

### 4. `core/src/registry_core/plugin/contracts/contracts.rs` (`flow_label_tests`) — pins F3

```rust
#[test]
fn compatible_with_agrees_with_the_owned_semantic_entry_point() {
    let a = FlowContract::new(ContractId::new("render.v1"), 1, "LocalCoordinates", "CanvasFrame");
    let b = FlowContract::new(ContractId::new("render.v1"), 1, "local_coordinates", "CanvasFrame");
    // Today: false, while `OwnedFlowContract::semantically_compatible_with` is true.
    assert_eq!(
        a.compatible_with(b),
        OwnedFlowContract::from(a).semantically_compatible_with(&OwnedFlowContract::from(b)),
    );
}
```

### 5. `core/src/registry_core/diagnostic/error.rs` (`mod tests`) — pins F5

```rust
#[test]
fn an_error_still_exposes_the_facts_a_host_must_report() {
    let error = RegistryError::new(
        NodeId::from_raw([7; 16]),
        "root/control/button",
        SourceLocation { file: "button.rs", line: 3, column: 1, function: "Button" },
        "rejected",
    );
    // These do not compile today; the fields are private and the readers are gone.
    // assert_eq!(error.node(), NodeId::from_raw([7; 16]));
    // assert_eq!(error.path(), "root/control/button");
    // assert_eq!(error.source().function(), "Button");
}
```

---

## Unresolved

1. **`compatible_with`'s intended semantics.** The doc (`contracts.rs:178-190, 250-256`)
   and the test oracle (`:495-499`) contradict each other (F3). I cannot tell whether the
   pre-B2 compiled `compatible_with` was semantic (a silent behavior change) or literal
   (a stale doc), because that requires history and git is out of bounds. The introduced
   semantic path is reachable through `semantically_compatible_with`, so the runtime slot
   check (`slot.rs:108-116`) is unaffected either way.
2. **`is_registration_path`'s pre-merge boundary.** `lexicon.rs:87-102` groups
   `registry_core.rs` with `ui2` as "outside", and the tests pin that. If the old
   `is_registration_path` was the "bare `strip_prefix`" site, then
   `is_registration_path("registry_core.rs")` and `is_registration_path("registry_core_extra/…")`
   changed from `true` to `false`. The `.rs` file is still force-included by the
   *name* match (`SCOPE_ALWAYS_INCLUDED`), but a host module directory named
   `registry_core_extra` is no longer force-included. Which spelling was old could not be
   established without git.
3. **`assert_static_registration` is a third copy** of the registration/output checks
   (`release.rs:224-250`). It agrees with the shared core today; B2's "change one place"
   claim is not literally true while it exists.
4. **`Registry::get`'s old signature.** It could not have returned the `pub(super)`
   `RegisteredEntry`; whether it duplicated `find` or returned `RegistrationInfo` is
   unknowable post-deletion, so its host-facing weight is a judgement call.
5. **`requires_isolation` and `ProductionPolicyNotStrict`.** Both are gone without a
   textual trace anywhere in the repo, so their old semantics and whether an equivalent
   remains (`open()`? matching `PluginSource`?) cannot be checked.
6. **The three `PluginArtifact` verify wrappers.** Names and semantics are unknown;
   `verify_artifact` remains the only documented path.
7. **F1's provenance.** The `$crate::NoPreset` hardcode in arm 175 may predate the B1
   edit (the B1 evidence points only at arm 239). Either way it is live and shipped, and
   the B1 report's ladder-wide default claim is wrong.
8. **F2 direction.** Treating the owner's parent face as internal may be the intended
   fix; the roadmap does not record the change, so it should be either reverted for the
   connector or documented as a behavior change in the 1.0 changelog.
