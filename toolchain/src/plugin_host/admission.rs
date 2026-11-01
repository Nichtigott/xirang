//! Host-side plugin admission: from the lock a host writes to a verified artifact.
//! 宿主侧插件准入：从宿主写下的锁，到已验证的工件。
//!
//! The kernel owns every rule of plugin selection and verification, and the
//! adapters own execution — but nothing walked the path between them. A host had
//! to read `.xirang/plugins/*.lock` itself, build a [`PluginCatalog`] itself,
//! remember to call [`PluginPolicy::decision`] *with* that catalog, then pick
//! between `verify_signed` and `verify_artifact`, and finally guess which lane
//! the result qualifies for. Because that path was nobody's job, the Official
//! lane was unreachable in practice: no in-tree code ever called `verify_signed`,
//! and a lock written by the host's own plugin UI could not even match an
//! official manifest (the record carries no signature, the manifest must).
//! 内核拥有插件筛选与校验的每条规则，适配器拥有执行——但两者之间的路没有人走。宿主必须自己读
//! `.xirang/plugins/*.lock`、自己构造 [`PluginCatalog`]、记得把该目录传给
//! [`PluginPolicy::decision`]、再在 `verify_signed` 与 `verify_artifact` 之间选择，最后还要猜
//! 结果够得上哪条通道。因为这段路不属于任何人，Official 通道在实践中不可达：树内没有任何代码调用过
//! `verify_signed`，而宿主自己的插件界面写下的锁甚至无法匹配官方 manifest（记录不带签名，
//! 而 manifest 必须带）。
//!
//! This module is that path, and it is deliberately thin: selection, then the
//! strongest verification the artifact's source allows, then the lane the earned
//! assurance buys. It reads files, so it lives in an execution surface and not in
//! the kernel.
//! 本模块就是这段路，且刻意很薄：先筛选，再按工件来源做它能做的最强校验，最后按换来的保证等级
//! 定通道。它读文件，因此住在执行面而不是内核。

use std::path::Path;

use crate::run_method::{
    PluginArtifact, PluginAssurance, PluginCatalog, PluginChannel, PluginDecision, PluginPolicy,
    PluginSignatureVerifier, PluginSource, PluginTrustPolicy, VerifiedPluginArtifact,
};

use crate::plugin_host::HostError;

/// Directory, under a package root, that holds the plugin locks.
/// 包根之下存放插件锁的目录。
pub const PLUGIN_LOCK_DIRECTORY: &str = ".xirang/plugins";

/// Lock file a publisher maintains for the official lane.
/// 发布者为官方通道维护的锁文件。
pub const OFFICIAL_LOCK: &str = "official.lock";

/// Lock file a host writes for user plugins.
/// 宿主为用户插件写下的锁文件。
pub const USER_LOCK: &str = "user.lock";

/// Read both plugin locks under `package_root` into one catalogue.
/// 把 `package_root` 下的两份插件锁读成一份目录。
///
/// A missing directory or a missing file is an empty catalogue, not a failure: a
/// package that links no plugin has no lock, and that absence is safe because the
/// lock check is what refuses an *unlisted official* plugin — the digest and
/// signature checks still run for everything the host does admit.
/// 目录或文件缺失都是空目录，而不是失败：不链接任何插件的包没有锁，而这种缺失是安全的，因为拒绝
/// **未登记的官方**插件靠的正是锁检查——宿主真正准入的每个工件仍然要过摘要与签名检查。
pub fn plugin_catalog(package_root: &Path) -> Result<PluginCatalog, HostError> {
    let directory = package_root.join(PLUGIN_LOCK_DIRECTORY);
    let mut lock = String::new();
    for name in [OFFICIAL_LOCK, USER_LOCK] {
        let path = directory.join(name);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                lock.push_str(&text);
                lock.push('\n');
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(HostError::Policy(format!(
                    "cannot read {}: {error}",
                    path.display()
                )));
            }
        }
    }
    PluginCatalog::parse_plugin_catalog(&lock).map_err(|error| {
        HostError::Policy(format!(
            "invalid plugin lock under {}: {error}",
            directory.display()
        ))
    })
}

/// The lane an admitted artifact may enter, from the assurance it earned.
/// 已准入工件可进入的通道，取决于它换来的保证等级。
///
/// A verified signature is the only way into [`PluginChannel::Official`], and the
/// digest path is the user lane. `Community` and `Local` carry the same rule in
/// the kernel, so a host that declares `Local` may remap this answer.
/// 已验证的签名是进入 [`PluginChannel::Official`] 的唯一途径，纯摘要路径则是用户通道。
/// `Community` 与 `Local` 在内核中规则相同，因此声明 `Local` 的宿主可以改写这个答案。
pub fn lane_for(artifact: &VerifiedPluginArtifact) -> PluginChannel {
    match artifact.assurance() {
        PluginAssurance::Signature => PluginChannel::Official,
        PluginAssurance::Digest => PluginChannel::Community,
    }
}

/// Selection policy, trust root, catalogue, and verifier in one admission step.
/// 把筛选策略、信任根、目录与验证器合成一次准入。
pub struct PluginAdmission<V> {
    policy: PluginPolicy,
    trust: PluginTrustPolicy,
    catalog: PluginCatalog,
    verifier: V,
}

impl<V: PluginSignatureVerifier> PluginAdmission<V> {
    /// Build one admission from an already-parsed catalogue.
    /// 用一份已解析的目录构造准入。
    pub fn new(
        policy: PluginPolicy,
        trust: PluginTrustPolicy,
        catalog: PluginCatalog,
        verifier: V,
    ) -> Self {
        Self {
            policy,
            trust,
            catalog,
            verifier,
        }
    }

    /// Build one admission from the plugin locks under `package_root`.
    /// 从 `package_root` 下的插件锁构造准入。
    pub fn from_package_root(
        package_root: &Path,
        policy: PluginPolicy,
        trust: PluginTrustPolicy,
        verifier: V,
    ) -> Result<Self, HostError> {
        Ok(Self::new(
            policy,
            trust,
            plugin_catalog(package_root)?,
            verifier,
        ))
    }

    /// The catalogue this admission consults.
    /// 本准入所咨询的目录。
    pub fn catalog(&self) -> &PluginCatalog {
        &self.catalog
    }

    /// Admit one raw artifact: selection first, then verification.
    /// 准入一个原始工件：先筛选，后校验。
    ///
    /// The order matters and is the kernel's: revocation and the lock are
    /// consulted before the signature, so a revoked version — and an unlisted
    /// **official** one — is refused whatever the signature says. An official
    /// manifest must pass
    /// `verify_signed`, which requires a trust root — a host that configures none
    /// gets `MissingOfficialKey` instead of a silent downgrade to checksums; a
    /// user manifest takes the digest path, because no verifier is consulted for
    /// that source, and no lock constrains it: a user-source artifact no lock lists
    /// is admitted on its digest (see [`plugin_catalog`] for why that absence is
    /// safe).
    /// 顺序很重要，而且由内核决定：撤销与锁在签名之前被检查，因此已吊销的版本——以及未登记的**官方**
    /// 版本——无论签名说什么都被拒绝。官方 manifest 必须过 `verify_signed`，而它要求信任根——没有
    /// 配置信任根的宿主会拿到 `MissingOfficialKey`，而不是被静默降级为纯摘要；用户 manifest 走纯摘要
    /// 路径，因为该来源不会咨询验证器，锁对它也没有约束：任何锁都没登记的用户来源工件凭摘要即可准入
    /// （这种"缺失"为何安全，见 [`plugin_catalog`]）。
    pub fn admit(&self, artifact: PluginArtifact) -> Result<VerifiedPluginArtifact, HostError> {
        let manifest = artifact
            .registration
            .plugin
            .ok_or_else(|| HostError::Policy("plugin artifact has no manifest".to_owned()))?;
        match self.policy.decision(manifest, Some(&self.catalog)) {
            PluginDecision::Accepted => {}
            PluginDecision::Rejected(reason) => {
                return Err(HostError::Policy(format!(
                    "plugin rejected by the selection policy: {reason}"
                )));
            }
        }
        let verified = match manifest.source {
            PluginSource::Official => artifact.verify_signed(self.trust, &self.verifier),
            PluginSource::User => artifact.verify_artifact(self.trust),
        };
        verified.map_err(|error| {
            HostError::Policy(format!("plugin refused by the trust policy: {error}"))
        })
    }
}

#[cfg(feature = "wasm")]
impl<V: PluginSignatureVerifier> PluginAdmission<V> {
    /// Admit an artifact and install it into a Wasm slot.
    /// 准入一个工件并把它安装进 Wasm 槽。
    ///
    /// The lane comes from the assurance the artifact earned, and the slot's own
    /// contract, framework, and channel list are checked by `install` before
    /// anything is queued.
    /// 通道来自工件换来的保证等级；槽自身的合同、框架与通道列表由 `install` 在挂起任何东西之前
    /// 检查。
    pub fn install(
        &self,
        table: &crate::plugin_host::WasmPluginTable,
        slot: &str,
        artifact: PluginArtifact,
    ) -> Result<u64, HostError> {
        let verified = self.admit(artifact)?;
        let channel = lane_for(&verified);
        table.install(slot, channel, verified)
    }
}

#[cfg(feature = "process-tools")]
impl<V: PluginSignatureVerifier> PluginAdmission<V> {
    /// Admit an artifact and stage it for process execution.
    /// 准入一个工件并为进程执行暂存它。
    pub fn load_process(
        &self,
        backend: &crate::plugin_host::ProcessBackend,
        program: crate::plugin_host::ProcessProgram,
        artifact: PluginArtifact,
    ) -> Result<crate::plugin_host::ProcessInstance, HostError> {
        let verified = self.admit(artifact)?;
        backend.load_process_backend(verified, program)
    }
}
