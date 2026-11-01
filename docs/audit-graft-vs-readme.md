# Graft feature vs. README design claims — read-only audit

Scope: the graft/overlay/cut/plugin claims in `README.md`, `README.zh-CN.md`,
`core/README*.md`, `build_method/README*.md`, `run_method/README*.md`,
`docs/migration.md`, `docs/migration.zh-CN.md`,
`docs/discussion-introduction.md`, `docs/discussion-introduction.en.md`, plus
the two related package READMEs (`studio/README.md`, `plugin-host/README.md`).
Verdicts are against the **behaviour a reader would expect**, not the existence
of a function.

No `.rs` file was edited and no git command was run. Evidence is quoted code.
Targeted read-only verification (all green, exit 0):

```
cargo test -p xirang-core        --offline graft          # 31 passed
cargo test -p xirang-build-method --offline graft          # 15 passed
cargo test -p xirang-run-method   --offline --test graft_record  # 4 passed
cargo test -p xirang-studio       --offline graft          # 9 passed
cargo test -p xirang-example-control-button --offline graft       # 11 passed
cargo test -p xirang-example-control-button --offline a_record_moves # 1 passed
```

## Claim table

| # | Claim | file:line | Verdict | Evidence |
| --- | --- | --- | --- | --- |
| C1 | Three layers: `static_graft_plan!` "is the **declaration** the build step reads: it keeps the named slot alive through pruning and fills the release-time static plan. It removes no code." | `README.md:557-559`; `README.zh-CN.md:517-518` | **MET** | `build_method/graft_view/query.rs:19-63` captures entry cuts; `build_method/renderer/pass.rs:63-100` emits `BUILTIN_GRAFT_CUTS`; `build_method/scope.rs:159-234` makes cut targets liveness roots. |
| C2 | `Registry::overlay` "is the **application**: it validates and returns the effective tree at runtime, leaving both the base registry and the external registry untouched." | `README.md:560-562`; `README.zh-CN.md:519` | **MET** | `core/.../tree/graft_ops/overlay.rs:95-121` (`overlay`/`overlay_static`), `:132` stages `let mut staged = self.clone()`; `external` is `&Registry` and only read (`:136-168`). |
| C3 | The record file "is not compiled and **nothing else applies it**; the screen reads it back to list, open, re-scope, and delete plans." | `README.md:563-565` | **NOT MET** | Runtime now applies it: `run_method/runtime/graft_record.rs:161-184` `apply_recorded_grafts` → `core/.../tree/graft_ops/record.rs:417-562` `overlay_recorded`. The CLI and build also read it (`cli/grafts.rs`, `cli/explain.rs:380`, `build_method/graft_plan_check.rs:49-75`). Only "not compiled" is true. |
| C4 | Same record claim in Chinese: "它不参与编译，**也没有别的消费者**" | `README.zh-CN.md:520-521` | **NOT MET** | Same as C3. |
| C5 | Studio keeps the three layers apart; `g` writes the record and never host source. | `studio/README.md:17-23` | **MET** | The three-layer description and "writes `.xirang/...`, never host source" are true (`studio/studio/app/graft.rs:81-126`, no `fs::copy`). (The *code comment* at `studio/studio/app/graft.rs:11-12` adds "consumed here and by nothing else", which C3 refutes — see S8.) |
| C6 | "A `.xirang/.../graft.plan` file is an authoring record, **not** a declaration the compiler sees and **not an overlay application**." | `docs/migration.md:179-181`; `docs/migration.zh-CN.md:95-96` | **PARTIAL** | "not a declaration the compiler sees" is true. "not an overlay application" is only true in the narrow sense that the file is not the `overlay` call; the document is now the **input** to one (`run_method/runtime/graft_record.rs:161-184` → `record.rs:417-562`), and the section never says so. The next sentence, "the format now has a reader", understates it. |
| C7 | "`overlay` returns a new effective Registry. Neither `base` nor `external` is modified." | `README.md:496-497`; `README.zh-CN.md:461`; `docs/migration.md:168-170` | **MET** | `overlay.rs:95-101` returns `RegistryResult<Self>`; mutation is on `staged` only; `overlay.rs:239-289` writes `staged`'s entry. Example test `graft_replaces_the_slot_and_leaves_both_trees_untouched`. |
| C8 | "If any cut fails its flow contract, destination parent rule, admission, or connector check, **none of the plan is published**." | `README.md:498-499`; `README.zh-CN.md:462-463`; `docs/migration.md:170-172` | **MET** | Sequential cuts mutate `staged` (overlay.rs:132), but any `?`/`return Err` discards it; connector check runs after all cuts (`overlay.rs:178-181`). Example test `graft_chain_rejects_atomically`. |
| C9 | "A normal cut replaces one node and keeps the base node's children." | `README.md:503-513`; `README.zh-CN.md:466-476`; `docs/migration.md:168` | **MET** | `overlay.rs:258-287` takes `let mut kept = entry.child.clone()`, reconfigures it with the replacement rule, and refuses if a kept child violates it. Example tests `graft_replaces_one_face_only`, `a_non_full_graft_does_not_install_a_rule_its_children_violate`. |
| C10 | "`full` explicitly drops the base subtree and uses the external subtree." | `README.md:515-524`; `README.zh-CN.md:478-487`; `docs/migration.md:168-169` | **MET** | `overlay.rs:239-257` adopts `external.entry_at(replacement.id)...child.clone()` and rebases; a replacement with no child registry drops base children. Example test `graft_replaces_a_whole_subtree_with_full`. |
| C11 | Static string form: `static_graft_plan!(FRAMEWORK, cut "root/control/button" graft "button_fast",)` and `base.overlay_static(builtin_static_plan().grafts(), external)`. | `README.md:481-494`; `README.zh-CN.md:446-459`; `docs/migration.md:161-165` | **MET** | `run_method/macros/entry.rs:60-66` (declaration macro) and `core/.../release/release.rs:105-131,175-190` (`StaticGraftCut::new`, `StaticPlan::grafts`). `core/.../syntax/entries/graft.rs:324-338` parses the quoted form. |
| C12 | Dynamic `graft_plan!` "only when an editor, command, or hot reload path must construct and modify a plan at runtime." | `README.md:537-538`; `README.zh-CN.md:499-500`; `run_method/README.md:14-15` | **MET** (macro) / **PARTIAL** (migration example) | `run_method/macros/entry.rs:80-119` implements it. But `docs/migration.md:161` and `docs/migration.zh-CN.md:79` write `xirang::graft_plan!`; the macro is `#[macro_export]` in `xirang-run-method`, so the canonical path is `xirang_run_method::graft_plan!` (as the README uses). |
| C13 | "A path range can target contiguous siblings under one parent Registry." | `README.md:540-548`; `README.zh-CN.md:502-510`; `docs/migration.md:169` | **PARTIAL** | Implemented and tested (`resolution.rs:42-99`; example `graft_replaces_a_contiguous_sibling_range`), and a cross-parent range is refused (`resolution.rs:73-75`). But "contiguous" is decided by **alphabetical `registry_name` order after sorting** (`resolution.rs:79-98`), not by tree/insertion order; the roadmap itself lists "`resolve_cut_targets` 按 `registry_name` 排序是否等价于'连续兄弟区间'的意图" as an open uncertainty (`docs/roadmap-1.0.md:230`). |
| C14 | The static macro "constructs no `Vec` or `String`; the builder writes its contents directly into the `StaticPlan`." | `README.md:477-479`; `README.zh-CN.md:443-444` | **MET** | `run_method/macros/entry.rs:61-65` expands to two `const _` items and `stringify!`, no collection. Build-time parsing does allocate `String`s, but the stored table is `&'static [StaticGraftCut]` (`renderer/pass.rs:63-100`). |
| C15 | Pruning: "it conservatively derives the faces needed by this crate — what the entry reaches plus the slot each `cut(` in `static_graft_plan!` names — and emits only those faces ... so a face nobody declared is not shipped (`XIRANG_SCOPE` widens that scope on purpose)" with full-tree fallback. | `README.md:591-603`; `README.zh-CN.md:542-549`; `build_method/README.md:35-37` | **PARTIAL** | Narrowing is real (`build_method/scope.rs:159-242`; `static_plan.rs:44-84`; tests `a_typed_cut_narrows_the_scope_to_the_declared_slot`, `the_declared_slots_define_the_build_time_scope`). Caveats: (a) an unplaceable/typed-unrecognized or root cut falls back to the **whole tree** (`scope.rs:199-228,273-282`), which the README does acknowledge at `:602-603`, so "not shipped" is not guaranteed; (b) the build's static-plan capture resolves the entry without `XIRANG_ENTRY` (`graft_view/query.rs:27-31`) while `SourceScope` honours it (`scope.rs:113-129`), so under that env var pruning and the captured plan can read different files. |
| C16 | Generated code stores checked topology as `&'static [StaticFace]` and grafts as `&'static [StaticGraftCut]`; `builtin_static_plan()` borrows that data; "no heap allocation, global constructor, inventory walk, or startup registration loop." | `README.md:617-621`; `README.zh-CN.md:560-563` | **MET** | `renderer/pass.rs:39-101` emits the two static slices and `builtin_static_plan()`; `release/release.rs:37-43,172-190` is `Copy` over slices. (Dev-only `registrations()` at `pass.rs:102-106` builds a `Vec`, but it is not the release read path.) |
| C17 | "`overlay_static` skips dynamic `GraftPlan` and selector-string allocation" but still validates once. | `README.md:498-501`; `README.zh-CN.md:463-464`; cost table `README.md:635-636`; `README.zh-CN.md:575-576` | **MET** | `overlay.rs:111-121` calls `overlay_cuts` with `GraftCutRef::static_cut` (`overlay.rs:63-77`); no `GraftPlan`/selector `String` is constructed. It still clones the tree and builds a `BTreeSet` (`overlay.rs:132-133,169-174`), so "allocates no plan" is the honest reading, not "allocates nothing". |
| C18 | Overlay validation gates on framework match, `FlowContract`, destination rule, admission, connectors. | `README.md:496-498`; `README.md:231-232`; `README.md:446-448`; `docs/migration.md:170-172` | **MET** | Framework: `overlay.rs:129-131`. Flow: `overlay.rs:195-209`. Destination rule: `overlay.rs:218-232`. Admission + connectors: the connector pass `overlay.rs:178-180` → `connector/connector.rs:285-296` ("the parent Registry admission gate rejects that external branch"). Example tests `graft_rejects_a_foreign_framework`, `graft_rejects_an_incompatible_flow_contract`. |
| C19 | "The diagnostic names the broken capability, consumer, candidate provider, source location, and phase." | `README.md:582-584`; `README.zh-CN.md:533-535` | **PARTIAL** | Connector/admission failures do name capability + consumer path + provider path + source (`connector.rs:244-296`); the phase is the top-level "registration connector rejected" (`connector.rs:201-216`). Contract/rule failures instead carry the expected/received contracts or rule failures and a synthetic `file: "<graft>"` location (`graft_ops.rs:289-302`, `overlay.rs:220-231`), and record reconciliation evidence (`RecordReport`) is a separate display-only type the READMEs never mention. |
| C20 | `GraftPlanDocument` parses/renders the record, "refuses an unknown version or key instead of guessing, and is the only place the layout is defined." | `docs/migration.md:181-184`; `docs/migration.zh-CN.md:96-98` | **MET** | `core/.../plugin/graft/document.rs:67-135` (version gate at `:95-97`, unknown key at `:125`), `:139-148` render. All readers go through it: `run_method/runtime/graft_record.rs:94,120`, `build_method/graft_plan_check.rs:64`, `cli/grafts.rs:22`. |
| C21 | `ExternalGraftPlanFile` "no longer exposes `target`, `graft`, `full`, and `root` as public fields; it carries the parsed `GraftPlanDocument` and the selector, and answers through `target()`, `target_path()`, `graft()`, `full()`, `plan_path()`, and `root`." | `docs/migration.md:187-190`; `docs/migration.zh-CN.md:101-103` | **PARTIAL** | `run_method/authoring/external_graft/plan.rs:38-65`: `target`/`graft`/`full` are indeed accessors only, but `pub root: PathBuf` is still a **public field**, and there is **no `root()` method** (grep for `pub fn root` finds none). The claim groups `root` with the removed fields and then lists it among method-style accessors. |
| C22 | `create_external_graft` "keeps its signature and now writes through the kernel document, so everything it writes reads back." | `docs/migration.md:193-194`; `docs/migration.zh-CN.md:105-106` | **MET** | `plan.rs:100-131` builds `GraftPlanDocument::new(...)`, writes `document.render()`, and returns the document. Test `a_created_plan_round_trips_is_listed_and_moves_to_trash`. |
| C23 | Studio `g` "never rewrites host source"; "There is no source-copy or source-replacement graft path." | `README.md:567-574`; `README.zh-CN.md:523-528`; `studio/README.md:17-19` | **MET** | `studio/studio/app/graft.rs:106-125` writes only via `create_external_graft`; no `fs::copy`/source write; `Registry::graft` no longer exists (grep finds no `pub fn graft` on `Registry`). Migration claim `docs/migration.md:174-175` matches. |
| C24 | `g` "checks the slot against the cuts the host entry declares" and prints the clause to paste. | `README.md:569-571`; `README.zh-CN.md:524-526`; `studio/README.md:20-21` | **PARTIAL** | It **displays** declared/absent/unknown and renders the clause (`studio/studio/app/graft.rs:227-258`; `ui/forms/graft.rs:66-160`), but `submit_graft` (`graft.rs:81-126`) does not refuse to write a plan for an undeclared slot; `state.declaration` is not consulted there. A reader could take "checks" for a gate. |
| C25 | `HotDeployment` "stages a validated graft and publishes it atomically, leaving the last healthy snapshot visible after a failure." | `plugin-host/README.md:8-9` | **MET** | `plugin-host/src/deployment.rs:45-68`: `health_check`, then `overlay(...)` with `?`, then `ArcSwap::store`; any failure returns before the store. |
| C26 | `core/README` module table: `tree` provides "graft overlay/replacement validation"; `plugin` provides "graft command parsing, slot validation". | `core/README.md:15-16`; `core/README.zh-CN.md:14-15` | **MET** | `overlay.rs`/`resolution.rs`/`graft_ops.rs` under `tree`; `plugin/graft/graft.rs:104-206` (`CutGraftCommand`) and `document.rs:186-203` (`validate_graft_selector`). |
| C27 | `run_method` "graft validation ... lives in the kernel"; declaration macros include `static_graft_plan!`, `graft_plan!`. | `run_method/README.md:5-15`; `run_method/README.zh-CN.md:5-14` | **MET** | Validation lives in `core` (`overlay.rs`, `record.rs`); macros in `run_method/macros/entry.rs`. The README omits the new ungated runtime record API (`apply_recorded_grafts`, `load_graft_records`, `LoadedGraft`, `GraftOverlay`) — see U8. |
| C28 | A `static_graft_plan!` plan is captured "whether or not the host also declares `application!(entry = ...)`". | `docs/migration.md:133-136`; `docs/migration.zh-CN.md:65` | **MET** | `graft_view/query.rs:27-31` falls back to `default_entry_source`; `entry.rs:22-48` picks the file that calls `host!()`. |
| C29 | "An extension only needs to satisfy the target registration rule. A replacement also needs an input/output contract that is semantically compatible with the slot." | `README.md:242-245`; `README.zh-CN.md:225-226` | **MET** | Registration rule on extension: `graft_ops.rs:151-162`; overlay additionally requires both declared (`overlay.rs:195-197`) and `semantically_compatible_with` (`overlay.rs:198-209`); the comparison itself is `core/.../plugin/contracts/contracts.rs:88,181` (id/version/input/output). |
| C30 | Records "coexist without turning upstream into a patch queue"; competing implementations remain separate crates. | `README.md:40-44`; `README.zh-CN.md:35-36`; `docs/discussion-introduction.md:18`; `docs/discussion-introduction.en.md:19` | **MET** | External registry is a separate `Registry`; example `examples/control-button-graft/` declares with `xirang_run_method::external_object!`. A bare selector matching two faces is refused (`resolution.rs:101-132`, `GraftError::AmbiguousReplacement`), which is the safe version of "coexist". |
| C31 | "The framework tree and third-party implementation remain in their own crates. The host only declares an overlay plan in its `main.rs` or `lib.rs`." | `README.md:457-459`; `README.zh-CN.md:424-425` | **MET** | Host declares `static_graft_plan!`/`graft_plan!`; no source of either tree is copied (`README.md:477-478`; verified no copy path). |
| C32 | `build_method` README: graft cut targets and plugin-declaring faces are forced liveness roots. | `build_method/README.md:35-37`; `build_method/README.zh-CN.md:30-31` | **MET** (with C15 caveat) | `scope.rs:159-242`. Plugin detection: `graft_view/matching.rs:74-81`. Fallback keeps the whole tree, which also keeps the slot. |
| C33 | Post-release plugin/graft: "`overlay_static` allocates no plan but still performs contract, admission, and connector validation once." | `README.md:636`; `README.zh-CN.md:576` | **MET** | One `overlay_cuts` call (`overlay.rs:111-121`); validations as C18. "Once" is per overlay call; no caching exists. **VERIFIED later (U2):** the static overlay is not zero-allocation — the tree clone and the visited-cut set are real — but it builds no `GraftPlan` and is cheaper than the dynamic spelling of the same cuts. Measured in `examples/control-button/tests/static_plan_allocations.rs`: 6 allocations/96 bytes with no cuts (the floor), 60/3 470 for one cut against the dynamic 77/4 053, and 107/5 628 for the example's two cuts against 138/6 438; the one-cut plan alone is 3/350. Numbers in `docs/performance-baseline.md`. |
| C34 | "The last two choices make the affected boundary visible instead of silently discarding data. A graft is accepted only after the chosen boundary contracts and the destination registration rule pass." | `README.md:446-448`; `README.zh-CN.md:415-416` | **MET** | `overlay.rs:195-232` (flow first, then `registration_rule.validate`). |
| C35 | Cost/binary claims: read-only topology has "No startup allocation"; the release plan has "no heap allocation, global constructor, inventory walk, or startup registration loop"; `overlay_static` "allocates no plan". | `README.md:617-621,631-637`; `README.zh-CN.md:560-563,573-577` | **VERIFIED later (U1 + U2)** | The read-only pass could only see the code shape (`release/release.rs:37-43,172-219` is `Copy` over slices; `renderer/pass.rs:39-101`; `overlay.rs:111-121`). Both halves are now measured. *Startup allocation:* `examples/control-button/tests/static_plan_allocations.rs` counts heap allocations with a global allocator and reads **0 allocations / 0 bytes** for `builtin_static_plan()` plus `faces`/`grafts`/`len`/`is_empty`, `find` (hit and miss), `children_of`, and a walk over every face; inserting a single `String::from` into `StaticPlan::find` makes it report 2 allocations/16 bytes, so the assertion is live. *Linked artifact:* `CARGO_NET_OFFLINE=true tools/xirang-release-audit` exits 0 and no artifact carries an `.inventory` section; `nm target/release/xirang` has **0** symbols matching `inventory`, and `.init_array` holds exactly two 8-byte entries — `std::sys::args::unix::imp::ARGV_INIT_ARRAY` and `__frame_dummy_init_array_entry` — neither of which is a registration constructor. `overlay_static` "allocates no plan" is C33's later verification. Numbers in `docs/performance-baseline.md`. |

## Not met or partial

Each item gives the smallest honest fix. "Docs" means the README/doc should be
corrected; "code" means the behaviour should change; "remove-the-claim" means
the sentence should go.

1. **C3/C4 — the record layer description is false; C6 is understated.** The
   READMEs say "nothing else applies it" / "没有别的消费者", but
   `run_method::apply_recorded_grafts` → `Registry::overlay_recorded` now applies
   the record; `docs/migration.md:179-181` calls it "not an overlay application"
   without saying it is the overlay's input. *Smallest honest fix: docs* —
   rewrite the record bullet to state that the record is runtime input consumed
   by `apply_recorded_grafts` (and by `xirang grafts` / `explain --overlay`),
   and that the build only warns about it. Do **not** remove the claim; the
   underlying feature is real.
2. **C13 — "contiguous siblings" is under-defined.** The code selects the span
   between two endpoints in sorted-`registry_name` order
   (`resolution.rs:79-98`). *Smallest honest fix: docs* — either define
   "contiguous" as "the logical-path span between the two endpoints under one
   parent" (current behaviour), or make the code use tree order. The roadmap
   already flags this as unconfirmed (`docs/roadmap-1.0.md:230`), so docs is the
   cheap fix; a code change needs a decision first.
3. **C15 — "a face nobody declared is not shipped" has fallback holes and an
   entry-resolution split.** An unplaceable/root/typed-unrecognized cut keeps
   the whole tree (`scope.rs:199-228,273-282`), and `XIRANG_ENTRY` is honoured
   by `SourceScope` but ignored by the build's plan capture
   (`scope.rs:113-129` vs `graft_view/query.rs:27-31`). *Smallest honest fix:
   code* — pass the same resolved entry into `host_graft_entries` (or honour the
   env var there) so pruning and the emitted `StaticGraftCut` table can never
   describe different files; *docs* — add the fallback caveat next to the
   sentence. The README's own `:602-603` fallback sentence partially covers the
   first hole but not the entry split.
4. **C19 — "the diagnostic names the broken capability, consumer, candidate
   provider, source location, and phase" is only true for the connector class.**
   Contract/rule rejections carry expected/received values with a `<graft>`
   location. *Smallest honest fix: docs* — scope the sentence to connector and
   admission failures, or broaden the error payloads (code).
5. **C21 — the migration claim about `ExternalGraftPlanFile::root` is
   self-contradictory.** `pub root: PathBuf` remains
   (`external_graft/plan.rs:39-43`) and there is no `root()` method.
   *Smallest honest fix: docs* — say "`target()`, `target_path()`, `graft()`,
   `full()`, `plan_path()`, and the public `root` field".
6. **C24 — Studio "checks" the declared slot but does not gate on it.**
   `submit_graft` writes the plan even when `GraftDeclaration::Absent`
   (`studio/app/graft.rs:81-126`; the warning is only drawn in
   `ui/forms/graft.rs:85-94`). *Smallest honest fix: docs* — call it a warning /
   pre-flight report, not a check, or make `s` refuse when absent (code).
7. **C12 — the migration example's macro path is wrong.** `docs/migration.md:161`
   and `docs/migration.zh-CN.md:79` use `xirang::graft_plan!`; the macro is
   exported from `xirang-run-method`. *Smallest honest fix: docs* — write
   `xirang_run_method::graft_plan!`. (Same class: `docs/migration.md:19` and
   `docs/migration.zh-CN.md:13` say `xirang::external_object!`, which is also a
   `xirang_run_method` macro — see S2.)
8. **C14-adjacent, C17, C16, C33 — "zero allocation" claims are narrow but
   honest; no change required.** `overlay_static` still clones the tree, builds a
   `BTreeSet`, and `resolve_node` collects a `Vec` (`overlay.rs:132-133,136-148`;
   `resolution.rs:111-119`). The README already scopes the word "plan"
   ("allocates no plan"), so this is a caution for readers, not a defect.

## Undocumented behaviour

Reverse direction: implemented graft behaviour that no README describes.
Recommendation is **document** unless noted.

- **U1 — record→overlay precedence policy.** Documented only in code
  (`core/.../graft_ops/record.rs:356-416`): a record overrides a **string-form**
  declaration and reports `DeclarationOverridden`; a **typed-form** declaration
  (`CutTarget::Id`) is final (`TypedDeclarationKept`); a record selector the
  external registry cannot resolve falls back to the declaration
  (`RecordSelectorUnresolved`); `full` is taken atomically with `graft` and a
  disagreement reports `GranularityOverridden`; a range declaration only partly
  covered by a record is split per node and the record cannot create a range
  (`record.rs:510-549`). Pinned by `a_string_declaration_yields_to_the_record`,
  `a_typed_declaration_stays_final`,
  `an_unresolved_record_selector_falls_back_to_the_declaration`,
  `granularity_is_overridden_as_one_atomic_record`, and the example test
  `a_record_moves_the_effective_tree_but_not_the_static_plan`. **Document**, and
  state the security boundary that `.xirang/` is machine-local runtime input
  (see U7).
- **U2 — `RecordReport` variants and which are fatal.** Seven advisory variants
  (`record.rs:64-112`); the only fatal record condition is a contradictory
  `target` vs `target_path` (`record.rs:328-345`), while a slot no declaration
  keeps alive is reported and skipped (`record.rs:346-352,446-452`). Overlay hard
  failures (ambiguous/unknown selector, contract, rule, connector, duplicate cut)
  stay fatal. **Document**.
- **U3 — identity-drift reconciliation.** A stored `target_path` that no longer
  matches is re-derived from the stored `NodeId` via `path_for`
  (`record.rs:283-354,564-567`); the stored text is used only when the identity
  no longer resolves, and then `IdentityDrifted` is reported. **Document** — it
  is the difference between "rename a face and the record follows" and "rename a
  face and the record silently stops applying".
- **U4 — build-time `cargo:warning` for an undeclared plan.** `graft_plan_check.rs:99-121`
  + `pipeline.rs:37-44,68-72` warn when no declaration names a plan's slot.
  Neither `build_method/README.md` nor the root README mentions it. **Document**.
- **U5 — `xirang grafts` and `explain --overlay`.** Implemented
  (`cli/grafts.rs:30-120`; `cli/explain.rs:371-456`) and in `--help`
  (`cli/lib.rs:22-24,34-38`), but absent from `README.md:697-711` and from
  `cli/README.md:13-19`. `explain --overlay` is explicitly labelled
  `"kind": "static-projection"` (`explain.rs:447-455`) because the CLI cannot
  link an arbitrary host's `base_registry`/`external_registry`; the host-side
  escape hatch is `Registry::dump_effective` (`core/.../tree/inspection/inspection.rs:119-125`).
  **Document** both.
- **U6 — the typed graft cut form.** `cut(expr)` / `cut(expr to expr)` /
  `graft(expr)` is implemented (`syntax/entries/graft.rs:145-154,245-249`,
  `renderer/pass.rs:75-96`, `StaticGraftCut::from_ids`/`from_id_range`), and the
  precedence policy turns on it (U1), yet no README shows or explains it. The
  only trace is the phrase "the slot each `cut(` in `static_graft_plan!` names"
  (`README.md:598`). **Document**.
- **U7 — `.xirang/external-grafts/` is a runtime trust boundary.** Because a
  record can re-route a string declaration, `apply_recorded_grafts` and
  `overlay_static` can produce different trees for identical inputs; the code
  requires the directory to be reviewed like source and kept out of untrusted
  checkouts (`run_method/runtime/graft_record.rs:147-160`). **Document** —
  this is the one place where the graft record changes shipped behaviour.
- **U8 — the ungated runtime record API.** `apply_recorded_grafts`,
  `load_graft_records`, `load_graft_record`, `graft_record_root`, `LoadedGraft`,
  `GraftOverlay`, and the re-exported `RecordReport`/`RecordedGraft`/
  `ResolvedRecord` (`run_method/runtime/graft_record.rs:29-184`) are public
  and deliberately not behind `authoring`, but `run_method/README.md:10-19` does
  not mention them. **Document**. Note also that no in-repo host or example
  *binary* calls `apply_recorded_grafts` — only unit/integration/example tests
  do; wiring it into `examples/control-button` would make the feature
  observable.
- **U9 — unreadable plans are evidence, not blockers.** A plan that fails to
  parse lands in `GraftOverlay::unreadable` while the rest apply
  (`graft_record.rs:53-57,161-184`; test
  `an_unparseable_plan_does_not_block_the_others`). **Document**.
- **U10 — `Registry::dump_effective`.** Public host-side effective-tree dump
  (`inspection.rs:119-125`), not mentioned in any README. **Document**.

No undocumented graft behaviour looks like it should be **removed**. The one
candidate for removal is the stale English comment in
`studio/studio/app/graft.rs:11-12` ("consumed here and by nothing else"),
which is a comment fix, not a feature.

## Stale claims

Claims that still assert pre-change behaviour:

1. **Record is inert — `README.md:563-565`**: "`.xirang/external-grafts/<selector>/graft.plan`
   is the **record** the screen writes. It is not compiled and nothing else
   applies it; the screen reads it back to list, open, re-scope, and delete
   plans." Runtime now applies it (C3).
2. **Record is inert (zh) — `README.zh-CN.md:520-521`**: "它不参与编译，也没有别的
   消费者" (not compiled, and no other consumer). Same defect.
3. **Record is not an application — `docs/migration.md:179-181`** and
   `docs/migration.zh-CN.md:95-96`: "not a declaration the compiler sees and not
   an overlay application." The narrow statement is defensible, but the section
   never says the record is now the **input** to `overlay_recorded` (the same
   change that makes S1/S2 false). Update it while fixing S1.
4. **External face requires many fields — `docs/migration.md:19-25`**:
   "A linked declaration must include plugin provenance, version, target
   framework, and a parent node identity." After B3b the external macro requires
   only `kind`; `plugin`, `parent`, rule, contracts, etc. all default
   (`run_method/macros/face_external.rs:12-26,112-147`;
   `docs/roadmap-1.0.md:18,177`). `docs/migration.zh-CN.md:13` repeats it:
   "必须填写插件来源、版本、目标框架和父节点身份".
5. **`ExternalGraftPlanFile::root` removed — `docs/migration.md:187-190`** and
   `docs/migration.zh-CN.md:101-103`: groups `root` with `target`/`graft`/`full`
   as no-longer-public and then lists `root` among accessors. `root` is still
   `pub` and there is no `root()` method (C21).
6. **`CutTarget::Path` still encodes a range — code comment
   `core/src/registry_core/release/release.rs:48-49`**: "`Path` is the
   human-written selector (`root/control/button`, or a `a to b` range)". After
   B3c-1 the far endpoint is `StaticGraftCut::cut_end`; a path is never a range.
   No README asserts the old encoding (the README range example at
   `README.md:545` is the still-valid *command* string form).
7. **`xirang::graft_plan!` / `xirang::external_object!` — `docs/migration.md:19,161`**
   and `docs/migration.zh-CN.md:13,79`. These macros are exported by
   `xirang-run-method`; the README itself uses
   `xirang_run_method::static_graft_plan!`. Same class as C12.
8. **Studio comment — `studio/studio/app/graft.rs:11-12`**: the record "is
   consumed here and by nothing else." Stale (C3). (Comment, not README.)

Step-4 checklist — for each named recent change, whether a README still asserts
the old behaviour:

- **`entry` module removal.** No README asserts a `run_method` `entry` module.
  `run_method/README.md:10-19` lists macros and the authoring executor only;
  `build_method/README.md:25-33` and `docs/migration.md:133-136` describe the
  entry as `host!()`/`application!`. **No stale README claim.**
- **External face now requiring only `kind`.** Stale in `docs/migration.md:19-25`
  and `docs/migration.zh-CN.md:13` (quoted above). **Stale claim found.**
- **Range end no longer encoded as `"a to b"`.** No README asserts the internal
  encoding. The remaining stale assertion is the code doc
  `core/.../release/release.rs:48-49` and the historical prose in
  `docs/audit-2026-09-21.md:113`. **No stale README claim.**
- **`LIVE SAMPLE` trace label.** No README advertises a bare `LIVE`.
  `README.md:720-722` already says "Studio currently renders a built-in sample
  trace in its data panel and has no trace-ingest path yet"; the label is
  `"  LIVE SAMPLE  "` (`studio/studio/ui/panels.rs:17-22`) and the trace is
  `sample_live_trace()` (`studio/studio/app/lifecycle.rs:46-57`). **No stale
  README claim** (the doc is already synced).
- **CLI cannot rebuild an arbitrary host's `Registry`.** No README claims the
  CLI can dump an effective tree. The limitation and the `dump_effective` host
  port live only in `docs/roadmap-1.0.md:24` and the `explain` code. **No stale
  README claim**; the gap is the *missing* documentation of `explain --overlay`
  (U5), not a false one.

## Verdict

The core graft model does what its READMEs say: a declaration (`static_graft_plan!`)
is captured by the build into a zero-allocation `StaticPlan`, its cut targets are
forced liveness roots, `cut`/`full`/string/typed/range selectors are resolved and
applied as an immutable overlay that never mutates the base or external trees,
and framework, flow-contract, destination-rule, admission, and connector checks
gate publication atomically. The examples and the targeted tests back every one
of those behaviours (C1, C2, C7–C11, C14–C18, C22, C23, C25, C31–C34). The
feature does **not** yet meet its documented design on the third layer: the
READMEs still describe `.xirang/external-grafts/<selector>/graft.plan` as an
inert authoring record that "nothing else applies it", while the code now feeds
it into `Registry::overlay_recorded` with a deliberate, code-only precedence
policy (record beats a string declaration, typed declaration is final,
unresolved selector falls back), an identity-drift reconciliation rule, seven
advisory `RecordReport` variants, and a build `cargo:warning` — none of which
appear in any README (C3, C4, C6, U1–U4). Additional documentation gaps are the typed
cut form (U6), the `xirang grafts` / `explain --overlay` surfaces and the fact
that the CLI cannot rebuild an arbitrary host's Registry (U5), the runtime trust
boundary of the record directory (U7), and the ungated runtime record API that
no host binary currently calls (U8). Three claims are outright false or
misleading beyond the record layer: the migration note that an external
declaration must carry provenance/version/target framework/parent (it needs only
`kind`, S4), the claim that `ExternalGraftPlanFile` no longer exposes `root` (it
still does, and has no `root()` accessor, C21/S5), and the implication that
Studio's declared-slot "check" gates plan writing (it only warns, C24). Two
behaviours need an owner decision rather than a doc edit: whether a sibling
range is defined by sorted `registry_name`/logical-path span or by tree order
(the roadmap's own open question), and the `XIRANG_ENTRY` split where pruning
and the captured static plan can read different entry files (C13, C15). The gap
to 1.0 is therefore not the graft engine — it is documentation truth: bring the
root/en/zh READMEs, `studio/README.md`, `migration*.md`, `cli/README.md`, and
`run_method/README.md` in line with the record→overlay layer (including its
precedence and fatal-vs-reporting rules), document the typed form and the two new
CLI commands, and either wire `apply_recorded_grafts` into a real example host or
state plainly that it is a library port the host must call; after that the open
range-order and entry-resolution questions are the only remaining behavioural
unknowns before the documented design and the implementation agree.
