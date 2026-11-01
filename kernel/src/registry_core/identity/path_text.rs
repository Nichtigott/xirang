use super::*;

/// Return the part of `value` after `prefix`, or `None` when it is not one.
/// 返回 `value` 中 `prefix` 之后的部分；`prefix` 不匹配时返回 `None`。
///
/// Written with scalar indexing and `split_at` because range indexing and
/// `slice::get` are not usable in constant functions on the supported
/// toolchain. 使用标量索引与 `split_at`：在受支持的工具链上，范围索引和
/// `slice::get` 无法用于常量函数。
///
/// Internal to the identity module: the two helpers that need it from outside
/// (`manifest_relative_source`, `strip_path_prefix`) are exported instead.
/// 身份模块内部使用：真正需要它对外的两个辅助函数（`manifest_relative_source`、
/// `strip_path_prefix`）已经导出。
pub(crate) const fn strip_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let value_bytes = value.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.len() > value_bytes.len() {
        return None;
    }
    let mut index = 0;
    while index < prefix_bytes.len() {
        if value_bytes[index] != prefix_bytes[index] {
            return None;
        }
        index += 1;
    }
    let (_, rest) = value_bytes.split_at(index);
    match core::str::from_utf8(rest) {
        Ok(text) => Some(text),
        Err(_) => None,
    }
}

/// Like `strip_prefix`, but `/` and `\` count as the same separator.
/// 与 `strip_prefix` 相同，但 `/` 与 `\` 视为同一分隔符。
///
/// Cargo reports `CARGO_MANIFEST_DIR` with backslashes on Windows while the
/// build step writes `#[path]` literals with forward slashes, so an exact
/// comparison would fail there and leave the absolute path as the identity.
/// Cargo 在 Windows 上以反斜杠给出 `CARGO_MANIFEST_DIR`，而构建步骤写出的
/// `#[path]` 字面量用正斜杠；精确比较在那边会失败，身份会退化成绝对路径。
pub const fn strip_path_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let value_bytes = value.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.len() > value_bytes.len() {
        return None;
    }
    let mut index = 0;
    while index < prefix_bytes.len() {
        let left = value_bytes[index];
        let right = prefix_bytes[index];
        if left != right && !(is_separator(left) && is_separator(right)) {
            return None;
        }
        index += 1;
    }
    let (_, rest) = value_bytes.split_at(index);
    // The prefix has to end on a component boundary: `/work/app` is not a path
    // prefix of `/work/application/src/a.rs`, and stripping it there would invent
    // the identity `lication/src/a.rs` for a file that was never inside the
    // project.
    // 前缀必须结束在组件边界上：`/work/app` 不是 `/work/application/src/a.rs` 的路径
    // 前缀，在那里剥离会为一个从来不在项目里的文件凭空造出身份
    // `lication/src/a.rs`。
    // A prefix that already ends with a separator is on a boundary by itself;
    // otherwise the character after the prefix has to be one.
    // 前缀本身以分隔符结尾时它已经落在边界上；否则前缀之后必须是分隔符。
    // `slice::last`/`Option::is_some_and` are not const-stable here, so the last
    // byte is read by index like the loop above does.
    // `slice::last`/`Option::is_some_and` 在本工具链上不是 const 稳定的，因此像上面的
    // 循环一样用下标读取最后一个字节。
    let prefix_ends_on_separator = match prefix_bytes.len() {
        0 => false,
        length => is_separator(prefix_bytes[length - 1]),
    };
    if !prefix_ends_on_separator
        && let Some(next) = rest.first()
        && !is_separator(*next)
    {
        return None;
    }

    match core::str::from_utf8(rest) {
        Ok(text) => Some(text),
        Err(_) => None,
    }
}

/// Last `::`-separated segment of a module path.
/// 模块路径中最后一段 `::` 分隔的分量。
///
/// Public because the declaration macros call it from the host crate to derive
/// a face's registry name from `module_path!()`.
/// 公开是因为注册面宏在宿主 crate 里调用它，从 `module_path!()` 推导注册机名。
pub const fn last_path_segment(path: &str) -> &str {
    let bytes = path.as_bytes();
    let mut index = bytes.len();
    while index > 0 {
        index -= 1;
        if bytes[index] == b':' {
            let (_, rest) = bytes.split_at(index + 1);
            return match core::str::from_utf8(rest) {
                Ok(text) => text,
                Err(_) => path,
            };
        }
    }
    path
}

/// Whether `file!()` came back absolute: a leading separator, or a drive prefix on Windows.
/// `file!()` 是否是绝对路径：前导分隔符，或 Windows 上的盘符前缀。
const fn is_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    match bytes.first() {
        Some(b'/') | Some(b'\\') => true,
        Some(byte) if byte.is_ascii_alphabetic() => bytes.len() > 1 && bytes[1] == b':',
        _ => false,
    }
}

/// The tail of an absolute path below the **last** `src/` component — the owning package's rule.
/// Returns the input unchanged when there is no such component (a `tests/` target, a differently
/// laid out crate), which is what the strict `manifest_dir` arm already produced for those.
/// 绝对路径中**最后一个** `src/` 分量之后的那一段——属主包的规则。没有该分量时原样返回（`tests/` 目标、
/// 别的布局的 crate），这也正是严格 `manifest_dir` 那一支过去对它们给出的结果。
const fn after_owning_src(path: &str) -> &str {
    let bytes = path.as_bytes();
    let mut index = 0;
    let mut found: Option<usize> = None;
    while index + 5 <= bytes.len() {
        let slash = bytes[index] == b'/' || bytes[index] == b'\\';
        if slash
            && bytes[index + 1] == b's'
            && bytes[index + 2] == b'r'
            && bytes[index + 3] == b'c'
            && (bytes[index + 4] == b'/' || bytes[index + 4] == b'\\')
        {
            found = Some(index + 5);
        }
        index += 1;
    }
    match found {
        // The **last** `src/`, not the first: a host may itself live under a directory named `src`
        // (`/home/me/src/proj/src/panel/…`), and the owning package's `src/` is the one nearest the
        // file. Both real mounts — `/tmp/…/app/src/panel/…` and the nested one — land on the right
        // tail with this rule.
        // 取**最后一个** `src/` 而不是第一个：宿主自己就可能住在叫 `src` 的目录下
        // （`/home/me/src/proj/src/panel/…`），而属主包的 `src/` 是**离文件最近**的那个。两种真实挂载
        // ——`/tmp/…/app/src/panel/…` 与上面那种嵌套——在这条规则下都取到正确的尾巴。
        Some(start) => {
            let (_, rest) = bytes.split_at(start);
            match core::str::from_utf8(rest) {
                Ok(text) => text,
                Err(_) => path,
            }
        }
        None => path,
    }
}

/// Derive a declaration's `source` identity input from `file!()`.
/// 从 `file!()` 推导声明的 `source` 身份输入。
///
/// Drops `manifest_dir` and one leading `src/` component so the value matches
/// the repository-relative path the build step used to inject, which keeps
/// `NodeId` stable across the generated and derived forms. The original value
/// is returned when `file` is not under `manifest_dir` (an external crate, a
/// `tests/` target, or a differently laid out package).
/// 去掉 `manifest_dir` 与其后的一个 `src/` 分量，使结果与构建步骤原本注入的
/// 仓库相对路径一致，从而让生成式与推导式产生相同的 `NodeId`。当 `file`
/// 不在 `manifest_dir` 下（外部 crate、`tests/` 目标或其它布局）时返回原值。
pub const fn manifest_relative_source<'a>(manifest_dir: &str, file: &'a str) -> &'a str {
    let after_manifest = match strip_path_prefix(file, manifest_dir) {
        Some(rest) => rest,
        // A face compiled by **another** package (a partition fragment, a facade) is mounted by
        // absolute path, so `file!()` is not under this crate's manifest dir. The owning package is
        // what the build derived the plan from, and the build's rule is "the path below the owning
        // package's `src/`" — so this derives the same value from the tail instead of returning an
        // absolute path, which used to make the identity depend on where the checkout sits on disk
        // (measured: a fragment's face hashed `/tmp/…/app/src/…` and disagreed with the plan's id).
        // 由**另一个**包编译的面（分区碎片、facade）是按绝对路径挂进来的，因此 `file!()` 不在本 crate 的
        // manifest 目录下。计划是构建按**属主包**推导的，而构建那条规则是"属主包 `src/` 之下那一段"——
        // 所以这里从尾部推出同一个值，而不是返回绝对路径：后者曾让身份取决于检出在磁盘上的位置
        // （实测：碎片的某个面把 `/tmp/…/app/src/…` 揉进了哈希，与计划里的 id 不一致）。
        // Only an **absolute** path can be a mount from another package; a relative one is the
        // spelling rustc was given inside this workspace, and it is returned as it always was.
        // 只有**绝对**路径才可能是"另一个包挂进来的"；相对路径是 rustc 在这个工作区里被给到的拼法，
        // 因此像过去一样原样返回。
        None if is_absolute_path(file) => return after_owning_src(file),
        None => return file,
    };
    let after_separator = strip_leading_separator(after_manifest);
    match strip_prefix(after_separator, "src/") {
        Some(rest) => rest,
        None => match strip_prefix(after_separator, "src\\") {
            Some(rest) => rest,
            None => after_separator,
        },
    }
}
