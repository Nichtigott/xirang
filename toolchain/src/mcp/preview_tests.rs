//! Tests for the throwaway copy and the diff a preview reports.
//! 一次性副本与预览报告的 diff 的测试。

use std::path::PathBuf;

/// A directory tree that is not a package on purpose: the copy and the diff walk
/// names, not manifests, so a fixture with the directories in question is the
/// honest one.
/// 一个有意不像包的目录树：副本与 diff 遍历的是名字而不是清单，因此放上相关目录的 fixture 才是
/// 诚实的。
fn tree(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-toolchain-preview-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(root.join("src/lib.rs"), "fn a() {}\n").expect("source file");
    std::fs::create_dir_all(root.join("target")).expect("build output directory");
    std::fs::write(root.join("target/artifact"), "built\n").expect("build artifact");
    std::fs::create_dir_all(root.join(".git")).expect("version-control directory");
    std::fs::write(root.join(".git/config"), "[core]\n").expect("version-control file");
    std::fs::create_dir_all(root.join(xirang_kernel::lexicon::XIRANG_DIR))
        .expect("runtime directory");
    std::fs::write(
        root.join(xirang_kernel::lexicon::XIRANG_DIR).join("state"),
        "runtime\n",
    )
    .expect("runtime file");
    root
}

/// The copy and the diff skip the same three directories: the build output
/// `target/`, the version-control store `.git/`, and XiRang's runtime data
/// `.xirang/`. Both halves were once copied and walked — every preview carried
/// `.git/` (9.9 MB in this checkout, routinely gigabytes in a real repository) and
/// reported files the edit never touched (audit `BR-C2`, `BR-4`).
/// 副本与 diff 跳过同样那三个目录：构建产物 `target/`、版本库 `.git/` 与 XiRang 的运行期数据
/// `.xirang/`。两处过去都既复制又遍历——每一次预览都带上 `.git/`（本检出里 9.9 MB，真实仓库里
/// 常是 GB 级）并报出编辑根本没碰过的文件（审计 `BR-C2`、`BR-4`）。
///
/// This is the in-repo form of the probe t5 used out of tree (`/tmp/t5probe.py`
/// drove `xirang.apply` over stdio against a fixture with a binary `.git/index`):
/// same set, same verdict, expressed as the unit the behaviour is decided in.
/// 这是 t5 在仓外所用探针（`/tmp/t5probe.py` 经 stdio 驱动 `xirang.apply`，夹具里有二进制
/// `.git/index`）的仓内形态：同一集合、同一结论，只是用决定该行为的那个单位表达。
#[test]
fn a_preview_skips_the_build_output_and_both_stores() {
    let root = tree("skip");
    let copy = super::copy_package(&root).expect("the copy succeeds");
    for skipped in ["target", ".git", xirang_kernel::lexicon::XIRANG_DIR] {
        assert!(
            !copy.join(skipped).exists(),
            "the preview copy must skip `{skipped}`: {}",
            copy.display()
        );
    }
    assert!(
        copy.join("src/lib.rs").is_file(),
        "the copy must still carry the edited sources"
    );
    // The other half of the same rule: the diff walk skips them too, so nothing
    // under them can be reported as added, changed, or removed.
    // 同一条规则的另一半：diff 遍历也跳过它们，因此它们下面的任何东西都不会被报成新增、改动或删除。
    let diff = super::diff_package(&root, &copy).expect("the diff renders");
    for skipped in ["target", ".git", xirang_kernel::lexicon::XIRANG_DIR] {
        assert!(
            !diff.contains(skipped),
            "the diff walk must skip `{skipped}` too: {diff}"
        );
    }
    assert!(
        diff.is_empty(),
        "nothing changed between the tree and its copy: {diff}"
    );
    super::remove_copy(&root, &copy);
    let _ = std::fs::remove_dir_all(&root);
}

/// Every file under `root`, as tree-relative POSIX paths.
/// `root` 下的每个文件，写成树内相对 POSIX 路径。
fn all_files(root: &std::path::Path) -> Vec<String> {
    fn walk_preview_tests(
        root: &std::path::Path,
        directory: &std::path::Path,
        files: &mut Vec<String>,
    ) {
        for entry in std::fs::read_dir(directory).expect("a readable directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk_preview_tests(root, &path, files);
            } else if let Ok(relative) = path.strip_prefix(root) {
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut files = Vec::new();
    walk_preview_tests(root, root, &mut files);
    files.sort();
    files
}

/// A preview copy never follows a directory symlink: an outside link used to drag that tree into
/// the scratch directory, and a self-link (`ln -s . src/self`) recursed until the path limit and
/// reported the same faces dozens of times — both measured, both silent (audit `LGC-LG-19`).
/// 预览副本绝不跟进目录符号链接：指向外部的链接过去会把那棵树拖进临时目录，而自指链接
/// （`ln -s . src/self`）会递归到路径上限、把同一批面报上几十次——两条都实测过、都无声
/// （审计 `LGC-LG-19`）。
#[cfg(unix)]
#[test]
fn a_directory_symlink_is_not_followed_out_of_the_package() {
    let root = tree("symlink");
    let outside = std::env::temp_dir().join(format!(
        "xirang-toolchain-outside-{}-{}",
        std::process::id(),
        root.file_name().unwrap_or_default().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).expect("outside directory");
    std::fs::write(outside.join("secret.txt"), "secret\n").expect("outside file");
    std::os::unix::fs::symlink(&outside, root.join("src/outside_link")).expect("outside link");
    std::os::unix::fs::symlink(".", root.join("src/self")).expect("self link");
    let copy = super::copy_package(&root).expect("the copy succeeds");
    let files = all_files(&copy);
    assert!(
        !files.iter().any(|file| file.ends_with("secret.txt")),
        "an outside tree must not be copied: {files:?}"
    );
    assert!(
        !copy.join("src/self").exists(),
        "a self-link must not be followed: {files:?}"
    );
    super::remove_copy(&root, &copy);
    let _ = std::fs::remove_dir_all(&outside);
    let _ = std::fs::remove_dir_all(&root);
}

/// A copy that fails leaves nothing behind. The half-made copy used to stay under the temp
/// directory on every failed preview (measured: seven files of a package), while the docs call it
/// throwaway — one that disappears on the success path only is not throwaway (audit `LGC-LG-20`).
/// 失败的复制不留任何东西。半成品副本过去会在每次失败的预览后留在临时目录里（实测：一个包的七个
/// 文件），而文档把它称作一次性——只在成功路径上消失的一次性不是一次性（审计 `LGC-LG-20`）。
#[cfg(unix)]
#[test]
fn a_failed_copy_removes_the_destination_it_created() {
    let root = tree("failed-copy");
    std::fs::write(root.join("src/unreadable.rs"), "fn c() {}\n").expect("source file");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        root.join("src/unreadable.rs"),
        std::fs::Permissions::from_mode(0o000),
    )
    .expect("unreadable source");
    let destination = std::env::temp_dir().join(format!(
        "xirang-toolchain-preview-pin-{}-{}",
        std::process::id(),
        root.file_name().unwrap_or_default().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&destination);
    std::fs::create_dir(&destination).expect("the caller creates the destination");
    let error = super::copy_into(&root, destination.clone(), false)
        .expect_err("an unreadable source fails the copy");
    assert!(error.contains("unreadable.rs"), "{error}");
    assert!(
        !destination.exists(),
        "a failed copy must remove the directory it created: {}",
        destination.display()
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Two byte-identical non-UTF-8 files are not "added": the comparison is on bytes, so an edit that
/// never touched a file cannot list it — and the preview used to list every non-UTF-8 file in the
/// package (36 in this checkout) as `+ …` (audit `LGC-LG-23`).
/// 两个逐字节相同的非 UTF-8 文件不是"新增"：比较发生在字节上，因此一次没碰过某文件的编辑不能把它
/// 列出来——而预览过去会把包里每个非 UTF-8 文件（本检出 36 个）都列成 `+ …`（审计 `LGC-LG-23`）。
#[test]
fn an_identical_non_utf8_file_is_not_reported_as_added() {
    let root = tree("binary");
    std::fs::write(root.join("src/bin.dat"), [0xff, 0xfe, 0x00, b'b']).expect("binary file");
    let copy = super::copy_package(&root).expect("the copy succeeds");
    let diff = super::diff_package(&root, &copy).expect("the diff renders");
    assert!(
        !diff.contains("bin.dat"),
        "an untouched binary file must not be listed: {diff}"
    );
    assert!(diff.is_empty(), "{diff}");
    super::remove_copy(&root, &copy);
    let _ = std::fs::remove_dir_all(&root);
}

/// A preview reports at most `MAX_DIFF_LINES` diff lines and says it truncated. Every other answer
/// this bridge gives is bounded and says so; the preview was the one that could not, which is how a
/// real checkout produced a reply an agent could not read (audit `LGC-LG-23`).
/// 一条预览最多打印 `MAX_DIFF_LINES` 行 diff，并说明自己被截断。本桥其它每个答案都有上限并说明
/// 截断；预览曾是唯一没有上限的那个，于是一次真实检出能产出代理读不下的回复（审计 `LGC-LG-23`）。
#[test]
fn a_huge_diff_is_bounded_and_says_so() {
    let root = tree("bounded");
    let copy = super::copy_package(&root).expect("the copy succeeds");
    // 400 changed lines: past the documented cap, and small enough that a reply without one is
    // still visibly wrong rather than merely enormous.
    // 400 行改动：超过文档化的上限，又小到"没有上限"的回复一眼就能看出不对，而不是仅仅巨大。
    let changed = 400usize;
    let big = (0..changed)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(copy.join("src/lib.rs"), format!("{big}\n")).expect("a big change");
    let diff = super::diff_package(&root, &copy).expect("the diff renders");
    assert!(
        diff.contains("truncated:"),
        "the reply must say it withheld: {diff}"
    );
    assert!(
        diff.lines().count() < changed,
        "the reply must be bounded below the change itself, not merely below a constant: {} lines",
        diff.lines().count()
    );
    super::remove_copy(&root, &copy);
    let _ = std::fs::remove_dir_all(&root);
}

/// A directory that already exists at a candidate name is neither deleted nor reused: the preview
/// creates its own destination exclusively and moves to the next candidate. The old form deleted
/// the same-named directory first, which destroyed whatever was there and made the copy's path
/// something another process on the same machine could predict — and pre-place (audit `LGC-LG-20`).
/// 已存在于某个候选名上的目录既不被删除也不被复用：预览独占创建自己的目标，然后换下一个候选。早先的
/// 形式先删掉同名目录，这会毁掉那里原本的东西，也让副本路径成为同一台机器上另一个进程可以预测——因而
/// 可以预先放置——的东西（审计 `LGC-LG-20`）。
#[test]
fn a_pre_placed_directory_is_neither_deleted_nor_reused() {
    let root = tree("pre-placed");
    let temp = std::env::temp_dir().join(format!(
        "xirang-toolchain-pre-placed-{}-{}",
        std::process::id(),
        root.file_name().unwrap_or_default().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("the scratch directory");
    let pre_placed = temp.join("candidate");
    std::fs::create_dir(&pre_placed).expect("the pre-placed directory");
    std::fs::write(pre_placed.join("marker.txt"), "kept\n").expect("its content");
    let work = super::copy_package_with(&root, &temp, false, |attempt| {
        if attempt == 0 {
            "candidate".to_owned()
        } else {
            format!("candidate-{attempt}")
        }
    })
    .expect("the copy lands on a later candidate");
    assert_ne!(
        work, pre_placed,
        "a directory that already exists must not be reused as the copy"
    );
    assert!(
        pre_placed.join("marker.txt").is_file(),
        "a directory that already exists must not be deleted"
    );
    assert!(
        work.join("src/lib.rs").is_file(),
        "the copy itself is complete"
    );
    // The production names are not predictable either: eight calls produce eight distinct paths
    // whose names carry an OS-seeded component, so another process cannot pre-place one.
    // 生产路径的名字同样不可预测：八次调用产出八个互不相同的路径，名字里带着一个由操作系统播种的
    // 分量，因此另一个进程无法预先放置一个。
    let mut names = Vec::new();
    for _ in 0..8 {
        let path = super::copy_package(&root).expect("the copy succeeds");
        names.push(
            path.file_name()
                .expect("a name")
                .to_string_lossy()
                .into_owned(),
        );
        super::remove_copy(&root, &path);
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 8, "every copy gets its own name: {names:?}");
    super::remove_copy(&root, &work);
    let _ = std::fs::remove_dir_all(&temp);
    let _ = std::fs::remove_dir_all(&root);
}

/// One pathologically long line is still one line, so a line cap alone would hand the client an
/// unbounded reply: a byte cap bounds it whatever the file looks like, and the reply says it
/// truncated instead of silently cutting. The measured finding was "no cap at all", and this is
/// the half a line count alone cannot cover (audit `LGC-LG-23`).
/// 一行病态长仍然只是一行，因此只按行设上限会把一份无界回复交给客户端：字节上限无论文件长什么样都能
/// 给它设界，而回复会说明自己截断了，而不是悄悄截断。实测的缺陷是"完全没有上限"，而这一半是只数行
/// 数覆盖不到的（审计 `LGC-LG-23`）。
#[test]
fn a_single_enormous_line_cannot_blow_up_the_reply() {
    let root = tree("one-line");
    let copy = super::copy_package(&root).expect("the copy succeeds");
    let huge = "x".repeat(200_000);
    std::fs::write(copy.join("src/lib.rs"), format!("{huge}\n")).expect("one huge line");
    let diff = super::diff_package(&root, &copy).expect("the diff renders");
    assert!(
        diff.contains("truncated:"),
        "the reply must say it withheld: {} bytes",
        diff.len()
    );
    assert!(
        diff.len() < 100_000,
        "the reply must stay inside the byte cap, not merely the line cap: {} bytes",
        diff.len()
    );
    super::remove_copy(&root, &copy);
    let _ = std::fs::remove_dir_all(&root);
}
