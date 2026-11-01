//! Named indices of the New Project wizard's rows.
//! 新建项目向导各行的具名下标。
//!
//! The wizard's array is tiny, but a bare `values[0]` at a call site says nothing
//! about which row it is: the reader has to jump back to the struct to find out.
//! Naming the rows is what makes `values[new_project_field::DIRECTORY]` readable
//! where it is used, and what turns a reordering into a compile error. The labels
//! live here too, so the meaning of a row and the text the form prints for it
//! cannot drift apart.
//! 向导的数组很小，但调用点上的裸 `values[0]` 说明不了它是哪一行：读者得跳回结构体才
//! 知道。给行起名，正是让 `values[new_project_field::DIRECTORY]` 在使用处可读、并让
//! 重新排序变成编译错误的原因。标签也住在这里，因此某一行的含义与表单为它打印的文本不会
//! 各自漂移。

/// Index of the project directory row.
/// 项目目录行的下标。
pub(crate) const DIRECTORY: usize = 0;
/// Index of the package-name row.
/// 包名行的下标。
pub(crate) const PACKAGE: usize = 1;
/// Index of the project-kind row.
/// 项目类型行的下标。
pub(crate) const KIND: usize = 2;

/// How many rows the wizard has.
/// 向导的行数。
pub(crate) const COUNT: usize = 3;

/// Row labels in index order.
/// 按下标顺序排列的行标签。
pub(crate) const LABELS: [&str; COUNT] = ["directory", "package", "kind"];

#[cfg(test)]
mod tests {
    use super::*;

    /// The constants are the layout: dense, ordered, and each one's label names
    /// the row it indexes.
    /// 常量就是布局：稠密、有序，且每个常量的标签点名的正是它索引的那一行。
    #[test]
    fn the_row_names_are_dense_and_labelled() {
        assert_eq!([DIRECTORY, PACKAGE, KIND], [0, 1, 2]);
        assert_eq!(COUNT, LABELS.len());
        assert_eq!(LABELS[DIRECTORY], "directory");
        assert_eq!(LABELS[PACKAGE], "package");
        assert_eq!(LABELS[KIND], "kind");
    }
}
