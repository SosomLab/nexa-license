//! Ed25519 검증 어댑터(`ed25519` feature · dalek 2.x). 서명 **생성**은 여기 없다(비공개 발급기).

use crate::{types::Alg, verify::SigVerifier};

/// dalek `verify_strict`(비정규 서명 거부).
#[derive(Debug, Default, Clone, Copy)]
pub struct Ed25519Verifier;

impl SigVerifier for Ed25519Verifier {
    fn verify(&self, alg: Alg, pubkey: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        if alg != Alg::Ed25519 {
            return false;
        }
        let Ok(pk_bytes): Result<&[u8; 32], _> = pubkey.try_into() else {
            return false;
        };
        let Ok(pk) = ed25519_dalek::VerifyingKey::from_bytes(pk_bytes) else {
            return false;
        };
        let Ok(s) = ed25519_dalek::Signature::from_slice(sig) else {
            return false;
        };
        pk.verify_strict(msg, &s).is_ok()
    }
}
