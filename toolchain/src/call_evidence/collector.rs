//! Debug-only linker-section backend for XiRang declarations.
//! 仅调试期使用的 XiRang 声明链接段后端。

/// Iterate every declaration that opted into the debug collector in this
/// binary; outside debug builds the result is empty rather than absent.
/// 遍历本二进制中选择加入调试收集器的所有声明；非调试构建下结果为空而非缺失。
#[macro_export]
macro_rules! registrations {
    ($ty:ty) => {{
        #[cfg(debug_assertions)]
        {
            $crate::call_evidence::inventory::iter::<$crate::call_evidence::CollectedRegistration>
                .into_iter()
                .map(|entry| entry.0)
        }
        #[cfg(not(debug_assertions))]
        {
            std::iter::empty::<&'static $ty>()
        }
    }};
}

/// Submit one `'static` registration value to the debug collector; the value
/// stays in the binary's inventory section and is only read in debug builds.
/// 向调试收集器提交一个 `'static` 注册值；该值留在二进制的 inventory 段中，
/// 仅在调试构建中被读取。
#[macro_export]
macro_rules! submit {
    ($value:expr) => {
        $crate::call_evidence::inventory::submit! {
            $crate::call_evidence::CollectedRegistration(&$value)
        }
    };
}
