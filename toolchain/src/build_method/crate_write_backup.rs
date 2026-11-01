//! The backup a partition write restores from when any step fails.
//! 分区写入失败时用于恢复的备份。

use std::fs;
use std::path::{Path, PathBuf};

/// Backup of the files and generated directories one write pass may replace.
/// 一次写入可能替换的文件与生成目录的备份。
pub(super) struct Backup {
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
    directories: Vec<(PathBuf, Option<PathBuf>)>,
    temp: PathBuf,
}

impl Backup {
    pub(super) fn take(
        root: &Path,
        directories: &[(&Path, &str)],
        config: Option<&(PathBuf, String)>,
        members: Option<&(PathBuf, String)>,
        host: Option<&(PathBuf, String)>,
    ) -> Result<Self, String> {
        let temp = root.join(".xirang/partition-backup");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp)
            .map_err(|error| format!("cannot create {}: {error}", temp.display()))?;
        let mut files = Vec::new();
        if let Some((path, _)) = config {
            files.push((path.clone(), fs::read(path).ok()));
        }
        if let Some((path, _)) = members {
            files.push((path.clone(), fs::read(path).ok()));
        }
        if let Some((path, _)) = host {
            files.push((path.clone(), fs::read(path).ok()));
        }
        let mut backed_up = Vec::new();
        for (directory, _) in directories {
            let (_, existing) = &fields_of(directory);
            if !existing.exists() {
                backed_up.push(((*directory).to_path_buf(), None));
                continue;
            }
            let name = directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("dir");
            let destination = temp.join(name);
            copy_tree(existing, &destination)?;
            backed_up.push(((*directory).to_path_buf(), Some(destination)));
        }
        Ok(Self {
            files,
            directories: backed_up,
            temp,
        })
    }

    pub(super) fn restore(&self) -> String {
        let mut problems = Vec::new();
        for (path, bytes) in &self.files {
            let outcome = match bytes {
                Some(bytes) => fs::write(path, bytes),
                None => {
                    let outcome = fs::remove_file(path);
                    if outcome.is_ok()
                        && let Some(parent) = path.parent()
                    {
                        let _ = fs::remove_dir(parent);
                    }
                    outcome
                }
            };
            if let Err(error) = outcome {
                problems.push(format!("{}: {error}", path.display()));
            }
        }
        for (path, backup) in &self.directories {
            let _ = fs::remove_dir_all(path);
            let outcome = match backup {
                Some(backup) => copy_tree(backup, path),
                None => Ok(()),
            };
            if let Err(error) = outcome {
                problems.push(format!("{}: {error}", path.display()));
            }
        }
        let _ = fs::remove_dir_all(&self.temp);
        if problems.is_empty() {
            "the partition restored every file and directory it had replaced".to_owned()
        } else {
            format!(
                "the partition could not restore: {}; the backup is at {}",
                problems.join("; "),
                self.temp.display()
            )
        }
    }
}

/// The write set of one generated directory, for backup purposes.
/// 一个生成目录的写入集合，供备份使用。
fn fields_of(directory: &Path) -> (&Path, PathBuf) {
    (directory, directory.to_path_buf())
}

/// Copy a directory tree, for backup/restore.
/// 复制目录树，供备份/恢复。
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|error| format!("cannot create {}: {error}", to.display()))?;
    for entry in
        fs::read_dir(from).map_err(|error| format!("cannot read {}: {error}", from.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot read {}: {error}", from.display()))?;
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            fs::copy(&path, &target)
                .map_err(|error| format!("cannot copy {}: {error}", path.display()))?;
        }
    }
    Ok(())
}
