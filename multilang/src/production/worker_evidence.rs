//! The worker's evidence, parsed into the one line a client prints.
//! worker 的证据，解析成客户端打印的一行。

use serde_json::Value;

/// Summarise one evidence document for a client that did not run the build.
/// 为没有运行构建的客户端概括一份证据文档。
pub(super) fn summarise(text: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(text)
        .map_err(|error| format!("the worker's evidence is not JSON: {error}"))?;
    let plan = value.get("plan").and_then(Value::as_str).unwrap_or("?");
    let actions = value
        .get("actions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let reused = actions
        .iter()
        .filter(|action| action.get("reused").and_then(Value::as_bool) == Some(true))
        .count();
    let restored = actions
        .iter()
        .filter(|action| action.get("restored").and_then(Value::as_bool) == Some(true))
        .count();
    let tests = value
        .get("tests")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let counted = value
        .get("counts")
        .and_then(|counts| counts.get("tests"))
        .and_then(|tests| {
            tests
                .get("passed")
                .and_then(Value::as_u64)
                .zip(tests.get("failed").and_then(Value::as_u64))
        });
    let (passed, failed) = match counted {
        Some((passed, failed)) => (passed as usize, failed as usize),
        None => {
            let passed = tests
                .iter()
                .filter(|test| test.get("passed").and_then(Value::as_bool) == Some(true))
                .count();
            (passed, tests.len().saturating_sub(passed))
        }
    };
    let failed_actions = value
        .get("counts")
        .and_then(|counts| counts.get("actions"))
        .and_then(|actions| actions.get("failed").and_then(Value::as_u64))
        .unwrap_or(0);
    let result = value
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    Ok(format!(
        "plan {plan}: {} action(s) ({reused} reused, {restored} restored), {failed_actions} action(s) \
         failed, {passed} test(s) passed / {failed} failed, result {result}",
        actions.len()
    ))
}

/// Read an even-length hex string back into bytes.
/// 把偶数长度的十六进制字符串读回字节。
pub(super) fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("an odd number of hex digits".to_owned());
    }
    let digits = text.as_bytes();
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    let mut index = 0;
    while index < digits.len() {
        let high = hex_digit(digits[index]);
        let low = hex_digit(digits[index + 1]);
        if high == 0xff || low == 0xff {
            let pair = &text[index..index + 2];
            return Err(format!("`{pair}` is not a hex byte"));
        }
        bytes.push((high << 4) | low);
        index += 2;
    }
    Ok(bytes)
}

/// One hex digit as its value, or `0xff` when the byte is not a hex digit. A table lookup rather
/// than `from_str_radix` per byte: a piece is megabytes, and the parse used to be most of the time
/// this transport spent.
/// 一个十六进制数字的值；不是十六进制数字时为 `0xff`。用查表而不是逐字节 `from_str_radix`：一片有
/// 数兆字节，那种解析曾是这条传输花掉的大部分时间。
const fn hex_digit(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => 0xff,
    }
}
