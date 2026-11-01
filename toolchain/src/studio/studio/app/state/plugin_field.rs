//! Named indices of the plugin-selection form's rows.
//! 插件选择表单各行的具名下标。
//!
//! Same reason as the New Project wizard: one noun per row, named where it is
//! used, with the printed label kept beside the index so the two cannot drift.
//! 与新建项目向导同理：一行一个名词，在使用处具名，打印标签与下标放在一起，因此两者不会
//! 各自漂移。

/// Index of the plugin source row.
/// 插件来源行的下标。
pub(crate) const SOURCE: usize = 0;
/// Index of the plugin framework row.
/// 插件框架行的下标。
pub(crate) const FRAMEWORK: usize = 1;
/// Index of the plugin package row.
/// 插件包名行的下标。
pub(crate) const PACKAGE: usize = 2;
/// Index of the plugin version row.
/// 插件版本行的下标。
pub(crate) const VERSION: usize = 3;
/// Index of the plugin crate anchor row.
/// 插件 crate 锚点行的下标。
pub(crate) const CRATE: usize = 4;
/// Index of the plugin checksum row.
/// 插件校验和行的下标。
pub(crate) const CHECKSUM: usize = 5;
/// Index of the plugin mode row.
/// 插件模式行的下标。
pub(crate) const MODE: usize = 6;
/// Index of the recorded-signature row, empty when the plugin was not signed.
/// 记录签名行的下标；插件未签名时留空。
pub(crate) const SIGNATURE: usize = 7;
/// Index of the signing-key-fingerprint row, empty when no key was recorded.
/// 签名密钥指纹行的下标；未记录密钥时留空。
pub(crate) const FINGERPRINT: usize = 8;
/// Index of the revocation-list-snapshot row, empty when none was recorded.
/// 撤销列表快照行的下标；未记录时留空。
pub(crate) const REVOCATIONS: usize = 9;

/// How many rows the form has.
/// 表单的行数。
pub(crate) const COUNT: usize = 10;

/// Row labels in index order.
/// 按下标顺序排列的行标签。
pub(crate) const LABELS: [&str; COUNT] = [
    "source",
    "framework",
    "package",
    "version",
    "crate",
    "checksum",
    "mode",
    "signature",
    "fingerprint",
    "revocations",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The constants are the layout: dense, ordered, and each one's label names
    /// the row it indexes.
    /// 常量就是布局：稠密、有序，且每个常量的标签点名的正是它索引的那一行。
    #[test]
    fn the_row_names_are_dense_and_labelled() {
        assert_eq!(
            [
                SOURCE,
                FRAMEWORK,
                PACKAGE,
                VERSION,
                CRATE,
                CHECKSUM,
                MODE,
                SIGNATURE,
                FINGERPRINT,
                REVOCATIONS,
            ],
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        );
        assert_eq!(COUNT, LABELS.len());
        assert_eq!(LABELS[SOURCE], "source");
        assert_eq!(LABELS[FRAMEWORK], "framework");
        assert_eq!(LABELS[PACKAGE], "package");
        assert_eq!(LABELS[VERSION], "version");
        assert_eq!(LABELS[CRATE], "crate");
        assert_eq!(LABELS[CHECKSUM], "checksum");
        assert_eq!(LABELS[MODE], "mode");
        assert_eq!(LABELS[SIGNATURE], "signature");
        assert_eq!(LABELS[FINGERPRINT], "fingerprint");
        assert_eq!(LABELS[REVOCATIONS], "revocations");
    }
}
