//! Source rendering for a registration face.
//! 注册面的源码渲染。

use std::path::Path;

use super::super::FaceManifest;
use crate::run_method::authoring::GENERATED_MARKER;
use crate::run_method::authoring::context::{normalized_path, rule_path_for_source, rust_string};
use crate::run_method::authoring::parse::*;

/// On-disk keys: the fields this renderer writes back into a face file — the whole
/// on-disk field set, listed once. The name says which of the two classes this list is,
/// so a maintainer can tell the writable set from the rest without reading the body.
/// 落盘键：本渲染器会写回注册面文件的字段——全部落盘字段，只列一次。名字本身说明了这份清单属于
/// 两类里的哪一类，因此维护者不必读正文就能把"会落盘的集合"与其余键分开。
///
/// `FaceManifest::values` carries three classes of key and marks none of them
/// (audit `LGC-LG-38`), so this list is the one place a maintainer can read what
/// actually lands on disk:
/// - the fields below: read here and re-emitted;
/// - keys derived from the file's own location, which are inputs to that
///   emission rather than fields — `source`, `parent_source`, `registry_rule_path`
///   and the `namespace`/`parent_node`/`parent_kind`/`provided_parts`/
///   `required_parts` metadata;
/// - keys no template emits — `registry_name`, `handle`, `params`, which `edit`
///   refuses by name instead of storing an edit nothing would write.
///
/// The round-trip pin below takes this list as the set of rows it must cover and
/// asserts the two are equal, so the renderer and its evidence cannot drift into
/// two half-lists.
/// `FaceManifest::values` 混装三类键且都不加标记（审计 `LGC-LG-38`），因此这份清单是维护者能读到
/// "真正落盘的是什么"的唯一位置：
/// - 下面这些字段：从这里读出并重新发射；
/// - 由文件自身位置派生的键，它们是那次发射的**输入**而不是字段——`source`、`parent_source`、
///   `registry_rule_path`，以及 `namespace`/`parent_node`/`parent_kind`/`provided_parts`/
///   `required_parts` 这些元数据；
/// - 任何模板都不发射的键——`registry_name`、`handle`、`params`，`edit` 会按名拒绝它们，而不是
///   存下一次没有任何东西会写出的编辑。
///
/// 下面的往返钉子把这份清单当作它必须覆盖的行集合，并断言两者相等，因此渲染器与它的证据不可能各自
/// 漂成半份清单。
const ON_DISK_FIELDS: &[&str] = &[
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "exports",
    "stable_name",
    "getting_from_other_registry",
    "requires",
    "provides",
    "runtime_checks",
    "flow",
    "flow_provider",
    "handle_traits",
    "handle_contracts",
    "part_traits",
    "part_contracts",
    "needs_registry",
    "registration_rule",
    "admission",
];

// Opaque Rust expressions survive editing without becoming editable form fields.
// 不透明 Rust 表达式在编辑时保留，但不成为可编辑表单字段。
const PRESERVED_FIELDS: &[&str] = &["plugin", "collector"];

/// Off-disk keys: the keys `values` carries that are not on-disk fields — inputs, derived
/// metadata, and the keys no template emits. The name states the class, mirroring
/// [`ON_DISK_FIELDS`]; nothing here lands on a file, and a key that no template emits is
/// refused by name instead of being stored.
/// 非落盘键：`values` 里并非落盘字段的键——输入、派生元数据，以及任何模板都不发射的键。名字写出
/// 了类别，与 [`ON_DISK_FIELDS`] 相对；这里没有任何东西会落盘，而任何模板都不发射的键会被按名
/// 拒绝，而不是被存下来。
///
/// They are listed beside [`ON_DISK_FIELDS`] so the partition is stated in one
/// place, and the pin checks that none of them slipped into the rendered set.
/// 它们与 [`ON_DISK_FIELDS`] 并列，好让这份划分只在一处陈述；钉子会核对它们没有混进渲染集合。
const OFF_DISK_KEYS: &[&str] = &[
    "admission_line",
    "declaration_line",
    "handle",
    // The identity the writer bakes into a `Registration`-literal face (`set_identity`): an input to the
    // emission, never an edit — and never a line the author types by hand.
    // 写者烘焙进 `Registration` 字面量形面的身份（`set_identity`）：它是那次发射的**输入**，不是一次编辑——
    // 也永远不是作者手写的一行。
    "id",
    "module",
    "namespace",
    "params",
    "parent_kind",
    "parent_module",
    "parent_node",
    "parent_reference",
    "parent_registry_name",
    "parent_source",
    "provided_parts",
    "registry_name",
    "registry_rule_path",
    // The rule's module chain, from the discovered tree: a derived input the renderer writes
    // verbatim (an island's rule is `reading::registry_rule`, its directories are `components/…`).
    // 规则的模块链，来自发现树：渲染器原样写出的派生输入（孤岛的规则是 `reading::registry_rule`，
    // 它的目录是 `components/…`）。
    "registry_rule_module",
    "required_parts",
    "root",
    "source",
    "written_as_literal",
];

impl FaceManifest {
    /// Render a face written as a `Registration` literal (H/A2e) back into that spelling.
    /// 把一个写成 `Registration` 字面量的面（H/A2e）按那种拼法渲染回去。
    ///
    /// Only the declaration is rendered: the identity, the type names and the contracts live outside it
    /// (`impl FaceContract` beside it, and the build's half), and the rewriter splices exactly the
    /// literal's expression, so nothing outside it is touched.
    /// 只渲染那份声明：身份、类型名与合同都在它之外（旁边的 `impl FaceContract`，以及构建那一半），而重写器
    /// 拼接的正是那个字面量的表达式，因此它之外的东西一个字都不会动。
    ///
    /// The contracts have a home here: `handle_traits` / `part_traits` are the associated constants of
    /// `impl FaceContract`, and the check "the face implements each listed trait" is the author-facing
    /// `assert_contracts!` line the new spelling ships with.
    /// 合同在这里有家：`handle_traits` / `part_traits` 是 `impl FaceContract` 的关联常量，而"面实现了列出的
    /// 每一个 trait"这条检查，就是新拼法自带的、给作者看的 `assert_contracts!` 那一行。
    fn render_literal(&self) -> Result<String, String> {
        let value = |key: &str| self.value_of(key);
        let kind = value("kind");
        let mut body = String::new();
        let parent = if !value("parent_reference").is_empty() {
            // A parent declared in another language has no Rust module path; the execution surface
            // derived the reference the generated `xirang_external` module exposes, and this
            // renderer writes it verbatim.
            // 声明在别的语言里的父没有 Rust 模块路径；执行面推导了生成的 `xirang_external` 模块
            // 暴露的引用，本渲染器原样写出它。
            value("parent_reference").to_owned()
        } else if value("parent_source") == "<root>" {
            "crate::root_node_id(crate::XIRANG_NAMESPACE)".to_owned()
        } else if !value("parent_module").is_empty() {
            // The execution surface handed the generated module address in; using it keeps the
            // declaration and the generated tree on the same string.
            // 执行面把生成的模块地址传了进来；用它才能让声明与生成树用同一个字符串。
            format!("crate::{}::NODE_ID", value("parent_module"))
        } else {
            let source = Path::new(value("parent_source"));
            let module = source
                .parent()
                .map_or_else(|| source.to_string_lossy().into_owned(), normalized_path);
            format!("crate::{}::NODE_ID", module.replace('/', "::"))
        };
        body.push_str(&format!("        parent: {parent},\n"));
        if !value("stable_name").is_empty() {
            body.push_str(&format!(
                "        stable_name: Some(\"{}\"),\n",
                rust_string(value("stable_name"))
            ));
        }
        body.push_str(&format!(
            "        name: LocalizedText {{ zh: \"{}\", en: \"{}\" }},\n",
            rust_string(value("name_zh")),
            rust_string(value("name_en"))
        ));
        body.push_str(&format!(
            "        summary: LocalizedText {{ zh: \"{}\", en: \"{}\" }},\n",
            rust_string(value("summary_zh")),
            rust_string(value("summary_en"))
        ));
        let exports = value("exports")
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| format!("\"{}\"", rust_string(item)))
            .collect::<Vec<_>>()
            .join(", ");
        if !exports.is_empty() {
            body.push_str(&format!("        exports: &[{exports}],\n"));
        }
        if value("needs_registry") == "true" {
            body.push_str("        needs_registry: true,\n");
        }
        if !value("getting_from_other_registry").is_empty() {
            body.push_str(&format!(
                "        getting_from_other_registry: Some(\"{}\"),\n",
                rust_string(value("getting_from_other_registry"))
            ));
        }
        if self.owns_rule_source() {
            body.push_str(&format!(
                "        registry_rule_path: \"{}\",\n",
                rust_string(&self.rule_module_path()?)
            ));
        } else {
            body.push_str("        registry_rule: crate::RegistrationRule::ANY,\n");
        }
        body.push_str(&format!(
            "        admission: {},\n",
            render_admission(value("admission"))?
        ));
        // The **literal** spelling has to write the struct the normalizer knows: `a => b` is the macro
        // spelling, and a literal face carrying it is not parseable at all (measured: the tool wrote
        // `requires: &["cap.render" => "Provider"]` and its own reader answered `30:34: expected ',' or ';'`).
        // **字面量**拼法必须写出归一化器认识的那个结构体：`a => b` 是宏拼法，字面量面带它是**根本解不开**的
        // （实测：工具写下 `requires: &["cap.render" => "Provider"]`，它自己的读者回 `30:34: expected ',' or ';'`）。
        let requirements = literal_requirements(value("requires"));
        if !requirements.is_empty() {
            body.push_str(&format!("        requires: &[{requirements}],\n"));
        }
        let provides = render_literal_list(value("provides"));
        if !provides.is_empty() {
            body.push_str(&format!("        provides: &[{provides}],\n"));
        }
        if !value("flow").is_empty() {
            body.push_str(&format!(
                "        flow: Some({}),\n",
                flow_contract_expression(value("flow"))?
            ));
        }
        if !value("flow_provider").is_empty() {
            body.push_str(&format!(
                "        flow_provider: Some(\"{}\"),\n",
                rust_string(value("flow_provider"))
            ));
        }
        let checks = render_literal_list(value("runtime_checks"));
        if !checks.is_empty() {
            body.push_str(&format!("        runtime_checks: &[{checks}],\n"));
        }
        if !value("plugin").is_empty() {
            body.push_str(&format!("        plugin: Some({}),\n", value("plugin")));
        }
        // The identity is **baked**, not computed: a hand-written face cannot hash its own path in a const
        // context, so whatever writes this file writes the id it already knows (see `set_identity`), and
        // the build's per-face identity assertion compares that constant against the identity it computes
        // — which is what turns a moved or renamed file red instead of silently re-keying every graft
        // record that names this face.
        // 身份是**烘焙**的、不是算出来的：手写的面在常量上下文里无法给自己的路径做哈希，因此写下这个文件的
        // 东西写下它已经知道的那个 id（见 `set_identity`），而构建期那句逐面身份断言会拿这个常量与自己算出的
        // 身份核对——正是这一步让"被移动或改名"当场变红，而不是静默地把每一条点名这个面的嫁接记录换了键。
        let identity = match self.values.get("id") {
            Some(bytes) if !bytes.is_empty() => format!(
                "/// {kind}'s identity, baked by the tool that wrote this face and asserted against every build.\n/// {kind} 的身份，由写下这个面的工具烘焙，并在每次构建时被断言核对。\npub const NODE_ID: NodeId = NodeId::from_raw([{bytes}]);\n\n"
            ),
            _ => String::new(),
        };
        let preset = if value("preset").is_empty() {
            "NoPreset"
        } else {
            value("preset")
        };
        let parts = if value("parts").is_empty() {
            "NoParts"
        } else {
            value("parts")
        };
        let label_list = |key: &str| {
            value(key)
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| format!("\"{}\"", rust_string(item)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        // The type-class half. It lives **outside** the declaration, which is why the rewriter has to own the
        // whole region (`replace_face_region`) rather than splice one expression.
        // 类型类那一半。它住在声明**之外**，这正是重写器必须拥有整段区域（`replace_face_region`）而不能只拼接
        // 一个表达式的原因。
        let type_class = format!(
            "impl FaceContract for {kind} {{\n    type Preset = {preset};\n    type Parts = {parts};\n    const HANDLE_TRAITS: &'static [&'static str] = &[{}];\n    const PART_TRAITS: &'static [&'static str] = &[{}];\n}}\n",
            label_list("handle_traits"),
            label_list("part_traits")
        );
        // The check the macro spelling used to hide inside its expansion: an author-facing assertion that the
        // kind implements every contract this face names. Emitted only when there is something to assert.
        // 宏拼法过去藏在展开里的那条检查：一句给作者看的断言——这个 kind 实现了本面点名的每一份合同。
        // 只有在确实有东西可断言时才发出。
        // The assertion is the **same statement** as the associated constants above, so it is read from the
        // same two fields: the reader refuses a face whose labels and whose `assert_contracts!` line name
        // different traits ('two statements of one fact'), and building both from one source keeps them equal
        // by construction.
        // 这句断言与上面的关联常量是**同一句话**，因此从同样的两个字段取材：读者会拒绝'标签与
        // `assert_contracts!` 行点名了不同 trait'的面（'同一件事的两句话'），两者同源即天然相等。
        // The assertion names **trait paths**, the constants hold **string labels**: the reader compares the
        // path's last segment against the label, so the same field is rendered twice — quoted for the
        // constants, bare for the assertion.
        // 断言里写的是 **trait 路径**，关联常量里放的是**字符串标签**：读者拿路径的末段与标签比对，因此同一个
        // 字段渲染两次——常量带引号、断言不带。
        let path_list = |key: &str| {
            value(key)
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let asserted = [path_list("handle_traits"), path_list("part_traits")]
            .into_iter()
            .filter(|list| !list.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        let assertion = if asserted.is_empty() {
            String::new()
        } else {
            format!("\nxirang_toolchain::assert_contracts!({kind}: {asserted});\n")
        };
        // A rendered face is a **whole file**, the way the macro branch renders one: the author's own items
        // (module docs, imports, the marker type) live **outside** the generated block, and the block itself
        // is what a rewrite replaces — `replace_face_region` splices exactly that span, so an edit never
        // duplicates the struct or the imports, and a creation always writes them.
        // 渲染出来的是一个面的**整份文件**，与宏分支一致：作者自己的条目（模块文档、导入、标记类型）住在**生成块
        // 之外**，而重写替换的正是那个块——`replace_face_region` 拼接的恰是这一段，因此编辑永远不会重复 struct 或
        // 导入，而新建永远会写下它们。
        let toolchain = xirang_kernel::lexicon::RUN_METHOD_CRATE;
        let doc = {
            let name = if value("name_en").is_empty() {
                kind.to_owned()
            } else {
                value("name_en").to_owned()
            };
            format!("//! The `{name}` registration face.\n//! `{name}` 注册面。")
        };
        let generated = format!(
            "{identity}{type_class}\nimpl {kind} {{\n    /// The face's declaration, written the way this file writes it.\n    /// 本面的声明，按本文件所用的拼法书写。\n    pub const REGISTRATION: Registration = Registration {{\n{body}        ..Registration::DEFAULT\n    }};\n}}\n{assertion}"
        );
        Ok(format!(
            "{doc}\n\nuse {toolchain}::run_method::registry_core::{{LocalizedText, NoParts, NoPreset, NodeId}};\nuse {toolchain}::run_method::{{FaceContract, Registration}};\n\npub struct {kind};\n\n{generated}"
        ))
    }

    /// One key of `values`, empty when absent — the read both renderings share.
    /// `values` 里的一个键，缺失时为空串——两种渲染共用这一次读取。
    fn value_of(&self, key: &str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or("")
    }

    pub(crate) fn render_source(&self) -> Result<String, String> {
        let value = |key| self.values.get(key).map(String::as_str).unwrap_or("");
        if value("written_as_literal") == "true" {
            return self.render_literal();
        }
        // Refuse unknown keys instead of silently dropping data on rewrite.
        // 重写时拒绝未知键，避免静默丢失数据。
        if let Some(unknown) = self.values.keys().find(|key| {
            !ON_DISK_FIELDS.contains(&key.as_str())
                && !OFF_DISK_KEYS.contains(&key.as_str())
                && !PRESERVED_FIELDS.contains(&key.as_str())
        }) {
            return Err(format!(
                "this face carries `{unknown}`, which no template emits and the editor cannot \
                 rewrite; edit that line in the file by hand"
            ));
        }
        let kind = value("kind");
        let preset = if value("preset").is_empty() {
            "NoPreset"
        } else {
            value("preset")
        };
        let parts = if value("parts").is_empty() {
            "NoParts"
        } else {
            value("parts")
        };
        let module_doc = if value("needs_registry") == "true" {
            format!(
                "//! {kind} face and its recursively requested registry.\n//! {kind} 注册面及其递归申请的注册机。"
            )
        } else {
            format!("//! {kind} registration face.\n//! {kind} 注册面。")
        };
        let handle_doc = format!(
            "/// Registration-only marker for the {kind} face.\n/// 仅用于 {kind} 注册面的 handle 标记，不代表运行时 object 实现。"
        );
        let registration_rule = if self.owns_rule_source() {
            // The module chain the execution surface handed in wins over the path-derived spelling:
            // an island's directories and its module chain differ, and the derived spelling named a
            // module that does not exist (`crate::components::reading::…`).
            // 执行面交进来的模块链胜过从路径推导的拼写：孤岛的目录与模块链不同，而推导出的拼写命名了一个
            // 不存在的模块（`crate::components::reading::…`）。
            match self.values.get("registry_rule_module") {
                Some(module) if !module.is_empty() => {
                    format!("crate::{module}::REGISTRATION_RULE")
                }
                _ => self.rule_module_path()?,
            }
        } else {
            "crate::RegistrationRule::ANY".to_owned()
        };
        let parent = if !value("parent_reference").is_empty() {
            // A parent declared in another language has no Rust module path; the execution surface
            // derived the reference the generated `xirang_external` module exposes, and this
            // renderer writes it verbatim.
            // 声明在别的语言里的父没有 Rust 模块路径；执行面推导了生成的 `xirang_external` 模块
            // 暴露的引用，本渲染器原样写出它。
            value("parent_reference").to_owned()
        } else if value("parent_source") == "<root>" {
            "crate::root_node_id(crate::XIRANG_NAMESPACE)".to_owned()
        } else if !value("parent_module").is_empty() {
            // The execution surface handed the generated module address in; using it keeps the
            // declaration and the generated tree on the same string.
            // 执行面把生成的模块地址传了进来；用它才能让声明与生成树用同一个字符串。
            format!("crate::{}::NODE_ID", value("parent_module"))
        } else {
            let source = Path::new(value("parent_source"));
            let module = source
                .parent()
                .map_or_else(|| source.to_string_lossy().into_owned(), normalized_path);
            format!("crate::{}::NODE_ID", module.replace('/', "::"))
        };
        // The declaration macro names the registry that owns this face. The
        // generated aliases are emitted by the build crate from the folder
        // tree; `__xirang_object!` remains their single hidden implementation.
        // 注册声明的宏名表达当前注册面所属的父注册机。别名由 build crate
        // 根据文件夹树生成，`__xirang_object!` 仍是唯一隐藏实现。
        let object_macro = if value("parent_source") == "<root>" {
            "root_object".to_owned()
        } else {
            let source = Path::new(value("parent_source"));
            let module = source
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("registry");
            format!("{module}_object")
        };
        let admission = render_admission(value("admission"))?;
        let exports = value("exports")
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| format!("\"{}\"", rust_string(item)))
            .collect::<Vec<_>>()
            .join(", ");
        let exports_decl = if exports.is_empty() {
            String::new()
        } else {
            format!("    exports: [{exports}],\n")
        };
        let handle_traits = render_face_list("handle_traits", value("handle_traits"));
        let handle_contracts = render_path_list(value("handle_contracts"));
        let handle_contracts_decl = if handle_contracts.is_empty() {
            String::new()
        } else {
            format!("    handle_contracts: [{handle_contracts}],\n")
        };
        let part_traits = render_face_list("part_traits", value("part_traits"));
        let part_contracts = render_path_list(value("part_contracts"));
        let part_contracts_decl = if part_contracts.is_empty() {
            String::new()
        } else {
            format!("    part_contracts: [{part_contracts}],\n")
        };
        // The strict entry, because this result is written back into the author's
        // file: an entry the lossy published path would drop has to refuse the
        // rewrite instead of deleting a field the author wrote.
        // 严格入口：这个结果会被写回作者的文件，因此有损入口会丢掉的条目必须让这次重写失败，而不是
        // 删掉作者写下的一个字段。
        let requirements =
            try_render_requirements(value("requires")).map_err(|error| error.to_string())?;
        let provides = render_literal_list(value("provides"));
        let runtime_checks = render_expression_list(value("runtime_checks"))?;
        let flow = render_flow_expression(value("flow"))?;
        let flow_provider = render_flow_provider(value("flow_provider"))?;
        let getting = render_optional_source(value("getting_from_other_registry"))?;
        let registry_rule_path = if value("registry_rule_path").is_empty() {
            rule_path_for_source(value("source"))
        } else {
            value("registry_rule_path").to_owned()
        };
        let stable_decl = if value("stable_name").is_empty() {
            String::new()
        } else {
            format!(
                "\x20   stable_name: \"{}\",\n",
                rust_string(value("stable_name"))
            )
        };
        let name_zh = if value("name_zh").is_empty() {
            kind
        } else {
            value("name_zh")
        };
        let name_en = if value("name_en").is_empty() {
            kind
        } else {
            value("name_en")
        };
        let name_decl = if name_zh == kind && name_en == kind {
            String::new()
        } else {
            format!(
                "    name: {{ zh: \"{}\", en: \"{}\" }},\n",
                rust_string(name_zh),
                rust_string(name_en)
            )
        };
        let summary_decl = if value("summary_zh").is_empty() && value("summary_en").is_empty() {
            String::new()
        } else {
            format!(
                "    summary: {{ zh: \"{}\", en: \"{}\" }},\n",
                rust_string(value("summary_zh")),
                rust_string(value("summary_en"))
            )
        };
        // Keep structural type markers explicit; prose and identity fields may
        // use defaults without losing the contract surface.
        // 保留结构类型标记；说明文字和身份字段可以使用默认值。
        let preset_decl = if !is_default_type(preset, "NoPreset") {
            format!("    preset: {preset},\n")
        } else {
            String::new()
        };
        let parts_decl = if !is_default_type(parts, "NoParts") {
            format!("    parts: {parts},\n")
        } else {
            String::new()
        };
        let custom_shape =
            !is_default_type(preset, "NoPreset") || !is_default_type(parts, "NoParts");
        let preset_decl = if custom_shape && preset_decl.is_empty() {
            format!("    preset: {preset},\n")
        } else {
            preset_decl
        };
        let parts_decl = if custom_shape && parts_decl.is_empty() {
            format!("    parts: {parts},\n")
        } else {
            parts_decl
        };
        let needs_decl = if value("needs_registry") == "true" {
            "    needs_registry: true,\n".to_owned()
        } else {
            String::new()
        };
        // Keep the parent visible even for root faces. The macro still has a
        // root fallback for hand-written declarations, but generated faces
        // should show their registration target explicitly.
        // 即使父级是 root 也保留 parent 字段。手写声明仍可使用宏的 root
        // 默认值，但生成注册面应明确展示自己的挂载目标。
        let parent_decl = format!("    parent: {parent},\n");
        let canonical_rule_path = rule_path_for_source(value("source"));
        let registry_fields = if self.owns_rule_source() {
            let canonical = registry_rule_path == canonical_rule_path;
            let rule_path = if canonical {
                String::new()
            } else {
                format!(
                    "    registry_rule_path: \"{}\",\n",
                    rust_string(&registry_rule_path)
                )
            };
            // A face that owns a registry derives its rule from the canonical
            // module beside it, so writing the full path back would repeat the
            // face's own location in every declaration. A leaf face with a custom
            // rule still names it: the default for a face that owns no registry is
            // permissive, not the sibling rule.
            // 拥有注册机的面从旁边的规范模块推导规则，因此把完整路径写回去等于在每份声明里
            // 重复注册面自己的位置。带自定义规则的叶子面仍需写出它：不拥有注册机的面默认是
            // 宽松规则，而不是同目录规则。
            let derived = canonical && self.derives_rule_from_the_sibling();
            let rule = if derived {
                String::new()
            } else {
                format!("    registry_rule: {registration_rule},\n")
            };
            format!("{rule_path}{rule}")
        } else {
            String::new()
        };
        let getting_decl = if !getting.is_empty() && getting != "None" {
            format!("    getting_from_other_registry: {getting},\n")
        } else {
            String::new()
        };
        let admission_decl = if value("admission").is_empty() || value("admission") == "ANY" {
            String::new()
        } else {
            format!("    admission: {admission},\n")
        };
        let requirements_decl = if !requirements.is_empty() {
            format!("    requires: [{requirements}],\n")
        } else {
            String::new()
        };
        let provides_decl = if !provides.is_empty() {
            format!("    provides: [{provides}],\n")
        } else {
            String::new()
        };
        let runtime_decl = if !runtime_checks.is_empty() {
            format!("    runtime_checks: [{runtime_checks}],\n")
        } else {
            String::new()
        };
        // The declaration comes **first** in the file, right after the module docs and the imports:
        // a reader (and an agent) should see what this face *is* before reading what it does, and the
        // marker type it names follows immediately below. Items are order-independent in Rust, so the
        // marker type does not have to precede the macro that names it — measured on the example host:
        // moving the declaration above `pub struct Slider;` compiles and leaves every identity byte
        // identical.
        // 声明写在文件的**最前面**，紧跟模块文档与导入：读者（以及代理）应当先看到这个面**是什么**，再读它做什么；
        // 它点名的标记类型紧随其后。Rust 条目与顺序无关，因此标记类型不必写在点名它的宏之前——在示例宿主上实测：
        // 把声明移到 `pub struct Slider;` 之前照样编译，且每个身份逐字节不变。
        let plugin_decl = if value("plugin").is_empty() {
            String::new()
        } else {
            format!("    plugin: {},\n", value("plugin"))
        };
        let collector = if value("collector").is_empty() {
            String::new()
        } else {
            format!("    collector: {},\n", value("collector"))
        };
        let source = format!(
            "{module_doc}\n\nuse crate::{{NoParts, NoPreset}};\n\ncrate::{object_macro}! {{\n{collector}    kind: {kind},\n{preset_decl}{parts_decl}{name_decl}{summary_decl}{exports_decl}{stable_decl}{needs_decl}{parent_decl}{getting_decl}{registry_fields}{admission_decl}{handle_traits}{handle_contracts_decl}{part_traits}{part_contracts_decl}{requirements_decl}{provides_decl}{flow}{flow_provider}{plugin_decl}{runtime_decl}}}\n\n{handle_doc}\npub struct {kind};\n"
        );
        xirang_kernel::syntax::render_native_face(&format!("{GENERATED_MARKER}\n{source}"))
            .map_err(|error| error.to_string())
    }
}

fn is_default_type(value: &str, default: &str) -> bool {
    let value = value.trim();
    value == default || value.rsplit("::").next() == Some(default)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;

/// The literal spelling of a requirement list: `capability => provider` becomes
/// `RequirementSpec { capability: "…", provider: "…" }`, which is what the reader normalizes back. An
/// entry with no `=>` is passed through untouched rather than guessed at.
/// 需求列表的字面量拼法：`capability => provider` 变成
/// `RequirementSpec { capability: "…", provider: "…" }`，正是读者会归一化回去的那一形。没有 `=>` 的条目
/// 原样透过，而不是去猜。
fn literal_requirements(raw: &str) -> String {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| match item.split_once("=>") {
            Some((capability, provider)) => format!(
                "RequirementSpec {{ capability: \"{}\", provider: \"{}\" }}",
                rust_string(capability.trim()),
                rust_string(provider.trim())
            ),
            None => item.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
