//! Read the faces a native object declares in its `xirang.faces` section — the language-neutral
//! half of a native language implementation.
//! 读取原生目标文件在 `xirang.faces` 段里声明的面——原生语言实现中语言中立的那一半。
//!
//! The format is a contract between a language's SDK and this reader, not between this crate and
//! C++: any language whose SDK emits the same record is read here, which is why the decoder lives
//! in the kit rather than under one language.
//! 该格式是某语言的 SDK 与本读取器之间的契约，而不是本 crate 与 C++ 之间的契约：任何语言的 SDK
//! 发出同样的记录都在这里被读取，这也是解码器住在 kit 而不是某一语言之下的原因。
//!
//! The other half of this contract is `toolchain/include/xirang/soil.hpp`: a
//! `XIRANG_FACE` declaration compiles to one self-describing record, and this
//! module turns those bytes back into the three fields the tree needs (parent,
//! name, whether children may hang from it) plus the exports list.
//! 这份契约的另一半是 `toolchain/include/xirang/soil.hpp`：一条 `XIRANG_FACE` 声明
//! 编译成一条自描述记录，本模块把这些字节翻译回树需要的三个字段（父面、名字、可否挂
//! 子面）以及导出列表。
//!
//! Records are found by their magic word and skipped forward by the length each one
//! records, because a linker may pad or reorder entries in a custom section: the gap
//! between two records is bytes this reader walks past, not a layout it assumes.
//! 记录按自身魔数找到、再按每条自报的长度向前跳，因为链接器可以给自定义段里的条目补齐
//! 或换序：两条记录之间的空隙是本读取器走过去的字节，而不是它假定的布局。
//!
//! The section is read out by `objcopy` (or `llvm-objcopy`); when neither is on
//! `PATH` this refuses instead of reporting that the object declares no faces,
//! because "I could not look" and "there was nothing there" are different answers.
//! 段由 `objcopy`（或 `llvm-objcopy`）读出；两者都不在 `PATH` 上时本模块**拒绝**，
//! 而不是报"该目标文件没有声明面"——"我没看成"与"那里本来就没有"是两个答案。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use xirang_kernel::identity::NodeId;

use crate::external_faces::{ExternalFaceRecord, ExternalSource};

/// The section a native face declaration compiles into.
/// 原生面声明编译进的那个段。
pub const SECTION: &str = "xirang.faces";

/// The first four bytes of every record, matching `multilang/sdk/cpp/xirang/soil.hpp`.
/// 每条记录的前四字节，与 `multilang/sdk/cpp/xirang/soil.hpp` 一致。
pub const MAGIC: [u8; 4] = *b"XFRC";

/// The older layout version, still read so a declaration written against version 1 keeps working.
/// 旧布局版本，仍然读取，使按版本 1 写下的声明继续可用。
pub const VERSION_ONE: u16 = 1;

/// The layout version this reader writes and prefers.
/// 本读取器所写并首选的布局版本。
pub const VERSION: u16 = 2;

/// A version-1 record's length: parent, flags, kind and exports only.
/// 版本 1 记录的长度：只有父、标志、kind 与导出。
pub const RECORD_BYTES_ONE: usize = 76;

/// One version-2 record's length, matching the SDK header's `layout::RECORD_BYTES`.
/// 一条版本 2 记录的长度，与 SDK 头文件的 `layout::RECORD_BYTES` 一致。
pub const RECORD_BYTES: usize = 1100;

/// The layout version a **build-time record file** writes: version 2 plus the source spelling.
/// **构建期记录文件**写下的布局版本：版本 2 再加源码拼写。
///
/// A section record is read by the build that just compiled its object, so the build already knows
/// the source. A record file is written by another toolchain (a JVM annotation processor, a `go run`
/// generator, a Roslyn generator) and read later, so the record has to carry the spelling that is
/// part of its identity. The field is appended, not interleaved, so every version-2 offset keeps
/// meaning what it meant.
/// 段记录由刚编译过目标文件的构建读取，因此构建已经知道源码。记录文件由另一套工具链写出（JVM 注解处理器、
/// `go run` 生成器、Roslyn 生成器）并被稍后读取，因此记录必须携带那部分属于身份的拼写。字段追加在末尾、
/// 不穿插在前面，因此版本 2 的每个偏移保持原意。
pub const VERSION_FILE: u16 = 3;

/// The source field's width in a version-3 record.
/// 版本 3 记录里源码字段的宽度。
pub const SOURCE_BYTES: usize = 128;

/// One version-3 record's length: the version-2 layout plus the source field.
/// 一条版本 3 记录的长度：版本 2 布局加源码字段。
pub const RECORD_BYTES_FILE: usize = RECORD_BYTES + SOURCE_BYTES;

/// The only flag bit this reader knows: this face owns a registry.
/// 本读取器唯一认识的标志位：该面拥有注册机。
pub const OWNS_REGISTRY: u16 = 1;

/// One face a C++ translation unit declared.
/// 一个 C++ 翻译单元声明的一个面。
///
/// A version-1 record carries only parent, name, exports and the flag; every field added by
/// version 2 defaults to empty, which is what the standard registration defaults are. "The record
/// did not carry it" is therefore the same answer as "the author did not declare it", and both are
/// different from "the adapter wrote `ANY` over it".
/// 版本 1 记录只携带父、名字、导出与标志；版本 2 新增的字段一律默认为空，而空就是标准注册默认值。
/// 因此"记录没携带它"与"作者没有声明它"是同一个答案，而两者都与"适配器用 `ANY` 盖住它"不同。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalFace {
    /// The parent face this declaration names.
    /// 该声明点名的父面。
    pub parent: NodeId,
    /// Whether children may hang from this face.
    /// 是否可以有子面挂在它下面。
    pub owns_registry: bool,
    /// The name this face has inside its parent, which is also the kind the registry keys on.
    /// 面在父面内的名字，也就是注册机据以索引的 kind。
    pub name: String,
    /// The exported names, in declaration order.
    /// 导出名，按声明顺序。
    pub exports: Vec<String>,
    /// The package-relative source spelling a version-3 (file) record carries.
    /// 版本 3（文件）记录携带的包内相对源码拼写。
    ///
    /// Empty for versions 1 and 2: the object carrier knows its source, so those records do not
    /// need to repeat it. A **file** record is read long after it was written, so it must.
    /// 版本 1 与 2 为空：目标文件载体知道自己的源码，因此那些记录不必重复它。**文件**记录在被写出很久之后
    /// 才被读取，因此必须携带。
    pub source: String,
    /// The name the registry keys this face by, when it differs from [`Self::name`].
    /// 注册机索引本面所用的名字（当它与 [`Self::name`] 不同时）。
    pub registry_name: String,
    /// The optional author-owned identity that survives a source move.
    /// 可选的、作者拥有的、可跨源码移动保持的身份名。
    pub stable_name: String,
    /// Capability names this face provides.
    /// 本面提供的能力名。
    pub provides: Vec<String>,
    /// Capability requirements, as `(capability, provider)` pairs.
    /// 能力需求，按 `(capability, provider)` 对给出。
    pub requires: Vec<(String, String)>,
    /// The preset type path this face names for display and diagnostics.
    /// 本面为显示与诊断点名的 preset 类型路径。
    pub preset: String,
    /// The parts type path this face names.
    /// 本面点名的 parts 类型路径。
    pub parts: String,
    /// The construction parameter type path this face names.
    /// 本面点名的构造参数类型路径。
    pub params: String,
    /// The handle type path this face names.
    /// 本面点名的 handle 类型路径。
    pub handle: String,
    /// The Chinese half of the display name.
    /// 显示名的中文部分。
    pub name_zh: String,
    /// The English half of the display name.
    /// 显示名的英文部分。
    pub name_en: String,
    /// The Chinese half of the one-line summary.
    /// 单行摘要的中文部分。
    pub summary_zh: String,
    /// The English half of the one-line summary.
    /// 单行摘要的英文部分。
    pub summary_en: String,
    /// The external registry this face is provisioned from.
    /// 本面取自哪个外部注册机。
    pub getting_from_other_registry: String,
}

/// Read every face these objects declare, assigning each one the identity its
/// spelling earns in `namespace`.
/// 读出这些目标文件声明的每个面，并按拼写在 `namespace` 下给每个面分配身份。
pub fn records(
    objects: &[ExternalSource<'_>],
    namespace: &str,
) -> Result<Vec<ExternalFaceRecord>, String> {
    let mut records = Vec::new();
    for object in objects {
        for face in read_faces(object.object)? {
            records.push(record(face, object.source, namespace));
        }
    }
    Ok(records)
}

/// Read every face a **build-time record file** declares, each record naming its own source.
/// 读出**构建期记录文件**声明的每个面，每条记录点名自己的源码。
///
/// The file is written by a toolchain that does not produce a linkable object (a JVM annotation
/// processor, a `go run` generator, a Roslyn source generator), so the record carries the source
/// spelling the build cannot otherwise learn. The bytes are the same record layout the section
/// reader walks: one decoder, two carriers. A version-1 or version-2 record in a file is refused by
/// name, because those layouts have no source field and the build would have to guess it.
/// 该文件由不产出可链接目标文件的工具链写出（JVM 注解处理器、`go run` 生成器、Roslyn 源生成器），因此记录
/// 携带构建否则学不到的源码拼写。字节就是段读取器所走的同一种记录布局：一个解码器、两种载体。文件里的
/// 版本 1 或 2 记录按名拒绝，因为那些布局没有源码字段，构建只能猜。
pub fn read_file_faces(path: &Path) -> Result<Vec<ExternalFace>, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!(
            "{} is empty; a build-time record file that declares nothing is a producer bug, not a \
             tree with no faces",
            path.display()
        ));
    }
    let faces = decode(&bytes)?;
    for face in &faces {
        if face.source.is_empty() {
            return Err(format!(
                "{}: a `{SECTION}` record names no source; a build-time record file needs layout \
                 version {VERSION_FILE} (the source field), and this record is an older layout \
                 whose source the build cannot learn",
                path.display()
            ));
        }
    }
    Ok(faces)
}

/// Read a build-time record file into the records the renderer consumes.
/// 把构建期记录文件读成渲染器消费的记录。
pub fn records_from_file(path: &Path, namespace: &str) -> Result<Vec<ExternalFaceRecord>, String> {
    Ok(read_file_faces(path)?
        .into_iter()
        .map(|face| {
            let source = face.source.clone();
            record(face, &source, namespace)
        })
        .collect())
}

/// One decoded face as the record the tree renders, with the source its identity is derived from.
/// 一条解码后的面，按树所渲染的记录给出，并带上推导身份所用的源码。
fn record(face: ExternalFace, source: &str, namespace: &str) -> ExternalFaceRecord {
    ExternalFaceRecord {
        id: NodeId::from_namespaced_path(namespace, source, &face.name),
        parent: face.parent,
        owns_registry: face.owns_registry,
        name: face.name,
        exports: face.exports,
        source: source.to_owned(),
        registry_name: face.registry_name,
        stable_name: face.stable_name,
        provides: face.provides,
        requires: face.requires,
        preset: face.preset,
        parts: face.parts,
        params: face.params,
        handle: face.handle,
        name_zh: face.name_zh,
        name_en: face.name_en,
        summary_zh: face.summary_zh,
        summary_en: face.summary_en,
        getting_from_other_registry: face.getting_from_other_registry,
    }
}

/// Read every face `object` declares.
/// 读出 `object` 声明的全部面。
///
/// An object without the section is not a failure: it contributes nothing.
/// 没有该段的目标文件不算失败：它什么也不贡献。
/// Compatibility alias for the published `cpp_record::read` spelling.
/// 已发布拼法 `cpp_record::read` 的兼容别名。
#[deprecated(note = "use cpp_record::read_faces; this alias keeps 0.2 callers compiling")]
pub fn read(object: &Path) -> Result<Vec<ExternalFace>, String> {
    read_faces(object)
}

/// Decodes the external faces recorded in one object file.
/// 解码一个目标文件里记录的外部面。
pub fn read_faces(object: &Path) -> Result<Vec<ExternalFace>, String> {
    match dump(object)? {
        Some(bytes) => decode(&bytes),
        None => Ok(Vec::new()),
    }
}

/// Turn the section bytes into faces, refusing any shape this reader does not
/// understand. Exposed separately so the format can be tested without a compiler.
/// 把段字节翻译成面，拒绝本读取器不理解的任何形状。单独暴露出来，使得格式可以在没有
/// 编译器的情况下被测试。
pub fn decode(bytes: &[u8]) -> Result<Vec<ExternalFace>, String> {
    let mut faces: Vec<ExternalFace> = Vec::new();
    let mut at = 0usize;
    while at + MAGIC.len() <= bytes.len() {
        if bytes[at..at + MAGIC.len()] != MAGIC {
            at += 1;
            continue;
        }
        let length = frame(bytes, at)?;
        let face = one(bytes, at)?;
        if faces
            .iter()
            .any(|seen| seen.parent == face.parent && seen.name == face.name)
        {
            return Err(format!(
                "the `{SECTION}` section declares `{}` twice under the same parent; one                  translation unit declares each face once",
                face.name
            ));
        }
        faces.push(face);
        at += length;
    }
    Ok(faces)
}

/// The length a record at `at` claims, checked against the layouts this reader walks.
/// `at` 处记录自报的长度，并与本读取器所走的布局核对。
///
/// The length is read from the record itself, not assumed: a linker may pad a custom section, and a
/// version-2 record is longer than a version-1 one. What is refused is a length this reader has no
/// layout for, because walking it would mean guessing where the next record starts.
/// 长度从记录自身读出，而不是假定：链接器可能给自定义段补齐，而版本 2 的记录比版本 1 长。被拒绝的是
/// 本读取器没有对应布局的长度——按它走就意味着猜下一条记录从哪里开始。
fn frame(bytes: &[u8], at: usize) -> Result<usize, String> {
    if at + 12 > bytes.len() {
        return Err(format!(
            "a `{SECTION}` record header at offset {at} is truncated: {} byte(s) remain",
            bytes.len() - at
        ));
    }
    let length = usize::from(u16::from_le_bytes([bytes[at + 8], bytes[at + 9]]));
    if length != RECORD_BYTES && length != RECORD_BYTES_ONE && length != RECORD_BYTES_FILE {
        return Err(format!(
            "a `{SECTION}` record at offset {at} says it is {length} bytes; this reader walks \
             {RECORD_BYTES_ONE} (version 1), {RECORD_BYTES} (version 2) or {RECORD_BYTES_FILE} \
             (version 3), so it cannot find the next record"
        ));
    }
    if at + length > bytes.len() {
        return Err(format!(
            "a `{SECTION}` record at offset {at} is truncated: {} byte(s) remain of {length}",
            bytes.len() - at
        ));
    }
    Ok(length)
}

/// Decode the record at `at`, whose four magic bytes the caller already matched.
/// 解码 `at` 处的记录；调用方已经匹配过它的四个魔数字节。
fn one(bytes: &[u8], at: usize) -> Result<ExternalFace, String> {
    let length = frame(bytes, at)?;
    let version = u16::from_le_bytes([bytes[at + 4], bytes[at + 5]]);
    let expected = match length {
        RECORD_BYTES_ONE => VERSION_ONE,
        RECORD_BYTES => VERSION,
        _ => VERSION_FILE,
    };
    if version != expected {
        return Err(format!(
            "a `{SECTION}` record at offset {at} is {length} bytes but claims layout version \
             {version}; that length is the version-{expected} layout"
        ));
    }
    let flags = u16::from_le_bytes([bytes[at + 6], bytes[at + 7]]);
    if flags & !OWNS_REGISTRY != 0 {
        return Err(format!(
            "a `{SECTION}` record at offset {at} sets unknown flag bits {:#x}; this reader knows \
             {:#x} (owns a registry) and refuses to guess the rest",
            flags & !OWNS_REGISTRY,
            OWNS_REGISTRY
        ));
    }
    if bytes[at + 10] != 0 || bytes[at + 11] != 0 {
        return Err(format!(
            "a `{SECTION}` record at offset {at} has non-zero reserved bytes; a record this reader \
             can read writes them as zero, so this one came from a writer it does not know"
        ));
    }
    let mut parent = [0u8; 16];
    parent.copy_from_slice(&bytes[at + 12..at + 28]);
    let version_one = version == VERSION_ONE;
    let name = text(field(bytes, at, 28, if version_one { 16 } else { 32 })).map_err(|why| {
        format!("a `{SECTION}` record at offset {at} has an unusable name: {why}")
    })?;
    if name.is_empty() {
        return Err(format!(
            "a `{SECTION}` record at offset {at} names no face; a record without a name would be a \
             face nothing can refer to"
        ));
    }
    if version_one {
        let exports = list(field(bytes, at, 44, 32)).map_err(|why| {
            format!("a `{SECTION}` record at offset {at} has an unusable exports list: {why}")
        })?;
        return Ok(ExternalFace {
            parent: NodeId::from_raw(parent),
            owns_registry: flags & OWNS_REGISTRY != 0,
            name,
            exports,
            source: String::new(),
            registry_name: String::new(),
            stable_name: String::new(),
            provides: Vec::new(),
            requires: Vec::new(),
            preset: String::new(),
            parts: String::new(),
            params: String::new(),
            handle: String::new(),
            name_zh: String::new(),
            name_en: String::new(),
            summary_zh: String::new(),
            summary_en: String::new(),
            getting_from_other_registry: String::new(),
        });
    }
    // A version-2 field is read through one helper so an unusable field names both its role and
    // the record offset, rather than reporting an anonymous "bad bytes".
    // 版本 2 的每个字段都经同一个助手读取，因此读不懂的字段会同时点名它的角色与记录偏移，而不是报一句
    // 无名的"字节不对"。
    let read = |role: &str, at_field: usize, width: usize| -> Result<String, String> {
        text(field(bytes, at, at_field, width)).map_err(|why| {
            format!("a `{SECTION}` record at offset {at} has an unusable {role}: {why}")
        })
    };
    let read_list = |role: &str, at_field: usize, width: usize| -> Result<Vec<String>, String> {
        list(field(bytes, at, at_field, width)).map_err(|why| {
            format!("a `{SECTION}` record at offset {at} has an unusable {role}: {why}")
        })
    };
    let requires_text = read("requirements list", 332, 128)?;
    let mut requires = Vec::new();
    for entry in requires_text
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (capability, provider) = entry.split_once("=>").ok_or_else(|| {
            format!(
                "a `{SECTION}` record at offset {at} has a requirement `{entry}` without `=>`; a \
                 capability requirement names both the capability and the provider expected to \
                 supply it"
            )
        })?;
        let capability = capability.trim();
        let provider = provider.trim();
        if capability.is_empty() || provider.is_empty() {
            return Err(format!(
                "a `{SECTION}` record at offset {at} has an empty half in the requirement `{entry}`"
            ));
        }
        requires.push((capability.to_owned(), provider.to_owned()));
    }
    let source = if length == RECORD_BYTES_FILE {
        read("source spelling", 1100, SOURCE_BYTES)?
    } else {
        String::new()
    };
    Ok(ExternalFace {
        parent: NodeId::from_raw(parent),
        owns_registry: flags & OWNS_REGISTRY != 0,
        name,
        exports: read_list("exports list", 140, 96)?,
        source,
        registry_name: read("registry name", 60, 48)?,
        stable_name: read("stable name", 108, 32)?,
        provides: read_list("provides list", 236, 96)?,
        requires,
        preset: read("preset path", 460, 64)?,
        parts: read("parts path", 524, 64)?,
        params: read("params path", 588, 64)?,
        handle: read("handle path", 652, 64)?,
        name_zh: read("Chinese name", 716, 64)?,
        name_en: read("English name", 780, 64)?,
        summary_zh: read("Chinese summary", 844, 96)?,
        summary_en: read("English summary", 940, 96)?,
        getting_from_other_registry: read("source registry", 1036, 64)?,
    })
}

/// One fixed-width field of the record at `at`.
/// `at` 处记录里的一个定宽字段。
fn field(bytes: &[u8], at: usize, offset: usize, width: usize) -> &[u8] {
    &bytes[at + offset..at + offset + width]
}

/// A NUL-terminated UTF-8 field. The bytes after the terminator must be NUL too,
/// so a shorter string written into a fixed field cannot hide a stale tail.
/// NUL 结尾的 UTF-8 字段。终止符之后的字节必须也是 NUL，于是写进定长字段的较短字符串
/// 藏不住残留尾巴。
fn text(field: &[u8]) -> Result<String, String> {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| "it has no NUL terminator".to_owned())?;
    if field[end + 1..].iter().any(|byte| *byte != 0) {
        return Err(format!(
            "it has {} non-NUL byte(s) after its terminator",
            field.len() - end - 1
        ));
    }
    String::from_utf8(field[..end].to_vec()).map_err(|_| "it is not UTF-8".to_owned())
}

/// A NUL-terminated, comma-separated list. An empty list is allowed.
/// NUL 结尾、逗号分隔的列表。空列表是允许的。
fn list(field: &[u8]) -> Result<Vec<String>, String> {
    let text = text(field)?;
    Ok(text
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Copy the section out of `object`, or `None` when the object has none.
/// 把段从 `object` 里拷出来；目标文件没有该段时返回 `None`。
fn dump(object: &Path) -> Result<Option<Vec<u8>>, String> {
    let tool = objcopy().ok_or_else(|| {
        format!(
            "cannot read `{}`: neither `objcopy` nor `llvm-objcopy` is on PATH, and a `{SECTION}` \
             section can only be read out of a compiled object by one of them",
            object.display()
        )
    })?;
    let dumped = scratch("section");
    let scratch_object = scratch("object");
    let argument = format!("{SECTION}={}", dumped.display());
    let output = Command::new(&tool)
        .arg("--dump-section")
        .arg(&argument)
        .arg(object)
        .arg(&scratch_object)
        .output()
        .map_err(|error| format!("cannot run `{tool}` on `{}`: {error}", object.display()))?;
    let bytes = std::fs::read(&dumped).ok();
    // A leftover temporary file is this read's own mess rather than its failure:
    // the answer is the section bytes, and they have already been copied out.
    // 残留的临时文件只是本读取器自己的垃圾，不是它的失败：答案就是段字节，而它们已经拷出来了。
    for path in [&dumped, &scratch_object] {
        let _ = std::fs::remove_file(path);
    }
    if let Some(bytes) = bytes {
        return Ok(Some(bytes));
    }
    let said = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if said.contains("can't dump section") || said.contains("does not exist") {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(format!(
            "`{tool}` could not read `{}`: {said}",
            object.display()
        ));
    }
    Err(format!(
        "`{tool}` reported success on `{}` but produced no `{SECTION}` bytes and did not say the \
         section was missing: {said}",
        object.display()
    ))
}

/// A unique temporary path. The name carries a counter, so concurrent reads in one
/// process cannot collide.
/// 唯一的临时路径。名字带计数器，因此同一进程里的并发读取不会撞车。
fn scratch(kind: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "xirang-faces-{}-{}-{kind}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// The first section-copying program on PATH.
/// `PATH` 上第一个能拷段的程序。
fn objcopy() -> Option<String> {
    for tool in ["objcopy", "llvm-objcopy"] {
        if Command::new(tool)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Some(tool.to_owned());
        }
    }
    None
}

#[cfg(test)]
#[path = "record_tests.rs"]
mod record_tests;
