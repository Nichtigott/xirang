//! Landlock write-domain confinement, shared by every execution surface that starts a process.
//! 由每个会启动进程的执行面共用的 Landlock 写域限制。
//!
//! The restriction is one-way and applies to the calling process and every process it starts; a
//! surface that wants to confine only its children starts a child first and confines that child.
//! 限制是单向的，对调用进程及其启动的每个进程生效；只想约束子进程的执行面应先启动子进程，再约束那个子进程。
//!
//! 本模块只处理写：读保持不受限（构建需要工具链、缓存与源码）；写域之外的写入由内核返回 `EPERM`。

use std::path::{Path, PathBuf};

/// Linux syscall numbers for the three Landlock calls (x86_64 and aarch64 share these; the number is part of the
/// kernel's ABI, not of a libc).
/// 三个 Landlock 调用的 Linux 系统调用号（x86_64 与 aarch64 共用；号码属于内核 ABI，不属于某个 libc）。
const SYS_CREATE_RULESET: std::ffi::c_long = 444;
const SYS_ADD_RULE: std::ffi::c_long = 445;
const SYS_RESTRICT_SELF: std::ffi::c_long = 446;

/// `LANDLOCK_CREATE_RULESET_VERSION`: the query that asks the kernel which ABI it speaks.
/// `LANDLOCK_CREATE_RULESET_VERSION`：问内核它说哪一版 ABI 的那次查询。
const CREATE_RULESET_VERSION: u32 = 1;
/// `LANDLOCK_RULE_PATH_BENEATH`: the only rule type used here.
/// `LANDLOCK_RULE_PATH_BENEATH`：这里唯一用到的规则类型。
const RULE_PATH_BENEATH: u32 = 1;
/// `PR_SET_NO_NEW_PRIVS`: required before `landlock_restrict_self`, and the reason the restriction survives `exec`.
/// `PR_SET_NO_NEW_PRIVS`：`landlock_restrict_self` 之前必须设置，也是限制能熬过 `exec` 的原因。
const PR_SET_NO_NEW_PRIVS: std::ffi::c_int = 38;

// The filesystem rights this module handles. Only writes are handled: reads stay unrestricted, because a build needs
// the toolchain, the registry cache and the sources, and a read restriction would refuse the work rather than the
// escape.
// 本模块处理的文件系统权限。只处理写：读保持不受限，因为一次构建需要工具链、注册表缓存与源码，而读限制会拒绝工作本身，
// 而不是拒绝越界。
const WRITE_FILE: u64 = 1 << 1;
const REMOVE_DIR: u64 = 1 << 4;
const REMOVE_FILE: u64 = 1 << 5;
const MAKE_CHAR: u64 = 1 << 6;
const MAKE_DIR: u64 = 1 << 7;
const MAKE_REG: u64 = 1 << 8;
const MAKE_SOCK: u64 = 1 << 9;
const MAKE_FIFO: u64 = 1 << 10;
const MAKE_BLOCK: u64 = 1 << 11;
const MAKE_SYM: u64 = 1 << 12;
const REFER: u64 = 1 << 13; // ABI 2
const TRUNCATE: u64 = 1 << 14; // ABI 3

#[repr(C)]
struct RulesetAttr {
    handled_access_fs: u64,
}

#[repr(C)]
struct PathBeneathAttr {
    allowed_access: u64,
    parent_fd: i32,
    reserved: i32,
}

unsafe extern "C" {
    fn syscall(number: std::ffi::c_long, ...) -> std::ffi::c_long;
    fn prctl(option: std::ffi::c_int, ...) -> std::ffi::c_int;
    /// The C `open`; the Rust binding is named `open_handle` because a bare verb is only allowed at an
    /// entry position (`conventions::verb_table`), and the symbol a linker looks for is still `open`.
    /// C 的 `open`；Rust 绑定叫 `open_handle`，因为裸动词只许出现在入口位（`conventions::verb_table`），
    /// 而链接器要找的符号仍是 `open`。
    #[link_name = "open"]
    fn open_handle(path: *const std::ffi::c_char, flags: std::ffi::c_int, ...) -> std::ffi::c_int;
    /// The C `close`; same reason for the Rust-side name.
    /// C 的 `close`；Rust 侧改名同理。
    #[link_name = "close"]
    fn close_handle(fd: std::ffi::c_int) -> std::ffi::c_int;
}

/// `O_PATH | O_CLOEXEC`: open for a handle rather than a stream, which is what a Landlock rule needs.
/// `O_PATH | O_CLOEXEC`：为句柄而不是流打开，这正是 Landlock 规则需要的。
const O_PATH: std::ffi::c_int = 0o10000000;
const O_CLOEXEC: std::ffi::c_int = 0o2000000;

/// The Landlock ABI this kernel speaks, or `None` when it cannot confine anything.
/// 本内核所说的 Landlock ABI；内核无法限制任何东西时为 `None`。
fn abi() -> Option<i32> {
    // SAFETY: the version query passes no pointer and no size; the kernel returns a non-negative ABI or -1.
    // 安全性：版本查询不传指针也不传大小；内核返回非负 ABI 或 -1。
    let value = unsafe {
        syscall(
            SYS_CREATE_RULESET,
            std::ptr::null::<RulesetAttr>(),
            0usize,
            CREATE_RULESET_VERSION,
        )
    };
    if value < 1 {
        None
    } else {
        i32::try_from(value).ok()
    }
}

/// The rights this module handles at the given ABI, and separately the rights a **directory** rule may grant.
/// 在给定 ABI 下本模块处理的权限，以及单独一份**目录**规则可以授予的权限。
///
/// `REFER` and `TRUNCATE` arrived in later ABIs; asking an older kernel to handle a right it does not know is
/// `EINVAL`, so the mask follows the kernel rather than the header this file was written against.
/// `REFER` 与 `TRUNCATE` 在更晚的 ABI 才出现；要求更老的内核处理它不认识的权限会得到 `EINVAL`，因此掩码跟着内核走，
/// 而不是跟着写这个文件时手边的头文件。
fn handled(abi: i32) -> u64 {
    let mut mask = WRITE_FILE
        | REMOVE_DIR
        | REMOVE_FILE
        | MAKE_CHAR
        | MAKE_DIR
        | MAKE_REG
        | MAKE_SOCK
        | MAKE_FIFO
        | MAKE_BLOCK
        | MAKE_SYM;
    if abi >= 2 {
        mask |= REFER;
    }
    if abi >= 3 {
        mask |= TRUNCATE;
    }
    mask
}

/// The rights a rule for one path may name: a directory takes everything, a file only what applies to files.
/// 一条针对某路径的规则可以点名的权限：目录取全部，文件只取适用于文件的那几条。
///
/// Naming `MAKE_DIR` on a regular file is `EINVAL` from the kernel, so the shape of the rule follows the shape of
/// the path — a detail that would otherwise be discovered as "managed mode does not start on any host".
/// 在普通文件上点名 `MAKE_DIR` 会被内核判 `EINVAL`，因此规则的形状跟着路径的形状走——否则这个细节会以"托管模式在任何宿主上都起不来"
/// 的形式被发现。
fn rights_for(abi: i32, path: &Path) -> u64 {
    let mask = handled(abi);
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => mask,
        Ok(_) => {
            let mut file = WRITE_FILE;
            if abi >= 3 {
                file |= TRUNCATE;
            }
            mask & file
        }
        Err(_) => mask,
    }
}

/// Open a path the way a Landlock rule needs it.
/// 以 Landlock 规则需要的方式打开一条路径。
fn open_path(path: &Path) -> Result<std::ffi::c_int, String> {
    let bytes = path.as_os_str().as_encoded_bytes();
    let mut terminated = bytes.to_vec();
    terminated.push(0);
    // SAFETY: `terminated` is NUL-terminated and outlives the call; `open` does not retain the pointer.
    // 安全性：`terminated` 以 NUL 结尾且活过这次调用；`open` 不保留这个指针。
    let fd = unsafe {
        open_handle(
            terminated.as_ptr().cast::<std::ffi::c_char>(),
            O_PATH | O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(format!(
            "cannot open {} for a write-domain rule: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(fd)
}

/// Restrict **this process** to writing under `writable`, and return the ABI that did it.
/// 把**本进程**限制在 `writable` 之下写入，并返回做到这件事的 ABI。
///
/// This is the whole guarantee, and it is deliberately not a policy object: there is no "managed" flag a caller could
/// set to make it true, and no way to turn it off again — Landlock is one-way, which is why the process that publishes
/// has to be a different one.
/// 这就是整个保证，而且它有意不是一个政策对象：没有哪个开关能由调用方设成真，也没有办法再关掉——Landlock 是单向的，这正是
/// 发布必须由另一个进程来做的原因。
pub(crate) fn restrict(writable: &[PathBuf]) -> Result<i32, String> {
    let abi = abi().ok_or_else(|| {
        "this kernel does not provide Landlock (landlock_create_ruleset returned -1): managed mode is not available \
         here, and this binary will not pretend that a description of a boundary is one"
            .to_owned()
    })?;
    let mask = handled(abi);
    let attr = RulesetAttr {
        handled_access_fs: mask,
    };
    // SAFETY: `attr` is a valid `struct landlock_ruleset_attr` of the declared size and outlives the call.
    // 安全性：`attr` 是一份声明了大小的合法 `struct landlock_ruleset_attr`，且活过这次调用。
    let ruleset = unsafe {
        syscall(
            SYS_CREATE_RULESET,
            (&raw const attr).cast::<std::ffi::c_void>(),
            std::mem::size_of::<RulesetAttr>(),
            0u32,
        )
    };
    if ruleset < 0 {
        return Err(format!(
            "cannot create a write-domain ruleset: {}",
            std::io::Error::last_os_error()
        ));
    }
    for path in writable {
        let fd = open_path(path)?;
        let rule = PathBeneathAttr {
            allowed_access: rights_for(abi, path),
            parent_fd: fd,
            reserved: 0,
        };
        // SAFETY: `rule` points at a live parent fd and a valid attribute struct for the duration of the call.
        // 安全性：`rule` 在调用期间指向一个活着的父 fd 与一份合法属性结构。
        let added = unsafe {
            syscall(
                SYS_ADD_RULE,
                ruleset,
                RULE_PATH_BENEATH,
                (&raw const rule).cast::<std::ffi::c_void>(),
                0u32,
            )
        };
        // SAFETY: `close` on a descriptor this function opened and no longer uses.
        // 安全性：`close` 的是本函数打开、此后不再使用的描述符。
        unsafe { close_handle(fd) };
        if added < 0 {
            return Err(format!(
                "cannot add a write-domain rule for {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            ));
        }
    }
    // SAFETY: the call takes two integers; `NO_NEW_PRIVS` must precede the restriction.
    // 安全性：该调用取两个整数；`NO_NEW_PRIVS` 必须在限制之前设置。
    let privileged = unsafe { prctl(PR_SET_NO_NEW_PRIVS, 1 as std::ffi::c_ulong, 0, 0, 0) };
    if privileged != 0 {
        return Err(format!(
            "cannot set no_new_privs, so a setuid program could escape the restriction: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: `ruleset` is a live ruleset descriptor created above; the second argument is unused by the kernel.
    // 安全性：`ruleset` 是上面创建的活规则集描述符；第二个参数内核不使用。
    let restricted = unsafe { syscall(SYS_RESTRICT_SELF, ruleset, 0u32) };
    // SAFETY: the ruleset descriptor is not needed after the restriction is installed.
    // 安全性：限制装上之后不再需要规则集描述符。
    unsafe { close_handle(ruleset as std::ffi::c_int) };
    if restricted != 0 {
        return Err(format!(
            "cannot apply the write-domain restriction: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(abi)
}
