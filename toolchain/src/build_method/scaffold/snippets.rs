//! Editor snippet generation for registration-face fields, in the **default** author interface.
//! 注册面字段的编辑器 snippet 生成，按**默认**作者接口。
//!
//! Why these still exist, in one sentence: every editor's field completion inserts the bare name and leaves the
//! `: ` (or the `.`) to you, and the two spellings write their fields in different places — a `Soil` builder
//! chain under `#[xirang_toolchain::soil]`, whose method names live inside a macro's token tree (where a
//! language server's snippet never fires), and an ordinary `Registration { … }` struct literal. The snippets
//! therefore spell out the **default** spelling's shapes: one builder call per field, plus `.preset::<T>()` /
//! `.parts::<T>()` for the type-like half. The literal spelling stays readable and writable for compatibility
//! (maintainer ruling 2026-10-16) and its shapes are kept as a test-scoped record, not as snippets.
//! 它们为什么还在，一句话：所有编辑器的字段补全都只插裸名字、把 `: `（或 `.`）留给你，而两种拼法把字段写在不同
//! 的地方——`#[xirang_toolchain::soil]` 下的 `Soil` 建造器链（方法名住在宏的 token 树里，语言服务器的 snippet
//! 在那里不会触发）与普通 `Registration { … }` 结构体字面量。因此这些片段写出的是**默认**拼法的形状：每个字段
//! 一次建造器调用，类型类那一半用 `.preset::<T>()` / `.parts::<T>()`。字面量拼法保留可读可写（维护者
//! 2026-10-16 裁定），它的形状作为测试期记录保留，而不是作为片段发出去。
//!
//! A snippet exists only for what the **author** writes: `kind` is the type name, `source`/`id`/`namespace` are
//! the build's half, and `registry_name` is build-supplied too — so none of them has one.
//! 只有**作者**写的字段才有片段：`kind` 就是类型名，`source`/`id`/`namespace` 属于构建那一半，`registry_name`
//! 同样是构建供给的——因此它们都没有。

use std::fs;
use std::path::Path;

/// The value-like fields of `Registration`, in the order the struct declares them.
/// `Registration` 的值类字段，按结构体声明的顺序。
///
/// The pin below proves these are real field names by constructing a literal out of them.
/// 下面的钉子用这些名字构造一个字面量，从而证明它们是真的字段名。
const REGISTRATION_FIELDS: &[&str] = &[
    "parent",
    "stable_name",
    "name",
    "summary",
    "exports",
    "needs_registry",
    "registry_name",
    "getting_from_other_registry",
    "registry_rule_path",
    "registry_rule",
    "admission",
    "requires",
    "provides",
    "flow",
    "flow_provider",
    "runtime_checks",
    "plugin",
];

// The type-like half a face writes on its `FaceContract` implementation.
// 一个面写在 `FaceContract` 实现上的类型类那一半。
//
// `handle_contracts`/`part_contracts` are deliberately absent: in the new spelling they are still the one
// declaration whose home is undecided (the check that "the face implements each listed trait" has no
// macro-free equivalent), so no snippet promises a shape for them yet.
// `handle_contracts`/`part_contracts` **有意不在**：在新拼法里，它们仍是那唯一一处"家还没定"的声明
// （"面实现了列出的每一个 trait"这条检查没有无需宏的等价物），因此还没有片段替它们承诺形状。

/// The two type-like shapes, written as associated types rather than fields.
/// 两个类型类形状，写成关联类型而不是字段。
const TYPE_LIKE: &[(&str, &str)] = &[
    ("Preset", "type Preset = $1;"),
    ("Parts", "type Parts = $1;"),
];

/// The **default** shapes: the `Soil` builder chain (`#[xirang_toolchain::soil]`), which is the maintainer's
/// default author interface (ruling 2026-10-16). Each entry is one method call, because that is what an author
/// writes — the attribute preserves the declaration and derives identity, contract checks and `REGISTRATION`.
/// **默认**形状：`Soil` 建造器链（`#[xirang_toolchain::soil]`），即维护者 2026-10-16 裁定的默认作者接口。
/// 每一项是一次方法调用，因为作者写的就是它——属性保留声明并派生身份、合同检查与 `REGISTRATION`。
const SOIL_SHAPES: &[(&str, &str)] = &[
    ("parent", ".under($1)"),
    ("stable_name", ".stable_name(\"$1\")"),
    ("name", ".name(\"$2\", \"$1\")"),
    ("summary", ".summary(\"$2\", \"$1\")"),
    ("exports", ".exports(&[\"$1\"])"),
    ("needs_registry", ".needs_registry($1)"),
    (
        "getting_from_other_registry",
        ".getting_from_other_registry(Some(\"$1\"))",
    ),
    ("registry_rule_path", ".registry_rule_path(\"$1\")"),
    ("registry_rule", ".registry_rule($1)"),
    ("admission", ".admission($1)"),
    ("requires", ".requires(\"$1\", \"$2\")"),
    ("provides", ".provides(&[\"$1\"])"),
    ("flow", ".flow($1)"),
    ("flow_provider", ".flow_provider::<$1>()"),
    ("runtime_checks", ".runtime_checks(&[$1])"),
    (xirang_kernel::lexicon::FACE_FIELD_PLUGIN, ".plugin($1)"),
];

/// The type-like half in the default spelling: two builder calls, not associated types.
/// 默认拼法里的类型类那一半：两次建造器调用，而不是关联类型。
const SOIL_TYPE_LIKE: &[(&str, &str)] =
    &[("Preset", ".preset::<$1>()"), ("Parts", ".parts::<$1>()")];

/// Compatibility record: the `Registration` struct-literal spelling (H/A2e) stays readable and writable — an
/// existing literal face keeps its spelling on edit — but nothing this generator emits uses these shapes any
/// more (maintainer ruling 2026-10-16), so they are kept as a **test-scoped** record and pinned below rather
/// than offered as snippets. `registry_name` **is** here: the literal `Registration` really has that field,
/// even though the chain builder deliberately does not (it is build-supplied).
/// 兼容记录：`Registration` 结构体字面量拼法（H/A2e）保留可读可写——已有的字面量面在编辑时保持自己的拼法——
/// 但本生成器发射的东西已不再使用这些形状（维护者 2026-10-16 裁定），因此它们作为**测试期**记录保留、由下面的
/// 钉子守着，而不是作为片段发出去。`registry_name` **在这里**：字面量 `Registration` 确实有这个字段，尽管链式
/// 建造器有意没有它（它是构建供给的）。
#[cfg(test)]
const FIELD_SHAPES: &[(&str, &str)] = &[
    ("parent", "parent: $1,"),
    ("stable_name", "stable_name: Some(\"$1\"),"),
    ("name", "name: LocalizedText { zh: \"$1\", en: \"$2\" },"),
    (
        "summary",
        "summary: LocalizedText { zh: \"$1\", en: \"$2\" },",
    ),
    ("exports", "exports: &[\"$1\"],"),
    ("needs_registry", "needs_registry: $1,"),
    ("registry_name", "registry_name: \"$1\","),
    (
        "getting_from_other_registry",
        "getting_from_other_registry: Some(\"$1\"),",
    ),
    ("registry_rule_path", "registry_rule_path: \"$1\","),
    ("registry_rule", "registry_rule: $1,"),
    ("admission", "admission: $1,"),
    ("requires", "requires: &[$1],"),
    ("provides", "provides: &[\"$1\"],"),
    ("flow", "flow: Some($1),"),
    ("flow_provider", "flow_provider: Some(\"$1\"),"),
    ("runtime_checks", "runtime_checks: &[$1],"),
    (
        xirang_kernel::lexicon::FACE_FIELD_PLUGIN,
        "plugin: Some($1),",
    ),
];

/// One snippet name per shape, in the order a face writes them.
/// 每个形状一个 snippet 名，按一个面书写它们的顺序。
fn snippet_names() -> impl Iterator<Item = &'static str> {
    REGISTRATION_FIELDS
        .iter()
        .copied()
        .chain(TYPE_LIKE.iter().map(|(name, _)| *name))
}

/// Fields the **build** supplies, so no author snippet may promise a method for them. `registry_name` is one of
/// the two build-supplied keys (the kernel's `registration.rs` classifies it there), no template emits it, and
/// `render.rs` refuses an edit that names it — so the default spelling has no builder method for it, and a
/// snippet offering `.registry_name(…)` would promise a call that does not compile.
/// **构建**供给的字段，因此任何作者片段都不许为它承诺一个方法。`registry_name` 就是那两个构建供给键之一
/// （内核 `registration.rs` 把它归在那里），没有任何模板发射它，`render.rs` 也会按名拒绝点名它的编辑——因此
/// 默认拼法里没有它的建造器方法，而一个提供 `.registry_name(…)` 的片段会承诺一次编译不过的调用。
const BUILD_SUPPLIED: &[&str] = &["registry_name"];

/// The template for one name in the **default** spelling (`Soil` builder chain). `None` means the default
/// spelling does not author this field at all (see `BUILD_SUPPLIED`). Unknown names keep a permissive fallback
/// so a name added here before its shape is still usable (the test below keeps that from going unnoticed).
/// 某个名字在**默认**拼法（`Soil` 建造器链）里的模板。`None` 表示默认拼法根本不写这个字段（见
/// `BUILD_SUPPLIED`）。未知名字保留宽容回退：先加名字、后写形状时仍有可用形状（下面的测试会让这种遗漏无法悄悄
/// 存在）。
fn field_shape(field: &str) -> Option<String> {
    if BUILD_SUPPLIED.contains(&field) {
        return None;
    }
    Some(
        SOIL_TYPE_LIKE
            .iter()
            .chain(SOIL_SHAPES.iter())
            .find(|(name, _)| *name == field)
            .map_or_else(|| format!(".{field}($1)"), |(_, shape)| (*shape).to_owned()),
    )
}

/// Turn a template into LuaSnip nodes: literal text plus `i(n)` tab stops.
/// 把模板转成 LuaSnip 节点：字面文本加 `i(n)` 跳转位。
fn luasnip_body(shape: &str) -> String {
    let mut nodes = Vec::new();
    let mut literal = String::new();
    let mut chars = shape.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '$' && chars.peek().is_some_and(char::is_ascii_digit) {
            let mut digits = String::new();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                digits.push(chars.next().expect("peeked digit"));
            }
            if !literal.is_empty() {
                nodes.push(format!("t(\"{}\")", lua_escape(&literal)));
                literal.clear();
            }
            nodes.push(format!("i({digits})"));
        } else {
            literal.push(character);
        }
    }
    if !literal.is_empty() {
        nodes.push(format!("t(\"{}\")", lua_escape(&literal)));
    }
    nodes.join(", ")
}

/// Escape literal text for a Lua string.
/// 把字面文本转义成 Lua 字符串。
fn lua_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Which editor the snippet file is written for.
/// snippet 文件写给哪个编辑器。
///
/// The file is the last resort for the `: ` after a face field name: every
/// editor's field completion inserts the bare name (rust-analyzer does this for
/// plain structs too), and a language-server snippet does not fire inside a
/// macro call's token tree, so the colon only comes from the editor's own
/// snippet layer — which speaks a different format per editor.
/// 这份文件是注册面字段名后那个 `: ` 的最后办法：所有编辑器的字段补全都只插裸名字
/// （rust-analyzer 对普通结构体也一样），而语言服务器的 snippet 在宏调用的 token 树里
/// 不会触发，因此冒号只能来自编辑器自己的 snippet 层——而每个编辑器的格式不同。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Editor {
    /// VS Code (and forks that read project-scoped `.vscode` files).
    /// VS Code（以及读取项目级 `.vscode` 文件的衍生编辑器）。
    Vscode,
    /// Neovim with LuaSnip, as NvChad and most distributions configure it.
    /// 使用 LuaSnip 的 Neovim（NvChad 与多数发行版的默认装配）。
    Nvim,
    /// Neovim with blink.cmp's default (native `vim.snippet`) provider, which is
    /// what a Nix-packaged Neovim such as nvf ships. It reads VS Code-format
    /// JSON from `<config>/snippets/<filetype>/`.
    /// 使用 blink.cmp 默认（原生 `vim.snippet`）snippet 源的 Neovim，也就是 nvf 这类
    /// Nix 打包 Neovim 的默认装配；它读 `<config>/snippets/<filetype>/` 下的 VS Code
    /// 格式 JSON。
    Blink,
}

impl Editor {
    /// Every editor this writer can inject snippets for, in CLI order.
    /// 本写入方能为哪些编辑器注入 snippet，按命令行顺序。
    pub const ALL: [Editor; 3] = [Editor::Vscode, Editor::Nvim, Editor::Blink];

    /// The CLI spelling of an editor name.
    /// 编辑器名在命令行里的写法。
    pub fn parse_editor(name: &str) -> Option<Self> {
        match name {
            "vscode" | "code" => Some(Editor::Vscode),
            "nvim" | "neovim" => Some(Editor::Nvim),
            "blink" | "blink.cmp" => Some(Editor::Blink),
            _ => None,
        }
    }

    /// The CLI spelling of this editor.
    /// 该编辑器在命令行里的写法。
    pub fn name(self) -> &'static str {
        match self {
            Editor::Vscode => "vscode",
            Editor::Nvim => "nvim",
            Editor::Blink => "blink",
        }
    }

    /// The file name inside the editor's snippet directory.
    /// 编辑器 snippet 目录里的文件名。
    pub fn file_name(self) -> &'static str {
        match self {
            Editor::Vscode => "xirang-face.code-snippets",
            Editor::Nvim => "xirang-face.lua",
            Editor::Blink => "xirang-face.json",
        }
    }
}

/// Where VS Code reads project-scoped snippets from, relative to the package.
/// VS Code 从项目里读取项目级 snippet 的相对路径。
pub const SNIPPET_FILE: &str = ".vscode/xirang-face.code-snippets";

/// Where Neovim's LuaSnip loader finds Rust snippets, relative to the nvim
/// config directory.
/// Neovim 的 LuaSnip 加载器寻找 Rust snippet 的相对路径（相对 nvim 配置目录）。
///
/// LuaSnip derives the filetype from the directory directly below `luasnippets`,
/// so `luasnippets/rust/` is Rust-only and leaves a hand-written
/// `luasnippets/rust.lua` untouched. NvChad already calls
/// `luasnip.loaders.from_lua.load()`, which scans the runtimepath for exactly
/// this directory, so no config change is needed to pick the file up.
/// LuaSnip 用 `luasnippets` 下一层目录名作为 filetype，因此 `luasnippets/rust/`
/// 只作用于 Rust，也不会碰到手写的 `luasnippets/rust.lua`。NvChad 启动时已经调用
/// `luasnip.loaders.from_lua.load()`，它正是扫描 runtimepath 里的这个目录，因此无需改配置
/// 就能生效。
pub const NVIM_SNIPPET_FILE: &str = "luasnippets/rust/xirang-face.lua";

/// Where blink.cmp's default snippet provider looks for Rust snippets, relative
/// to the Neovim config directory.
/// blink.cmp 默认 snippet 源寻找 Rust snippet 的相对路径（相对 Neovim 配置目录）。
///
/// Its registry takes the directory name under `snippets/` as the filetype and
/// parses VS Code-format JSON, so the same file we write for VS Code works here.
/// 它把 `snippets/` 下的目录名当作 filetype 并解析 VS Code 格式的 JSON，因此给 VS Code
/// 写的那份文件在这里同样可用。
pub const BLINK_SNIPPET_FILE: &str = "snippets/rust/xirang-face.json";

/// Optional field shortcuts, using distinct triggers beside native Rust completion.
/// 可选字段快捷片段；用独立触发词与原生 Rust 补全并存。
pub fn editor_snippets(editor: Editor) -> String {
    match editor {
        Editor::Vscode => vscode_snippets(),
        Editor::Nvim => nvim_snippets(),
        Editor::Blink => vscode_snippets(),
    }
}

fn vscode_snippets() -> String {
    let mut output = String::from("{\n");
    let mut written = 0;
    for field in snippet_names() {
        // A field the default spelling does not author (`BUILD_SUPPLIED`) gets no snippet at all: a snippet
        // offering a method the builder does not have would be a promise that cannot compile.
        // 默认拼法不写的字段（`BUILD_SUPPLIED`）一个片段都不给：提供建造器并不拥有的方法的片段，是一句编译不过
        // 的承诺。
        let Some(shape) = field_shape(field) else {
            continue;
        };
        if written > 0 {
            output.push_str(",\n");
        }
        written += 1;
        output.push_str(&format!(
            "  \"xirang-{field}\": {{\n    \"prefix\": [\"xirang-{field}\"],\n    \"body\": [{body}],\n    \"scope\": \"rust\",\n    \"description\": \"插入 `xirang-{field}` 的定式并把光标停在值位 / insert the `xirang-{field}` shape and stop at its value\"\n  }}",
            // `json_string` emits the quotes itself, so the template must not
            // add a second pair around the body.
            // `json_string` 自带引号，因此模板不得再给 body 加一对。
            body = xirang_kernel::json::json_string(&shape)
        ));
    }
    output.push_str("\n}\n");
    output
}

fn nvim_snippets() -> String {
    let mut output = String::from(
        "-- Generated by `xirang snippets --editor nvim`; edit the face fields, not this file.\n\
         -- 由 `xirang snippets --editor nvim` 生成；要改字段请改注册面词表，不要改这个文件。\n\
         local ls = require(\"luasnip\")\n\
         local s = ls.snippet\n\
         local t = ls.text_node\n\
         local i = ls.insert_node\n\
         \n\
         return {\n",
    );
    for field in snippet_names() {
        let Some(shape) = field_shape(field) else {
            continue;
        };
        output.push_str(&format!(
            "  s({{ trig = \"xirang-{field}\", dscr = \"write the `xirang-{field}` shape and stop at its value / 写出 `xirang-{field}` 的定式并把光标停在值位\", priority = 2000 }}, {{ {} }}),\n",
            luasnip_body(&shape)
        ));
    }
    output.push_str("}\n");
    output
}

/// Write one editor's snippets, leaving the file alone when it already has the
/// current content.
/// 写入某个编辑器的 snippet；内容已经是最新时不改动文件。
pub fn write_editor_snippets(root: &Path, editor: Editor) -> Result<bool, String> {
    let relative = match editor {
        Editor::Vscode => SNIPPET_FILE,
        Editor::Nvim => NVIM_SNIPPET_FILE,
        Editor::Blink => BLINK_SNIPPET_FILE,
    };
    write_snippets_file(&root.join(relative), &editor_snippets(editor))
}

/// Write a snippet file, leaving it alone when it already has the current
/// content.
/// 写入 snippet 文件；内容已经是最新时不改动它。
pub fn write_snippets_file(path: &Path, content: &str) -> Result<bool, String> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(path, content)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{
        BUILD_SUPPLIED, Editor, FIELD_SHAPES, NVIM_SNIPPET_FILE, REGISTRATION_FIELDS, SNIPPET_FILE,
        TYPE_LIKE, editor_snippets, field_shape, luasnip_body, snippet_names,
        write_editor_snippets,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use xirang_kernel::registry_core::declaration::Registration;

    fn temporary_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "xirang-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    /// Every declared field gets exactly one snippet, keyed and prefixed by its
    /// own name, whose body is that field's shape.
    /// 每个声明的字段恰好得到一条 snippet，键与前缀都是字段名，body 是该字段的定式形状。
    #[test]
    fn the_editor_snippets_cover_every_declared_field() {
        let snippets = editor_snippets(Editor::Vscode);
        assert_eq!(
            snippets.matches("\"prefix\"").count(),
            snippet_names().count() - BUILD_SUPPLIED.len(),
            "one snippet per field the default spelling authors"
        );
        for field in snippet_names() {
            let Some(shape) = field_shape(field) else {
                assert!(
                    !snippets.contains(&format!("\"xirang-{field}\": {{")),
                    "{field} is build-supplied, so it gets no snippet at all"
                );
                continue;
            };
            assert!(
                snippets.contains(&format!("\"xirang-{field}\": {{")),
                "{field}"
            );
            assert!(
                snippets.contains(&format!("\"prefix\": [\"xirang-{field}\"]")),
                "{field}"
            );
            assert!(
                snippets.contains(&format!(
                    "\"body\": [{}],",
                    xirang_kernel::json::json_string(&shape)
                )),
                "{field}"
            );
        }
    }

    /// Every default (`Soil`) snippet names a method the builder really has, and the fields the default spelling
    /// does not author are **exactly** the build-supplied ones. The method list is read from the builder's own
    /// source, so a renamed method — or a snippet promising one that never existed, which is what
    /// `registry_name` would have done — fails here instead of shipping as a completion that cannot compile.
    /// 每条默认（`Soil`）片段都点名建造器真有的方法，而默认拼法不写的字段**恰好**是构建供给的那几个。方法表从
    /// 建造器自己的源码读，因此一个改名的方法——或一条承诺从未存在过的方法的片段（`registry_name` 本来就会那样）
    /// ——在这里失败，而不是作为一条编译不过的补全发出去。
    #[test]
    fn every_default_snippet_names_a_real_soil_builder_method() {
        let builder =
            include_str!("../../../../kernel/src/registry_core/declaration/soil_builder.rs");
        for field in snippet_names() {
            let Some(shape) = field_shape(field) else {
                continue;
            };
            let method = shape
                .trim_start_matches('.')
                .split(['(', ':', '<'])
                .next()
                .expect("a method name");
            assert!(
                builder.contains(&format!("fn {method}("))
                    || builder.contains(&format!("fn {method}<"))
                    || builder.contains(&format!("({method}, ")),
                "`{field}` promises `.{method}(…)`, which the builder does not declare: {shape}"
            );
        }
        let unauthored: Vec<&str> = REGISTRATION_FIELDS
            .iter()
            .copied()
            .filter(|field| field_shape(field).is_none())
            .collect();
        assert_eq!(
            unauthored, BUILD_SUPPLIED,
            "the fields the default spelling does not author are exactly the build-supplied ones"
        );
    }

    /// The shapes table covers `Registration`'s value-like fields, and every shape is a complete
    /// struct-literal field (or an associated type) with a value tab stop.
    /// 形状表覆盖 `Registration` 的值类字段，且每个形状都是完整的结构体字面量字段（或关联类型），带一个值跳转位。
    #[test]
    fn every_declared_field_has_a_complete_shape() {
        for field in REGISTRATION_FIELDS {
            let shape = FIELD_SHAPES
                .iter()
                .find(|(name, _)| name == field)
                .map(|(_, shape)| *shape)
                .unwrap_or_else(|| panic!("no shape for `{field}`"));
            assert!(shape.contains('$'), "xirang-{field}{shape}");
            assert!(
                shape.contains(": ") && shape.ends_with(','),
                "a value field is written `name: value,`: {field}: {shape}"
            );
        }
        for (name, shape) in TYPE_LIKE {
            if REGISTRATION_FIELDS.contains(name) {
                continue;
            }
            assert!(
                shape.starts_with("type ") && shape.ends_with(';'),
                "everything outside `Registration` is an associated type: {name}: {shape}"
            );
        }
        assert_eq!(
            FIELD_SHAPES.len(),
            REGISTRATION_FIELDS.len(),
            "one value-shaped snippet per `Registration` field"
        );
    }

    /// Every value-like name a snippet offers is a **real** field of `Registration`: the literal below
    /// names each one, so a rename in the kernel breaks this test rather than shipping a snippet that
    /// inserts a field nobody has.
    /// snippet 提供的每个值类名字都是 `Registration` 的**真实**字段：下面的字面量逐个点名它们，因此内核里改
    /// 名会让这条测试失败，而不是发出一条插入不存在字段的片段。
    #[test]
    fn every_value_like_snippet_names_a_real_registration_field() {
        // Reading each field off the kernel's own default: the expression below compiles only while every
        // name in the list is a field of `Registration`, so a rename in the kernel lands here.
        // 从内核自己的默认值上逐个读字段：下面这些表达式只有在列表里每个名字都是 `Registration` 的字段时才
        // 编译得过，因此内核里改名会在这里现形。
        const _: () = {
            let _ = Registration::DEFAULT.parent;
            let _ = Registration::DEFAULT.stable_name;
            let _ = Registration::DEFAULT.name;
            let _ = Registration::DEFAULT.summary;
            let _ = Registration::DEFAULT.exports;
            let _ = Registration::DEFAULT.needs_registry;
            let _ = Registration::DEFAULT.registry_name;
            let _ = Registration::DEFAULT.getting_from_other_registry;
            let _ = Registration::DEFAULT.registry_rule_path;
            let _ = Registration::DEFAULT.registry_rule;
            let _ = Registration::DEFAULT.admission;
            let _ = Registration::DEFAULT.requires;
            let _ = Registration::DEFAULT.provides;
            let _ = Registration::DEFAULT.flow;
            let _ = Registration::DEFAULT.flow_provider;
            let _ = Registration::DEFAULT.runtime_checks;
            let _ = Registration::DEFAULT.plugin;
        };
        assert_eq!(
            REGISTRATION_FIELDS.len(),
            17,
            "seventeen value-like fields; a change here is a change to the vocabulary"
        );
    }

    /// The Neovim file carries the same shapes in LuaSnip's Lua format, with the
    /// colon in the trigger so the menu shows what it will write.
    /// Neovim 文件用 LuaSnip 的 Lua 格式承载同样的形状；trigger 带冒号，菜单里因此看得见
    /// 它将写出什么。
    #[test]
    fn the_luasnip_snippets_cover_every_declared_field() {
        let snippets = editor_snippets(Editor::Nvim);
        assert_eq!(
            snippets.matches("trig = ").count(),
            snippet_names().count() - BUILD_SUPPLIED.len(),
            "one snippet per field the default spelling authors"
        );
        for field in snippet_names() {
            let Some(shape) = field_shape(field) else {
                assert!(
                    !snippets.contains(&format!("trig = \"xirang-{field}\"")),
                    "{field} is build-supplied, so it gets no snippet at all"
                );
                continue;
            };
            assert!(
                snippets.contains(&format!("trig = \"xirang-{field}\"")),
                "{field}"
            );
            assert!(
                snippets.contains(&format!("{{ {} }}", luasnip_body(&shape))),
                "{field}"
            );
        }
        assert!(
            snippets.contains("i(1)"),
            "the cursor lands at the value position"
        );
    }

    /// The Neovim file lives in the rust-only directory LuaSnip already scans,
    /// and the CLI spellings map to the two supported editors.
    /// Neovim 文件位于 LuaSnip 本就会扫描的 rust 专属目录，命令行写法映射到两种受支持编辑器。
    #[test]
    fn the_nvim_file_is_rust_only_and_the_names_round_trip() {
        assert!(NVIM_SNIPPET_FILE.starts_with("luasnippets/rust/"));
        assert!(NVIM_SNIPPET_FILE.ends_with(Editor::Nvim.file_name()));
        assert_eq!(
            Editor::parse_editor("vscode").map(Editor::name),
            Some("vscode")
        );
        assert_eq!(
            Editor::parse_editor("code").map(Editor::name),
            Some("vscode")
        );
        assert_eq!(Editor::parse_editor("nvim").map(Editor::name), Some("nvim"));
        assert_eq!(
            Editor::parse_editor("neovim").map(Editor::name),
            Some("nvim")
        );
        assert_eq!(Editor::parse_editor("emacs"), None);
    }

    /// Injecting twice leaves the second call with nothing to write.
    /// 注入两次时第二次无事可做。
    #[test]
    fn injecting_the_editor_snippets_is_idempotent() {
        let root = temporary_directory("snippets");
        assert!(write_editor_snippets(&root, Editor::Vscode).expect("first injection"));
        let path = root.join(SNIPPET_FILE);
        let first = fs::read_to_string(&path).expect("snippet file");
        assert_eq!(first, editor_snippets(Editor::Vscode));
        assert!(!write_editor_snippets(&root, Editor::Vscode).expect("second injection"));
        assert_eq!(fs::read_to_string(&path).expect("snippet file"), first);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
