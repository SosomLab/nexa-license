//! 발급 — 키 생성 · 서명(feature `issuer` · nexa-sql docs/25 §12-2). **앱은 이 모듈을 켜지 않는다**(검증 전용) — 발급기·서버만.
//! 안전의 근거는 이 코드가 아니라 **루트 비밀키 보관**(docs/91 §0)이다.

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

use crate::{base32, format::Doc, LICENSE_DOMAIN};

/// Ed25519 키쌍(비밀키 32B · 공개키 32B).
#[derive(Debug, Clone)]
pub struct Keypair {
    pub secret: [u8; 32],
    pub public: [u8; 32],
}

impl Keypair {
    /// OS 난수로 생성.
    #[must_use]
    pub fn generate() -> Keypair {
        let sk = SigningKey::generate(&mut rand_core::OsRng);
        Keypair {
            secret: sk.to_bytes(),
            public: sk.verifying_key().to_bytes(),
        }
    }

    /// 비밀키 32B에서 복원(공개키는 유도).
    #[must_use]
    pub fn from_secret(secret: [u8; 32]) -> Keypair {
        let sk = SigningKey::from_bytes(&secret);
        Keypair {
            secret,
            public: sk.verifying_key().to_bytes(),
        }
    }

    /// 공개키 base32(사람이 옮겨 적는 표현 · `.pub` 파일 · `keys.rs`).
    #[must_use]
    pub fn public_b32(&self) -> String {
        base32::encode(&self.public)
    }

    /// 임의 메시지 서명(64B).
    #[must_use]
    pub fn sign(&self, msg: &[u8]) -> [u8; 64] {
        SigningKey::from_bytes(&self.secret).sign(msg).to_bytes()
    }

    /// 문서 서명 — `sig`를 제외한 정규화 본문(`domain`)에 서명해 `sig=`(base32)를 채운다. `key=`가 비어 있으면 `key_id`를 넣는다.
    pub fn sign_doc(&self, doc: &mut Doc, domain: &str, key_id: &str) {
        if doc.get("key").is_none() && !key_id.is_empty() {
            doc.set("key", key_id);
        }
        doc.pairs.retain(|(k, _)| k != crate::format::SIG_KEY);
        let sig = self.sign(&doc.canonical(domain));
        doc.set(crate::format::SIG_KEY, &base32::encode(&sig));
    }

    /// 라이선스 서명(도메인 = [`LICENSE_DOMAIN`]).
    pub fn sign_license(&self, doc: &mut Doc, key_id: &str) {
        self.sign_doc(doc, LICENSE_DOMAIN, key_id);
    }

    /// 공개키가 비밀키와 짝인지(봉투 손상 검출).
    #[must_use]
    pub fn matches_public(&self, public: &[u8]) -> bool {
        VerifyingKey::from_bytes(&self.public).is_ok() && public == self.public
    }
}

/// 공개키 파일(`.pub`) 본문 — key=value(라이선스와 같은 파서).
#[must_use]
pub fn public_key_file(key_id: &str, public: &[u8]) -> String {
    let mut d = Doc::default();
    d.set("format", "nxp1");
    d.set("key", key_id);
    d.set("alg", "ed25519");
    d.set("public", &base32::encode(public));
    d.serialize()
}

/// `.pub` 본문 → (key id · 공개키 32B).
#[must_use]
pub fn parse_public_key_file(text: &str) -> Option<(String, [u8; 32])> {
    let d = Doc::parse(text);
    if d.get("format") != Some("nxp1") || d.get("alg").is_some_and(|a| a != "ed25519") {
        return None;
    }
    let pk = base32::decode(d.get("public")?)?;
    let pk: [u8; 32] = pk.try_into().ok()?;
    Some((d.get("key").unwrap_or("").to_string(), pk))
}

/// `keys.rs` 본문 생성 — 여러 `.pub`(회전 = 둘 이상)을 `ROOT_KEYS` 목록으로. 앱 저장소의 `crates/nexa-license/src/keys.rs`를 이것으로 덮는다.
#[must_use]
pub fn keys_rs_source(keys: &[(String, [u8; 32])]) -> String {
    let mut s = String::from(
        "//! SosomLab 루트 **공개키**(검증용 · 공개 무방). 비밀키는 어느 저장소에도 없다 — 발급 PC의 암호 봉투에만(nexa-sql docs/25 §12-6 · docs/91).\n\
         //! 회전(D-27): 새 키를 다음 자리에 더해 두 키를 동시에 검증하다가, 모든 앱이 새 판이 된 뒤 옛 키를 뺀다.\n\
         //! ★ 이 파일은 `nexa-license-tool keys-rs <a.pub> [<b.pub>…]`가 만든다 — 손으로 고치지 말 것.\n\n\
         use crate::types::Alg;\n\n\
         /// 루트 키 한 개 — `id`는 라이선스의 `key=` 값과 맞춘다(없으면 같은 `alg`의 모든 키를 시도).\n\
         #[derive(Debug, Clone, Copy)]\n\
         pub struct RootKey {\n    pub id: &'static str,\n    pub alg: Alg,\n    pub public: &'static [u8],\n}\n\n\
         /// 현재 유효한 루트 공개키 목록.\n\
         pub const ROOT_KEYS: &[RootKey] = &[\n",
    );
    for (id, pk) in keys {
        s.push_str("    RootKey {\n        id: \"");
        s.push_str(id);
        s.push_str("\",\n        alg: Alg::Ed25519,\n        public: &[\n");
        for chunk in pk.chunks(8) {
            s.push_str("            ");
            for b in chunk {
                s.push_str(&format!("0x{b:02x}, "));
            }
            s.push('\n');
        }
        s.push_str("        ],\n    },\n");
    }
    s.push_str("];\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keys::RootKey,
        types::{Alg, Product},
        verify::{verify_license, Verdict},
    };

    #[test]
    fn keygen_sign_and_verify_round_trip() {
        let kp = Keypair::generate();
        let kp2 = Keypair::from_secret(kp.secret);
        assert_eq!(kp.public, kp2.public);
        assert!(kp.matches_public(&kp.public));
        let mut d = Doc::default();
        for (k, v) in [
            ("format", "nxl1"),
            ("product", "nexa-sql"),
            ("id", "NSL-2026-000001"),
            ("features", "*"),
            ("issued", "2026-09-27"),
            ("updates_until", "2027-09-27"),
        ] {
            d.set(k, v);
        }
        kp.sign_license(&mut d, "root-v1");
        assert_eq!(d.get("key"), Some("root-v1"));
        let text = d.serialize();
        let pk: &'static [u8] = Box::leak(kp.public.to_vec().into_boxed_slice());
        let roots = [RootKey {
            id: "root-v1",
            alg: Alg::Ed25519,
            public: pk,
        }];
        let p = Product {
            id: "nexa-sql",
            build_date: "2026-09-27",
        };
        let v = verify_license(
            &p,
            &roots,
            &crate::ed25519::Ed25519Verifier,
            &text,
            None,
            20_723,
        );
        assert!(matches!(v, Verdict::Licensed(_)), "{v:?}");
        // 두 번 서명해도 sig는 하나(이전 sig 제거 뒤 서명).
        kp.sign_license(&mut d, "root-v1");
        assert_eq!(d.pairs.iter().filter(|(k, _)| k == "sig").count(), 1);
    }

    #[test]
    fn pub_file_and_keys_rs() {
        let kp = Keypair::generate();
        let f = public_key_file("root-v1", &kp.public);
        let (id, pk) = parse_public_key_file(&f).expect("parse");
        assert_eq!(id, "root-v1");
        assert_eq!(pk, kp.public);
        assert!(parse_public_key_file("format=nxl1\n").is_none());
        let src = keys_rs_source(&[(id, pk), ("root-v2".into(), kp.public)]);
        assert!(src.contains("pub const ROOT_KEYS: &[RootKey] = &["));
        assert_eq!(
            src.matches("RootKey {").count(),
            3,
            "struct 정의 1 + 항목 2"
        );
        assert!(src.contains(&format!("0x{:02x}", kp.public[0])));
    }
}
