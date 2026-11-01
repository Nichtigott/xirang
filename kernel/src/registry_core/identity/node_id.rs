use super::*;

/// Stable source identity shared by registration, diagnostics, and pruning.
/// 注册、诊断和修剪共用的稳定源码身份。
///
/// This is a compact 128-bit prefix of SHA-256 for indexing and diagnostics,
/// not a cryptographic signature. Plugin trust must use the full digest and a
/// signature verifier.
/// 这是用于索引和诊断的 SHA-256 128 位短标识，不是密码学签名。插件信任必须
/// 使用完整摘要和签名验证器。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId([u8; 16]);

/// A logical face identity that can survive a source-file move.
/// 可跨源码文件移动保持不变的逻辑注册面身份。
///
/// `NodeId` is the source-instance identity used by the tree and pruning.
/// `StableFaceId` is opt-in and survives a source-file move.
/// `NodeId` 是注册树和修剪使用的源码实例身份；`StableFaceId` 可跨文件移动保持不变。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableFaceId([u8; 16]);

/// The only node not registered through an object declaration.
/// 唯一不通过对象声明注册的节点。
pub const ROOT_NODE_ID: NodeId = NodeId::from_path("<root>", "root");

impl NodeId {
    /// Rebuild an identity emitted by XiRang's build step.
    /// 从 XiRang 构建步骤生成的字节恢复身份。
    #[doc(hidden)]
    pub const fn from_raw(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Hash `relative_path + 0x00 + declared_name` during constant evaluation.
    /// 在常量求值期间散列“相对路径 + 0x00 + 声明名”。
    ///
    /// Both slices are separator-folded here; `from_namespaced_path` explains why
    /// the namespaced entry point folds only its first two components.
    /// 这里两个切片都会折叠分隔符；命名空间入口为何只折叠前两个组成部分，见
    /// `from_namespaced_path`。
    pub const fn from_path(relative_path: &str, declared_name: &str) -> Self {
        Self(sha256_path_prefix(
            relative_path.as_bytes(),
            declared_name.as_bytes(),
            true,
        ))
    }

    /// Hash a declaration together with the package namespace that owns it.
    /// 将声明与拥有它的包命名空间一起散列，避免不同库的相对路径串库。
    pub const fn from_namespaced_path(
        namespace: &str,
        relative_path: &str,
        declared_name: &str,
    ) -> Self {
        // Hashing the first two components and feeding that digest into the
        // final hash keeps the implementation const and allocation-free while
        // making the namespace part of the identity domain.
        // 先散列前两个组成部分、再把该摘要喂进最后一次散列，既让实现保持 const 且不分配，
        // 又让命名空间成为身份域的一部分。
        //
        // The outer hash deliberately does NOT fold the declared name, while
        // `from_path` does fold its name slice. A declared name is a Rust
        // identifier and cannot contain a path separator, so the two spellings
        // are unobservable in practice; both entry points are pinned to literals
        // by `pinned_identities_are_an_on_disk_compatibility_contract` anyway.
        // Do not "align" them: identities are written into on-disk graft records,
        // so any change here renames every existing node.
        // 外层散列有意不折叠声明名，而 `from_path` 会折叠它的名字切片。声明名是 Rust
        // 标识符，不可能包含路径分隔符，因此两种拼法在实践中不可观测；两个入口也都已由
        // `pinned_identities_are_an_on_disk_compatibility_contract` 钉在字面量上。
        // 不要为了“对齐”而改动这里：身份会写入落盘的 graft 记录，任何改动都会重命名现有
        // 的每一个节点。
        let scoped = sha256_path_prefix(namespace.as_bytes(), relative_path.as_bytes(), true);
        Self(sha256_prefix(&scoped, declared_name.as_bytes(), true))
    }

    /// Hash one byte slice; primarily useful for standard-vector tests.
    /// 散列单个字节切片，主要用于标准向量测试。
    pub const fn from_bytes(input: &[u8]) -> Self {
        Self(sha256_prefix(input, &[], false))
    }

    /// The raw 128-bit identity, round-trippable through `from_raw` at build time.
    /// 原始 128 位身份，构建期可经 `from_raw` 还原。
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Produce a node-specific compile-time payload for pruning verification.
    /// 为修剪验证生成节点专属的编译期负载。
    ///
    /// The table is emitted only when the owning node's probe is reached.
    /// 只有触达所属节点探针时，这张表才会进入最终产物。
    pub const fn pruning_table(self) -> [u64; 4096] {
        let bytes = self.0;
        let mut state = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let mut table = [0_u64; 4096];
        let mut index = 0;
        while index < table.len() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            table[index] = state.wrapping_add(index as u64);
            index += 1;
        }
        table
    }
}

/// Return the root identity for one isolated package namespace.
/// 返回某个隔离包命名空间的根身份。
pub const fn root_node_id(namespace: &str) -> NodeId {
    NodeId::from_namespaced_path(namespace, "<root>", "root")
}

impl StableFaceId {
    /// Hash an author-owned logical name, independent of its source path.
    /// 对作者拥有的逻辑名称做哈希，与源码路径无关。
    pub const fn from_name(name: &str) -> Self {
        Self(sha256_prefix(name.as_bytes(), &[], false))
    }

    /// Hash a name inside the package namespace that owns it, so two packages that both derive the
    /// same logical path do not produce one object identity.
    /// 在拥有它的包命名空间里散列一个名字，使两个推导出同一逻辑路径的包不会产生同一个对象身份。
    ///
    /// This is the **derived** object key: it is what a face gets when the author declared no
    /// `stable_name`, and its pre-image is the logical registration path (the registry position), not
    /// the source path — that is why a physical move keeps it. An explicit `stable_name` keeps using
    /// [`Self::from_name`], exactly the value the tree already computes, so the two derivations never
    /// disagree about the same declaration.
    /// 这是**推导出的**对象键：作者没有声明 `stable_name` 时一个面得到的就是它，而它的原像是逻辑注册路径
    /// （注册位置），不是源码路径——这正是物理搬动保持它的原因。显式 `stable_name` 仍用
    /// [`Self::from_name`]，也就是树本来就算出的那个值，因此两种推导对同一条声明不会给出不同答案。
    pub const fn from_namespaced_name(namespace: &str, name: &str) -> Self {
        // Same domain-separation shape as `NodeId::from_namespaced_path`: the namespace is folded
        // into an inner digest that becomes the outer input, so the namespace is part of the domain
        // and not a prefix an author could collide with.
        // 与 `NodeId::from_namespaced_path` 同一个域分离形状：命名空间先折进一个内层摘要，再作为外层
        // 输入，因此命名空间是域的一部分，而不是作者能撞上的前缀。
        let scoped = sha256_path_prefix(namespace.as_bytes(), name.as_bytes(), false);
        Self(sha256_prefix(&scoped, &[], false))
    }

    /// The raw 128-bit identity behind this author-owned stable name.
    /// 该作者拥有的稳定名称背后的原始 128 位身份。
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Display for StableFaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for StableFaceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

/// The error [`NodeId::from_str`] returns for input that is not exactly 32
/// hexadecimal digits.
/// [`NodeId::from_str`] 对不是恰好 32 个十六进制数字的输入返回的错误。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseNodeIdError;

impl fmt::Display for ParseNodeIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("node identity must contain exactly 32 hexadecimal digits")
    }
}

impl std::error::Error for ParseNodeIdError {}

impl FromStr for NodeId {
    type Err = ParseNodeIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        decode_id(value).map(Self)
    }
}

impl FromStr for StableFaceId {
    type Err = ParseNodeIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        decode_id(value).map(Self)
    }
}

/// Decode exactly 32 hexadecimal digits into one 128-bit identity.
/// 把恰好 32 个十六进制数字解码成一个 128 位身份。
///
/// Both identity types persist as the same spelling, so the reader is one reader: a second copy
/// would be a second answer to "is this string an identity".
/// 两种身份都以同一种拼写持久化，因此读取方只有一个：第二份拷贝就是"这个字符串是不是身份"的第二个答案。
fn decode_id(value: &str) -> Result<[u8; 16], ParseNodeIdError> {
    if value.len() != 32 {
        return Err(ParseNodeIdError);
    }
    let bytes = value.as_bytes();
    let mut id = [0_u8; 16];
    for index in 0..16 {
        let high = hex_nibble(bytes[index * 2]).ok_or(ParseNodeIdError)?;
        let low = hex_nibble(bytes[index * 2 + 1]).ok_or(ParseNodeIdError)?;
        id[index] = (high << 4) | low;
    }
    Ok(id)
}

/// Decode one hexadecimal digit (0-9, a-f, A-F).
/// 解码一个十六进制数字符（0-9、a-f、A-F）。
///
/// Internal to the identity module: a caller who needs to read bytes out of a
/// hex string uses [`hex_decode`], which owns the pairing and the error case.
/// 身份模块内部使用：需要从十六进制字符串取出字节的调用方用 [`hex_decode`]，配对与
/// 错误情形由它负责。
pub(crate) const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Decode a whole even-length hexadecimal string.
/// 解码完整偶数长度的十六进制字符串。
///
/// Returns `None` for odd lengths or any non-hex digit.
/// 长度为奇数或含有非十六进制字符时返回 `None`。
pub fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    let chunk = bytes.as_chunks::<2>().0;
    for pair in chunk {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        decoded.push((high << 4) | low);
    }
    Some(decoded)
}
