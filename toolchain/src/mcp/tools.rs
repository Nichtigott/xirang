//! Tool catalog, the query implementations, and the write path's dispatch.
//! 工具目录、查询实现与写入路径的分派。
//!
//! The catalog below is the authority on how many tools there are and what each one reads. This
//! paragraph describes the *families* instead of counting them, because a count here goes stale the
//! moment a tool is added — it said "five + eight, plus `apply`" while the catalog held seventeen
//! (audit `L4`).
//! 下面的目录才是"有几个工具、每个读什么"的权威。这一段描述的是**几类**而不是数个数，因为这里
//! 的数字会在新加一个工具的那一刻过期——它曾说"五个 + 八个，外加 `apply`"，而目录里其实有十七个
//! （审计 `L4`）。
//!
//! One family reads Rust source text — `xirang.search` also matches registry faces
//! (logical path, kind, module, registry_name) and annotates each with the build's verdict.
//! Another answers from evidence that is not
//! source text: `xirang.registry` derives the tree the build derives
//! (`crate::build_method::face_views`), `xirang.explain` reads the files the
//! build *published* (`target/xirang/out`) for scope and release pruning,
//! `xirang.diff` states the face-level delta between those two sides, and
//! `xirang.trace` reads a recorded trace artifact, refusing one that describes
//! another tree; `xirang.mir` reads a `-Zunpretty=mir` dump or a JSONL artifact
//! and can emit that JSONL, which nothing in the workspace ever wrote; and
//! `xirang.unified` merges the two, where a live call confirms a compiler
//! candidate. Three tools write, and all three preview before they do:
//! `xirang.apply` edits faces through the authoring executor, `xirang.new_project`
//! scaffolds a host with `build_method::scaffold::create_project`, and `xirang.plugin`
//! stores one plugin lock record through the kernel's `PluginCatalog` gate
//! (`apply.rs`, `new_project.rs` and `plugin.rs` explain each contract). What none of
//! them reports is contract,
//! admission, or registration-rule data: those live in the built
//! `RegistrationSnapshot`s, which need the compiled registrations rather than a
//! scan or a manifest.
//! 一类工具读取 Rust 源码文本——`xirang.search` 还会匹配注册面（逻辑路径、kind、module、registry_name），
//! 并把构建的判断标在每一条上。另一类用非源码文本的证据作答：`xirang.registry` 推导出构建所推导
//! 的那棵树（`crate::build_method::face_views`）；`xirang.explain` 读构建**发布**的文件
//! （`target/xirang/out`），回答作用域与发布剪枝；`xirang.diff` 说出两侧的面级差异；
//! `xirang.trace` 读取已记录的 trace artifact，并拒绝描述另一棵树的那份；`xirang.mir` 读
//! `-Zunpretty=mir` 转储或 JSONL artifact，并能输出那份无人写过的 JSONL；`xirang.unified` 把两者
//! 合并，真实调用在其中确认编译器候选。三个工具会写入，而三者都先预览再写：`xirang.apply`
//! 经 authoring 执行器编辑注册面，`xirang.new_project` 用
//! `build_method::scaffold::create_project` 脚手架出一个宿主，`xirang.plugin` 经内核的
//! `PluginCatalog` 闸门存下一条插件锁记录（三份契约分别见 `apply.rs`、`new_project.rs` 与
//! `plugin.rs`）。它们都没有报告的是 contract、admission 与 registration rule 数据：那些住在已构建的
//! `RegistrationSnapshot` 里，需要已编译的注册，而不是扫描或清单。
//!
//! online: the dispatch layer answers for trees that were never built as well as for built ones, and a face added since the build is only in the sources.

#[path = "catalogue.rs"]
mod catalogue;
use catalogue::advertised_schema;

use serde_json::{Value, json};
use std::path::Path;

use crate::mcp::adopted::adopted;
use crate::mcp::affected::affected;
use crate::mcp::apply::apply;
use crate::mcp::build_evidence::explain;
use crate::mcp::callgraph::callgraph;
use crate::mcp::check_jobs::check;
use crate::mcp::converge::converge;
use crate::mcp::diff::diff;
use crate::mcp::grafts::grafts;
use crate::mcp::impact::impact;
use crate::mcp::mir::{mir, unified};
use crate::mcp::multilang_tool::multilang;
use crate::mcp::new_project::new_project;
use crate::mcp::overlay::overlay;
use crate::mcp::plugin::plugin;
use crate::mcp::protocol::{error_response, success};
use crate::mcp::read::read_source;
use crate::mcp::registry::registry_brief;
use crate::mcp::search::search;
use crate::mcp::source_index::{load_one, load_sources, required_path, resolve_root};
use crate::mcp::trace::trace;
use crate::mcp::usages::usages;
use crate::mcp::verify::verify;

#[path = "tool_metadata.rs"]
mod metadata;
use metadata::tool;

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "xirang.explore",
            crate::mcp::explore::DESCRIPTION,
            crate::mcp::explore::schema(),
        ),
        tool(
            "xirang.search",
            "Find registration faces, source files, and Rust function declarations by name. Faces \
             come first and match on logical path, kind, module, or registry_name — the spellings the \
             other tools use — and each carries the build's verdict: `ok`, `added since build`, \
             `re-identified` (a `kind` change under an unmoved file, with both identities), or \
             `build unknown` when nothing has been published. A manifest that no longer describes \
             these sources is announced above the verdicts as `stale (run xirang check)`, because \
             they then describe the tree that build saw rather than this one. A registration file \
             the derivation could not parse is counted as `unparsable faces N` above the hits, and \
             that line is **not** subject to `limit`: it reports the tree that was read rather \
             than one more result row, so it keeps appearing when the limit has already cut the \
             face list short. Below them the file \
             and function hits are unchanged. The tree half needs the identity namespace; a root \
             Cargo cannot name still answers the source half and says the tree half is unavailable. \
             A virtual workspace root is a root too, and it is answered as the workspace it is: \
             every member appears with its status (`published`, `not built`, `no faces`, or \
             `unresolvable` with the reason), each member's matching faces are grouped under it \
             with a line naming which tree those rows came from — a face is matched on facts the \
             published record does not carry, so a member's own records are read first and the \
             sources are derived when they cannot answer — and a member whose tree cannot be \
             derived is named rather than hidden behind the file hits. With `converge: true` the answer is layered — the tree's verdicts, then the call chain the name leads into, then the next step and the bounds of what was **not** looked at — so one call localizes instead of three. `query` matches **names** — faces, files, functions — while `literal` matches **text** anywhere in a source file: raw bytes, case-sensitive, comments and string literals included. The literal mode is what a failing assertion's message usually needs (find where that message is produced), and it replaces leaving the tool to grep for it. Pass one of the two, not both.",
            json!({"type":"object","properties":{"names":{"type":"array","items":{"type":"string"},"description":"several bare names in one call: each gets its own group, answered by the same path a single name takes; `--names a,b` on the command line, or a JSON array in `--json`"},"query":{"type":"string","description":"a name: face, file or function"},"literal":{"type":"string","description":"text to find verbatim anywhere in a source file (raw bytes, case-sensitive, comments and string literals included)"},"context":{"type":"integer","minimum":0,"maximum":10,"description":"literal mode: also print this many lines around each hit (default 0; the reply points at it)"},"converge":{"type":"boolean","description":"answer in layers: the tree's verdicts, then the call chain this name leads into, then the next step and the bounds of what was looked at"},"root":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}},"anyOf":[{"required":["query"]},{"required":["names"]},{"required":["literal"]}]}),
        ),
        tool(
            "xirang.digest",
            "One bounded summary of one file, for the case the maintainer named: a file holding \
             several algorithms where one branch of one of them is wrong. Each row gives a \
             function's line range, how many names it calls, how many callers it has, whether a test \
             names it, and the first line of the contract above it. It does **not** read the file \
             out in full, and it says so: which branches are dead or covered and whether the static \
             walk reaches a function are the census's columns (`check {face}`), and a body is \
             `read {path, line}`'s.",
            json!({"type":"object","properties":{"file":{"type":"string","description":"a path as this root sees it (e.g. `src/model/entry.rs`)"},"root":{"type":"string"}},"required":["file"]}),
        ),
        tool(
            "xirang.conformance",
            "Say what an anchor's claim is right now: which ledger revision is in force (an \
             adoption is a lease, so the newest line wins), whether it still holds or lapsed at a \
             named file, and which files it covers. It answers the half of \"was this design carried \
             through?\" that the ledger itself can prove, and it names the half it does not answer — \
             whether the siblings follow the same shape (that is `consistency --specimen <anchor>`) \
             and the declared fields a specimen carries (those live in its own source, and \
             `consistency --specimen` reads them from there).",
            // No `required`: without an anchor the tool lists **every** anchor, one line each. The
            // schema used to demand it, and the client's `keys:` line derives its `*` marks from
            // `required` — so every reader was told `anchor*` about a call that no longer needs it,
            // which is a self-description that disagrees with the behaviour (found in the round-8
            // review, which checked the acceptance shape against the schema).
            // 不再 `required`：不带 anchor 时该工具**逐行列出每个锚点**。schema 以前要求它，而客户端的 `keys:`
            // 行正是从 `required` 推出那些 `*` 的——于是每个读者都被告诉 `anchor*`，而那次调用早已不需要它；
            // 这是**自述与行为不一致**（第八轮复核对着 schema 核接受形状时发现）。
            json!({"type":"object","properties":{"anchor":{"type":"string","description":"the route a ledger entry names; omit it to list every anchor, one line each"},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.consistency",
            "Compare siblings directly under one logical path and name the ones that differ — \
             the shape the maintainer calls a mixed supply chain: one object following a different \
             convention from its siblings while every file is locally plausible. The sibling set is \
             read from the registration tree (a fact only a tree has); `by` picks the signal: `api` \
             (default) compares the names each sibling's own file calls, `kind` and `source` compare \
             the declared value. It prints what the majority shares that an outlier lacks, and what \
             the outlier calls that nobody else does. `specimen` is the other baseline: given a \
             ledger anchor it compares every sibling's own file against the declared shape the \
             ledger's entry **in force** certifies (parts, exports, handle_traits, part_traits, read \
             with the kernel's face parser) and names who lacks or re-states which declaration — the \
             design was written down and the question is whether it was carried through, so a \
             sibling that declares more is not reported. **It reads text**: a convention living in a \
             shared helper or generated code is invisible, units and arithmetic are not compared, \
             and an outlier is a place to look rather than a defect. A tree that comes from \
             published records is answered as such instead of being filled with defaults.",
            json!({"type":"object","properties":{"parent":{"type":"string","description":"the logical path whose children to compare (one level under it)"},"specimen":{"type":"string","description":"a ledger anchor: compare its siblings against the shape the ledger's entry in force certifies (its parent is derived from the anchor)"},"by":{"type":"string","enum":["api","kind","source","shape"],"description":"which signal to compare; omit it and both `api` (the names each sibling's own file calls) and `shape` (the fields each sibling declares) are compared — a sibling that declares an extra or a missing field is named in one call"},"full":{"type":"boolean","description":"print every member's row; the default prints the family line, the outliers and their source excerpts, and withholds the rest (the reply says how many were withheld)"},"root":{"type":"string"}},"anyOf":[{"required":["parent"]},{"required":["specimen"]}]}),
        ),
        tool(
            "xirang.why",
            "Gather the upstream facts one `path:line` depends on, in one call: the contract lines \
             above its definition, who calls it (with the same test-file/outside-directory notes \
             `callgraph` prints), which adoption-ledger entries name that file, and the **plan half** \
             — whether this file is one of the sources the build selected or one the entry's scope \
             leaves out, whether a `#[cfg]` gates the definition, and which declared graft cut names \
             the face that owns the file (with the entry and line). Those three read the same \
             implementations `explain` and `grafts` read, so the answers cannot disagree. It also \
             names what it does **not** answer — how a grafted subtree looks at runtime — and which \
             call answers that.",
            json!({"type":"object","properties":{"at":{"type":"string","description":"a `path:line` inside this tree; a registration node is `explain`'s question"},"root":{"type":"string"}},"required":["at"]}),
        ),
        tool(
            "xirang.locate",
            "Rank the places a symptom's own words point at, so a reader holding an assertion \
             message or an error does not have to guess a name first. The reply lists up to five \
             candidates as `file:line \u{60}name\u{60}` with the reason each ranked — word overlap \
             with each function's own name, doc comments and body/error literals, a whole-phrase \
             bonus within that function, and whether it looks like a test. A production candidate \
             covering as many distinct terms as the top hit keeps a place ahead of test-only hits. \
             **It reads text only**: no call graph, no build records, no runtime evidence, so \
             a paraphrase that shares no word finds nothing, dynamic dispatch and macro expansion \
             are invisible, and a hit is a place to look rather than the defect. An empty answer \
             carries the root and the route back (`search {literal}` for an exact phrase).",
            json!({"type":"object","properties":{"symptom":{"type":"string","description":"the words of the symptom: a failing assertion's message, an error, or a description of what goes wrong"},"root":{"type":"string"}},"required":["symptom"]}),
        ),
        tool(
            "xirang.inspect",
            "Summarize functions and registration declarations in one Rust file. A **virtual \
             workspace root** is answered as the workspace: the member whose root contains the \
             path answers it, from that member's own root, or the reply says no member owns the \
             path.",
            json!({"type":"object","properties":{"path":{"type":"string"},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "xirang.callgraph",
            "Show direct static callers and callees for one function. `limit` bounds how many \
             definitions are listed (default 5, at most 50); a truncation line names how many \
             were withheld, and Each definition's own lines are included unless `source: false` says otherwise — the short
             answer is available, it is just no longer the default, because every evaluation round
             spent one extra call to see the body. `path` selects one definition when several share
             the name. A \
             **virtual workspace root** is answered as the workspace: a named `path` is answered \
             by the member that owns it, and without one every member is asked and its answer \
             grouped under it, with `tree unavailable (reason)` where a member could not answer. \
             `orphans: true` asks the other question — which functions this tree defines and no \
             function here calls — and counts the test-file half separately, because the harness is \
             what calls those.",
            json!({"type":"object","properties":{"function":{"oneOf":[{"type":"string"},{"type":"array","items":{"type":"string"},"maxItems":8}],"description":"one symbol name, or up to 8 in one call (each answer is byte-identical to asking alone)"},"orphans":{"type":"boolean","description":"list the functions this tree defines that no function here calls; functions in test files are counted separately because the test harness calls them"},"path":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":50},"source":{"type":"boolean","description":"also print each definition's own lines (capped, declared through the truncation outlet)"},"root":{"type":"string"}},"anyOf":[{"required":["function"]},{"required":["orphans"]}]}),
        ),
        tool(
            "xirang.read",
            "Read a bounded window around a line, an explicit `lines` range, or the \
             `whole` file — of **any UTF-8 text file this tree carries** (the adoption ledger \
             and a build log included; the `.rs` ones additionally carry a symbol index). \
             **Every header states the file's total**, as `path:START-END (N \
             lines)`: `N` is the file's own line count, not the printed range, so a windowed \
             read still says how much of the file is left — which is the fact the old \
             81-line window withheld, forcing six calls for a 473-line file. Default: a \
             window of `context` lines either side of `line` (context 8, at most 120, \
             240 lines) — a peek, because a caller that named only a line asked for one; \
             pass `context` for a wider window and `whole` for the file. \
             `whole: true` prints the file; `lines: \"120-260\"` prints that \
             forward, 1-based range. The three shapes are exclusive and mixing them is \
             refused by name. An explicit whole/range read is bounded at 1200 lines and a \
             reply past that bound is cut by the shared truncation notice, which names the \
             withheld count, the cap, and the narrower request that reaches the rest — \
             never a silently short answer. The header keeps the file's total either way. A \
             **virtual workspace root** is answered as the workspace: the member whose root \
             contains the path answers it, from that member's own root, or the reply says no \
             member owns the path.",
            json!({"type":"object","properties":{
                "offset":{"type":"integer","minimum":0,"description":"Directory cursor; read a directory path to list children."},
                "limit":{"type":"integer","minimum":1,"maximum":200},
                "offset_bytes":{"type":"integer","minimum":0,"description":"Byte cursor for bounded UTF-8 log/file readback; follow returned cursor."},
                "max_bytes":{"type":"integer","minimum":4,"maximum":24000},
                "path":{"type":"string"},
                "line":{"type":"integer","minimum":1,"description":"window: the line to centre on (default 1)"},
                "context":{"type":"integer","minimum":0,"maximum":120,"description":"window: lines either side of `line` (default 8)"},
                "whole":{"type":"boolean","description":"print the whole file (bounded at 1200 lines)"},
                "lines":{"type":"string","description":"print one forward, 1-based range, e.g. `120-260` (bounded at 1200 lines)"},
                "root":{"type":"string"}
            },"required":["path"]}),
        ),
        tool(
            "xirang.status",
            "Report the source root and indexed Rust file/function counts.",
            json!({"type":"object","properties":{"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.apply",
            "Edit this package's registration faces through the same authoring executor Studio \
             uses, so the kernel's admission, parent-rule, and topology checks run on the change. \
             **The eight actions fall into two classes, and the class decides whether a \
             hand-written face is a legal subject**: `add`, `deepen`, `cut`, `promote` and `move` are \
             **additive** — they write declarations (a new face, a layer inside one, the entry a \
             graft plan reads, a landed record) and reach an existing hand-written face fine; \
             `edit`, `rename` and `delete` are **rewrites** — they rewrite the face's own file, and \
             the executor only rewrites what it generated, so a hand-written face is refused by \
             name (taking one over is a separate, explicit adoption, not a spelling these actions \
             accept). Read this before choosing an action, because the refusal arrives after the \
             request. \
             `action` is `add` (create `fields.module` under `parent`), `edit` (change the \
             `fields` the request names and keep the rest), `rename` (change `fields.module`), or \
             `delete` (move the face's module into XiRang's recoverable trash). **`add` is a \
             composite command, and its two shapes are one request**: without `file` the new face \
             is mounted at the default place under `parent` (`<parent dir>/object/<module>/<module>.<ext>`); \
             with `file` it is written exactly at that package-relative path, and the rest of the \
             wiring is configured automatically — the placement base (identity base for a path below \
             the library target's source root, package root for a path outside it, so an explicit \
             placement or a custom `[lib] path` needs no second setting), the object-ledger deviation \
             that makes discovery find it, and the generated module mount (`#[path]` for a file \
             outside the walk root). Studio's add and `xirang face add --write` go through the same \
             service, so the three surfaces place a face the same way. `node` names \
             the face and `parent` the parent, by logical path — the one xirang.registry \
             reports — or by identity. `handle_contracts` and `part_contracts` are set by `add` \
             only: the executor's edit field order does not carry them, so `edit`/`rename` refuse \
             those two keys by name rather than returning success while the value is dropped. \
             **A request is previewed unless `apply` is true**: the \
             preview runs the real operation on a throwaway copy and returns the file diff plus \
             the registration tree it produces; `apply: true` writes it and names the files it \
             wrote. Every reply is the tree that results, so the next call can be aimed with it. \
             Every reply also states the **consequences** of the change, as static facts with their \
             boundary: the in-tree test lines that spell this face's name, and whether the entry \
             plan names it at all — so what this change will break, and whether the application \
             ships it, are answered before the write instead of after a red run. A refusal that has \
             a way forward says so: a parent that owns no registry is told `needs_registry`, and a \
             slot whose face now owns children is told `full graft`, while the rule itself still \
             refuses. `deepen` is the opposite direction: `inside.parts` adds a layer **inside** one \
             face — a parts struct and an accessor in that face's own file — and leaves its \
             declaration, its public path, its tree row and every factory-shape pin alone; the reply \
             also prices the other reading (another face under it), so the decision is made before \
             the write rather than by a red run. `promote` lands a confirmed external graft record into the base tree: the target face's declaration is rewritten to carry the external implementation's fields, the `cut(…)` entry that handed the slot over is retired, and the record directory moves to `.xirang/trash/`. It needs `selector`, `apply: true` and `confirm: true`, and it refuses by name a declaration that names its implementation by string — a string says nothing about where that implementation lives, so redeclare the slot as `graft(<crate>::<module>::NODE_ID)` and run it again — an external face it cannot find in the tree that declaration names, and a target file that is hand-written rather than generated, because the executor only rewrites what it wrote; `kind` moves with the rest and is an identity input, so the reply warns when it changed. A \
             **virtual workspace root** names no package and no unique owner, so a write there is \
             refused with the candidate member directories and nothing is copied or written — a \
             write runs against a member it is the unique owner of, or it does not run. **`fields.module` is a bare snake_case module name** (`widget`), not the logical path the tree reports: the executor names the module's directory and file, so `control::object::widget` is refused by name. Every other `fields` value is a **string**: `exports` is one export per call rather than an array, `requires` entries are spelled `capability=>provider`, and `handle_traits` entries are the **labels** the registration rule checks (a Rust path such as `crate::control::ControlHandle` is accepted here and only the parent rule refuses it later). The one non-string key is `needs_registry`, a boolean. **A request the kernel refuses comes back as an error** (`isError` true), because a request this tool cannot carry out is a tool failure rather than a fact about the tree; the read tools are the other way round and print their verdict in the body.",
            json!({"type":"object","properties":{
                "action":{"type":"string","enum":["add","edit","rename","delete","deepen","cut","promote","move"]},
                "cut":{"type":"string","description":"cut: the face handed over (`crate::…::NODE_ID`)"},
                "graft":{"type":"string","description":"cut: the replacement implementation"},
                "to":{"type":"string","description":"cut: range end; move: destination parent (or root)"},
                "name":{"type":"string","description":"move: optional new leaf name"},
                "selector":{"type":"string","description":"promote: the .xirang/external-grafts/<selector>/ directory to land"},
                "implementation":{"type":"string","description":"promote: where the external implementation crate lives, when the declaration names it as a Rust path but that crate is not a path dependency of this package"},
                "node":{"type":"string","description":"edit/rename/delete/deepen/move: the face, by logical path or identity"},
                 "inside":{"type":"object","description":"deepen: the layer to add inside the face. `parts` is an object of `field: Type` pairs (simple types only); the action writes a parts struct plus an accessor into the face's own file and touches nothing else — no declaration, no tree row, no public path, no factory-shape pin","properties":{"parts":{"type":"object","additionalProperties":{"type":"string"}}},"required":["parts"]},
                "parent":{"type":"string","description":"add: the parent's logical path or identity; defaults to the registry root"},
                "language":{"type":"string","description":"add/edit: the language to author in, `rust` by default. A non-Rust language needs this build's matching `lang-*` feature; without it the refusal names the missing support rather than writing a Rust file"},
                "file":{"type":"string","description":"add: the package-relative source path, for any language — omit it to mount at the default `<parent dir>/object/<module>/<module>.<ext>`, set it to write exactly there (including outside the library target's source root; the placement base, ledger mapping and module mount are configured automatically). edit: required for a non-Rust face, because the bridge's node resolver only knows Rust faces"},
                "fields":{"type":"object","description":"the face's fields; edit and rename change only the keys given, add takes the rest as defaults. Values are strings unless noted: `module` is a bare snake_case name, `exports` one export per call, `requires` entries `capability=>provider`, `handle_traits` entries rule labels","properties":{"needs_registry":{"type":"boolean","description":"the one non-string field"}},"additionalProperties":{"type":"string"}},
                "apply":{"type":"boolean","description":"false (the default) previews on a copy; true writes to the project"},
                "rebind_grafts":{"type":"boolean","description":"move: carry what names the moved object — the graft records under `.xirang/external-grafts` (their `target` and `target_path`) and the `static_graft_plan!` entries that point at it — rewriting them to the new identity. Without it such an entry refuses the move, because following the object is the caller's decision. Only determined entries are carried; an inconsistent record or a cut that names the face as one end of a range is refused for hand repair"},
                "plan":{"type":"string","description":"move: the `plan <digest>` a preview printed. Supplying it makes an apply refuse a tree that changed since that preview — a different declaration, different bytes, or a different destination — instead of writing something the preview never showed. Omit it and the apply recomputes the move from the current tree"},
                "full":{"type":"boolean","description":"cut: replace the entire subtree; also show every resulting face (default false)"},
                "confirm":{"type":"boolean","description":"delete and promote: must be true. A delete is the one operation whose preview a caller can step past by accident, so the request says it rather than the bridge adding it"},
                "root":{"type":"string"}
            },"required":["action"]}),
        ),
        tool(
            "xirang.new_project",
            "Create a host project with the same scaffold `xirang new` (the CLI) and Studio's \
             new-project wizard run — `crate::build_method::scaffold::create_project` — so the \
             manifest, build script, source entry, editor snippets, and detected dependency \
             source are the ones this workspace ships rather than a second renderer written here. \
             `directory` is where the project goes: a relative one is taken against `root` (the \
             package or workspace root this call resolved), and a destination outside that root, \
             or one that steps out with `..`, is refused by name before anything is created. A \
             **virtual workspace root** is answered against that root's own directory — the \
             destination is explicit, so containment in the root is the whole ownership question \
             and no member has to be picked. \
             `package` is the crate name and `kind` is `binary` or `library`. `faces` creates \
             registration faces **in the same call**, so starting a project that already has two \
             objects is one request rather than a scaffold plus one `apply add` per object: each \
             entry is `{\"fields\": {\"module\": \"button\", \"kind\": \"Button\"}, \"parent\": \"<node>\"}` \
             — the shape `apply add` takes, without `action` — and `parent` defaults to the project \
             root. The entries run through that same executor, in order, and the reply reports the \
             faces the project now derives, so a second `registry` call is not needed; an entry the \
             kernel refuses leaves the destination untouched, because the whole project is built \
             beside it and moved in with one rename. **A request is \
             previewed unless `apply` is true**: the preview runs that executor in a throwaway \
             directory and prints every path it would write, with the bytes, under the real \
             destination and the root it is inside; `apply: true` writes them. A destination that \
             already exists must be empty, and then the request itself has to say \
             `confirm: true`, because the write lands in a directory the caller already has. The \
             scaffolded manifest declares its own `[workspace]`, so the project is not a member of \
             the root this call ran in.",
            json!({"type":"object","properties":{
                "directory":{"type":"string","description":"where the project is written; relative paths are taken against `root` and a destination outside it is refused"},
                "package":{"type":"string","description":"the new crate's name (letters, digits, `_` or `-`)"},
                "kind":{"type":"string","enum":["binary","library"],"description":"the entry the scaffold writes: `src/main.rs` or `src/lib.rs`"},
                "faces":{"type":"array","description":"registration faces to create in the same call, in order; each entry is `{\"fields\": {\"module\": \"button\", \"kind\": \"Button\"}, \"parent\": \"<node>\"}` (the shape `apply add` takes, without `action`); `parent` defaults to the project root","items":{"type":"object","properties":{"fields":{"type":"object","description":"the new face's fields, the same ones `apply add` takes (`module` is required in practice: a module name is what the executor names the file and directory after)"},"parent":{"type":"string","description":"the logical path or identity of the parent face; defaults to the project root"}},"required":["fields"]}},
                "apply":{"type":"boolean","description":"false (the default) previews in a throwaway directory; true writes the project"},
                "confirm":{"type":"boolean","description":"must be true when the destination directory already exists: the write lands in a directory the caller already has, so the request says it rather than the bridge assuming it"},
                "dependency":{"type":"string","enum":["path","registry","git"],"description":"where the manifest's `xirang-toolchain` lines point: `path` at a checkout (this tool's own, or the one `path` names — the only spelling that resolves with no network), `registry` at the published release, `git` at a repository. Never guessed into `git`"},
                "path":{"type":"string","description":"a XiRang checkout for the manifest to point at; the way to get offline-resolving dependencies from a tool outside one"},
                "git":{"type":"string","description":"the repository the generated manifest pulls from; it is the URL for `dependency: \"git\"` and defaults to this project's own. Passing it without saying `dependency: \"git\"` is refused rather than obeyed"},
                "root":{"type":"string"}
            },"required":["directory","package","kind"]}),
        ),
        tool(
            "xirang.plugin",
            "Write one plugin record into this package's lock — the same path Studio's plugin \
             form takes (`submit_plugin`). `source` is `official` or `user` and selects \
             `official.lock` or `user.lock` under `.xirang/plugins/`; `framework`, `package`, \
             `version`, `crate`, and `checksum` are the record's identity fields and `mode` is \
             `extension` or `replacement`. An `official` record is admitted only when the kernel's \
             `PluginCatalog::contains_record` says the lock already accounts for that package \
             identity — the trust rule the runtime reads — and the lock's own parser decides \
             whether the append is legal, both before anything is written. The entry file that \
             imports the crate (`official.rs` or `user.rs`) carries the other half of the same \
             decision, so a failed lock write restores it. A record the lock already carries is \
             reported as already selected and nothing is written. **A request is previewed unless \
             `apply` is true**: the preview computes the exact bytes with the same kernel calls \
             and prints what each file would gain, so it cannot drift from the write; `apply: \
             true` stores them. A plugin write requires the request to say `confirm: true` itself, \
             because the lock is the artifact the host admits plugins from. The three provenance \
             columns of the ten-field spelling — `signature`, `fingerprint`, `revocations` — are \
             optional and are what an `official` write is *for*: the trusted lock already accounts \
             for that package identity, and a record carrying provenance the bare one does not is \
             the promotion the kernel accepts (it replaces that record rather than colliding with \
             it), while a request that names none of them writes the seven-field line. A \
             **virtual workspace root** names no package, so a plugin write there is refused with \
             the candidate member directories.",
            json!({"type":"object","properties":{
                "source":{"type":"string","enum":["official","user"],"description":"which lock the record goes into: `official.lock` or `user.lock`"},
                "framework":{"type":"string","description":"the target framework the record was written for"},
                "package":{"type":"string","description":"the plugin's package name (its identity)"},
                "version":{"type":"string","description":"the plugin version, exactly as the lock should spell it"},
                "crate":{"type":"string","description":"the Rust crate that carries the plugin implementation; must be an identifier"},
                "checksum":{"type":"string","description":"the digest the plugin bytes must match, with or without a `sha256:` prefix"},
                "mode":{"type":"string","enum":["extension","replacement"],"description":"whether the plugin extends a slot or replaces the face in it"},
                "signature":{"type":"string","description":"optional provenance (ten-field spelling): the signature over the plugin bytes; naming any provenance column writes the ten-field form"},
                "fingerprint":{"type":"string","description":"optional provenance: the signing key's fingerprint"},
                "revocations":{"type":"string","description":"optional provenance: the revocation list the record was checked against"},
                "apply":{"type":"boolean","description":"false (the default) previews the exact bytes; true stores them"},
                "confirm":{"type":"boolean","description":"must be true to write: the lock is the artifact the host admits plugins from, so the request says it rather than the bridge assuming it"},
                "root":{"type":"string"}
            },"required":["source","framework","package","version","crate","checksum","mode"]}),
        ),
        tool(
            "xirang.registry",
            "Report the registration faces this package declares, as the build sees them. A \
             **member that published records** under `target/xirang/out` is answered from them — \
             the face rows `pruning_manifest.tsv` carries (`node`, `source`, the tracked symbol) \
             and the scope verdict `source_scope.tsv` carries — without re-deriving its tree, and \
             the reply opens with `tree published from <dir>` naming the `discovery.fingerprint` \
             and whether that output still describes these sources (`build current`, or `build \
             stale (run xirang check)`). Those records carry no logical path and no `kind`, so a \
             member answered that way has no `root/...` column and the reply's last line says so: \
             `xirang.explain` reports the derived per-face projection that has them. A member \
             that published nothing is derived now and says `tree derived now (no published records \
             at ...)` above the derived rows. Contract, admission, and registration-rule data need \
             the built snapshots and are not included. `root` is a package root; omitting it uses \
             XIRANG_PACKAGE_ROOT. A **virtual workspace root** (a manifest with no `[package]`) \
             names no package, so it is answered as the workspace: a census naming every member's \
             status — `published`, `not built` (no records, so its tree was derived now), `no faces` \
             (a framework crate declares none), or `unresolvable` with the reason — and, in the \
             **default** answer, one row per member with the faces it declares. The per-member \
             trees are what that default leaves out: pass `full: true` for one section per member \
             under that member's own package name, as the build sees it. Point `root` at one member \
             for that package's own answer, which is always the whole tree.",
            json!({"type":"object","properties":{
                "root":{"type":"string"},
                "full":{"type":"boolean","description":"print the face rows (default: the census — the count and how many sit at each level), one page at a time"},
                "offset":{"type":"integer","description":"with `full`: the first row of this page (default 0)"},
                "limit":{"type":"integer","minimum":1,"maximum":200,"description":"with `full`: rows per page (default 200, capped at 200)"}
            }}),
        ),
        tool(
            "xirang.explain",
            "Report the build's own evidence for one face — identity, logical path, kind, source, \
             module, parent, registry_name — or the tree projection the build scoped. \
             `xirang.registry` derives from source text and is therefore never `stale`; this reads \
             the files the build *published* under `target/xirang/out`, so it answers what ships: \
             whether the scope selected the face and whether release pruning strips its symbols. \
             The `build` line says `current`, or `stale (run xirang check)` when the published \
             output no longer describes these sources; an unbuilt project is told which command \
             publishes the evidence. `node` names one face by logical path or identity; omit it for \
             the projection, bounded by `limit`. With `overlay: true` it renders the *overlay* \
             projection instead — which slot each declared cut replaces, and which faces the scope \
             prunes — the published state after replacement, from the same traversal the CLI's \
             `explain --overlay` uses. That is a static projection and not `Registry::dump`: an \
             overlay needs two live registries, and the reply carries the note saying where the \
             live tree comes from. `overlay` and `node` are mutually exclusive. Declared graft \
             state stays in `xirang grafts`; contract and admission fields need a loaded registry \
             and are not reported here. A **virtual workspace root** is answered as the workspace: \
             a census of every member with its status, then each member's own report under it, with \
             `tree unavailable (reason)` where a member could not answer the named face.",
            json!({"type":"object","properties":{"node":{"type":"string"},"overlay":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.diff",
            "Report the face-level delta between two sides of this package. By default the sources \
             now against the build's own manifest: which faces were added, which are gone, and which \
             changed identity under a file that did not move (a `kind` change is an identity change, \
             so only this comparison sees it). The unit is the face rather than the line, because the \
             face is what the registry ships. Needs a prior `xirang check` or `build`; a project \
             with no build evidence is told so instead of being handed an empty diff. With \
             `records: true` the other pair: every external graft record against the sources, where a \
             record stores the identity it was written for, so a face that changed identity under an \
             unmoved slot breaks it silently. A record comes back `ok`; `undeclared` (the identity \
             is in the tree, but no `static_graft_plan!` cut names its slot — the release prunes \
             that slot, which is the same verdict `xirang.grafts` gives and the same one the \
             build refuses); `stale` (nothing in the tree has that identity or that path); \
             `re-identified` (the path is there, the identity moved); or `unreadable` (a record \
             file that could not be read), which is counted rather than dropped. `undeclared` wins \
             over `stale`/`re-identified`, because whether a record can ever take effect is the \
             question that changes what a caller does. There is no separate bucket for a typed \
             cut: the plan's path is always a logical path (every writer uses `registry.path_for`), \
             and with the identity absent nothing resolves a typed declaration's module, so such a \
             record is judged by the declaration rule like any other — and the reply prints the \
             path it looked for rather than guessing. A **virtual workspace root** is answered as \
             the workspace: a census of every member with its status, then each member's own \
             comparison, so a member with no build evidence and a member that cannot be resolved \
             are told apart rather than merged into one empty diff. With `against`, the other side is a **second directory of published records** rather than the sources: the reply compares the two sets as data (scope identities, pruning rows, the discovery token) and names the sections it cannot compare because this tree has no reader for them.",
            json!({"type":"object","properties":{"records":{"type":"boolean"},"against":{"type":"string","description":"a second directory of published records: the reply compares the two sets of records as data instead of comparing this one against the sources"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.trace",
            "Read this package's recorded trace artifact (`XIRANG_TRACE_FILE`, else \
             `<root>/.xirang/traces/xirang.trace`) and answer with the headless call report it \
             implies — what actually ran, which no static read can tell you. With `values: true` the \
             same read answers what that run *saw* instead: the recorded locals grouped by the frame \
             they belong to (name, type, rendered value, role, observed-or-inferred, callsite), the \
             locals recorded outside every traced call, and the observed data edges between them. A \
             local or edge naming something the artifact does not contain is counted rather than \
             dropped, so an incomplete artifact cannot read as a complete one. The artifact's identity \
             is checked first (namespace, registry root, every frame's node), and an artifact that \
             describes a different tree is refused by name rather than rendered, because frames are \
             node identities and foreign ones would draw a plausible, wrong call tree. Absence is \
             reported together with the way to produce one; `query` filters either report and a long \
             one is truncated with its total named. A **virtual workspace root** is answered as the \
             workspace: a census of every member with its status, then each member's own read of \
             its own trace artifact, with `tree unavailable (reason)` where a member could not \
             answer.",
            json!({"type":"object","properties":{"query":{"type":"string"},"values":{"type":"boolean"},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.mir",
            "Read a MIR artifact and report the compiler's call candidates, emit it as canonical \
             JSONL, or report the call-graph delta against another artifact. A `rustc \
             -Zunpretty=mir` text dump and the compact JSONL artifact are both accepted, chosen by \
             extension: JSONL parses strictly, so a malformed line fails the whole read, while a \
             text dump never fails because a line that is not a call is simply not a call. The JSONL \
             form is the portable channel Studio could already render and parse and nothing in this \
             workspace ever wrote — `jsonl: true` makes this tool that writer, and what it prints \
             reads back here. It prints only a snapshot it can print **whole**: a graph whose JSONL \
             would run past this bridge's reply cap is refused by name — the line count, the cap, and \
             the narrower dump that would fit are all named — instead of being printed short, because \
             a payload cut at the cap would read back here as a complete snapshot of a smaller graph. \
             What it writes is a *snapshot*: a header naming the identity namespace \
             and registry root of the tree the artifact came from, which is what makes two artifacts \
             comparable — a snapshot of another tree is refused by name, and an unidentified one (a \
             text dump can't name its tree) makes the delta say what it cannot rule out. With \
             `against`, that other artifact is the baseline and the reply is the call-graph delta \
             from it forward: added and gone relations, plus function symbols. With capture:true, produce text through the Studio Cargo command and bind it to current source/configuration; a missing artifact says \
             exactly that instead of reporting an empty graph. A **virtual workspace root** is \
             answered as the workspace: the member whose root contains the artifact path answers \
             it, from that member's own root, or the reply says no member owns the path.",
            json!({"type":"object","properties":{
                "capture":{"type":"boolean","description":"Run Studio-compatible stable cargo rustc --emit=mir; immutable artifact with source/config/argv identity. Select root/package and bin for binary targets; default lib."},
                "package":{"type":"string"},"bin":{"type":"string"},"face":{"type":"string"},"target":{"type":"string"},"no_default_features":{"type":"boolean"},"timeout_ms":{"type":"integer","minimum":1000,"maximum":3600000},
                "path":{"type":"string"},"against":{"type":"string"},"against_trace":{"type":"string"},"jsonl":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.unified",
            "Merge a MIR artifact with this package's recorded trace: the compiler's call candidates \
             together with what actually ran, where a live call confirms its candidate and carries \
             `evidence=Live` while the rest stay `evidence=Mir`. This is `debug_method`'s \
             `UnifiedCallGraph`, the one place the two evidence sources are joined, so the merge \
             cannot drift from the library's own. The trace is read from this package's artifact \
             path; when none has been recorded the merge still answers and labels every relation a \
             compiler candidate rather than failing, because an absent trace is a weaker answer and \
             not a broken one. A **virtual workspace root** is answered as the workspace: the \
             member whose root contains the artifact path answers it, from that member's own root, \
             or the reply says no member owns the path. With `against`, the two artifacts are compared rather than merged: the reply lists the relations only one side has and the ones **both** carry under **different** evidence (`Live` against a compiler candidate), which is the difference an edge-level delta cannot express. `against_trace` gives the baseline side its own trace artifact; without it both sides share this package's.",
            json!({"type":"object","properties":{"path":{"type":"string"},"against":{"type":"string","description":"a second MIR artifact: the reply compares the two chains at the evidence level instead of merging one"},"against_trace":{"type":"string","description":"a trace artifact for the baseline side; omit it to compare both chains under this package's own trace"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "xirang.grafts",
            "Every external graft plan under `.xirang/external-grafts/<selector>/graft.plan` \
             joined with the host entry's `static_graft_plan!` declarations: the selector, the \
             logical path and replacement each plan targets, whether it covers the whole subtree, \
             and whether the host entry names that slot. A plan the entry does not declare is \
             reported as `NOT declared by the host entry` and counted under `unkept plans`: the \
             release prunes that slot, so the record can never take effect — the build warns about \
             it and a cargo log is where that warning goes to die. An unreadable plan carries its \
             reason rather than being skipped, and with no readable declaration the state is \
             `declaration unknown` rather than a false `not declared`. A declaration is about \
             the *slot* a plan targets, not the implementation it selects, so each row shows the \
             plan's own target and graft next to the declaration's cut and graft. Read-only. A \
             **virtual workspace root** is answered as the workspace: a census of every member \
             with its status (`published`, `not built`, `no faces`, or `unresolvable` with the \
             reason), then each member's own plans, since a plan is a record under one package's \
             `.xirang/` and a declaration is judged against a logical path, which is a fact the \
             published record does not carry.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":400},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.impact",
            "The transitive blast radius of a change to one face, over the three dependency kinds \
             this tree actually declares: a face's descendants (its registry owns them), the faces \
             whose `requires` names a capability it provides, and the declared graft cuts that hand \
             it over. Each reached node carries its shortest hop distance and the chain of reasons \
             that got there, so a capability cycle is a shorter path rather than a hang. A face the \
             traversal did not reach is reported as unreached within `depth` — which is not proof of \
             independence — and graft records or recorded traces naming the same identity are not \
             traversed, which the reply states. A **virtual workspace root** is answered as the \
             workspace: a census of every member with its status, then each member's own radius \
             under it, with `tree unavailable (reason)` where a member could not answer the named \
             face — and a face no member declares is reported as such rather than as another \
             member's answer.",
            json!({"type":"object","properties":{"node":{"type":"string"},"depth":{"type":"integer","minimum":1,"maximum":16},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
        ),
        tool(
            "xirang.usages",
            "Report the neighbourhood of one face: its parent and children as the tree has them, every \
             field the write path accepts read back from the generated module (`module`, `preset`, \
             `parts`, `name_zh`, `name_en`, `summary_zh`, `summary_en`, `stable_name`, `exports`, \
             `requires`, `provides`, `handle_traits`, `handle_contracts`, `part_traits`, \
             `part_contracts`, `registration_rule`, `admission`, `flow`, `flow_provider`, \
             `runtime_checks`, `getting_from_other_registry`, `needs_registry`) — so every field \
             `xirang.apply` can set becomes reportable — and which other faces mention the same \
             capability tokens. Capability matches are on declared tokens rather than a resolved graph, \
             and the reply says so. A hand-written module has no generated field list and the executor \
             refuses to invent one, so those faces are counted as unreadable rather than shown empty. \
             A package whose own faces the authoring connector refuses still gets its tree half — the \
             parent, the children and this face's identity come from the source derivation — with the \
             verdict printed and the field and capability halves named unavailable rather than shown \
             empty; that is the same verdict `xirang.verify` prints on its connector line, and it is \
             an answer about the tree rather than a tool failure, so `isError` stays false. \
             Declared graft cuts are not reported here; the CLI's `explain --json` carries them. A \
             **virtual workspace root** is answered as the workspace: a census of every member \
             with its status, then each member's own neighbourhood under it, with `tree \
             unavailable (reason)` where a member could not answer the named face.",
            json!({"type":"object","properties":{"node":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
        ),
        tool(
            "xirang.converge",
            "The converged starting point for one face or a recorded run. `node` focuses an object; \
             `trace: true` without a node seeds from observed object identities. Combining them retains \
             the node focus. One answer joins current contracts and ancestor-resolved capability \
             providers/consumers, build scope/pruning freshness, declared graft cuts and plan selection, \
             recorded parent-frame paths and value edges, and verification targets. Available runtime \
             evidence is included for node queries too. Frame NodeId owns values; source locations are \
             callsites, not callee ownership. Missing/refused traces preserve static evidence. Build \
             projections do not prove live replacement; trace identity checks do not establish source, \
             config or input freshness, causality, or absence of bypass. `limit` bounds connected objects \
             and each evidence list; the reply is also byte-bounded and names omissions. Use status to \
             select a test face for check; a registry node is not a test-face name. A virtual workspace \
             root returns each member's own evidence with unavailable members named.",
            json!({"type":"object","properties":{"node":{"type":"string"},"trace":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.adopted",
            "Report this package's adoption ledger at `.xirang/adopted/entries`: which routes \
             were adopted **provisionally**, and whether the bytes each decision was taken on are \
             still here. An entry reads `adopted since <at> at <fingerprint> (provisional)` while \
             its files are unchanged, and `adoption lapsed at <file>; needs confirmation` once one \
             of them moved or went missing — the file is named because that is where to look. \
             Reading never renews anything, and the word `verified` is deliberately absent: an \
             adoption is a lease whose honest level is `provisional`, and re-earning it takes a \
             person. With `anchor`, `certifies`, `evidence`, `verifier`, `reason` and `files` the \
             same tool **appends** one line — a preview unless the request also says `apply: true` \
             and `confirm: true` — so a confirmation is one more line rather than a rewrite. The \
             read also names the action for a route this ledger does not carry: a route the ledger \
             has never named is a **new anchor** and a first confirmation, not a renewal.",
            json!({"type":"object","properties":{"anchor":{"type":"string"},"certifies":{"type":"string"},"evidence":{"type":"string"},"verifier":{"type":"string"},"reason":{"type":"string"},"files":{"type":"array","items":{"type":"string"}},"apply":{"type":"boolean"},"confirm":{"type":"boolean"},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.affected",
            "Find transitive test candidates for changed files. One current source inventory covers \
             all files; reverse lexical calls cross member boundaries without a hop cutoff. \
             This is not resolved dispatch or coverage: aliases, dynamic calls and name collisions \
             remain uncertain. Deleted files and no matches require conservative workspace regression. \
             `files` accepts an array or comma-separated string; `limit` bounds test files per change.",
            json!({"type":"object","properties":{"files":{"oneOf":[{"type":"array","items":{"type":"string"},"maxItems":128},{"type":"string"}]},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["files"]}),
        ),
        tool(
            "xirang.check",
            "Run Cargo test/check/build/clippy and report the observed verdict. Compilation modes never prove behavioral tests. `face` may be omitted (default), \
             or be `all`, comma-separated features, or package/feature selectors. Select `workspace` \
             or `package`, optionally `test`, `filter`, and `no_default_features`. A passed result \
             verifies only the tests actually executed under these flags, never all consumers or \
             replacement implementations. A `files` plan is durable: its reply prints `plan <id>` and \
             `record <path>`, the reply names the input manifest the verdict is bound to, and a later call \
             reads or continues it with `plan_id`. Timeout, signal, or zero executed tests is unknown. \
             The reply includes bounded compiler errors and failed assertions; read the named log \
             for omitted details without rerunning. Omit `census` for sampled static analysis, use \
             true for detail, false to skip source analysis and reduce latency/context. Census is \
             lexical evidence, not coverage; dynamic dispatch, aliases, macros and runtime behavior \
             remain unknown. Unknown parameters and invalid types are rejected before Cargo runs. \
             Timeout/cancel terminates the execution process group; detached descendants are outside it. `target_dir` separates \
             build artifacts and logs from the source tree. The reply's first line is the run's verdict: `verdict  passed (cargo exit 0)` or \
             `verdict  failed (cargo exit 101)`, not the one-shot client's exit code: \
             `0` when the tool answered, `1` when it refused, `2` on a usage error.",
            json!({"type":"object","properties":{
                "files":{"type":"array","items":{"type":"string"},"description":"Create and execute repair obligations: current impact plus workspace default/all/no-default tests, or explicit face. State and source/config identity persisted; narrowed probes cannot close workspace obligations. A plan also derives **consumer-protection duties**: when a named file defines a function that two or more other, non-test files call, every one of those consumer entries must be called by name from some test file, or the plan stays `open` even though every cargo run was green. The duty is lexical and says so: a test that exists but never executes, a filter that matched nothing and an ignored test leave it open, and a test that calls the shared implementation directly protects the implementation rather than the consumer's delegation."},
                "cases":{"type":"array","maxItems":16,"items":{"type":"object"},"description":"Additional named test requests, each `{name, expected: \"passed\"|\"failed\", request, expect?, variant?}`. `variant` declares `{edits:[{path, content|null}]}`; it is materialised into an immutable copy under the plan output directory (named by the digest of the declared edits, beside its own build directory) and the obligation runs **there**, so the base tree is never the fault-injected tree and no `cargo clean` is needed. A variant may not edit the file that declares the `expect.test` it must be caught by. `kind` is `fault` (default; a counterproof, so `expected` must be `failed`) or `replacement` (a legal substitute provider, so `expected` must be `passed`); the contradictory pairings are refused before anything is copied. `route: {caller, callee}` asserts that after the edits `caller` still calls `callee` — that is how a consumer that copied the formula and stopped delegating is detected, since its behaviour is identical and every behavioural test passes. A broken route leaves a duty in the obligation ledger and keeps the plan open even when every cargo run was green. A `failed` (negative-control) case must also declare `expect: {test, target?, violation}`: `test` is the exact full test name cargo prints, `target` is the Cargo target source path when that name exists in more than one target, and `violation` is a phrase from that same test's own assertion output. A case whose failure is unqualified, whose named test did not fail on that violation, or whose name is ambiguous across targets without a bound target is `unknown`, never `passed`; compiler errors, timeouts, cancellations and zero executed tests never certify. Cases share the plan build directory; set target_dir only on the outer plan."},
                "action":{"type":"string","enum":["plan","run","start","list","status","result","cancel"],"description":"Default run; start returns a bounded server-session job; poll status/result or cancel its process group. `plan` with `files` seals the obligations and prints what a run would do **without starting Cargo**. With `plan_id` the action addresses a durable plan instead: `status` (default) or `resume`."},
                "plan_id":{"type":"string","description":"A durable verification plan: the `plan <id>` a `files` plan reply printed, or that plan's task key. `status` reads it back and writes nothing; `resume` continues an interrupted plan and re-runs only obligations that are unfinished or whose inputs changed — a pass is evidence only while the bytes it was bound to still hold, and an obligation that already landed an effect is never replayed automatically. A second writer is refused while another live process owns the plan."},
                "job":{"type":"string","description":"Job ID returned by start."},
                "mode":{"type":"string","enum":["test","check","build","clippy"],"default":"test"},
                "profile":{"type":"string","enum":["debug","release"],"default":"debug","description":"Cargo build profile. `debug` and `release` compile different code (`debug_assert!`, overflow checks, `cfg(debug_assertions)`), so a verdict is bound to the profile it was produced under and is never reused for the other. `release` reaches Cargo as `--release`."},
                "reuse":{"type":"boolean","description":"With files: reuse a recent passing plan instead of running Cargo when that plan was produced against this exact input closure and this exact execution identity (candidate, toolchain, target, profile, features, no-default-features, selection, mode, checker, isolation). Default false: a `files` call always executes, so no existing promise is quietly narrowed. The reply names the plan that was reused and every dimension that matched."},
                "target":{"type":"string","description":"Cargo target triple."},
                "all_targets":{"type":"boolean"},
                "deny_warnings":{"type":"boolean","description":"Clippy -D warnings."},
                "workspace":{"type":"boolean","description":"Test all workspace members, including non-default members."},
                "package":{"type":"string","description":"Select one Cargo package; conflicts with workspace:true."},
                "test":{"type":"string","description":"Select an integration-test target."},
                "filter":{"type":"string","description":"Test name filter; zero matches yields unknown."},
                "no_default_features":{"type":"boolean","description":"Disable default features for the selected packages."},
                "face":{"type":"string","description":"`default`, `all`, or Cargo features such as `dev-supervisor,studio` or `package/feature`; omit for default. Unsupported parameters and invalid types are refused before running tests."},
                "timeout_ms":{"type":"integer","minimum":1000,"maximum":3600000,"description":"how long the run may take before it is reported as unknown (default 900000)"},
                "target_dir":{"type":"string","description":"Ordinary run: CARGO_TARGET_DIR, subject to Cargo freshness rules. With files: base for logs, ledger and content-specific verification-build directories, preventing another source revision's stale green. Cases inherit this base. Must not contain source inputs."},
                "census":{"type":"boolean","description":"Omitted: sample; true: detail; false: skip source analysis."},
                "verbose":{"type":"boolean","description":"print every `test result:` line, including the groups that passed (default: the failing ones line by line, the passing ones as a count and the first)"},
                "root":{"type":"string"}
            }}),
        ),
        tool("xirang.candidate", super::candidate::DESCRIPTION, super::candidate::schema()),
        tool("xirang.rules", super::rules::DESCRIPTION, super::rules::schema()),
        tool(
            "xirang.refactor",
            super::refactor::DESCRIPTION,
            super::refactor::schema(),
        ),
        tool("xirang.task", super::task::DESCRIPTION, super::task::schema()),
        tool(
            "xirang.constraints",
            "Decide one expectation against what a node's file **declares**: `met`, `unmet`,              `unknown` or `invalid`. It reads `declares`/`provides`/`requires` and nothing else, so a              behavioural claim (`must_call`, a route through a named provider) is `unknown` rather              than `met`: a capability requirement is not a statement that every method must call the              provider, and a majority of siblings is a candidate rather than a verdict. An `unmet`              expectation is a conflict between the expectation and the tree — this tool prints what              the node does declare and never edits the business to make the expectation true. It              changes nothing on disk.",
            json!({"type":"object","properties":{
                "node":{"type":"string","description":"logical path, module, or identity of the one face the expectation is about"},
                "expect":{"type":"object","description":"`{declares: \"<field>\", value: \"<expected text>\"}`, `{provides: \"<capability>\"}` or `{requires: \"<capability>\"}`. Any other key is reported as `unknown`, never `met`."},
                "control":{"type":"string","enum":["positive","negative"],"default":"positive","description":"Which way the expectation is aimed. `positive` (default): the tree should carry it, so `met` is a passing control. `negative`: the tree must **not** carry it, so `unmet` is the passing control — that is how a check is shown to be able to fail at all. An undecidable expectation is `unknown` under both directions, never a passing control."},
                "root":{"type":"string"}
            },"required":["node","expect"]}),
        ),
        tool(
            "xirang.verify",
            "Static registration validation ONLY; does not compile Rust or run tests. Use `check` for behavior. \
             Run the kernel's registration validation over this package and report the tree delta the \
             run just published. It drives the same entry the CLI's `check` drives, so a verdict here \
             cannot drift from `xirang check`, and it refreshes the build evidence as a side effect — \
             which is why the delta below it describes the tree that was just verified rather than the \
             last build. **Two verdicts are printed, because this package is judged on two surfaces.** \
             The first is the static one: it judges the faces the build ships, and a failure there is \
             the answer and not a tool failure — the reply says `verdict failed` with the diagnostics \
             (each naming its phase, node, source and line) while `isError` stays false, because the \
             verification itself succeeded. The second is a line of its own, `connector verdict: ok` or \
             `connector verdict: rejected`, from the authoring connector every face tool validates \
             through (`apply`, `usages`, `converge`): it registers the faces on disk rather than the \
             ones the build scoped, so a tree can pass the static line and be refused here. A rejection \
             is rendered the same way — the diagnostic names the field and the line — and `isError` \
             stays false for it too, because `rejected` is what this surface says about the tree rather \
             than a failure to ask. A **virtual workspace root** is answered as the workspace: a \
             census of every member with its status, then each member's own verdict and delta \
             under it, with `tree unavailable (reason)` where a member has no tree to verify.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "xirang.graph",
            crate::mcp::graph::DESCRIPTION,
            crate::mcp::graph::schema(),
        ),
        tool(
            "xirang.crates",
            "The crate partition this host declares at its package root (`add_crates.rs`), and the \
             three writers that make it real. A host can hand a subtree to a crate of its own; the \
             declaration is ordinary Rust (`Crate::named(\"widgets\").at(&[…::SUBTREE])` inside a \
             the registration tree, so XiRang checks the paths against it (no compiler can: a \
             partitioned host no longer compiles the subtrees its declaration names). This tool \
             answers all three questions a split raises before it is written: **what would be \
             written** (one package per declared crate, plus a *facade* that carries the cross-crate \
             half — the graft table and the two contract assertions), **what is on disk now** \
             (absent, the development shape, the publishable release shape, or a directory that is \
             not this action's), and **how big each package is and whether it could be published** \
             (its file count and bytes, its dependencies inside the partition — which is the order \
             they can be published in — and what `cargo publish` would still want: a `description`, \
             a `license`, a `version` on a path-only sibling, or the release shape instead of the \
             development one, which mounts the host's files and needs the workspace remap). \
             `action` is `plan` (the default, and the only one that writes nothing), `write` (the \
             development shape), `release` (the publishable shape) or `revert` (take the generated \
             packages back) — and the **declaration layer** is operable too: `declare` (needs \
             `crate` and `subtree`, the host's own path to a `SUBTREE` constant) adds a crate to \
             `add_crates.rs`, creating the file when the host has none, and `undeclare` (needs \
             `crate`) removes exactly that entry, leaving every other byte of the author's \
             hand-written source alone; removing the **last** declared crate removes the file, which \
             is what a host going back to one crate means. `undeclare` refuses while that crate's \
             package is still on disk, because after the entry is gone nothing knows that directory: \
             `revert` first. **A request is a preview unless `apply` is true**: without it the reply \
             is the partition plus the sentence naming what the action would change; with it the \
             write happens and the reply is the tree as read back from the disk. Identities survive \
             both shapes — a face derives its identity input from `file!()` by dropping \
             `CARGO_MANIFEST_DIR` and one leading `src/` — so a split never renames a face.",
            json!({"type":"object","properties":{
                "action":{"type":"string","enum":["plan","write","release","revert","declare","undeclare"],"description":"plan (default) reads only; write/release/revert change the generated packages; declare/undeclare edit the host's `add_crates.rs`"},
                "crate":{"type":"string","description":"declare/undeclare: the crate's name"},
                "subtree":{"description":"declare: the host's path to a `SUBTREE` constant, e.g. `crate::panel::frame::SUBTREE` (or an array of them)","anyOf":[{"type":"string"},{"type":"array","items":{"type":"string"}}]},
                "apply":{"type":"boolean","description":"false previews the action; true writes it"},
                "root":{"type":"string"}
            }}),
        ),
        tool(
            "xirang.multilang",
            crate::mcp::multilang_tool::DESCRIPTION,
            json!({"type":"object","properties":{"action":{"type":"string","enum":["inventory","plan","report"],"description":"inventory (default): the units; plan: the actions a build would run, without running them; report: the economics one finished build recorded"},"plan":{"type":"string","description":"report: the plan id to summarize; without it, the newest build"},"root":{"type":"string"}}}),
        ),
        catalogue_entry(),
    ]
}

/// What `tools/list` advertises: the two entry points, plus the catalogue tool.
/// `tools/list` 广告的东西：两个入口，加上目录工具。
///
/// Audit `W1-1`: the client used to be handed **all twenty-seven** tools in one frame — around
/// 36,000 characters of prose — so a session's first decision was made against a wall of text, and
/// the round measured what that costs (25 steps against the compared tool's 14). Nothing is removed:
/// the catalogue still holds every tool, `--list <tool>` still prints a whole description, and this
/// meta-tool lists them all. What changed is **what arrives unasked** — the two calls the workflows
/// actually enter through, and one tool whose whole job is to show the rest on request.
/// 审计 `W1-1`：客户端过去在**一帧里**拿到**全部二十七个**工具——约 36,000 字符的散文——于是一个会话的
/// 第一个决定是在一堵文字墙前做出的，而那一轮量到了代价（25 步 vs 对照工具的 14 步）。什么都没有删：
/// 目录里仍然是每个工具，`--list <tool>` 仍然印完整描述，而这个元工具会把它们全列出来。变的是
/// **不请自来的东西**——工作流真正进入的那两个调用，以及一个"按需展示其余"就是它全部职责的工具。
pub(crate) fn advertised() -> Vec<Value> {
    vec![
        tool(
            "xirang.explore",
            crate::mcp::explore::DESCRIPTION,
            advertised_schema("xirang.explore"),
        ),
        tool(
            "xirang.apply",
            ADVERTISED_APPLY,
            advertised_schema("xirang.apply"),
        ),
        catalogue_entry(),
    ]
}

/// The catalogue tool's entry, shared by the catalogue and the advertisement.
/// 目录工具的条目，目录与广告共用。
///
/// One builder for both lists, because two copies of a description drift exactly the way the round
/// measured them drifting: what `tools/list` says and what `--list` says are the same sentence about
/// the same tool, or one of them is wrong.
/// 两份清单共用一个构造器，因为描述的两份拷贝会按那一轮量到的方式漂移：`tools/list` 说的与 `--list` 说的
/// 是关于同一个工具的同一句话，否则其中一句是错的。
fn catalogue_entry() -> Value {
    tool(
        "xirang_tools",
        "Discover and call all XiRang tools. No arguments: list names and keys. \
         `tool: \"read\"`: bounded manual; `full: true`: full manual. \
         To execute, pass `tool` AND `arguments`, e.g. \
         {\"tool\":\"read\",\"arguments\":{\"path\":\"src/lib.rs\"}}. \
         Writes retain the target tool's preview, apply and confirm rules.",
        json!({"type":"object","properties":{
            "tool":{"type":"string","description":"tool name; short names such as read are accepted"},
            "arguments":{"type":"object","description":"execute the named tool with these arguments; omit to read its manual"},
            "full":{"type":"boolean","description":"the whole manual"},
            "slim":{"type":"boolean","description":"with no tool: the entry tools"},
            "root":{"type":"string"}
        }}),
    )
}

const ADVERTISED_APPLY: &str = "\
**The only write path**. `add`/`deepen`/`cut`/`promote`/`move` are additive; \
`edit`/`rename`/`delete` rewrite generated faces only. Previewed unless `apply: true`.";

/// The slim schema of an advertised tool: the keys that decide the call, one line each.
/// One tool's implementation: the package root plus the `arguments` object.
/// 一个工具的实现：包根加上 `arguments` 对象。
type Handler = fn(&Path, &Value) -> Result<String, String>;

/// The dispatch table, in the catalog's order.
/// 分派表，顺序与目录一致。
///
/// This is one ordered table rather than a `match` because the two lists used to be
/// aligned only by hand: the catalog put `xirang.apply` sixth while the dispatch put
/// it last, and adding a tool to one side alone was silent — the missing arm only
/// surfaced at call time as `unknown tool` (audit `BR-12`).
/// `tools_tests::the_dispatch_table_follows_the_catalog` pins this table's names and
/// their order against `tools()`, so a tool added to the catalog without a handler, or
/// in another position, fails the suite instead of shipping.
/// 这里用一张有序表而不是 `match`，因为两张名单过去只靠手工对齐：目录把 `xirang.apply` 放在第 6，
/// 分派把它放在最后；而只往一侧新增工具是静默的——缺的那条臂只在调用时以 `unknown tool` 现身
/// （审计 `BR-12`）。`tools_tests::the_dispatch_table_follows_the_catalog` 用 `tools()` 钉住本表的
/// 名字与顺序：只往目录里加工具而漏掉处理函数、或把它放错位置，都会让测试失败而不是出厂。
const DISPATCH: &[(&str, Handler)] = &[
    ("xirang.explore", crate::mcp::explore::explore),
    ("xirang.search", search),
    ("xirang.digest", crate::mcp::digest::digest),
    ("xirang.conformance", crate::mcp::adopted::conformance),
    ("xirang.consistency", crate::mcp::consistency::consistency),
    ("xirang.why", crate::mcp::why::why),
    ("xirang.locate", crate::mcp::locate::locate),
    ("xirang.inspect", inspect),
    ("xirang.callgraph", callgraph),
    ("xirang.read", read_source),
    ("xirang.status", status_tool),
    ("xirang.apply", apply),
    ("xirang.new_project", new_project),
    ("xirang.plugin", plugin),
    ("xirang.registry", registry_tool),
    ("xirang.explain", explain_tool),
    ("xirang.diff", diff),
    ("xirang.trace", trace),
    ("xirang.mir", mir),
    ("xirang.unified", unified),
    ("xirang.grafts", grafts),
    ("xirang.impact", impact),
    ("xirang.usages", usages),
    ("xirang.converge", converge),
    ("xirang.adopted", adopted),
    ("xirang.affected", affected),
    ("xirang.check", check),
    ("xirang.candidate", crate::mcp::candidate::candidate),
    ("xirang.rules", crate::mcp::rules::rules),
    ("xirang.refactor", crate::mcp::refactor::refactor),
    ("xirang.task", crate::mcp::task::task),
    ("xirang.constraints", crate::mcp::constraints::constraints),
    ("xirang.verify", verify),
    ("xirang.graph", crate::mcp::graph::graph),
    ("xirang.crates", crate::mcp::crates::crates),
    ("xirang.multilang", multilang),
    ("xirang_tools", catalogue::describe),
];

/// `xirang.status` reads no arguments, so it joins the table through a shim.
/// `xirang.status` 不读参数，因此经一个转接函数进入分派表。
fn status_tool(root: &Path, _arguments: &Value) -> Result<String, String> {
    status(root)
}

/// `xirang.registry` reads one argument: `full`, which buys the per-member trees a virtual root's
/// default answer leaves out.
/// `xirang.registry` 只读一个参数：`full`，它买下虚拟根的默认答案省掉的那些逐成员树。
pub(crate) fn registry_tool(root: &Path, arguments: &Value) -> Result<String, String> {
    // The stage decides whether there is a tree to answer about at all, and it is checked here
    // because this is the entry both spellings share. Measured (T-12): on a directory with no
    // manifest this tool returned Cargo's own sentence about a missing file — naming the failure and
    // not the way to start — and this is one of the two calls an agent reaches for first.
    // 阶段决定这里到底有没有树可答，检查放在这里是因为它是两种拼法共用的入口。量到的（T-12）：在没有清单的
    // 目录上，这个工具回的是 Cargo 自己关于文件缺失的那句话——点名失败、不点名起步的方式——而它正是代理最先
    // 够到的两个调用之一。
    if let Some(entry) = crate::mcp::workspace::entry_point(crate::mcp::workspace::stage(root)) {
        // The line opens with `next` on purpose: `with_next_hint` adds the catalogue's own hint
        // only when the answer has none, and for a bare tree that hint ("explain one face's
        // contract") is about faces that do not exist yet.
        // 这一行有意以 `next` 开头：`with_next_hint` 只在答案里没有 hint 时才补目录里那条，而对一棵空树，
        // 那条（"解释某个面的契约"）说的是还不存在的面。
        return Ok(format!("no registration tree here yet\nnext   {entry}\n"));
    }
    if arguments.get("full").and_then(Value::as_bool) == Some(true) {
        // Audit `W2-1`: the rows are bought, and they arrive a page at a time so a 50,000-face tree
        // does not answer with a 5 MB list.
        // 审计 `W2-1`：行是买来的，而且一次一页，因此五万面的树不会回一份 5 MB 的清单。
        let offset = arguments.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let limit = arguments
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(crate::mcp::registry::PAGE_ROWS, |value| {
                (value as usize).clamp(1, crate::mcp::registry::PAGE_ROWS)
            });
        crate::mcp::registry::registry_page(root, offset, limit)
    } else {
        registry_brief(root)
    }
}

/// `xirang.explain` is the one name that routes to two implementations: `overlay:
/// true` renders the overlay projection, everything else the per-face report.
/// `xirang.explain` 是唯一会分到两个实现的名字：`overlay: true` 渲染覆盖投影，其余走逐面报告。
fn explain_tool(root: &Path, arguments: &Value) -> Result<String, String> {
    if arguments.get("overlay").and_then(Value::as_bool) == Some(true) {
        overlay(root, arguments)
    } else {
        explain(root, arguments)
    }
}

/// Run one tool by name, the single place a name becomes a handler.
/// 按名跑一个工具——名字变成处理函数的唯一位置。
///
/// The stdio bridge and the one-shot client both come through here, so a tool cannot answer two
/// different ways depending on which client asked.
/// stdio 桥与一次性客户端都从这里进，因此同一个工具不会因为"谁问的"而答出两种样子。
pub(crate) fn run_tool(root: &Path, name: &str, arguments: &Value) -> Result<String, String> {
    if !arguments.is_null() && !arguments.is_object() {
        return Err("tool arguments must be an object".into());
    }
    if let Some(value) = arguments.get("freshness")
        && !matches!(value.as_str(), Some("reuse" | "verify" | "reuse-only"))
    {
        return Err("freshness must be reuse, verify, or reuse-only".into());
    }
    // The gateway delegates before decoration so evidence describes the actual operation once.
    // 网关先委派再渲染，证据只记录一次实际执行的操作。
    if name == "xirang_tools" && arguments.get("arguments").is_some() {
        let target = arguments
            .get("tool")
            .and_then(Value::as_str)
            .ok_or("execution requires `tool` and an `arguments` object")?;
        let target = crate::mcp::client::resolve_name(target);
        if target == "xirang_tools" {
            return Err("the catalogue cannot invoke itself".to_owned());
        }
        if arguments.get("full").is_some() || arguments.get("slim").is_some() {
            return Err(
                "manual flags `full`/`slim` cannot accompany execution arguments".to_owned(),
            );
        }
        let nested = &arguments["arguments"];
        if !nested.is_object() {
            return Err("execution `arguments` must be an object".to_owned());
        }
        let root = resolve_root(root, nested.get("root").and_then(Value::as_str))?;
        return run_tool(&root, &target, nested);
    }
    crate::mcp::freshness::set_policy(crate::mcp::freshness::policy_from(arguments));
    // `target_dir` is a request-level path the same way `freshness` is a request-level policy: the
    // evidence directory follows the build's own target directory, and setting it here is what keeps
    // every reader from having to remember the argument. Item ⑤ of the 2026-10-10 queue is what
    // happens without it — a reader that reads `<tree>/target/xirang/out` no matter where the build
    // actually published (report §7h).
    // `target_dir` 与 `freshness` 一样是请求级的东西：证据目录跟着构建自己的 target 目录，而在这里设置
    // 它，就不必让每个读取方记得这个参数。没有它的后果正是 2026-10-10 队列第 ⑤ 条——无论构建发布到了
    // 哪里，读取方都去读 `<tree>/target/xirang/out`（报告 §7h）。
    crate::mcp::build_evidence::set_target_dir(arguments.get("target_dir").and_then(Value::as_str));
    let _sources = (name == "xirang.search").then(crate::mcp::source_request::scope);
    let _metadata = metadata::shares_reads(name).then(crate::cargo_metadata::scope);
    let _syntax = (!matches!(name, "xirang.check" | "xirang_tools"))
        .then(|| crate::source_cache::scope(root));
    let answer = match DISPATCH.iter().find(|(listed, _)| *listed == name) {
        Some((_, handler)) => crate::mcp::ownership::dispatch(root, name, arguments, *handler),
        None => Err(format!("unknown tool `{name}`")),
    };
    // W1.3: every read answer ends by naming the next call. The rounds measured that a separate
    // `--list` lookup is never made (0 calls in two rounds) while a next step written into the
    // answer is taken, so the hint rides here — the single place a name becomes a handler.
    // W1.3：每个读答案末尾点名"下一次调用"。两轮实测 `--list` 都是 0 次调用，而写进答案的下一步会被
    // 照做，因此这句话挂在这里 —— 名字变成处理函数的唯一位置。
    // T-23: the answer carries a line the caller can **quote** instead of re-running the call. The
    // measured reason is bookkeeping, not curiosity: the brief demands that every claim come with
    // the command, its raw output and its exit code, so the round's arm re-ran calls purely to have
    // something to cite — "I claimed a closing `status` call … I haven't run it. Let me run it", and
    // "I did NOT actually run explain … to make the claim true". Two calls spent on making a written
    // sentence true is a call the answer can save.
    // T-23：答案携带一行**可引用**的东西，让调用方不必为了引用而重跑。量出来的理由是记账而不是好奇：题面
    // 要求每条声称都给命令、原始输出与退出码，于是那一轮的臂**为了有东西可引**而补跑调用——原文「I claimed
    // a closing `status` call … I haven't run it. Let me run it」与「I did NOT actually run explain …
    // to make the claim true」。为让一句写下来的话成真而花掉两次调用，正是答案能替它省下的。
    //
    // The line is rendered **from the same `arguments` value the dispatch ran with**, so it cannot
    // describe a different call than the one that produced this answer; a refusal carries no line,
    // because the refusal text is its own evidence.
    // 这一行**由派发实际使用的那个 `arguments` 值**渲染，因此它不可能描述与产出本答案不同的另一次调用；
    // 被拒的调用不携带这一行——拒绝文案本身就是它的证据。
    answer.and_then(|text| {
        if arguments.get("jsonl").and_then(Value::as_bool) == Some(true) && text.len() > 48_000 {
            return Err("JSONL exceeds the 48000-byte reply cap; snapshot must be returned whole; capture/read a smaller compiler target".into());
        }
        let text = if name != "xirang_tools" {
            crate::mcp::truncation::bounded_bytes(&text, 48_000, "narrow root/scope or read the immutable result log")
        } else { text };
        if name == "xirang_tools" {
            return Ok(format!("documentation only; no project operation executed\n{text}"));
        }
        Ok(with_evidence(name, arguments, with_next_hint(name, text)))
    })
}

/// The quotable line: this call, and that it answered.
/// 可引用的那一行：这次调用，以及它作答了。
fn with_evidence(name: &str, arguments: &Value, text: String) -> String {
    const BUDGET: usize = 120;
    let mut request = arguments.to_string();
    if request.chars().count() > BUDGET {
        request = request.chars().take(BUDGET).collect::<String>() + "…";
    }
    let line = format!("evidence {name} {request} → exit 0");
    if text.ends_with('\n') {
        format!("{text}{line}\n")
    } else {
        format!("{text}\n{line}\n")
    }
}

/// Append the next-call hint, unless the answer already names one.
/// 追加"下一次调用"的提示，除非答案里已经点名了一次。
fn with_next_hint(name: &str, text: String) -> String {
    match next_hint(name) {
        Some(hint) if !text.lines().any(|line| line.starts_with("next")) => {
            if text.ends_with('\n') {
                format!("{text}{hint}")
            } else {
                format!("{text}\n{hint}")
            }
        }
        _ => text,
    }
}

/// The next call a read tool's answer should name, when it has one.
/// 一个读工具的答案应当点名的那次调用；没有就什么都不加。
fn next_hint(name: &str) -> Option<&'static str> {
    let hint = match name {
        "xirang.status" => {
            "next   `registry` lists the tree, `check` runs one face (`face` defaults to `default`; pass \
             `all` or a feature name for another). On a tree with **no registered face**, \
             \"the object\" means a *symbol*: go `search {query}` / `locate {symptom}` / \
             `read {path, line}` first\n"
        }
        "xirang.registry" => {
            // A tree with no registered face is the case the round measured: `registry` printed
            // `0 face(s) (no faces)` while this line still sent the reader to `explain {node}`, and
            // four steps and 37 KB went into working out what "the object" could mean. The line now
            // covers that branch itself.
            // 没有任何注册面的树就是那一轮量到的情形：`registry` 印 `0 face(s) (no faces)`，而这一行
            // 仍把读者送去 `explain {node}`，四步与 37 KB 花在猜"对象"指什么。这一行现在自己覆盖那一支。
            "next   `explain {node}` for one face's contract, `check {face}` for whether it builds; \
             on a tree with **no registered face** `explain` can only say `no such node` — there \
             \"the object\" means a *symbol*, so go `search {query}` / `locate {symptom}` / \
             `read {path, line}` first\n"
        }
        "xirang.crates" => {
            "next   `crates {action: \"declare\", crate: \"…\", subtree: \"crate::…::SUBTREE\"}` adds a crate to the declaration (it creates `add_crates.rs` when the host has none) and `undeclare` takes one out — both preview the exact text diff — while `crates {action: \"write\"}` (or `\"release\"`) previews the write — the reply \
             names what it would change — and the same call with `apply: true` writes it; `revert` \
             takes the generated packages back\n"
        }
        "xirang.explain" => {
            "next   `callgraph {function}` for its callers and callees, `read {path, line}` for the body\n"
        }
        "xirang.callgraph" => {
            "next   `read {path, line}` for a body, `affected {files}` for what depends on it\n"
        }
        "xirang.inspect" => {
            "next   `callgraph {function}` for its callers, `read {path, line}` for the body\n"
        }
        "xirang.affected" => {
            "next   `check {workspace:true, census:false}` runs conservative regression; select required features explicitly; candidates do not prove complete coverage\n"
        }
        _ => return None,
    };
    Some(hint)
}

/// The closure line: whether this answer is everything, and the calls left when it is not.
/// 闭合行：这个答案是不是全部；不是时还剩下哪些调用。
///
/// Audit `W4-5` measured both failure directions on the debug battery: an answer that stopped at a
/// deviation and said nothing about the rest left the agent guessing whether to keep going, and an
/// answer that always appended a `next` line made every answer look unfinished. So this is stated as
/// one of exactly two things, and it is judged **conservatively** — `closed` is written only when
/// every question the answer's own shape raises has been answered; anything the tool could not read,
/// or a deviation it named, is `open` with the call that resolves it.
/// 审计 `W4-5` 在 debug 电池上量到了两个方向的失败：一个停在一处偏离、对其余只字不提的答案，让代理猜
/// 要不要继续；而一个总是追加 `next` 行的答案，让每个答案看起来都没做完。因此这里只说两种之一，而且判定
/// **从保守**——只有"这个答案自己的形状会提出的每个问题都已被回答"时才写 `closed`；任何工具读不了的
/// 东西、或它点名的一处偏离，都是 `open` 并附上解决它的那次调用。
pub(crate) fn closure(closed: bool, remaining: &[String]) -> String {
    if closed {
        return "closure    closed — nothing this answer leaves open\n".to_owned();
    }
    let mut text = String::from("closure    open\n");
    if remaining.is_empty() {
        // An `open` with nothing to do would be the worst of both: it says "unfinished" and gives
        // no call. The invariant is checked rather than assumed, because a caller that forgets to
        // collect a command is exactly how that shape would ship.
        // 一个没有下文的 `open` 是两者中最糟的：它说"没做完"，又不给调用。这条不变量是**检查**出来的而
        // 不是假设的，因为"调用方忘了收集一条命令"正是那种形状出厂的方式。
        text.push_str("remaining  (nothing to run — this answer's own gaps are named above)\n");
        return text;
    }
    for command in remaining {
        text.push_str(&format!("remaining  {command}\n"));
    }
    text
}

pub(crate) fn tool_call(root: &Path, id: Value, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, -32602, "tools/call requires name".to_owned());
    };
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let requested_root = arguments.get("root").and_then(Value::as_str);
    let root = match resolve_root(root, requested_root) {
        Ok(root) => root,
        Err(error) => {
            return success(
                id,
                json!({ "content": [{"type":"text","text":error}], "isError": true }),
            );
        }
    };
    // The `freshness` argument is a request-level policy: it decides how much this one call is
    // willing to pay for a content verification. Setting it here — the single place every tool
    // enters through — is what keeps a body from having to remember it.
    // `freshness` 参数是**请求级**策略：它决定这一次调用愿意为内容核验付多少。在这里设置它——每个
    // 工具进入的唯一位置——就不必让每个主体自己记着这件事。
    crate::mcp::freshness::set_policy(crate::mcp::freshness::policy_from(arguments));
    crate::mcp::build_evidence::set_target_dir(arguments.get("target_dir").and_then(Value::as_str));
    let result = run_tool(&root, name, arguments);
    match result {
        Ok(value) => success(
            id,
            json!({ "content": [{"type":"text","text":value}], "isError": false }),
        ),
        Err(error) => success(
            id,
            json!({ "content": [{"type":"text","text":error}], "isError": true }),
        ),
    }
}

fn inspect(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = required_path(arguments)?;
    let file = load_one(root, &relative)?;
    let mut output = format!("file {}\n", file.relative);
    for function in &file.functions {
        output.push_str(&format!(
            "fn {} lines {}-{} calls=[{}]\n",
            function.name,
            function.line,
            function.end_line,
            function.calls.join(", ")
        ));
    }
    let registrations = xirang_kernel::source::registration_kinds(&file.source);
    if !registrations.is_empty() {
        output.push_str("registrations: ");
        output.push_str(&registrations.join(", "));
        output.push('\n');
    }
    if file.functions.is_empty() && registrations.is_empty() {
        output.push_str("no function or registration declaration found\n");
    }
    Ok(output)
}

pub(crate) fn status(root: &Path) -> Result<String, String> {
    let files = load_sources(root)?;
    let functions = files.iter().map(|file| file.functions.len()).sum::<usize>();
    // The census says how much tree was read; the faces say which face a red run would appear on.
    // They belong together because the blindness they describe is a fact about *this* tree: a file
    // compiled only under a non-default feature cannot fail on the default face, and a caller who
    // has just counted the files is the one about to run the tests.
    // 普查说读到了多少树；特性面说红会出现在哪个面。它们该在一起，因为它们描述的那种失明是关于**这棵**
    // 树的事实：只在非默认特性下编译的文件在默认面上不可能失败，而刚数完文件的调用方正是要去跑测试的人。
    let mut lines = vec![format!(
        "root {}\nrust_files={} functions={} tool=xirang-toolchain",
        root.display(),
        files.len(),
        functions
    )];
    lines.extend(crate::mcp::faces::faces_lines(root));
    // The line above answers "which **Cargo feature** faces exist"; this one answers "how many
    // **registration faces** are declared", derived the same way `registry` derives them, so the two
    // answers cannot contradict each other. The 2026-10-10 queue item ⑥ (4 reproductions) measured
    // the contradiction: this tool printed `faces … default=[] all=[]` while `registry` listed three
    // faces on the same tree, and the reader had no way to tell the two words apart (report §7h).
    // Both halves live in `faces.rs` so the two meanings sit side by side.
    // 上面那行答的是"有哪些 **Cargo 特性**面"，这一行答的是"声明了多少个**注册面**"，用 `registry`
    // 同一套推导，因此两个答案不可能互相矛盾。2026-10-10 队列第 ⑥ 条（4 次复现）量到的正是这对矛盾：
    // 本工具印 `faces … default=[] all=[]`，而 `registry` 在同一棵树上列出三个面，读者无从分辨这两个词。
    // 两半都住在 `faces.rs`，让两个意思并排放在一起。
    let (registered_line, registered) = crate::mcp::faces::registered_faces_line(root);
    lines.push(registered_line);
    let stage = crate::mcp::workspace::stage(root);
    if let Some(entry) = crate::mcp::workspace::entry_point(stage) {
        lines.push(format!("next   {entry}"));
    } else if let Some(count) = registered.filter(|count| *count > 0) {
        // An answer that already says what the tree is gets its own `next`, which is also what keeps
        // the catalogue's fallback hint (the one about a tree with **no registered face**) out of a
        // reply about a tree that has some — that sentence is the other half of item ⑥.
        // 已经说清这棵树是什么的答案，自己给一条 `next`；这也是把目录里那条兜底提示（"没有任何注册面的
        // 树"）挡在"有注册面的树"的回复之外的方式——那句话正是第 ⑥ 条的另一半。
        lines.push(format!(
            "next   `registry` lists the {count} registration face(s); `check` runs one feature face; `explain` reads one face's contract"
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
#[path = "tools_tests.rs"]
pub(crate) mod tools_tests;
