//! Parallel syntax readers exercised outside the pure kernel source tree.
//! 在纯内核源码树之外验证并行语法读取。

#![cfg(feature = "syntax")]

use xirang_kernel::syntax::{application_entries, parse_face};

#[test]
fn parallel_native_parsers_preserve_results_without_process_state() {
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..32 {
                    assert!(
                        parse_face("pub fn shared_ordinary() {} ")
                            .unwrap()
                            .is_none()
                    );
                    assert!(
                        application_entries("pub fn shared_ordinary() {} ")
                            .unwrap()
                            .is_empty()
                    );
                    assert!(parse_face("pub fn shared_ordinary() {@}").is_err());
                }
            });
        }
    });
}
