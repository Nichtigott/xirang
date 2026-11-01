//! Tests for what this adapter refuses to frame, without allocating what it refuses.
//! 本适配器拒绝成帧什么——无需真的分配被它拒绝的那块内存。

use super::{MAX_FRAMEABLE_INPUT, check_input_length};

/// The frame width is a ceiling of its own: a length past `i32::MAX` cannot be handed to the
/// plugin at all, because the ABI's length field is an `i32` and the cast would wrap to a negative
/// number. Only the process adapter had such a ceiling (`u32::MAX`) and the two disagreed
/// (audit `PH-5`).
/// 帧宽本身就是一道上限：超过 `i32::MAX` 的长度根本无法交给插件，因为 ABI 的长度字段是 `i32`，
/// 强制转换会回绕成负数。过去只有进程适配器有这类上限（`u32::MAX`），两者口径不一致（审计
/// `PH-5`）。
#[test]
fn an_input_past_the_frame_width_is_refused() {
    assert!(
        check_input_length(MAX_FRAMEABLE_INPUT, usize::MAX).is_ok(),
        "the largest length an i32 can carry is still frameable"
    );
    let error = check_input_length(MAX_FRAMEABLE_INPUT + 1, usize::MAX)
        .expect_err("one byte past the frame width must be refused");
    assert!(
        error.to_string().contains("frames its length in an i32"),
        "the refusal says which ceiling refused it: {error}"
    );
}

/// The configured limit keeps its own message and its own precedence.
/// 配置上限保留自己的消息与优先级。
#[test]
fn the_configured_limit_still_refuses_with_its_own_message() {
    let error = check_input_length(1024, 512).expect_err("past the configured limit");
    assert!(
        error
            .to_string()
            .contains("input is 1024 bytes; limit is 512"),
        "{error}"
    );
}
