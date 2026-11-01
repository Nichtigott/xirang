//! `xirang package`: turn one finished build plan into an immutable delivery directory with a
//! manifest that names every artifact and its bytes. The package is what `xirang install` will
//! verify and place; producing it must not rebuild, so a missing artifact is a refusal, not an
//! invitation to run the build here.
//! `xirang package`：把一次已完成的构建计划变成一个不可变的交付目录，清单逐项记下每个产物及其字节。
//! 包是 `xirang install` 要校验并安放的东西；产出它不重新构建，因此缺产物就按名拒绝，而不是在这里顺手构建。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{exec, project};

/// The manifest's protocol word; a reader that does not know it must refuse the package.
/// 清单的协议词；不认得它的读取方必须拒绝这个包。
const PROTOCOL: u32 = 1;

/// Write the package for `root` (after a build) into `output`, or into the plan's own directory.
/// `--output` names the exact directory; without it the package lands under `target/xirang/package/<plan>`.
/// 把 `root` 的包（构建完成之后）写进 `output`，或写进计划自己的目录。`--output` 点名确切的目录；不给时
/// Verify a packaged directory against the manifest it carries: every file the manifest names must
/// be present with its recorded bytes and sha256, and the directory must hold no file the manifest
/// does not name. A mismatch is refused with the file named, because a package that is not exactly
/// what its manifest describes cannot be trusted as provenance.
/// 按包自带的清单核验一个交付目录：清单点名的每个文件必须在且字节与 sha256 相符；目录里不得有不被
/// 清单点名的文件。不符按名拒绝并点名——与清单不一致的包不能当作来源证明。
pub(crate) fn verify(directory: &Path) -> Result<String, String> {
    let path = directory.join("manifest.json");
    let text = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "{} carries no package manifest: {error}",
            directory.display()
        )
    })?;
    let manifest: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not a package manifest: {error}", path.display()))?;
    let files = manifest["files"]
        .as_array()
        .ok_or_else(|| format!("{} has no `files` list", path.display()))?;
    let mut named = std::collections::BTreeSet::new();
    let mut bytes = 0u64;
    for file in files {
        let relative = file["path"]
            .as_str()
            .ok_or_else(|| format!("{} has a file entry with no `path`", path.display()))?;
        let recorded = file["sha256"]
            .as_str()
            .ok_or_else(|| format!("`{relative}` has no `sha256` in the package manifest"))?;
        let path = directory.join(relative);
        if std::fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(format!(
                "`{relative}` is a symbolic link; a package is plain files, and following a link \
                 would let the package stand for bytes it does not carry"
            ));
        }
        let found_bytes = std::fs::read(&path).map_err(|error| {
            format!("`{relative}` is named by the manifest and cannot be read: {error}")
        })?;
        let found = xirang_kernel::sha256_hex(&found_bytes);
        if found != recorded {
            return Err(format!(
                "`{relative}` does not match the manifest: recorded {recorded}, found {found}; this directory is not the package the manifest describes"
            ));
        }
        bytes += found_bytes.len() as u64;
        named.insert(relative.to_owned());
    }
    let mut present = Vec::new();
    visit_files(directory, directory, &mut present)?;
    for relative in present {
        // The manifest and its signature files are the package's own envelope: `xirang verify`
        // checks the signatures separately when asked, so none of them is an unnamed payload file.
        // 清单与它的签名文件是包自己的信封：`xirang verify` 被要求时另验签名，它们都不是未点名的载荷文件。
        let envelope = relative == "manifest.json"
            || relative == "manifest.json.sig"
            || (relative.starts_with("manifest.json.") && relative.ends_with(".sig"));
        if envelope {
            continue;
        }
        if !named.contains(&relative) {
            return Err(format!(
                "`{relative}` is in this directory and no manifest entry names it; the package is not exactly what the manifest describes"
            ));
        }
    }
    let units = manifest["units"].as_array().map_or(0, |units| units.len());
    let runtime = manifest["runtime"]["python_modules"]
        .as_array()
        .map_or(0, |modules| modules.len());
    Ok(format!(
        "verified {} file(s) ({} bytes) of package `{}`: {} unit(s), {} runtime module(s)\n",
        files.len(),
        bytes,
        manifest["plan"].as_str().unwrap_or("?"),
        units,
        runtime
    ))
}

/// Collect every regular file under `directory`, as a path relative to `root`.
/// 收集 `directory` 下每个普通文件（相对 `root` 的路径）。
fn visit_files(root: &Path, directory: &Path, files: &mut Vec<String>) -> Result<(), String> {
    for entry in std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
            .path();
        let relative = path
            .strip_prefix(root)
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.display().to_string());
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "`{relative}` is a symbolic link; a package is plain files, and following a link \
                 would let the package stand for bytes it does not carry"
            ));
        }
        if metadata.is_dir() {
            visit_files(root, &path, files)?;
        } else {
            files.push(relative);
        }
    }
    Ok(())
}

/// 包落在 `target/xirang/package/<plan>` 下。
pub(crate) fn create(root: &Path, output: Option<&Path>) -> Result<String, String> {
    let compiler = exec::detect()?;
    let project = project::Project::load(root)?;
    let plan = project.plan(&compiler, false)?;
    let mut artifacts: BTreeSet<PathBuf> = BTreeSet::new();
    for action in &plan.actions {
        for output in &action.outputs {
            artifacts.insert(output.clone());
        }
    }
    for artifact in &artifacts {
        if !artifact.is_file() {
            return Err(format!(
                "`{}` is not there; `xirang package` describes a finished build, so run `xirang build` first",
                artifact.display()
            ));
        }
    }

    // A delivery needs more than the compiler's outputs: a unit with `[[export]]` also publishes a
    // generated interpreter module (`exports.py`) beside its shared library. Those files are not
    // action outputs, so a package that only copied outputs would look complete and fail on import.
    // The package therefore carries every generated `.py` and names the shared libraries it holds;
    // interpreters and system libraries are outside it and stay visible as such.
    // 交付需要的不只是编译器产物：带 `[[export]]` 的单元还会在动态库旁发布一个生成的解释器模块
    // （`exports.py`）。它们不是动作产物，只复制产物的包会看起来完整，却在 import 时失败。因此包会带上
    // 每一份生成的 `.py`，并点名自己持有的动态库；解释器与系统库在包外，仍如实可见。
    let runtime: Vec<PathBuf> = plan
        .generated
        .iter()
        .map(|(path, _)| path.clone())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("py"))
        .collect();
    for file in &runtime {
        if !file.is_file() {
            return Err(format!(
                "`{}` is not there; `xirang package` describes a finished build, so run `xirang build` first",
                file.display()
            ));
        }
    }
    let directory = match output {
        Some(directory) => directory.to_path_buf(),
        None => plan
            .directory
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new("target/xirang"))
            .join("package")
            .join(&plan.id),
    };
    if directory.join("manifest.json").exists() {
        return Err(format!(
            "`{}` already holds a package; remove it or pass `--output`",
            directory.display()
        ));
    }
    let mut files = Vec::new();
    let mut copied = 0u64;
    for artifact in &artifacts {
        let relative = artifact
            .strip_prefix(&plan.directory)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| PathBuf::from(artifact.file_name().unwrap_or_default()));
        let bytes = std::fs::read(artifact)
            .map_err(|error| format!("cannot read {}: {error}", artifact.display()))?;
        let digest = xirang_kernel::sha256_hex(&bytes);
        copied += bytes.len() as u64;
        let target = directory.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        std::fs::write(&target, &bytes)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        files.push(serde_json::json!({
            "path": relative.to_string_lossy(),
            "sha256": digest,
            "bytes": bytes.len(),
            "role": "artifact",
        }));
    }
    for file in &runtime {
        let relative = file
            .strip_prefix(&plan.directory)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| PathBuf::from(file.file_name().unwrap_or_default()));
        let bytes = std::fs::read(file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        let digest = xirang_kernel::sha256_hex(&bytes);
        copied += bytes.len() as u64;
        let target = directory.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        std::fs::write(&target, &bytes)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        files.push(serde_json::json!({
            "path": relative.to_string_lossy(),
            "sha256": digest,
            "bytes": bytes.len(),
            "role": "runtime",
        }));
    }
    let units: Vec<_> = project
        .units
        .iter()
        .map(|unit| {
            serde_json::json!({
                "name": unit.manifest.package.name,
                "version": unit.manifest.package.version,
                "language": super::read::language_name(&unit.manifest.language),
                "kind": super::read::kind_name(unit.manifest.package.kind),
            })
        })
        .collect();
    let python_modules: Vec<String> = files
        .iter()
        .filter(|file| file["role"] == "runtime")
        .filter_map(|file| file["path"].as_str().map(str::to_owned))
        .collect();
    let shared_libraries: Vec<String> = files
        .iter()
        .filter(|file| file["role"] != "runtime")
        .filter_map(|file| file["path"].as_str().map(str::to_owned))
        .filter(|path| path.ends_with(".so"))
        .collect();
    let manifest = serde_json::json!({
        "protocol": PROTOCOL,
        "plan": plan.id,
        "compiler": {
            "program": compiler.program,
            "version": compiler.version,
        },
        "units": units,
        "files": files,
        "runtime": {
            "python_modules": python_modules,
            "shared_libraries": shared_libraries,
        },
    });
    let text = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("cannot render the manifest: {error}"))?;
    std::fs::write(directory.join("manifest.json"), &text)
        .map_err(|error| format!("cannot write the manifest: {error}"))?;
    Ok(format!(
        "package {}  {} file(s)  {} bytes\ndigest {}\nplan {}\n",
        directory.display(),
        artifacts.len(),
        copied,
        xirang_kernel::sha256_hex(text.as_bytes()),
        plan.id
    ))
}
