//! Ed25519 verification at the plugin execution boundary.
//! 插件执行边界上的 Ed25519 验证。

use crate::run_method::{PluginManifest, PluginSignatureVerifier};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// One public key the host will trust, named by its SHA-256 fingerprint.
/// 宿主愿意信任的一个公钥，以它的 SHA-256 指纹命名。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustedPublicKey {
    /// SHA-256 hex of `bytes`; a key whose bytes do not hash to it is rejected.
    /// `bytes` 的 SHA-256 十六进制；字节哈希对不上的公钥会被拒绝。
    pub fingerprint: &'static str,
    /// Raw 32-byte Ed25519 public key.
    /// 32 字节的 Ed25519 原始公钥。
    pub bytes: [u8; 32],
}

impl TrustedPublicKey {
    /// Pair a fingerprint with the key bytes it must name.
    /// 把指纹与它必须指称的公钥字节配对。
    pub const fn new(fingerprint: &'static str, bytes: [u8; 32]) -> Self {
        Self { fingerprint, bytes }
    }
}

/// Verifies plugin signatures against a fixed set of trusted public keys.
/// 用一组固定可信公钥验证插件签名。
#[derive(Clone, Copy, Debug)]
pub struct Ed25519Verifier {
    keys: &'static [TrustedPublicKey],
}

impl Ed25519Verifier {
    /// Trust exactly the listed keys for the lifetime of the verifier.
    /// 在验证器生命周期内只信任列出的公钥。
    pub const fn new(keys: &'static [TrustedPublicKey]) -> Self {
        Self { keys }
    }

    fn key(&self, fingerprint: &str) -> Option<VerifyingKey> {
        self.keys
            .iter()
            .find(|key| key.fingerprint.eq_ignore_ascii_case(fingerprint))
            .filter(|key| {
                crate::run_method::sha256_hex(&key.bytes).eq_ignore_ascii_case(key.fingerprint)
            })
            .and_then(|key| VerifyingKey::from_bytes(&key.bytes).ok())
    }
}

impl PluginSignatureVerifier for Ed25519Verifier {
    /// Check `manifest.signature` against the canonical `payload`.
    /// 用给定的规范化 `payload` 校验 `manifest.signature`。
    ///
    /// The payload is the message the kernel built for the artifact — manifest
    /// fields, the registration that travels with the bytes, and the bytes
    /// themselves — so this verifier never reassembles it and cannot drop a
    /// field the kernel added.
    /// 载荷是内核为工件构造的消息——manifest 字段、随字节同行的注册声明，以及字节本身——因此
    /// 本验证器不自行拼装消息，也不会丢掉内核加入的字段。
    fn verify(&self, manifest: PluginManifest, payload: &[u8], fingerprint: &str) -> bool {
        let Some(key) = self.key(fingerprint) else {
            return false;
        };
        let Some(signature) = manifest.signature.and_then(decode_signature) else {
            return false;
        };
        key.verify(payload, &signature).is_ok()
    }
}

fn decode_signature(value: &str) -> Option<Signature> {
    if value.len() != 128 {
        return None;
    }
    let bytes: [u8; 64] = crate::run_method::hex_decode(value)?.try_into().ok()?;
    Some(Signature::from_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use crate::run_method::{FrameworkId, PluginMode, PluginSource, sha256_hex};
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    #[test]
    fn verifies_a_signature_over_the_payload_the_kernel_built() {
        let signing = SigningKey::from_bytes(&[7; 32]);
        let verifying = signing.verifying_key();
        let fingerprint = Box::leak(sha256_hex(verifying.as_bytes()).into_boxed_str());
        // The host never rebuilds this message: it verifies whatever the kernel
        // handed it, which is what makes the registration part of the signature.
        // 宿主从不重建这条消息：它验证内核交给它的字节，注册声明因此进入签名覆盖范围。
        let payload = b"manifest+registration+bytes";
        let encoded = signing
            .sign(payload)
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let key = TrustedPublicKey::new(fingerprint, verifying.to_bytes());
        let verifier = Ed25519Verifier::new(Box::leak(Box::new([key])));

        assert!(verifier.verify(
            manifest(Some(Box::leak(encoded.into_boxed_str())), fingerprint),
            payload,
            fingerprint,
        ));
        assert!(!verifier.verify(manifest(Some("bad"), fingerprint), payload, fingerprint));
    }

    fn manifest(signature: Option<&'static str>, fingerprint: &'static str) -> PluginManifest {
        PluginManifest {
            name: "fixture",
            crate_name: "fixture",
            version: "1.0.0",
            framework: FrameworkId::new("xirang.default"),
            source: PluginSource::Official,
            mode: PluginMode::Extension,
            checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            signature,
            public_key_fingerprint: Some(fingerprint),
            revocation_list: Some("official-1"),
        }
    }
}
