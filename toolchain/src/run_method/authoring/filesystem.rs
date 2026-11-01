//! Small filesystem primitives used by authoring transactions.
//! 创作事务使用的文件系统小工具。

// The file primitives this module owns. `create_new` moved into `WriteTransaction` (which captures
// before it creates), and the atomic replace is the transaction's `atomic_replace`: one
// implementation of each, shared by every writer in this crate.
// 本模块拥有的文件原语。`create_new` 移进了 `WriteTransaction`（它在创建之前保存），原子替换就是事务的
// `atomic_replace`：各自只有一份实现，由本 crate 每个写入方共用。
use std::path::Path;

pub(crate) fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    super::write_transaction::atomic_replace(path, contents.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// A throwaway directory, unique per call.
    /// 每次调用唯一的一次性目录。
    fn fixture(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-atomic-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture directory");
        root
    }

    /// A sibling that happens to look like a temporary is not this writer's to
    /// delete, and a successful write leaves no temporary of its own behind.
    /// 一个恰好长得像临时文件的同级文件不归本写者删除；而一次成功的写入不会留下自己的临时文件。
    #[test]
    fn writing_replaces_the_target_and_touches_nothing_else() {
        let root = fixture("sibling");
        let target = root.join("face.rs");
        fs::write(&target, "old").expect("target");
        let neighbour = root.join("face.xirang.tmp");
        fs::write(&neighbour, "someone else's bytes").expect("neighbour");

        atomic_write(&target, "new").expect("atomic write");

        assert_eq!(fs::read_to_string(&target).expect("target"), "new");
        assert_eq!(
            fs::read_to_string(&neighbour).expect("the neighbour survives"),
            "someone else's bytes"
        );
        let leftovers = fs::read_dir(&root)
            .expect("read the directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("xirang-"))
            .count();
        assert_eq!(leftovers, 0, "no temporary of our own is left behind");

        let _ = fs::remove_dir_all(&root);
    }
}
