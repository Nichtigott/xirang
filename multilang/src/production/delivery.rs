//! The delivery wrappers: packaging, verification, install generations, rollback, cache export and
//! import, build economics and the install history. They are thin: the rules live in their modules.
//! 交付类包装：打包、核验、安装世代、回退、缓存导出与导入、构建经济性与安装历史。它们都很薄：规则住在
//! 各自的模块里。

use std::path::Path;

/// Export the build cache (content store + stamps) for another checkout.
/// 把构建缓存（内容库＋印记）导出给另一个检出。
pub fn cache_export(root: &Path, destination: &Path) -> Result<String, String> {
    super::cache::export(root, destination)
}

/// Import an exported build cache after verifying it.
/// 核对之后导入一份导出的构建缓存。
pub fn cache_import(root: &Path, source: &Path, dry_run: bool) -> Result<String, String> {
    super::cache::import(root, source, dry_run)
}

/// Verify a packaged directory against the manifest it carries, without installing it.
/// 按包自带的清单核验交付目录，不安装。
pub fn verify_package(directory: &std::path::Path) -> Result<String, String> {
    super::package::verify(directory)
}

/// Package a finished build as an immutable delivery directory.
/// 把一次已完成的构建打包成不可变的交付目录。
pub fn package(root: &Path, output: Option<&Path>) -> Result<String, String> {
    super::package::create(root, output)
}

/// Install a package under a prefix as a new generation.
/// 把包装到前缀下成为一个新世代。
pub fn install(prefix: &Path, package: &Path) -> Result<String, String> {
    super::install::install(
        &super::install::absolute(prefix),
        &super::install::absolute(package),
    )
}

/// Make the previous generation current again.
/// 让上一代重新成为当前。
pub fn rollback(prefix: &Path) -> Result<String, String> {
    super::install::rollback(&super::install::absolute(prefix))
}

/// Summarize one build's economics from the evidence it wrote.
/// 从一次构建写下的证据汇总它的经济性。
pub fn report(root: &Path, plan: Option<&str>) -> Result<String, String> {
    super::report::summarize(root, plan)
}

/// Verify a prefix's append-only install log against the generations it still holds.
/// 让一个前缀的只追加安装日志与它仍在的世代互相校验。
pub fn log(prefix: &Path, json: bool) -> Result<(String, bool), String> {
    super::log::verify(&super::install::absolute(prefix), json)
}

/// Report what an install prefix holds and whether its generations still verify.
/// 报告一个安装前缀里有什么，以及它的世代是否仍能通过校验。
pub fn history(prefix: &Path) -> Result<String, String> {
    super::install::diagnose(prefix)
}
