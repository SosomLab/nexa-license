//! 비밀키 봉투(feature `issuer` · nexa-sql docs/25 §12-6 ⓐ) — **암호 기반 · 3-OS 동일 · 외부 crate 0**(sha2만).
//!
//! ```text
//! format=nxk1 · kdf=pbkdf2-sha256 · iter=600000 · salt=<base32 16B> · nonce=<base32 16B> · ct=<base32> · mac=<base32 32B> · key=<id> · public=<base32>
//! ```
//! 키 유도 = PBKDF2-HMAC-SHA256(암호 · salt · iter · 64B) → 앞 32B = 스트림 키 · 뒤 32B = MAC 키.
//! 스트림 = HMAC-SHA256(스트림 키 · nonce ‖ 카운터 u32 BE) 블록 이어붙임(비밀키 32B = 블록 1개) · ct = 평문 XOR 스트림.
//! mac = HMAC-SHA256(MAC 키 · "nxk1" ‖ salt ‖ nonce ‖ ct) — 봉투 손상·암호 오류 = mac 불일치(평문을 내주지 않는다).
//! `public=`은 봉투 밖(암호 없이 읽을 수 있게 — `keys-rs`·`verify`가 쓴다) · 열 때 비밀키와 짝인지 확인한다.

use sha2::{Digest, Sha256};

use crate::{base32, format::Doc};

/// 기본 반복 수(2026 · OWASP 권고 600k).
pub const DEFAULT_ITER: u32 = 600_000;

/// HMAC-SHA256(RFC 2104).
#[must_use]
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let ipad: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(data);
    let ih = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(ih);
    outer.finalize().into()
}

/// PBKDF2-HMAC-SHA256(RFC 8018) — `dk_len` 바이트.
#[must_use]
pub fn pbkdf2_sha256(password: &[u8], salt: &[u8], iter: u32, dk_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(dk_len);
    let mut block: u32 = 1;
    while out.len() < dk_len {
        let mut msg = salt.to_vec();
        msg.extend_from_slice(&block.to_be_bytes());
        let mut u = hmac_sha256(password, &msg);
        let mut t = u;
        for _ in 1..iter.max(1) {
            u = hmac_sha256(password, &u);
            for (a, b) in t.iter_mut().zip(u.iter()) {
                *a ^= b;
            }
        }
        out.extend_from_slice(&t);
        block += 1;
    }
    out.truncate(dk_len);
    out
}

fn keystream(stream_key: &[u8], nonce: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 32);
    let mut ctr: u32 = 0;
    while out.len() < len {
        let mut msg = nonce.to_vec();
        msg.extend_from_slice(&ctr.to_be_bytes());
        out.extend_from_slice(&hmac_sha256(stream_key, &msg));
        ctr += 1;
    }
    out.truncate(len);
    out
}

fn mac_input(salt: &[u8], nonce: &[u8], ct: &[u8]) -> Vec<u8> {
    let mut m = b"nxk1".to_vec();
    m.extend_from_slice(salt);
    m.extend_from_slice(nonce);
    m.extend_from_slice(ct);
    m
}

/// 봉투 오류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeError {
    Format,
    /// 암호가 틀렸거나 봉투가 손상됐다(둘을 구분하지 않는다).
    Mac,
    /// 열린 비밀키가 `public=`과 짝이 아니다.
    KeyMismatch,
}

/// 봉투에 넣는다. `salt`·`nonce`는 호출측이 OS 난수 16B씩 준다(시험 재현성).
#[must_use]
pub fn seal(
    secret: &[u8; 32],
    public: &[u8; 32],
    key_id: &str,
    password: &[u8],
    iter: u32,
    salt: &[u8; 16],
    nonce: &[u8; 16],
) -> String {
    let dk = pbkdf2_sha256(password, salt, iter, 64);
    let ks = keystream(&dk[..32], nonce, 32);
    let ct: Vec<u8> = secret.iter().zip(ks.iter()).map(|(p, k)| p ^ k).collect();
    let mac = hmac_sha256(&dk[32..], &mac_input(salt, nonce, &ct));
    let mut d = Doc::default();
    d.set("format", "nxk1");
    d.set("key", key_id);
    d.set("alg", "ed25519");
    d.set("public", &base32::encode(public));
    d.set("kdf", "pbkdf2-sha256");
    d.set("iter", &iter.to_string());
    d.set("salt", &base32::encode(salt));
    d.set("nonce", &base32::encode(nonce));
    d.set("ct", &base32::encode(&ct));
    d.set("mac", &base32::encode(&mac));
    d.serialize()
}

/// 봉투 밖 정보(암호 없이): key id · 공개키.
#[must_use]
pub fn peek(text: &str) -> Option<(String, [u8; 32])> {
    let d = Doc::parse(text);
    if d.get("format") != Some("nxk1") {
        return None;
    }
    let pk: [u8; 32] = base32::decode(d.get("public")?)?.try_into().ok()?;
    Some((d.get("key").unwrap_or("").to_string(), pk))
}

/// 봉투를 연다 → 비밀키 32B(공개키 짝 확인 포함).
pub fn open(text: &str, password: &[u8]) -> Result<[u8; 32], EnvelopeError> {
    let d = Doc::parse(text);
    if d.get("format") != Some("nxk1") || d.get("kdf") != Some("pbkdf2-sha256") {
        return Err(EnvelopeError::Format);
    }
    let field = |k: &str| base32::decode(d.get(k).unwrap_or("")).ok_or(EnvelopeError::Format);
    let iter: u32 = d
        .get("iter")
        .and_then(|s| s.parse().ok())
        .ok_or(EnvelopeError::Format)?;
    let salt = field("salt")?;
    let nonce = field("nonce")?;
    let ct = field("ct")?;
    let mac = field("mac")?;
    let public: [u8; 32] = field("public")?
        .try_into()
        .map_err(|_| EnvelopeError::Format)?;
    if ct.len() != 32 || salt.len() != 16 || nonce.len() != 16 {
        return Err(EnvelopeError::Format);
    }
    let dk = pbkdf2_sha256(password, &salt, iter, 64);
    let want = hmac_sha256(&dk[32..], &mac_input(&salt, &nonce, &ct));
    // 상수 시간 비교(길이 같음).
    let diff = want
        .iter()
        .zip(mac.iter())
        .fold(mac.len() ^ 32, |acc, (a, b)| acc | usize::from(a ^ b));
    if diff != 0 {
        return Err(EnvelopeError::Mac);
    }
    let ks = keystream(&dk[..32], &nonce, 32);
    let mut secret = [0u8; 32];
    for (i, (c, k)) in ct.iter().zip(ks.iter()).enumerate() {
        secret[i] = c ^ k;
    }
    let kp = crate::sign::Keypair::from_secret(secret);
    if kp.public != public {
        return Err(EnvelopeError::KeyMismatch);
    }
    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn hmac_rfc4231_case2() {
        let m = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&m),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        // 64B 초과 키(case 6 · 131×0xaa).
        let key = [0xaau8; 131];
        let m = hmac_sha256(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        assert_eq!(
            hex(&m),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn pbkdf2_rfc7914_vector() {
        let dk = pbkdf2_sha256(b"passwd", b"salt", 1, 64);
        assert_eq!(hex(&dk), "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783");
        let dk = pbkdf2_sha256(b"Password", b"NaCl", 80_000, 64);
        assert_eq!(hex(&dk), "4ddcd8f60b98be21830cee5ef22701f9641a4418d04c0414aeff08876b34ab56a1d425a1225833549adb841b51c9b3176a272bdebba1d078478f62b397f33c8d");
    }

    #[test]
    fn seal_open_wrong_password_and_tamper() {
        let kp = crate::sign::Keypair::generate();
        let env = seal(
            &kp.secret,
            &kp.public,
            "root-v1",
            b"correct horse",
            1_000,
            &[1u8; 16],
            &[2u8; 16],
        );
        assert_eq!(open(&env, b"correct horse").expect("open"), kp.secret);
        assert_eq!(
            open(&env, b"wrong").expect_err("must fail"),
            EnvelopeError::Mac
        );
        let (id, pk) = peek(&env).expect("peek");
        assert_eq!((id.as_str(), pk), ("root-v1", kp.public));
        // ct 한 글자 변조 = Mac.
        let ct = Doc::parse(&env).get("ct").expect("ct").to_string();
        let mut chars: Vec<char> = ct.chars().collect();
        chars[0] = if chars[0] == 'A' { 'B' } else { 'A' };
        let bad: String = chars.into_iter().collect();
        let tampered = env.replace(&ct, &bad);
        assert_eq!(
            open(&tampered, b"correct horse").expect_err("must fail"),
            EnvelopeError::Mac
        );
        // public을 다른 키로 바꾸면(mac은 public을 덮지 않음) KeyMismatch로 걸린다.
        let other = crate::sign::Keypair::generate();
        let swapped = env.replace(&base32::encode(&kp.public), &base32::encode(&other.public));
        assert_eq!(
            open(&swapped, b"correct horse").expect_err("must fail"),
            EnvelopeError::KeyMismatch
        );
        assert_eq!(
            open("hello", b"x").expect_err("must fail"),
            EnvelopeError::Format
        );
        // 같은 입력 = 같은 봉투(재현) · 다른 nonce = 다른 ct.
        let env2 = seal(
            &kp.secret,
            &kp.public,
            "root-v1",
            b"correct horse",
            1_000,
            &[1u8; 16],
            &[2u8; 16],
        );
        assert_eq!(env, env2);
        let env3 = seal(
            &kp.secret,
            &kp.public,
            "root-v1",
            b"correct horse",
            1_000,
            &[1u8; 16],
            &[3u8; 16],
        );
        assert_ne!(env, env3);
    }
}
