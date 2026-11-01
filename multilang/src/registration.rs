//! The adapter protocol and the registry of implementations this build carries.
//! 适配器协议，以及本构建携带的实现的注册表。
//!
//! An adapter turns a resolved [`FaceAuthoringRequest`] into a reviewable [`FaceAuthoringPlan`].
//! It never writes a file: applying the plan is the execution surface's transaction, so a language
//! template cannot bypass the authoring permission, preview, or rollback rules.
//! 适配器把一份已解析的 [`FaceAuthoringRequest`] 变成可审阅的 [`FaceAuthoringPlan`]。它从不写文件：
//! 落盘是执行面的事务，因此语言模板无法绕过创作权限、预览或回滚规则。

use std::path::PathBuf;

use xirang_kernel::authoring::{
    FaceAuthoringPlan, FaceAuthoringRequest, LanguageCapabilities, ParentReference, PlannedFile,
    RegistrationUpdate, validate_request,
};
use xirang_kernel::interchange::LanguageId;

#[cfg(feature = "lang-cpp")]
use crate::CppLanguage;
#[cfg(feature = "lang-python")]
use crate::PythonLanguage;
#[cfg(feature = "lang-zig")]
use crate::ZigLanguage;

#[cfg(feature = "lang-js")]
use crate::JsLanguage;

#[cfg(feature = "lang-ts")]
use crate::TsLanguage;

#[cfg(feature = "lang-go")]
use crate::GoLanguage;

#[cfg(feature = "lang-java")]
use crate::JavaLanguage;

#[cfg(feature = "lang-csharp")]
use crate::CSharpLanguage;

#[cfg(feature = "lang-c")]
use crate::CLanguage;

/// Where a language puts a child's source file.
/// 一门语言把子面的源码文件放在哪里。
///
/// The two placements are the two real layouts: beside the parent wherever the parent lives, or
/// under the package's source scan root. It is adapter data because it is a language decision, not
/// a surface decision.
/// 两种落点是现实中的两种布局：住在父旁边（无论父在哪），或住在包的源码扫描根之下。它是适配器数据，
/// 因为这是语言的决定，不是执行面的决定。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePlacement {
    /// The child's directory is its parent's directory, package-relative.
    /// 子面目录就是父的目录，按包内相对。
    BesideParent,
    /// The child lives under the package's source scan root.
    /// 子面住在包的源码扫描根之下。
    UnderScanRoot,
}

/// The context an adapter needs to plan editor material.
/// 适配器规划编辑器资料所需的上下文。
///
/// The adapter decides *which* file an editor needs (`compile_flags.txt`, a generated stub, …);
/// the surface only supplies where the package and its build output are.
/// 适配器决定编辑器需要**哪个**文件（`compile_flags.txt`、生成的存根等）；执行面只提供包根与构建
/// 产物的位置。
#[derive(Clone, Debug)]
pub struct EditorContext {
    /// The package root.
    /// 包根。
    pub package_root: PathBuf,
    /// The build output directory a generated header lives in.
    /// 生成头所在的构建产物目录。
    pub out_dir: PathBuf,
}

/// What an adapter reads back out of an existing declaration.
/// 适配器从一条已有声明里读回的内容。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceReadBack {
    /// The normalized registration fields the declaration writes.
    /// 声明写下的规范注册字段。
    pub registration: RegistrationUpdate,
    /// The parent as this language spells it.
    /// 父面按该语言的拼法。
    pub parent: ParentReference,
}

/// Which structural operations a language adapter can plan a source rewrite for.
/// 语言适配器能为哪些结构操作规划源码改写。
///
/// The generic move/graft/crate algorithms own impact, authorization and transactions; an adapter
/// only answers whether it can locate a declaration and rewrite its references. A language that
/// cannot yet answer says so here, and the operation must refuse rather than silently treat its
/// files as Rust.
/// 通用 move/graft/crate 算法拥有影响、授权与事务；适配器只回答它能否定位声明并改写引用。尚不能
/// 回答的语言在这里说出来，操作必须拒绝，而不是把它的文件静默当成 Rust。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructuralCapabilities {
    /// Can this adapter plan a source rewrite when an object's physical path changes?
    /// 对象的物理路径变化时，本适配器能否规划源码改写？
    pub move_source: bool,
    /// Can it plan the declaration rewrite a graft landing needs?
    /// 嫁接落地所需的声明改写能否规划？
    pub graft: bool,
    /// Can it plan the declaration rewrite a crate split needs?
    /// crate 拆分所需的声明改写能否规划？
    pub crate_split: bool,
}

/// One source rewrite an adapter plans for a structural move.
/// 适配器为一次结构搬动规划的一条源码改写。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEdit {
    /// Byte start in the original source.
    /// 原源码中的字节起点。
    pub start: usize,
    /// Byte end (exclusive) in the original source.
    /// 原源码中的字节终点（不含）。
    pub end: usize,
    /// Replacement text.
    /// 替换文本。
    pub replacement: String,
}

/// Apply adapter edits to the exact bytes they were planned against, refusing any shape the
/// generic layer cannot verify: out of bounds, reversed, overlapping, or splitting a UTF-8
/// character. The caller re-reads the file first; this function never touches disk.
/// 把适配器改写应用到它规划时依据的那份字节，并拒绝通用层无法核验的形状：越界、反向、重叠或切断
/// UTF-8 字符。调用方先重读文件；本函数从不碰磁盘。
pub fn apply_source_edits(source: &str, edits: &[SourceEdit]) -> Result<String, String> {
    let mut ordered: Vec<&SourceEdit> = edits.iter().collect();
    ordered.sort_by_key(|edit| (edit.start, edit.end));
    let mut output = String::with_capacity(source.len());
    let mut at = 0usize;
    for edit in ordered {
        if edit.start > edit.end || edit.end > source.len() {
            return Err(format!(
                "an edit spans {}..{}, outside the {} byte(s) it was planned against",
                edit.start,
                edit.end,
                source.len()
            ));
        }
        if edit.start < at {
            return Err(format!(
                "edits overlap at byte {}; the plan cannot be applied as one transaction",
                edit.start
            ));
        }
        if !source.is_char_boundary(edit.start) || !source.is_char_boundary(edit.end) {
            return Err(format!(
                "an edit boundary {}..{} splits a UTF-8 character",
                edit.start, edit.end
            ));
        }
        output.push_str(&source[at..edit.start]);
        output.push_str(&edit.replacement);
        at = edit.end;
    }
    output.push_str(&source[at..]);
    Ok(output)
}

/// The facts a structural move hands an adapter.
/// 结构搬动交给适配器的事实。
#[derive(Clone, Debug)]
pub struct MoveRequest<'a> {
    /// The declaration's source as it is now.
    /// 声明当前的源码。
    pub source: &'a str,
    /// The object's current module spelling.
    /// 对象当前的模块拼写。
    pub old_module: &'a str,
    /// The module spelling after the move.
    /// 搬动后的模块拼写。
    pub new_module: &'a str,
    /// The parent's old nested spelling (`faces::shop::panel`), when it changes.
    /// 父级旧的嵌套拼写（`faces::shop::panel`），在其变化时给出。
    pub old_parent: Option<&'a str>,
    /// The parent's new nested spelling, when it changes.
    /// 父级新的嵌套拼写，在其变化时给出。
    pub new_parent: Option<&'a str>,
}

/// The new parent's **shared catalog path**, when the caller knows it.
/// 新父的**共享目录路径**，当调用方知道时。
///
/// `path` is below the registry root (`shop/panel/Area`), not the language's module spelling.
/// `path` 在注册树根之下（`shop/panel/Area`），不是某门语言的模块拼写。
#[derive(Clone, Copy, Debug)]
pub struct CatalogParent<'a> {
    /// The parent's logical path below the registry root.
    /// 父在注册树根之下的逻辑路径。
    pub path: &'a str,
}

/// One language implementation.
/// 一个语言实现。
///
/// The trait is deliberately small: capability data, one planning method, and the file extensions
/// the language owns. Everything else — defaults, parent references, path derivation — is the
/// execution surface's job, because those decisions need the package root and the registry.
/// 这个 trait 刻意小：能力数据、一个计划方法，以及该语言拥有的文件扩展名。其余一切——默认值、父引用、
/// 路径推导——是执行面的事，因为那些判断需要包根与注册树。
pub trait LanguageAdapter: Sync {
    /// What this language can carry and which operations it implements.
    /// 该语言能携带什么、实现了哪些操作。
    fn capabilities(&self) -> &'static LanguageCapabilities;

    /// Which structural operations this adapter can plan a source rewrite for.
    /// 本适配器能为哪些结构操作规划源码改写。
    ///
    /// The default is "none": a language added for authoring only does not silently claim the
    /// structural operations, and the generic move must refuse its files instead of treating them
    /// as Rust.
    /// 默认是"都不行"：只为创作接入的语言不会静默声称支持结构操作，通用 move 必须拒绝它的文件，
    /// 而不是把它们当成 Rust。
    fn structural(&self) -> StructuralCapabilities {
        StructuralCapabilities::default()
    }

    /// Plan the source edits a structural move needs, or refuse by name.
    /// 规划一次结构搬动所需的源码改写，或按名拒绝。
    ///
    /// The generic move owns impact, authorization, transactions and identity; this method only
    /// locates the declaration and rewrites the language's reference syntax. A move that keeps the
    /// logical parent unchanged may return no edits at all.
    /// 通用 move 拥有影响、授权、事务与身份；本方法只定位声明并改写该语言的引用语法。逻辑父不变的
    /// 搬动可以返回空改写。
    fn plan_move(&self, _request: &MoveRequest<'_>) -> Result<Vec<SourceEdit>, String> {
        Err(format!(
            "`{}` does not plan structural moves yet",
            self.capabilities().language.as_str()
        ))
    }

    /// Plan a move with the new parent's shared catalog path, when the caller has it.
    /// 当调用方持有新父的共享目录路径时，规划一次搬动。
    ///
    /// The default keeps the historical behavior: a language that has not opted into the shared
    /// catalog spelling calls its own `plan_move`. This is an additive method with a default, so
    /// existing implementations keep compiling.
    /// 默认保留历史行为：尚未采用共享目录拼法的语言调用自己的 `plan_move`。这是一个带默认实现的
    /// 新增方法，现有实现继续编译。
    fn plan_move_with_catalog(
        &self,
        request: &MoveRequest<'_>,
        _parent: Option<&CatalogParent<'_>>,
    ) -> Result<Vec<SourceEdit>, String> {
        self.plan_move(request)
    }

    /// Turn one resolved request into a reviewable plan.
    /// 把一份已解析请求变成可审阅的计划。
    ///
    /// The adapter must refuse what it cannot carry rather than approximating it; the registry's
    /// [`AdapterRegistry::plan`] already ran the capability validation, so this method is about the
    /// language's own syntax rules.
    /// 适配器必须拒绝它携带不了的东西，而不是近似；注册表的 [`AdapterRegistry::plan`] 已跑过能力校验，
    /// 因此这个方法处理的是语言自身的语法规则。
    fn plan(&self, request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String>;

    /// Read one existing declaration back, so a modify request can preserve what it does not name.
    /// 读回一条已有声明，使修改请求能保留它没有点名的一切。
    ///
    /// An adapter that cannot read its own output must say so: a modify that silently rewrote a
    /// declaration from the request alone would delete every field the request omitted.
    /// 读不回自己输出的适配器必须说出来：只按请求重写声明的修改会删掉请求省略的每个字段。
    fn read(&self, _source: &str) -> Result<FaceReadBack, String> {
        Err(format!(
            "`{}` does not read declarations back, so a modify would drop every field the request \
             does not name",
            self.capabilities().language.as_str()
        ))
    }

    /// Where this language puts a child's source file.
    /// 这门语言把子面的源码文件放在哪里。
    ///
    /// The default is "beside the parent", which is what the compiled languages this build carries
    /// do; a language whose sources must live under the scan root overrides it.
    /// 默认是"住在父旁边"，也就是本构建携带的编译型语言的做法；源码必须住在扫描根下的语言覆盖它。
    fn placement(&self) -> SourcePlacement {
        SourcePlacement::BesideParent
    }

    /// The editor material this language's author needs, as planned files.
    /// 这门语言的作者需要的编辑器资料，按计划文件给出。
    ///
    /// The adapter owns the shape: a C++ adapter writes `compile_flags.txt`, a Python adapter may
    /// write nothing because its SDK is resolved through `PYTHONPATH`. The surface applies what it
    /// returns through the same transaction as the rest of the plan, and leaves an existing file
    /// untouched (a generated file the user edited is the user's).
    /// 形状由适配器拥有：C++ 适配器写 `compile_flags.txt`，Python 适配器可能什么都不写，因为它的 SDK
    /// 经 `PYTHONPATH` 解析。执行面把返回的文件与计划其余部分走同一个事务，并保留已存在的文件（用户改过
    /// 的生成文件属于用户）。
    fn editor_material(&self, _context: &EditorContext) -> Vec<PlannedFile> {
        Vec::new()
    }
}

/// The adapters one build carries, keyed by language id.
/// 一个构建携带的适配器，按语言标识索引。
pub struct AdapterRegistry {
    adapters: Vec<&'static dyn LanguageAdapter>,
}

impl AdapterRegistry {
    /// The adapters compiled into this build.
    /// 本构建编译进来的适配器。
    ///
    /// Adding a language means adding one entry here and one implementation module; no surface
    /// changes, because every surface asks this registry.
    /// 新增语言意味着在这里加一个条目、加一个实现模块；执行面不改，因为每个执行面都问这张注册表。
    pub fn builtin() -> Self {
        #[allow(unused_mut)]
        let mut adapters: Vec<&'static dyn LanguageAdapter> = Vec::new();
        #[cfg(feature = "lang-cpp")]
        {
            adapters.push(&CppLanguage);
        }
        #[cfg(feature = "lang-python")]
        {
            adapters.push(&PythonLanguage);
        }
        #[cfg(feature = "lang-zig")]
        {
            adapters.push(&ZigLanguage);
        }
        #[cfg(feature = "lang-js")]
        {
            adapters.push(&JsLanguage);
        }
        #[cfg(feature = "lang-ts")]
        {
            adapters.push(&TsLanguage);
        }
        #[cfg(feature = "lang-go")]
        {
            adapters.push(&GoLanguage);
        }
        #[cfg(feature = "lang-java")]
        {
            adapters.push(&JavaLanguage);
        }
        #[cfg(feature = "lang-csharp")]
        {
            adapters.push(&CSharpLanguage);
        }
        #[cfg(feature = "lang-c")]
        {
            adapters.push(&CLanguage);
        }
        Self { adapters }
    }

    /// The adapter for one language, when this build has it.
    /// 本构建拥有该语言时返回它的适配器。
    pub fn adapter(&self, language: &LanguageId) -> Option<&'static dyn LanguageAdapter> {
        self.adapters
            .iter()
            .copied()
            .find(|adapter| &adapter.capabilities().language == language)
    }

    /// The languages this build can author, in registry order.
    /// 本构建可以创作的语言，按注册表顺序。
    pub fn languages(&self) -> Vec<&'static LanguageId> {
        self.adapters
            .iter()
            .map(|adapter| &adapter.capabilities().language)
            .collect()
    }

    /// The capability tables this build carries.
    /// 本构建携带的能力表。
    pub fn capabilities(&self) -> Vec<&'static LanguageCapabilities> {
        self.adapters
            .iter()
            .map(|adapter| adapter.capabilities())
            .collect()
    }

    /// The language a file's extension names, when one adapter claims it.
    /// 文件扩展名点名的语言（当某个适配器认领它时）。
    pub fn language_of_path(&self, path: &str) -> Option<&'static LanguageId> {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())?;
        self.adapters
            .iter()
            .find(|adapter| {
                adapter
                    .capabilities()
                    .file_extensions
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            })
            .map(|adapter| &adapter.capabilities().language)
    }

    /// Validate and plan one request.
    /// 校验并计划一份请求。
    pub fn plan(&self, request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
        let adapter = self.adapter(&request.language).ok_or_else(|| {
            let known = self
                .languages()
                .iter()
                .map(|language| format!("`{}`", language.as_str()))
                .collect::<Vec<_>>();
            if known.is_empty() {
                format!(
                    "`{}` support is not compiled into this build; no language adapter is enabled",
                    request.language.as_str()
                )
            } else {
                format!(
                    "`{}` support is not compiled into this build; enabled languages are {}",
                    request.language.as_str(),
                    known.join(", ")
                )
            }
        })?;
        validate_request(request, adapter.capabilities())?;
        adapter.plan(request)
    }
}

/// The registry a caller should use unless it has a reason to build its own.
/// 调用方应当使用的注册表，除非它有理由自建。
pub fn registry() -> AdapterRegistry {
    AdapterRegistry::builtin()
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    use xirang_kernel::authoring::{
        AuthoringOperation, FaceAuthoringRequest, ParentReference, RegistrationUpdate,
        SourceContext,
    };

    fn cpp() -> LanguageId {
        LanguageId::parse_language("cpp").expect("a language id")
    }

    fn request(language: LanguageId) -> FaceAuthoringRequest {
        FaceAuthoringRequest {
            language,
            operation: AuthoringOperation::Create,
            destination: "src/shop/panel/object/area/area.cpp".to_owned(),
            parent: ParentReference::Named("::xirang::faces::shop__panel_Panel".to_owned()),
            registration: RegistrationUpdate::new(),
            source_context: SourceContext::new(),
        }
    }

    /// Without the feature the build refuses C++ by name instead of silently pretending it has no
    /// such language: "not compiled in" and "unknown" are different answers.
    /// 没有特性时，构建按名拒绝 C++，而不是静默装作没有这门语言："没有编译进来"与"不认识"是两个答案。
    #[cfg(not(feature = "lang-cpp"))]
    #[test]
    fn a_disabled_language_is_refused_as_disabled() {
        let error = registry().plan(&request(cpp())).unwrap_err();
        assert!(error.contains("not compiled into this build"), "{error}");
        assert!(error.contains("cpp"), "{error}");
    }

    /// With the feature the adapter is reachable through the same registry call every surface uses.
    /// 打开特性后，适配器经每个执行面都用的同一个注册表调用可达。
    #[cfg(feature = "lang-cpp")]
    #[test]
    fn an_enabled_language_is_reachable_through_the_registry() {
        let registry = registry();
        assert!(registry.adapter(&cpp()).is_some());
        assert_eq!(
            registry
                .language_of_path("src/shop/area.cpp")
                .map(LanguageId::as_str),
            Some("cpp")
        );
        let plan = registry.plan(&request(cpp())).expect("the adapter plans");
        assert_eq!(plan.language, cpp());
        assert!(!plan.files.is_empty());
    }

    /// A Rust request is refused by this crate: the Rust adapter lives in the toolchain because it
    /// reuses the existing generator, and the registry must not pretend otherwise.
    /// Rust 请求被本 crate 拒绝：Rust 适配器住在工具链里，因为它复用现有生成器；注册表不得假装不是。
    #[test]
    fn rust_is_not_a_language_of_this_crate() {
        let rust = LanguageId::parse_language("rust").expect("a language id");
        let error = registry().plan(&request(rust)).unwrap_err();
        assert!(error.contains("rust"), "{error}");
    }
}

#[cfg(test)]
#[path = "registration_tests.rs"]
mod registration_tests;
