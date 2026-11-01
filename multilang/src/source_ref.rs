//! The explicit source fact shared by readers: where a declaration comes from, which snapshot,
//! and where this machine reads it. No consumer picks a root by trying which file exists.
//! 读取方共享的显式源码事实：声明来自哪个来源、哪份快照、本机从哪里读取。消费者不再靠
//! "哪个文件存在"选择根目录。

use std::path::{Path, PathBuf};

use xirang_kernel::identity::NodeId;

/// Which snapshot this fact was read from. A workspace has no fixed version: its content is
/// checked by digest at preview/apply time, not replaced by a label.
/// 本事实读取自哪份快照。工作区没有固定版本：内容在预览/应用时用摘要校验，而不是用标签冒充。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceSnapshot {
    /// The live workspace; the content digest is taken when a plan is made.
    /// 活动工作区；计划生成时取内容摘要。
    Workspace,
    /// A fixed external source root.
    /// 固定的外部源码根。
    External {
        /// The root the fixed snapshot was materialized at on this machine.
        /// 该固定快照在本机物化到的根目录。
        root: PathBuf,
    },
}

/// One declaration's source fact.
/// 一条声明的源码事实。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRef {
    /// The stable object this declaration belongs to. Normalization must never change it.
    /// 该声明所属的稳定对象。规范化绝不能改变它。
    pub object: NodeId,
    /// The declaration file, relative to the source root (never an absolute path).
    /// 声明文件，相对来源根（绝不是绝对路径）。
    pub source: String,
    /// Which snapshot supplied it.
    /// 由哪份快照提供。
    pub snapshot: SourceSnapshot,
}

/// The base a recorded source string is relative to.
/// 一条源码字符串所相对的基准。
///
/// A record says which base its own format means; a consumer never discovers it by trying which
/// file exists. `Identity` is the build's identity path (relative to `SourceLayout::identity_base`,
/// the historical `src/`), `Package` is relative to the package root.
/// 记录自己说明它的格式以哪个基准为准；消费者绝不靠"哪个文件存在"去发现它。`Identity` 是构建的
/// 身份路径（相对 `SourceLayout::identity_base`，历史上就是 `src/`），`Package` 相对包根。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceBase {
    /// Relative to the layout's identity base.
    /// 相对布局的身份基准。
    Identity,
    /// Relative to the package root.
    /// 相对包根。
    Package,
}

/// Convert a record's source spelling into the package-relative standard, from the declared base.
/// 按声明的基准把记录的源码拼写转成包内相对的标准形式。
///
/// This is the compatibility conversion, and it takes the base as data instead of guessing from a
/// language, an extension, or which file happens to exist. The result is the spelling every
/// consumer joins to the package root.
/// 这是兼容转换：基准作为数据传入，而不是从语言、扩展名或"哪个文件恰好存在"去猜。结果就是每个消费者
/// 拼到包根上的那个拼写。
pub fn package_relative(
    base: SourceBase,
    package_root: &Path,
    identity_base: &Path,
    source: &str,
) -> String {
    let source = source.trim_start_matches("./");
    match base {
        SourceBase::Package => source.to_owned(),
        SourceBase::Identity => {
            // The only non-empty identity prefix this layout produces is `src`; a layout whose scan
            // root is elsewhere sets `identity_base` to the package root, where the join is empty.
            // 本布局唯一会产生的非空身份前缀就是 `src`；扫描根在别处时布局把 `identity_base` 设为包根，
            // 那时拼接为空。
            let prefix = identity_base
                .strip_prefix(package_root)
                .unwrap_or_else(|_| Path::new(""));
            if prefix.as_os_str().is_empty() {
                source.to_owned()
            } else {
                format!("{}/{}", prefix.to_string_lossy().replace('\\', "/"), source)
            }
        }
    }
}

/// Resolve the one readable path this fact names. Missing is an error, never a fallback.
/// 解析本事实点名的那一个可读路径。缺失是错误，绝不回退。
///
/// **The snapshot decides which root is read.** A `Workspace` fact is read from the caller's root; an
/// `External` fact is read from the root that snapshot was materialized at, and the caller's root is
/// not consulted at all. Before this, the field was ignored and a probe with `External { root: B }`
/// read the same-named file under `root: A` — a resolver answering a question nobody asked.
/// **由快照决定读哪个根。** `Workspace` 事实从调用方的根读取；`External` 事实从该快照物化到的根读取，
/// 调用方的根完全不参与。在此之前这个字段被忽略，`External { root: B }` 的探针读到的是 `root: A` 下的
/// 同名文件——一个在回答没人问过的问题的解析器。
pub fn locate(root: &Path, source: &SourceRef) -> Result<PathBuf, String> {
    let (snapshot_root, described) = match &source.snapshot {
        SourceSnapshot::Workspace => (root.to_path_buf(), "the workspace".to_owned()),
        SourceSnapshot::External { root } => {
            (root.clone(), format!("the snapshot at {}", root.display()))
        }
    };
    locate_relative(&snapshot_root, &source.source)
        .map_err(|error| format!("declaration `{}` in {described}: {error}", source.object))
}

/// Locate a package-relative source that has no object identity yet — a configured native source
/// being read or compiled. Same rule, one implementation: a compiler input must not be joined by a
/// second copy of "which root" or allowed to leave the package.
/// 定位一个尚无对象身份的包内相对源码——例如正在读取或编译的已配置本地源码。同一条规则、同一份实现：
/// 编译器输入不得由第二份"哪个根"去拼，也不得离开包。
pub fn locate_relative(root: &Path, source: &str) -> Result<PathBuf, String> {
    let relative = Path::new(source.trim_start_matches("./"));
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("`{source}` is not a package-relative source path"));
    }
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!(
            "`{source}` is not at `{}` (the source is missing; no other location is tried)",
            path.display()
        ));
    }
    // A symlink is a legitimate way to materialize a source, so it is resolved rather than refused —
    // but the **resolved** location must still be inside a root this project declared: the package
    // itself, or a source `xirang.sources.tsv` names (with this machine's binding). A link that
    // leaves both would make the caller read a file no source declared, which is the read-side half
    // of the rule the enumeration module owns. The declared roots are read from the same project
    // files `SourceScope::with_sources` uses, so "declared" means one thing in both places.
    // 符号链接是物化源码的合法方式，因此解析它而不是拒绝它——但**解析后**的位置仍必须落在项目声明过的根里：
    // 包自身，或 `xirang.sources.tsv` 点名（并由本机绑定）的来源。两者都越界才会让调用方读到没有任何来源
    // 声明过的文件，那是枚举模块所拥有规则在读取侧的那一半。已声明根读自 `SourceScope::with_sources`
    // 用的同一批项目文件，因此"已声明"在两边是同一个意思。
    let resolved = std::fs::canonicalize(&path)
        .map_err(|error| format!("cannot resolve {}: {error}", path.display()))?;
    let canonical_root = std::fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    if !resolved.starts_with(&canonical_root) {
        let declared = declared_roots(root)?;
        if declared
            .iter()
            .any(|declared| resolved.starts_with(declared))
        {
            return Ok(path);
        }
        return Err(format!(
            "`{source}` resolves to {}, which is outside {} and outside every source \
             `xirang.sources.tsv` declares; a link is not a way to read a source that was not \
             declared",
            resolved.display(),
            canonical_root.display()
        ));
    }
    Ok(path)
}

/// The roots this project declares, canonicalized, with unbound declarations refused.
/// 本项目声明的根，已规范化；未绑定的声明被拒绝。
///
/// This is the read side of `xirang.sources.tsv`: a declared and bound source is a place a source
/// may be materialized at, so a link to it is a link to something the project asked for. It shares
/// the parser and the binding rules with `SourceScope::with_sources` rather than re-reading them
/// with different semantics.
/// 这是 `xirang.sources.tsv` 的读取侧：已声明并绑定的来源是源码可以物化的位置，因此指向它的链接是指向项目
/// 要求过的东西。它与 `SourceScope::with_sources` 共用解析器与绑定规则，而不是用另一套语义再读一遍。
pub fn declared_roots(root: &Path) -> Result<Vec<PathBuf>, String> {
    let map = crate::source_map::SourceMap::read_from(root)?;
    let mut roots = Vec::new();
    for source in map.roots(root)? {
        for candidate in [Some(source.location), source.bound].into_iter().flatten() {
            let canonical = std::fs::canonicalize(&candidate)
                .map_err(|error| format!("cannot resolve {}: {error}", candidate.display()))?;
            if !roots.iter().any(|known| known == &canonical) {
                roots.push(canonical);
            }
        }
    }
    Ok(roots)
}

/// The identity spelling of one package-relative source: the historical base-relative form.
/// 一个包内相对源码的身份拼写：历史上的相对身份基准形式。
///
/// This is the **one** interpreter of the old spelling. Constructors call it; consumers receive the
/// record spelling as a fact and never strip a base themselves.
/// 这是旧拼写的**唯一**解释器。构造方调用它；消费者把记录拼写当事实接收，绝不自己 strip 基准。
pub fn identity_spelling(
    package_root: &Path,
    identity_base: &Path,
    package_source: &str,
) -> String {
    let package_source = package_source.trim_start_matches("./");
    let prefix = identity_base
        .strip_prefix(package_root)
        .unwrap_or_else(|_| Path::new(""));
    let prefix = prefix.to_string_lossy().replace('\\', "/");
    let prefixed = format!("{prefix}/");
    if !prefix.is_empty() && package_source.starts_with(&prefixed) {
        package_source[prefixed.len()..].to_owned()
    } else {
        package_source.to_owned()
    }
}

/// One file's package-relative source spelling.
/// 一个文件的包内相对源码拼写。
///
/// This is the spelling whose base is the package root, and the input [`identity_spelling`] takes.
/// A file outside the package is named as the caller handed it over rather than stripped into a
/// shorter, wrong fact.
/// 这是以包根为基准的拼写，也是 [`identity_spelling`] 所收的输入。包外的文件按调用方交来的样子
/// 命名，而不是被截成一条更短、错误的事实。
pub fn package_source(package_root: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(package_root).unwrap_or(file);
    xirang_kernel::declaration::portable_path(&relative.to_string_lossy())
}

/// The spelling the published records and identities carry for one file on disk.
/// 落盘文件所携带的记录拼写——已发布记录与身份用的那一个。
///
/// One fold with two inputs: the file's package-relative path, then the historical base-relative
/// form. A file inside the identity base loses that prefix; a file elsewhere in the package (a face
/// an explicit placement put outside the walk root) keeps its package-relative path.
///
/// **This is the only producer of that spelling.** The discovery walk and the authoring read-back
/// both call it, because the spelling is an identity input: when one of them derived it from the
/// identity base alone, a file outside that base came back as its absolute path, and the two
/// surfaces disagreed about one file's identity — an `edit` on the face then resolved to a node the
/// loaded registry did not hold (`node ... is not registered`).
///
/// 一次折叠、两个输入：文件的包内相对路径，再套历史上的相对身份基准形式。身份基准之内的文件去掉那段
/// 前缀；包内别处（显式落点放在遍历根之外的面）保留包内相对路径。
///
/// **这是该拼写的唯一生产者。** 发现遍历与创作读取都调用它，因为拼写是身份输入：当其中一方只从身份
/// 基准推导时，基准之外的文件会回成它的绝对路径，两个面对同一个文件的身份各说各话——对那个面的一次
/// `edit` 随后解析到已加载注册机并不持有的节点（`node ... is not registered`）。
pub fn record_spelling(package_root: &Path, identity_base: &Path, file: &Path) -> String {
    identity_spelling(
        package_root,
        identity_base,
        &package_source(package_root, file),
    )
}

/// The base a record's spelling is relative to, derived from the one package-relative fact.
/// 记录拼写所相对的基准，由包内相对这一唯一事实推出。
pub fn base_for(package_root: &Path, identity_base: &Path, package_source: &str) -> SourceBase {
    if identity_spelling(package_root, identity_base, package_source)
        == package_source.trim_start_matches("./")
    {
        SourceBase::Package
    } else {
        SourceBase::Identity
    }
}

/// Locate the one file a record spelling names, by folding each candidate's real path back.
/// 定位一条记录拼写所点名的那个文件：把每个候选的真实路径重新折叠回去。
///
/// A record spelling is an identity input, so only a file whose own path folds to exactly that
/// spelling can be the one it was written for. Both bases this layout has are tried, and a candidate
/// that does not fold back is skipped — the wrong root therefore fails the identity contract instead
/// of being accepted by accident. This is the read-side counterpart of [`record_spelling`], and it is
/// **not** an existence guess: the file is chosen by the fold, not by which path happens to be there.
/// 记录拼写是身份输入，因此只有一个自身路径恰好折叠回该拼写的文件才可能是它当初写的那个。布局拥有的两个
/// 基准都会被试，折叠不回去的候选被跳过——错误的那一个基准因此会让身份契约失败，而不是碰巧被接受。
/// 这是 [`record_spelling`] 的读取侧对应物，**不是**靠"哪个文件存在"去猜：文件由折叠选定，而不是由哪条
/// 路径恰好在那儿选定。
pub fn locate_record_spelling(
    package_root: &Path,
    identity_base: &Path,
    spelling: &str,
) -> Result<PathBuf, String> {
    let wanted = spelling.trim_start_matches("./");
    let relative = Path::new(wanted);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "`{spelling}` is not a package-relative source path; an identity spelling never \
             carries a root or a `..` step"
        ));
    }
    let mut candidates = vec![identity_base.join(relative), package_root.join(relative)];
    candidates.dedup();
    for candidate in candidates {
        if !candidate.is_file() {
            continue;
        }
        if record_spelling(package_root, identity_base, &candidate) == wanted {
            return Ok(candidate);
        }
    }
    Err(format!(
        "no file under {} folds to the record spelling `{wanted}`; the spelling is an identity \
         input, so only the file the build derived it from is accepted",
        package_root.display()
    ))
}

#[cfg(test)]
#[path = "source_ref_tests.rs"]
mod source_ref_tests;
