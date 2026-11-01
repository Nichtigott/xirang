//! Shared pieces of a face-declaration grammar: literal spelling and the validation obligations.
//! 面声明语法共享的部分：字面量拼法与验证义务。
//!
//! A declaration's *carrier* (a builder chain, a decorator, …) and its *field spellings* belong to
//! the language. What does not belong to it is how a string is quoted, how a comma-separated list is
//! written, and which obligations a plan must state — those are the same across languages, and they
//! are the half of the grammar the renderer and the parser already share today.
//! 声明的**载体**（构造器链、装饰器……）与**字段拼法**属于语言。不属于语言的是：字符串怎么引号、
//! 逗号列表怎么写、计划必须声明哪些义务——它们跨语言相同，也是语法中渲染器与解析器今天已经在共享的那一半。

use xirang_kernel::authoring::{
    ObligationKind, RegistrationField, RegistrationUpdate, ValidationObligation,
};

/// One language-independent string literal, with the escapes both renderers need.
/// 一个与语言无关的字符串字面量，带两个渲染器都需要的转义。
pub fn quote(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() + 2);
    escaped.push('"');
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

/// A comma-separated list as a language's list literal.
/// 逗号分隔列表按某语言的列表字面量写出。
///
/// `tuple` keeps the decorator spelling (`("a",)` for one entry); `quoted_list` keeps the
/// comma-separated string a builder method takes. Both sides read the same `RegistrationUpdate`
/// value, so a face renders the same logical list in either carrier.
/// `tuple` 保留装饰器拼法（单元素是 `("a",)`）；`quoted_list` 保留构造器方法接受的逗号串。两侧读同一个
/// `RegistrationUpdate` 取值，因此同一个面在两种载体里渲染出同一个逻辑列表。
pub fn tuple(list: &str) -> String {
    let entries = list
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(quote)
        .collect::<Vec<_>>();
    match entries.len() {
        0 => "()".to_owned(),
        1 => format!("({},)", entries[0]),
        _ => format!("({})", entries.join(", ")),
    }
}

/// What one field of a declaration is: the parent, the face kind, or one registration field.
/// 声明里的一个字段是什么：父级、面种类，或一个注册字段。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The face's parent position.
    /// 面的父级位置。
    Parent,
    /// The face's kind, which its display name normally repeats.
    /// 面的种类，显示名通常与它相同。
    Kind,
    /// One registration field.
    /// 一个注册字段。
    Field(RegistrationField),
}

/// How a field's value is written in one carrier.
/// 一个字段的取值在某种载体里怎么写。
#[derive(Clone, Copy, Debug)]
pub enum ValueStyle {
    /// An expression, written as the author wrote it (a parent path).
    /// 一个表达式，按作者写的样子（父路径）。
    Expr,
    /// One string literal.
    /// 一个字符串字面量。
    Quoted,
    /// A list of strings as one language's tuple literal.
    /// 字符串列表按某语言的元组字面量写。
    Tuple,
    /// A list of strings as a C# attribute's array literal (`new[] { "a", "b" }`).
    /// 字符串列表按 C# 特性的数组字面量写（`new[] { "a", "b" }`）。
    ///
    /// An attribute argument must be a compile-time constant, and `new[] { … }` infers its element
    /// type from the entries; an empty one cannot infer, so it is written `new string[0]`.
    /// 特性实参必须是编译期常量，`new[] { … }` 从条目推断元素类型；空数组无法推断，因此写成 `new string[0]`。
    CSharpArray,
    /// A list of strings as a Java annotation's array literal (`{"a", "b"}`).
    /// 字符串列表按 Java 注解的数组字面量写（`{"a", "b"}`）。
    ///
    /// An annotation element of array type is written without `new ...[]`; the braces are the
    /// language's spelling, and the entries read the same comma-separated value as every other style.
    /// 数组类型的注解元素不写 `new ...[]`；花括号是这门语言的拼法，条目读的仍是与其它风格相同的逗号分隔取值。
    JavaArray,
    /// A list of strings as one language's **typed slice** literal (`[]string{"a", "b"}`).
    /// 字符串列表按某语言的**带类型切片**字面量写（`[]string{"a", "b"}`）。
    ///
    /// Go spells a list with its element type in front; the entries still read the same
    /// comma-separated `RegistrationUpdate` value as every other style.
    /// Go 的列表把元素类型写在前面；条目读的仍是与其它风格相同的逗号分隔 `RegistrationUpdate` 取值。
    SliceList,
    /// A list of strings as one language's bracket literal (`["a", "b"]`).
    /// 字符串列表按某语言的方括号字面量写（`["a", "b"]`）。
    ///
    /// The bracket spelling is the one JavaScript and TypeScript share; it exists beside `Tuple`
    /// rather than replacing it because Python's single-entry tuple needs a trailing comma and a
    /// bracket list does not. Both read the same comma-separated `RegistrationUpdate` value.
    /// 方括号拼法是 JavaScript 与 TypeScript 共用的；它放在 `Tuple` 旁边而不是取代它，因为 Python 的单元素
    /// 元组需要尾逗号而方括号列表不需要。两者读的是同一个逗号分隔的 `RegistrationUpdate` 取值。
    BracketList,
    /// A comma-separated string, quoted once (a builder method's argument).
    /// 逗号分隔串，整体加一次引号（构造器方法的实参）。
    QuotedList,
    /// Capability pairs as one language's list of pairs.
    /// 能力对按某语言的成对列表写。
    RequiresPairs,
    /// Capability pairs as the comma-separated string a builder method takes.
    /// 能力对按构造器方法接受的逗号串写。
    RequiresComma,
    /// A boolean, with this language's spelling of the two values.
    /// 一个布尔值，用该语言的两种拼法。
    Bool {
        /// The spelling of true.
        /// 真值的拼法。
        when_true: &'static str,
        /// The spelling of false.
        /// 假值的拼法。
        when_false: &'static str,
    },
}

/// One field's spelling and value style in one language.
/// 一个字段在某语言里的拼法与取值风格。
#[derive(Clone, Copy, Debug)]
pub struct FieldSpec {
    /// What the field is.
    /// 这个字段是什么。
    pub role: Role,
    /// How this language spells it (a keyword, a method name, …).
    /// 这门语言怎么拼它（关键字、方法名……）。
    pub spelling: &'static str,
    /// How this language writes its value.
    /// 这门语言怎么写它的取值。
    pub style: ValueStyle,
}

/// One language's field table: the spellings and value styles both its renderer and its reader use.
/// 一门语言的字段表：它的渲染器与读取器共用的拼法与取值风格。
#[derive(Clone, Copy, Debug)]
pub struct FaceGrammar {
    /// The fields, in the order this language renders them.
    /// 字段，按这门语言渲染它们的顺序。
    pub fields: &'static [FieldSpec],
}

impl FaceGrammar {
    /// The field one spelling names, when this language spells it.
    /// 一个拼法命名的字段（当这门语言这样拼它时）。
    pub fn lookup(&self, spelling: &str) -> Option<&'static FieldSpec> {
        self.fields.iter().find(|spec| spec.spelling == spelling)
    }

    /// One field's registration content, when it is a registration field.
    /// 一个字段的注册内容（当它是注册字段时）。
    pub fn field_of(&self, spelling: &str) -> Option<RegistrationField> {
        match self.lookup(spelling)?.role {
            Role::Field(field) => Some(field),
            Role::Parent | Role::Kind => None,
        }
    }

    /// The spec for one role, when this language carries it.
    /// 某个角色的条目（当这门语言携带它时）。
    pub fn spec(&self, role: Role) -> Option<&'static FieldSpec> {
        self.fields.iter().find(|spec| spec.role == role)
    }

    /// One value, rendered the way this language writes it.
    /// 一个取值，按这门语言的写法渲染。
    pub fn declared(&self, role: Role, text: &str) -> String {
        let style = self
            .spec(role)
            .map(|spec| spec.style)
            .unwrap_or(ValueStyle::Quoted);
        match style {
            ValueStyle::Expr => text.to_owned(),
            ValueStyle::Quoted | ValueStyle::QuotedList | ValueStyle::RequiresComma => quote(text),
            ValueStyle::Tuple => tuple(text),
            ValueStyle::BracketList => bracket_list(text),
            ValueStyle::SliceList => slice_list(text),
            ValueStyle::JavaArray => java_array(text),
            ValueStyle::CSharpArray => csharp_array(text),
            ValueStyle::RequiresPairs => pairs(text),
            ValueStyle::Bool {
                when_true,
                when_false,
            } => {
                if text == "true" {
                    when_true.to_owned()
                } else {
                    when_false.to_owned()
                }
            }
        }
    }
}

/// A comma-separated list as C#'s attribute array literal (`new[] { "a", "b" }`, or `new string[0]`).
/// 逗号分隔列表按 C# 特性的数组字面量写出（`new[] { "a", "b" }`，空列表写 `new string[0]`）。
pub fn csharp_array(list: &str) -> String {
    let entries = list
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(quote)
        .collect::<Vec<_>>();
    if entries.is_empty() {
        "new string[0]".to_owned()
    } else {
        format!("new[] {{ {} }}", entries.join(", "))
    }
}

/// A comma-separated list as Java's annotation array literal (`{"a", "b"}`, or `{}` when empty).
/// 逗号分隔列表按 Java 注解的数组字面量写出（`{"a", "b"}`，空列表写 `{}`）。
pub fn java_array(list: &str) -> String {
    let entries = list
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(quote)
        .collect::<Vec<_>>();
    format!("{{{}}}", entries.join(", "))
}

/// A comma-separated list as Go's typed slice literal (`[]string{"a", "b"}`, or `[]string{}`).
/// 逗号分隔列表按 Go 的带类型切片字面量写出（`[]string{"a", "b"}`，空列表写 `[]string{}`）。
pub fn slice_list(list: &str) -> String {
    let entries = list
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(quote)
        .collect::<Vec<_>>();
    format!("[]string{{{}}}", entries.join(", "))
}

/// A comma-separated list as a bracket literal (`["a", "b"]`, or `[]` when empty).
/// 逗号分隔列表按方括号字面量写出（`["a", "b"]`，空列表写 `[]`）。
pub fn bracket_list(list: &str) -> String {
    let entries = list
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(quote)
        .collect::<Vec<_>>();
    format!("[{}]", entries.join(", "))
}

/// A comma-separated `capability=>provider` list as a language's list of pairs.
/// 逗号分隔的 `capability=>provider` 列表按某语言的成对列表写出。
pub fn pairs(list: &str) -> String {
    let entries = list
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let (capability, provider) = entry.split_once("=>").unwrap_or((entry, entry));
            format!("({}, {})", quote(capability.trim()), quote(provider.trim()))
        })
        .collect::<Vec<_>>();
    match entries.len() {
        0 => "()".to_owned(),
        1 => format!("({},)", entries[0]),
        _ => format!("({})", entries.join(", ")),
    }
}

/// The language-neutral payload of one declaration: where it hangs, what it registers, and the
/// fields no request vocabulary names.
/// 一条声明的语言中立载荷：它挂在哪里、注册什么，以及请求词汇点不到的那些字段。
///
/// A carrier adds its own metadata around this (a C++ spelling and byte range, a Python class and
/// argument range), so a shared reader can hand the payload to either surface without inventing a
/// second definition of "the declaration's registration".
/// 载体在它周围加自己的元数据（C++ 的拼法与字节区间、Python 的类与参数区间），因此一个共享读取器可以把
/// 载荷交给任一执行面，而不必再发明一份"声明的注册内容"。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredFace {
    /// The parent reference as the declaration spells it.
    /// 父引用，按声明写出的拼法。
    pub parent: String,
    /// The registration fields the declaration writes, normalized the way a request is.
    /// 声明写下的注册字段，按请求的同一套规则规范化。
    pub registration: RegistrationUpdate,
    /// Method calls / keyword arguments the standard vocabulary does not name, kept verbatim so a
    /// rewrite never drops them.
    /// 标准词汇没有点名的调用/关键字参数，逐字保留，因此改写不会丢掉它们。
    pub extras: Vec<(String, String)>,
}

impl DeclaredFace {
    /// The face kind (the `named` value).
    /// 面种类名（`named` 的取值）。
    pub fn kind(&self) -> String {
        self.registration
            .find_field(RegistrationField::Kind)
            .value()
            .copied()
            .unwrap_or_default()
            .to_owned()
    }
}

/// One text field, or the empty string.
/// 一个文本字段，或空串。
pub fn text_field(registration: &RegistrationUpdate, field: RegistrationField) -> &str {
    registration
        .find_field(field)
        .value()
        .copied()
        .unwrap_or_default()
}

/// The validation obligations a plan states for one face.
/// 计划为一个面声明的验证义务。
///
/// The build step is the one sentence the language owns: compiling a record and importing a module
/// are different promises, and the plan must make the true one. Everything else — read back, find
/// under the parent, keep the identity, call the exports — is one contract.
/// 构建那一步是语言独有的那一句：编译记录与 import 模块是不同的承诺，计划必须说出真的那一个。其余——
/// 读回、在父级下找到、保持身份、调用导出——是同一条契约。
pub fn obligations(
    kind: &str,
    registration: &RegistrationUpdate,
    build: &str,
) -> Vec<ValidationObligation> {
    let has_exports = !text_field(registration, RegistrationField::Exports).is_empty();
    let mut obligations = vec![
        ValidationObligation {
            kind: ObligationKind::RoundTrip,
            detail: format!("read `{kind}` back and compare the normalized registration fields"),
        },
        ValidationObligation {
            kind: ObligationKind::Build,
            detail: build.to_owned(),
        },
        ValidationObligation {
            kind: ObligationKind::Registration,
            detail: format!("confirm `{kind}` appears under its declared parent"),
        },
        ValidationObligation {
            kind: ObligationKind::Identity,
            detail: "confirm the identity is the same one the other language names".to_owned(),
        },
    ];
    if has_exports {
        obligations.push(ValidationObligation {
            kind: ObligationKind::Call,
            detail: format!("call `{kind}`'s exported behaviour through the built tree"),
        });
    }
    obligations
}

#[cfg(test)]
#[path = "grammar_tests.rs"]
mod grammar_tests;
