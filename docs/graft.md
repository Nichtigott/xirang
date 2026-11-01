# Graft records and the runtime overlay

[简体中文](graft.zh-CN.md) | English

This page documents the third graft layer: the `.xirang/external-grafts/`
record, how the runtime turns it into an effective tree, and the precedence
rules and reports that decision produces. The READMEs describe the declaration
and the application; this file is the reference for the record.

## The three layers

Three different artifacts meet at a graft, and none of them copies source:

| Layer | Artifact | Owner | What it does |
| --- | --- | --- | --- |
| **Declaration** | `static_graft_plan!` at the host entry | build step | Names the slot so pruning keeps it alive and fills the release-time `StaticPlan`. Removes no code. |
| **Record** | `.xirang/external-grafts/<selector>/graft.plan` | Studio (authoring) and the runtime host | The authoring input the runtime can apply over the declaration. Not compiled. |
| **Application** | `Registry::overlay` / `overlay_static` / `overlay_recorded` | runtime host | Validates and returns a new effective tree, mutating neither the base registry nor the external registry. |

The declaration macro is exported by `xirang-toolchain`
(`xirang_toolchain::run_method::static_graft_plan!`); the kernel only parses the text the
macro stringifies. The application methods live in the kernel
(`xirang_kernel::Registry`).

None of the three layers copies source — which is the point, and also the limit: a record is
*runtime* input, so the external implementation has to exist, and be built, for as long as the
record is in place. **`xirang.apply {action: "promote"}` is the fourth step**, and it is the one
that does copy source: it rewrites the target face's declaration to carry the external
implementation's fields, retires the `cut(…)` entry that handed the slot over, and moves the record
directory to `.xirang/trash/external-grafts/`. Everything else in this document still describes
the **runtime** overlay; landing is how a slot stops needing one.

Three limits come with it, and each one is a refusal rather than a silent approximation:

- **Generated faces only.** The rewrite goes through the authoring executor, which only rewrites
  files it generated (`// generated-by=XiRang`). A hand-written face file is refused by name —
  landing it is a code decision, not a field edit.
- **Host-relative fields do not travel.** `admission` and a registration rule richer than `ANY`
  name paths *inside the declaring crate*; copying them would produce a declaration that compiles
  nowhere. They are refused by name.
- **`kind` is an identity input.** Landing it renames the marker type (the executor migrates it
  properly), so `NodeId = hash(namespace, source, name)` changes with it: records keyed by the old
  identity need re-identifying, and references to the old type in the host need a hand. The reply
  says both.

See `toolchain/src/mcp/apply_promote.rs` for the landing and
`toolchain/src/mcp/apply_promote/source.rs` for how the external declaration is read (through
the kernel's own face parser, so a hand-written `external_object!` and a generated `*_object!` are
read by one reader).

## Which file is the host entry

One entry decides both halves of a graft: pruning keeps the slots it names, and
the generated `BUILTIN_GRAFT_CUTS` table (plus the `graft_plan.tsv` audit text)
describes the same set. The build resolves it once, in this order:

1. `XIRANG_ENTRY`, when set — a relative path resolves against the package
   root, an absolute path is used as written. A value that does not name a file
   **fails the build** (`XIRANG_ENTRY names <path>, which is not a file`).
   Falling back instead is the bug this rule removes: pruning would follow the
   variable while the cut table described `main.rs`, so the runtime could hold a
   table the release never kept, or silently lose a slot declared only in the
   configured file.
2. the file declaring `application!(entry = …)`;
3. the file that calls `host!()` — a lib+bin host keeps a stub in `main.rs`, so
   the real call wins over Cargo's order;
4. Cargo's `main.rs`, then `lib.rs`. This step may name no file at all, which is
   a host without an entry, not an error.

The authoring surfaces (`xirang grafts`, Studio) resolve the entry the same
way, but report a problem as a message instead of failing a build.

## How a record reaches the overlay

A record is loaded and applied by the ungated runtime API in
`xirang-toolchain` (it is deliberately **not** behind the `authoring`
feature, so a host does not need `syn` merely to read a plan file):

```rust
use xirang_toolchain::run_method::{apply_recorded_grafts, Registry, StaticGraftCut};

// `declared` is the build-captured static plan and the arbiter of which slots
// stay alive; `external` is the linked external registry.
let overlay = apply_recorded_grafts(
    base,
    external,
    builtin_static_plan().grafts(),
    package_root,
)?;
let effective: Registry = overlay.effective;
```

- `graft_record_root(package_root)` is `.xirang/external-grafts/`;
- `load_graft_records(package_root)` returns every plan directory, readable or
  broken (`LoadedGraft`), sorted by selector; a missing directory is an empty
  list, not an error;
- `load_graft_record(package_root, selector)` reads one plan back, validating
  the selector before joining it so a record cannot escape the directory;
- `apply_recorded_grafts(...)` loads the records and calls the kernel's
  `Registry::overlay_recorded`, returning `GraftOverlay { effective, reports }`;
  an unreadable plan or a record the kernel refuses is an `Err`, naming every
  offending plan at once;
- `Registry::overlay_recorded(records, declared, external)` is the pure kernel
  method: reconciliation lives in the kernel, filesystem access does not.

A no-record call is a drop-in for `overlay_static`: untouched declarations are
passed through as their original typed cuts, and the reports list is empty
(pinned by `no_records_reproduces_overlay_static`).

## Record-to-overlay precedence

A record is the newest, most specific artifact, and the build never applies it.
That does **not** make it unconditionally final. The policy keys on the form of
the declaration:

| Declaration form | Record effect | Report |
| --- | --- | --- |
| String form `cut "root/control/button" graft "button_fast"` (`CutTarget::Path`) | The record wins: its `graft` **and** its `full` replace the declaration's | `DeclarationOverridden`, plus `GranularityOverridden` when `full` differs |
| Typed form `cut(crate::…::NODE_ID) graft(…)` (`CutTarget::Id`) | The declaration is final; the record is ignored | `TypedDeclarationKept` |
| The record's `graft` selector does not resolve uniquely in the external registry | The declaration is kept | `RecordSelectorUnresolved` |
| The record targets a slot no declaration keeps alive | The record is skipped | `UnkeptSlot` |

Why the split: "the record always wins" would let an unreviewed, machine-local
`.xirang/` file re-route shipped behaviour and defeat a linked,
compiler-resolved face. "The declaration always wins" would leave the record's
`graft` with no production reader at all. Splitting by declaration form keeps
the dynamic-by-name trust the string form already grants while refusing to
defeat the typed form.

Two more boundaries:

- `graft` and `full` move together. A record never produces "the record's
  implementation with the declaration's `full`"; the whole record is atomic
  (`granularity_is_overridden_as_one_atomic_record`).
- A range declaration that a record only partly covers is split per node: the
  overridden node takes the record and every other node keeps the declaration.
  The plan format has no range field, so a record can never create a range.

## Identity drift

A record stores both a `NodeId` (`target`) and a logical path (`target_path`).
The runtime applies at the slot's **current** path, re-derived from the stored
identity — never at the stored text. A stored path is written when the plan is
created, and a cut selector must match exactly, so using it directly would make
a record silently stop applying after a face is renamed.

| Stored identity resolves | Stored path resolves | Outcome |
| --- | --- | --- |
| yes | to the same face | clean; applied at the current path |
| yes | no (or to a different face) | applied by identity, reports `IdentityDrifted` |
| no | yes | applied by path, reports `IdentityDrifted` |
| yes | yes, but a **different** face | **fatal**: the record is contradictory |
| no | no | `UnkeptSlot`; the record is skipped |

The only fatal record condition is that contradictory identity-vs-path pair:
either selector would silently change which face the author meant.

## Reports, and which failures are fatal

`RecordReport` has six advisory variants. `apply_recorded_grafts` prints every one
of them before it returns: a graft that did not take effect as `warning:`, a
precedence decision as `note:`. The same items stay in `GraftOverlay` for a host
that routes evidence into its own log. A host that ignores the return value
therefore still sees a skipped graft, which is otherwise invisible by
construction — the source tree looks fine and the running binary simply never
applied it.

| Variant | Meaning |
| --- | --- |
| `DeclarationOverridden` | A string-form declaration was superseded by the record. |
| `TypedDeclarationKept` | A typed-form declaration stayed final; the record was ignored. |
| `RecordSelectorUnresolved` | The record's `graft` does not resolve in the external registry; the declaration was kept. |
| `GranularityOverridden` | The record changed `full` relative to the declaration. |
| `UnkeptSlot` | No declaration keeps the slot alive, or the slot is gone; the record was skipped. |
| `IdentityDrifted` | Identity and stored path disagreed; the record was applied at the live face. |

Everything above is **reported, not fatal**: the overlay proceeds and the other
records still apply. `UnkeptSlot` and `RecordSelectorUnresolved` stay here rather
than becoming errors because the runtime cannot tell a legitimate configuration
from a mistake — a declaration can be `#[cfg]`-gated off in this build, and a
record can be written before the build that declares it. The unambiguous half of
that class is refused at build time instead (see below).

Three things **are** fatal, because none of them has a legitimate reading:

- a plan file that does not parse — the apply fails and names every unreadable
  plan, so no record is applied (`an_unparseable_plan_fails_the_apply`);
- a record whose directory selector disagrees with the `graft` its plan names —
  two candidate implementations with no way to tell which the author meant
  (`record_tests::a_directory_that_disagrees_with_its_plan_is_refused`);
- a record whose identity and path name different faces
  (`record_tests::contradictory_identity_and_path_are_refused`).

Hard failures from the underlying overlay stay byte-identical to an ordinary
`overlay_static`: ambiguous or unknown selectors, flow-contract mismatch,
destination registration rule, admission and connector rejection, and duplicate
cuts.

## The build-time refusal

The build never applies a plan, so it cannot wait for the runtime to notice a
pruned slot. It refuses the build instead: when a plan exists whose target slot
**no** declaration could name, `xirang-toolchain` emits a build error
(`phase=static-plan`, pointing at the plan file) whose message carries the exact
clause to paste:

```text
external graft plan `<selector>` targets `<path>`, which no declaration in the host entry names; the release-time plan keeps no such slot alive, so the record could never take effect. Declare it in static_graft_plan!: cut "<path>" graft "<implementation>",
```

A declaration that carries a feature gate counts even when this build evaluates
the gate to false, because the gate is the author's business: a record for a slot
the current feature set compiles out is a configuration, not a mistake. That is
why the check reads the entry's full declaration list, while the generated cut
table reads only the enabled subset. The build does not apply the plan;
`xirang grafts` exposes the same decision on demand (see below).

## The record directory is runtime input

`.xirang/external-grafts/` is **runtime input**, not generated state. Because
a record may re-route a string-form declaration, a package that does not review
this directory can have shipped behaviour changed by a machine-local file.
Review it the way source is reviewed, and keep it out of untrusted checkouts.
Because of that same power, `apply_recorded_grafts` and `overlay_static` can
produce different trees for identical inputs; the example test
`a_record_moves_the_effective_tree_but_not_the_static_plan` pins that intended
divergence rather than treating it as a bug.

Writing a record whose slot no declaration names is allowed, in Studio and in the
runtime API both: "write the plan, then declare the slot" is a legitimate order.
The consequence is easy to miss, so it is stated in all three places: Studio
warns while you write (see below), the build refuses a plan whose slot no
declaration can name (see below), and the runtime prints
`warning: … record skipped` when it applies records. The record is still skipped rather than fatal — one stale record
must not stop the others — but no path leaves the skip unstated. `xirang grafts`
is the read-only way to see the same fact after the fact.

## Reading the layers from the CLI

| Command | What it reports |
| --- | --- |
| `xirang grafts [path] [--json]` | Every `.xirang/external-grafts/*/graft.plan`, with selector, target path, graft, `full`, and whether the host entry declares that slot. Read-only. |
| `xirang explain <node-id\|logical/path> [--path <dir>] [--json]` | One node's identity, build scope, pruning state, and the declared cuts that name it. |
| `xirang explain --overlay [--path <dir>] [--json]` | Every slot with its kept/pruned state and replacement, plus the plan rows. |

`explain --overlay` is explicitly a **static projection**
(`"kind": "static-projection"`): the CLI cannot link an arbitrary host's
`base_registry()` and `external_registry()`, so it reports the build's scope and
declared cuts — what the overlay is computed from — and does not fabricate a
`Registry` from parsed source. The live effective tree is the host-side port
`Registry::dump_effective(cuts, external)` (it calls `overlay_static` and then
`dump`); a host that holds both real registries calls it with
`builtin_static_plan().grafts()`.

## Ranges over siblings

A cut may name two endpoints instead of one: `cut "root/a1 to root/a3" graft
"replacement"` in the string form, or `cut(a to b)` in the typed form. The
overlay then replaces every node in the span.

- The span is the **contiguous run of siblings between the two endpoints, in
  `registry_name` order** — the same order the sibling listing and the outline
  use. It is not file order, registration order, or rendered tree order: a
  folder that lists `a3, a1, a2` still has `a1 to a3` cover exactly those three
  faces.
- Both endpoints must share one parent. A range that crosses parents is
  refused; there is no implicit widening to the nearest common ancestor.
- A range written from the later sibling to the earlier one is refused, with an
  error that names both endpoints and the rule, instead of being silently
  swapped. A swap would select the same set while hiding that the order which
  actually decides is `registry_name`, so the next range written under the other
  mental model would quietly cover the wrong faces.
- The build forces both endpoints live as graft slots, so the range's targets
  survive pruning and reach the runtime overlay.

## Typed cuts and string cuts

Both forms are accepted in the same declaration, but a cut names both sides the
same way (both Rust expressions or both strings):

```rust
// String form: names the slot by logical path and the implementation by
// selector name. Resolved dynamically at overlay time; needs no link.
xirang_toolchain::run_method::static_graft_plan!(FRAMEWORK,
    cut "root/control/button" graft "button_fast",
);

// Typed form: the compiler resolves the target face's `NODE_ID` and the
// external implementation's `NODE_ID`; the external crate must be linked, so a
// typed declaration is final against a record. A range is `cut(a to b)`; see
// the "Ranges over siblings" section above.
xirang_toolchain::run_method::static_graft_plan!(FRAMEWORK,
    cut(crate::control::object::button::NODE_ID)
        graft(control_button_graft::button_fast::NODE_ID),
);
```

Use the typed form when the implementation is a linked crate and you want the
compiler and editor to resolve the target, and when the declaration must not be
overridden by a local record. Use the string form when the implementation is
bound by name (a runtime plugin) or is not linked, or when the record is meant
to be able to re-route the cut. See `examples/control-button/src/lib.rs` for a
real typed declaration next to the string form it replaces.
