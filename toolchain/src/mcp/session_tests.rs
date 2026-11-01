//! Pins for the session ledger: the first time is the full text, the second is a reference.
//! 会话账本的钉子：第一次给全文，第二次给一个回指。

use super::first_time;

/// The first time is recorded, and it carries the root so unrelated callers never share it.
/// 第一次会被记下，而且键带着根，因此不相干的调用方绝不会共享它。
///
/// `W2-2`/`W2-5` measure a constant block repeated in every answer; the ledger is what turns the
/// second one into a reference. The **root in the key** is the half that keeps a test's scratch
/// directory from being "already said" by another test's, which is why this pin exists at all.
/// `W2-2`/`W2-5` 量的是每条答案里重复的常量块；账本就是让第二次变成一次回指的东西。**键里的根**是
/// 让一个测试的临时目录不被另一个测试"已经说过"的那一半，这条钉子正是为此。
#[test]
fn the_first_time_is_recorded_per_key() {
    super::forget();
    assert!(
        first_time("block:/tmp/a"),
        "the first call is the first time"
    );
    assert!(!first_time("block:/tmp/a"), "and the second is not");
    assert!(
        first_time("block:/tmp/b"),
        "another root has its own first time"
    );
    super::forget();
    assert!(
        first_time("block:/tmp/a"),
        "forgetting makes the next call a first call again"
    );
}
