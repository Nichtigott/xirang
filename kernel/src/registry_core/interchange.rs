//! The cross-language interchange vocabulary: language and backend identities, the capability a
//! backend declares, and the C ABI spelling of the export subset this workspace supports.
//! 跨语言交换的词汇：语言与后端身份、后端声明的能力，以及本工作区支持的导出子集的 C ABI 拼法。
//!
//! Nothing here is a list of languages the kernel knows. A language is a string an execution
//! surface registers a backend for; the kernel only owns the shapes that a handshake and an export
//! are compared against.
//! 这里没有任何“内核已知语言”的名单。语言只是执行面为其注册后端的字符串；内核只拥有握手与导出
//! 被比较时用的形状。

use std::collections::BTreeSet;

/// The identity of a language, as a backend spells it.
/// 语言的身份，由后端拼写。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LanguageId(String);

impl LanguageId {
    /// Parse a language identity.
    /// 解析一个语言身份。
    ///
    /// The rule is deliberately mechanical: one to thirty-two ASCII lowercase letters, digits,
    /// `-` or `_`, starting with a letter. A name outside it is refused rather than normalized,
    /// because two spellings of one language would be two backends that never meet.
    /// 规则刻意机械：一到三十二个 ASCII 小写字母、数字、`-` 或 `_`，以字母开头。之外的拼写会被
    /// 拒绝而不是归一化，因为同一门语言的两种拼写会变成永远不相遇的两个后端。
    pub fn parse_language(text: &str) -> Option<Self> {
        let valid = !text.is_empty()
            && text.len() <= 32
            && text.starts_with(|c: char| c.is_ascii_lowercase())
            && text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        valid.then(|| Self(text.to_owned()))
    }

    /// The identity as it was written.
    /// 原样返回身份字符串。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One ability a backend declares, so a surface can refuse by name instead of discovering the gap
/// halfway through a task.
/// 后端声明的一项能力，执行面因此可以按名字拒绝，而不是做到一半才发现缺口。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    /// Finding the project's units and their sources.
    /// 发现项目的单元及其源码。
    Discovery,
    /// Reading the project's registration declarations.
    /// 读取项目的注册声明。
    Registration,
    /// Authoring edits, moves and renames inside the project.
    /// 在项目内进行编辑、移动与改名。
    Authoring,
    /// Resolving packages and their versions.
    /// 解析包及其版本。
    Resolution,
    /// Building, testing and packaging a unit.
    /// 构建、测试与打包一个单元。
    BuildTest,
    /// Producing an export a consumer in another language can call.
    /// 产出另一种语言的消费者可以调用的导出。
    Export,
    /// Consuming another unit's export.
    /// 消费另一个单元的导出。
    Consumer,
    /// Grafting a declared implementation into the project.
    /// 把声明的实现嫁接到项目里。
    Graft,
    /// Moving a declared object and repairing its references.
    /// 移动一个声明对象并修复它的引用。
    Move,
    /// Recording call evidence for the project.
    /// 为项目记录调用证据。
    Calls,
}

impl Capability {
    /// The word a reply and a status line use for this ability.
    /// 回复与状态行使用的能力词。
    pub fn name(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Registration => "registration",
            Self::Authoring => "authoring",
            Self::Resolution => "resolution",
            Self::BuildTest => "build-test",
            Self::Export => "export",
            Self::Consumer => "consumer",
            Self::Graft => "graft",
            Self::Move => "move",
            Self::Calls => "calls",
        }
    }
}

impl Capability {
    /// Every ability word, in the order a report lists them.
    /// 全部能力词，按报告列出的顺序。
    pub const ALL: [Capability; 10] = [
        Self::Discovery,
        Self::Registration,
        Self::Authoring,
        Self::Resolution,
        Self::BuildTest,
        Self::Export,
        Self::Consumer,
        Self::Graft,
        Self::Move,
        Self::Calls,
    ];

    /// The ability a word names, or `None` when it names no ability this vocabulary has.
    /// 某个词命名的能力；不是本词汇表里的能力时返回 `None`。
    /// Compatibility alias for the published `Capability::parse` spelling.
    /// 已发布拼法 `Capability::parse` 的兼容别名。
    #[deprecated(note = "use Capability::parse_ability; this alias keeps 0.2 callers compiling")]
    pub fn parse(text: &str) -> Option<Self> {
        Self::parse_ability(text)
    }

    /// Maps an ability word to its capability.
    /// 把能力词映射到对应能力。
    pub fn parse_ability(text: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|capability| capability.name() == text)
    }
}

/// What one backend says it can do for one language.
/// 一个后端为一门语言声明它能做什么。
#[derive(Clone, Debug)]
pub struct Handshake {
    language: LanguageId,
    backend: String,
    capabilities: BTreeSet<Capability>,
}

impl Handshake {
    /// Build a handshake from an identity, a backend name and the abilities it declares.
    /// 按身份、后端名与它声明的能力构造握手。
    pub fn new(
        language: LanguageId,
        backend: impl Into<String>,
        capabilities: impl IntoIterator<Item = Capability>,
    ) -> Self {
        Self {
            language,
            backend: backend.into(),
            capabilities: capabilities.into_iter().collect(),
        }
    }

    /// The language this handshake is for.
    /// 这份握手对应的语言。
    pub fn language(&self) -> &LanguageId {
        &self.language
    }

    /// The backend that declared it.
    /// 声明它的后端。
    pub fn backend(&self) -> &str {
        &self.backend
    }

    /// Refuse by name when an ability a task needs was not declared.
    /// 任务需要的能力未被声明时，按名字拒绝。
    pub fn require(&self, needed: &[Capability]) -> Result<(), String> {
        let missing: Vec<&str> = needed
            .iter()
            .filter(|capability| !self.capabilities.contains(capability))
            .map(|capability| capability.name())
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "backend `{}` for language `{}` does not declare {}; a language is not implemented \
                 until the backend says what it does, so this task stops before it starts",
                self.backend,
                self.language.as_str(),
                missing.join(", ")
            ))
        }
    }

    /// The declared abilities, in their stable order.
    /// 已声明的能力，按稳定顺序。
    pub fn declared(&self) -> Vec<&'static str> {
        self.capabilities
            .iter()
            .map(|capability| capability.name())
            .collect()
    }
}

/// One value type the export subset carries across a language boundary.
/// 导出子集跨语言边界携带的一种值类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scalar {
    /// A signed 8-bit integer.
    /// 有符号 8 位整数。
    I8,
    /// A signed 16-bit integer.
    /// 有符号 16 位整数。
    I16,
    /// A signed 32-bit integer.
    /// 有符号 32 位整数。
    I32,
    /// A signed 64-bit integer.
    /// 有符号 64 位整数。
    I64,
    /// An unsigned 8-bit integer.
    /// 无符号 8 位整数。
    U8,
    /// An unsigned 16-bit integer.
    /// 无符号 16 位整数。
    U16,
    /// An unsigned 32-bit integer.
    /// 无符号 32 位整数。
    U32,
    /// An unsigned 64-bit integer.
    /// 无符号 64 位整数。
    U64,
    /// A 32-bit float.
    /// 32 位浮点数。
    F32,
    /// A 64-bit float.
    /// 64 位浮点数。
    F64,
    /// A boolean.
    /// 布尔值。
    Bool,
    /// UTF-8 text, carried as a pointer and a byte length.
    /// UTF-8 文本，以指针与字节长度携带。
    Utf8,
    /// An opaque byte string, carried as an address and a length.
    /// 不透明字节串，以地址与长度携带。
    Bytes,
}

impl Scalar {
    /// Parse the spelling a declaration uses.
    /// 解析声明使用的拼写。
    ///
    /// `optional<…>`, records and typed errors are not in this slice: a declaration that names
    /// them is refused by name, because a spelling the backend cannot lay out is not support.
    /// `optional<…>`、记录与类型化错误不在本切片里：点名它们会被按名字拒绝，因为后端排不出布局的
    /// 拼写不是支持。
    pub fn parse_scalar(text: &str) -> Result<Self, String> {
        let scalar = match text {
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            "utf8" => Self::Utf8,
            "bytes" => Self::Bytes,
            other if other.starts_with("optional") => {
                return Err(format!(
                    "`{other}` is part of the interchange vocabulary but this ABI slice does not \
                     lay it out yet; declare a scalar, `utf8` or `bytes`, or treat the missing \
                     layer as work not done"
                ));
            }
            other if other.starts_with("record") || other.starts_with("result") => {
                return Err(format!(
                    "`{other}` is not in the export subset: records and typed errors need a layout \
                     and an error vocabulary this slice does not have"
                ));
            }
            other => {
                return Err(format!(
                    "`{other}` is not an export scalar; the subset is i8, i16, i32, i64, u8, u16, \
                     u32, u64, f32, f64, bool, utf8, bytes"
                ));
            }
        };
        Ok(scalar)
    }

    /// The spelling this scalar uses in a manifest (also the suffix of an optional struct).
    /// 该标量在清单里的拼写（也是可选结构体名的后缀）。
    pub fn token(self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Bool => "bool",
            Self::Utf8 => "utf8",
            Self::Bytes => "bytes",
        }
    }

    /// The C spelling of a parameter of this type.
    /// 该类型参数在 C 里的拼写。
    pub fn c_parameter(self, name: &str) -> String {
        match self {
            Self::I8 => format!("int8_t {name}"),
            Self::I16 => format!("int16_t {name}"),
            Self::I32 => format!("int32_t {name}"),
            Self::I64 => format!("int64_t {name}"),
            Self::U8 => format!("uint8_t {name}"),
            Self::U16 => format!("uint16_t {name}"),
            Self::U32 => format!("uint32_t {name}"),
            Self::U64 => format!("uint64_t {name}"),
            Self::F32 => format!("float {name}"),
            Self::F64 => format!("double {name}"),
            Self::Bool => format!("bool {name}"),
            Self::Utf8 => format!("const char *{name}, size_t {name}_len"),
            Self::Bytes => format!("const uint8_t *{name}, size_t {name}_len"),
        }
    }

    /// The C spelling of a return of this type, or the reason the slice refuses one.
    /// 该类型返回值在 C 里的拼写，或本切片拒绝它的理由。
    ///
    /// Text and bytes are parameters only in this slice: returning them needs an ownership
    /// decision (who frees the block, and when) that a consumer in another language cannot infer.
    /// 本切片里文本与字节只作参数：返回它们需要所有权决定（谁释放、何时释放），另一种语言的消费者
    /// 无法推断。
    pub fn c_return(self) -> Result<&'static str, String> {
        match self {
            Self::I8 => Ok("int8_t"),
            Self::I16 => Ok("int16_t"),
            Self::I32 => Ok("int32_t"),
            Self::I64 => Ok("int64_t"),
            Self::U8 => Ok("uint8_t"),
            Self::U16 => Ok("uint16_t"),
            Self::U32 => Ok("uint32_t"),
            Self::U64 => Ok("uint64_t"),
            Self::F32 => Ok("float"),
            Self::F64 => Ok("double"),
            Self::Bool => Ok("bool"),
            Self::Utf8 | Self::Bytes => Err(
                "returning text or bytes needs an ownership rule this slice does not have; pass \
                 an out-parameter or return a scalar"
                    .to_owned(),
            ),
        }
    }
}

/// What an exported call returns across the C boundary.
/// 一个导出调用跨 C 边界返回什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnType {
    /// One interchange scalar, returned directly.
    /// 一个交换标量，直接返回。
    Scalar(Scalar),
    /// UTF-8 text: the caller owns the buffer and the call reports the length it needs.
    /// UTF-8 文本：缓冲区归调用方所有，调用报告它需要的长度。
    Text,
    /// Bytes: the same ownership rule as text.
    /// 字节：与文本相同的所有权规则。
    Bytes,
    /// A number or boolean that may be absent, carried as a struct with a `present` flag.
    /// 可能缺席的数字或布尔值，装在一个带 `present` 标志的结构体里。
    Optional(Scalar),
}

impl ReturnType {
    /// Parse the `returns` spelling of one export.
    /// 解析一个导出的 `returns` 拼写。
    pub fn parse_return(text: &str) -> Result<Self, String> {
        if let Some(inner) = text
            .strip_prefix("optional<")
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let scalar = Scalar::parse_scalar(inner)?;
            return match scalar {
                Scalar::Utf8 | Scalar::Bytes => Err(format!(
                    "`optional<{inner}>` is part of the vocabulary, but a present value would \
                     need an ownership rule this ABI slice does not have; optional numbers and \
                     booleans are laid out"
                )),
                other => Ok(Self::Optional(other)),
            };
        }
        match text {
            "text" | "utf8" => Ok(Self::Text),
            "bytes" => Ok(Self::Bytes),
            other => Scalar::parse_scalar(other).map(Self::Scalar),
        }
    }

    /// The scalar inside an optional return, when this is one.
    /// 若这是可选返回，则为其内部的标量。
    pub fn optional_scalar(self) -> Option<Scalar> {
        match self {
            Self::Optional(scalar) => Some(scalar),
            _ => None,
        }
    }

    /// The name of the struct an optional return is carried in.
    /// 可选返回所装入的结构体名字。
    pub fn struct_name(scalar: Scalar) -> String {
        format!("xirang_optional_{}", scalar.token())
    }

    /// The C typedef an optional return needs.
    /// 可选返回所需的 C typedef。
    pub fn c_typedef(self) -> Result<Option<String>, String> {
        match self.optional_scalar() {
            Some(scalar) => Ok(Some(format!(
                "typedef struct {{ bool present; {} value; }} {};",
                scalar.c_return()?,
                Self::struct_name(scalar)
            ))),
            None => Ok(None),
        }
    }

    /// The C type this return is carried in.
    /// 承载该返回值的 C 类型。
    pub fn c_return(self) -> Result<&'static str, String> {
        match self {
            Self::Scalar(scalar) => scalar.c_return(),
            Self::Text | Self::Bytes => Ok("bool"),
            Self::Optional(_) => Err(
                "an optional return is carried in a struct, not in a bare C scalar; use \
                 `c_return_type`"
                    .to_owned(),
            ),
        }
    }

    /// The C type a caller declares for this return, including optional structs.
    /// 调用方为该返回声明的 C 类型，包括可选结构体。
    pub fn c_return_type(self) -> Result<String, String> {
        match self {
            Self::Optional(scalar) => Ok(Self::struct_name(scalar)),
            other => other.c_return().map(str::to_owned),
        }
    }

    /// The buffer parameter a text or bytes return needs, if any.
    /// 文本或字节返回所需的缓冲区参数（若有）。
    pub fn buffer(self) -> Option<&'static str> {
        match self {
            Self::Text => Some("char"),
            Self::Bytes => Some("uint8_t"),
            Self::Scalar(_) | Self::Optional(_) => None,
        }
    }
}

/// What one exported parameter may be: a scalar, or an optional scalar laid out as a struct.
/// 一个导出参数可能是什么：标量，或按结构体布局的可选标量。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterType {
    /// A plain scalar parameter.
    /// 普通标量参数。
    Scalar(Scalar),
    /// An optional scalar parameter, laid out as the struct `optional<T>` returns use.
    /// 可选标量参数，布局与 `optional<T>` 返回所用的结构体相同。
    Optional(Scalar),
}

impl ParameterType {
    /// Parse the manifest spelling of a parameter type.
    /// 解析参数类型的清单拼写。
    pub fn parse_parameter(text: &str) -> Result<Self, String> {
        // `utf8` and `bytes` are parameters as written: the pointer and the length are the ABI
        // spelling, so the parameter keeps the scalar path. Only a `text`/`bytes` *return* spelling
        // is refused here, because returning them needs the caller-owned buffer rule.
        // `utf8` 与 `bytes` 就是参数的拼写：指针加长度就是 ABI 拼法，因此参数走标量路径。只有
        // 返回位置上的 `text`/`bytes` 拼写会被拒，因为返回它们需要调用方缓冲区规则。
        if let Ok(scalar) = Scalar::parse_scalar(text) {
            return Ok(Self::Scalar(scalar));
        }
        match ReturnType::parse_return(text)? {
            ReturnType::Scalar(scalar) => Ok(Self::Scalar(scalar)),
            ReturnType::Optional(scalar) => Ok(Self::Optional(scalar)),
            ReturnType::Text | ReturnType::Bytes => Err(format!(
                "`{text}` cannot be a parameter as written: returning text or bytes is a \
                 caller-owned buffer, and a parameter spells those `utf8` and `bytes`"
            )),
        }
    }

    /// The C spelling of this parameter.
    /// 该参数在 C 里的拼写。
    pub fn c_parameter(self, name: &str) -> String {
        match self {
            Self::Scalar(scalar) => scalar.c_parameter(name),
            Self::Optional(scalar) => format!("{} {name}", ReturnType::struct_name(scalar)),
        }
    }

    /// The struct this parameter needs, when it needs one.
    /// 该参数需要的结构体（若需要）。
    pub fn c_typedef(self) -> Option<String> {
        let scalar = match self {
            Self::Scalar(_) => return None,
            Self::Optional(scalar) => scalar,
        };
        ReturnType::Optional(scalar).c_typedef().ok().flatten()
    }
}

/// Whether `text` is one C identifier: a name a caller can spell in a prototype.
/// `text` 是否是一个 C 标识符：调用方能在原型里拼出的名字。
fn is_c_identifier(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// One exported call: a C identifier, its parameters and its optional scalar return.
/// 一个导出调用：C 标识符、参数与可选的标量返回值。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Export {
    /// The C identifier a consumer calls.
    /// 消费者调用的 C 标识符。
    pub name: String,
    /// The parameters, in call order.
    /// 按调用顺序排列的参数。
    pub parameters: Vec<(&'static str, ParameterType)>,
    /// What the call returns, when it returns something.
    /// 若调用有返回值，则为该返回类型。
    pub returns: Option<ReturnType>,
    /// The typed error variants, in the order the status codes name them (1 is the first).
    /// 类型化错误变体，按状态码命名的顺序排列（1 是第一个）。
    pub errors: Vec<String>,
}

impl Export {
    /// Validate the identifier, the arity and every type.
    /// 校验标识符、参数个数与每个类型。
    pub fn validate_export(&self) -> Result<(), String> {
        if !is_c_identifier(&self.name) {
            return Err(format!(
                "`{}` is not a C identifier: an export is called by name from another language, so \
                 the name has to survive that boundary",
                self.name
            ));
        }
        if self.parameters.len() > 16 {
            return Err(format!(
                "`{}` declares {} parameters; the C ABI slice carries at most 16",
                self.name,
                self.parameters.len()
            ));
        }
        let mut seen = BTreeSet::new();
        for (parameter, _) in &self.parameters {
            if !seen.insert(*parameter) {
                return Err(format!(
                    "`{}` names the parameter `{parameter}` twice; a caller cannot say which one it \
                     means",
                    self.name
                ));
            }
        }
        if let Some(returns) = &self.returns {
            returns.c_return_type()?;
        }
        if self.errors.len() > 8 {
            return Err(format!(
                "`{}` declares {} typed errors; a status code carries at most 8 in this slice",
                self.name,
                self.errors.len()
            ));
        }
        if !self.errors.is_empty() && !matches!(self.returns, Some(ReturnType::Scalar(_))) {
            return Err(format!(
                "`{}` declares typed errors, and this slice only carries them beside a scalar \
                 return: an error and a text or optional return would need a second out-parameter",
                self.name
            ));
        }
        let mut variants = BTreeSet::new();
        for variant in &self.errors {
            if !is_c_identifier(variant) {
                return Err(format!(
                    "`{variant}` is not a C identifier: an error variant names a status a caller \
                     can switch on"
                ));
            }
            if !variants.insert(variant) {
                return Err(format!(
                    "`{}` names the error variant `{variant}` twice; two variants with one name \
                     cannot be told apart",
                    self.name
                ));
            }
        }
        Ok(())
    }

    /// The C type of this export's typed error, when it has one.
    /// 该导出的类型化错误在 C 里的类型（若有）。
    pub fn error_enum(&self) -> String {
        format!("xirang_error_{}", self.name)
    }

    /// The enum typedef that names every error variant, when the export declares any.
    /// 命名每个错误变体的枚举 typedef（若该导出声明了错误）。
    pub fn c_error_typedef(&self) -> Option<String> {
        if self.errors.is_empty() {
            return None;
        }
        let mut variants = Vec::new();
        for (position, variant) in self.errors.iter().enumerate() {
            variants.push(format!(
                "    XIRANG_ERROR_{}_{} = {}",
                self.name.to_ascii_uppercase(),
                variant.to_ascii_uppercase(),
                position + 1
            ));
        }
        Some(format!(
            "typedef enum {{\n{}\n}} {};",
            variants.join(",\n"),
            self.error_enum()
        ))
    }

    /// The C prototype this export is called through.
    /// 调用该导出所用的 C 原型。
    pub fn c_prototype(&self) -> Result<String, String> {
        self.validate_export()?;
        let mut parameters: Vec<String> = self
            .parameters
            .iter()
            .map(|(name, kind)| kind.c_parameter(name))
            .collect();
        if !self.errors.is_empty() {
            // With typed errors the call reports a status, writes the value through `out`, and
            // names the variant through `error`. This slice pairs errors with a scalar return.
            // 有类型化错误时，调用报告状态、经 `out` 写值、经 `error` 点名变体。本切片把错误与标量返回配对。
            let Some(ReturnType::Scalar(scalar)) = &self.returns else {
                return Err(format!(
                    "`{}` declares typed errors, and this slice only carries them beside a scalar \
                     return",
                    self.name
                ));
            };
            parameters.push(format!("{} *out", scalar.c_return()?));
            parameters.push(format!("{} *error", self.error_enum()));
            return Ok(format!("int32_t {}({})", self.name, parameters.join(", ")));
        }
        let returns = match &self.returns {
            Some(kind) => kind.c_return_type()?,
            None => "void".to_owned(),
        };
        if let Some(kind) = &self.returns
            && let Some(buffer) = kind.buffer()
        {
            parameters.push(format!("{buffer} *out"));
            parameters.push("size_t cap".to_owned());
            parameters.push("size_t *out_len".to_owned());
        }
        Ok(format!(
            "{returns} {}({})",
            self.name,
            parameters.join(", ")
        ))
    }
}

#[cfg(test)]
#[path = "interchange_tests.rs"]
mod tests;
