//! Endpoint spelling shared by the move planner and its rebind pass.
//! 搬动规划器与它的重绑趟共用的端点拼写。

/// An endpoint without `crate::`/`self::`, matching the parser's comparison.
/// 去掉 `crate::`/`self::` 的端点，与解析器的比较一致。
///
/// Both passes compare a declared endpoint against one they derive, and the parser folds the same
/// two prefixes. One home keeps the folding from drifting between "is this endpoint already seen"
/// and "does this endpoint match the expected identity".
/// 两趟都把声明的端点与它们推导出的端点相比，而解析器折叠同样两个前缀。一个家让折叠不会在"这个端点是否
/// 见过"与"这个端点是否等于预期身份"之间漂移。
pub(super) fn normalized(endpoint: &str) -> String {
    endpoint
        .trim()
        .strip_prefix("crate::")
        .or_else(|| endpoint.trim().strip_prefix("self::"))
        .unwrap_or(endpoint.trim())
        .to_owned()
}
