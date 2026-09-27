//! 라이선스 검증(nexa-sql docs/23 §1-4 · §5) — **검증 전용**. 서명 알고리즘은 포트([`SigVerifier`] · D-40)로 받는다.
//!
//! 순서: 형식 → 제품 → 중복 키 → 서명(루트 공개키 중 하나) → 기기 일치 → 기한(영구 = 빌드일 vs `updates_until` · 만료형 = 오늘 vs `expires`).
//! `features=*` 는 전 기능(초기 버전 D-48: 이후 추가된 기능도 포함) · 그 밖은 낱개 이름 집합.

use std::collections::BTreeSet;

use crate::{
    base32, date,
    format::Doc,
    keys::RootKey,
    types::{Alg, Kind, Product, SeatMode},
    FORMAT, LICENSE_DOMAIN,
};

/// 서명 검증 포트 — 호출측이 어댑터를 준다(`ed25519` feature = [`crate::ed25519::Ed25519Verifier`]).
pub trait SigVerifier {
    /// `pubkey`로 `msg`의 `sig`가 맞는가.
    fn verify(&self, alg: Alg, pubkey: &[u8], msg: &[u8], sig: &[u8]) -> bool;
}

/// 검증을 통과한 라이선스의 내용.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct License {
    pub id: String,
    pub licensee: String,
    pub kind: Kind,
    /// 표시용 등급 이름(`tier=` · 판정은 `features`가 한다).
    pub tier: String,
    pub features: BTreeSet<String>,
    /// 기기 ID(base32 그대로) — 비어 있으면 기기 묶음 없음(Team/Org 파일).
    pub machines: Vec<String>,
    pub issued: String,
    pub updates_until: String,
    pub expires: String,
    pub seats: u32,
    pub seat_mode: Option<SeatMode>,
    /// 서명한 루트 키 id(`key=` · 없으면 빈 문자열).
    pub key_id: String,
    /// `max_major=`(없으면 제한 없음) — 앱 Major가 이보다 크면 Outdated(Major 바뀌면 무효 · 기본 발급값 = 요청 앱의 Major).
    pub max_major: Option<u64>,
    /// `max_version=`(없으면 제한 없음) — 앱 버전이 이 값 **이상**이면 Outdated(Major 무관 · 특정 버전 이후 무효 조항).
    pub max_version: String,
}

impl License {
    /// 기능 허용 — `*`면 전부.
    #[must_use]
    pub fn allows(&self, feature: &str) -> bool {
        self.features.contains("*") || self.features.contains(feature)
    }
}

/// 무효 사유.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    Format,
    Product,
    DuplicateKeys,
    NoRootKey,
    Signature,
    Machine,
    Malformed,
}

/// 판정.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Licensed(License),
    /// 영구 모델: 이 빌드가 `updates_until` 뒤에 나왔다 — 그 전 판만 정식(features ∅로 취급).
    Outdated(License),
    /// 만료형: 오늘이 `expires` 뒤.
    Expired(License),
    Invalid(Invalid),
}

/// 검증. `machine` = 이 PC의 기기 ID 20B(못 얻으면 None → 기기 묶음 파일은 `Invalid(Machine)`) · `today` = 1970-01-01부터 일수.
#[must_use]
pub fn verify_license(
    product: &Product,
    roots: &[RootKey],
    verifier: &dyn SigVerifier,
    text: &str,
    machine: Option<&[u8]>,
    today: i64,
) -> Verdict {
    let doc = Doc::parse(text);
    if doc.get("format") != Some(FORMAT) {
        return Verdict::Invalid(Invalid::Format);
    }
    if !doc.get("product").is_some_and(|p| product.accepts(p)) {
        return Verdict::Invalid(Invalid::Product);
    }
    if doc.has_duplicate_keys() {
        return Verdict::Invalid(Invalid::DuplicateKeys);
    }
    let Some(sig) = doc.get("sig").and_then(base32::decode) else {
        return Verdict::Invalid(Invalid::Malformed);
    };
    let alg = doc.get("alg").map_or(Some(Alg::Ed25519), Alg::parse);
    let Some(alg) = alg else {
        return Verdict::Invalid(Invalid::Malformed);
    };
    let key_id = doc.get("key").unwrap_or("");
    let msg = doc.canonical(LICENSE_DOMAIN);
    let candidates: Vec<&RootKey> = roots
        .iter()
        .filter(|k| k.alg == alg && (key_id.is_empty() || k.id == key_id))
        .collect();
    if candidates.is_empty() {
        return Verdict::Invalid(Invalid::NoRootKey);
    }
    if !candidates
        .iter()
        .any(|k| verifier.verify(alg, k.public, &msg, &sig))
    {
        return Verdict::Invalid(Invalid::Signature);
    }
    let machines: Vec<String> = doc
        .get_numbered("machine")
        .iter()
        .map(|s| s.to_string())
        .collect();
    if !machines.is_empty() {
        let Some(mine) = machine else {
            return Verdict::Invalid(Invalid::Machine);
        };
        let mine = base32::encode(mine);
        if !machines.iter().any(|m| m.eq_ignore_ascii_case(&mine)) {
            return Verdict::Invalid(Invalid::Machine);
        }
    }
    let kind = doc.get("kind").map_or(Some(Kind::User), Kind::parse);
    let Some(kind) = kind else {
        return Verdict::Invalid(Invalid::Malformed);
    };
    let features: BTreeSet<String> = doc
        .get("features")
        .unwrap_or("")
        .split(',')
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect();
    let lic = License {
        id: doc.get("id").unwrap_or("").to_string(),
        licensee: doc.get("licensee").unwrap_or("").to_string(),
        kind,
        tier: doc.get("tier").unwrap_or("").to_string(),
        features,
        machines,
        issued: doc.get("issued").unwrap_or("").to_string(),
        updates_until: doc.get("updates_until").unwrap_or("").to_string(),
        expires: doc.get("expires").unwrap_or("").to_string(),
        seats: doc.get("seats").and_then(|s| s.parse().ok()).unwrap_or(0),
        seat_mode: doc.get("seat_mode").and_then(SeatMode::parse),
        key_id: key_id.to_string(),
        max_major: doc.get("max_major").and_then(|s| s.trim().parse().ok()),
        max_version: doc.get("max_version").unwrap_or("").trim().to_string(),
    };
    if let (Some(until), Some(build)) = (
        date::parse_days(&lic.updates_until),
        date::parse_days(product.build_date),
    ) {
        if build > until {
            return Verdict::Outdated(lic);
        }
    }
    // 버전 조항(09-27): Major 초과 · 특정 버전 이후 = 이 판을 덮지 않는 라이선스(Outdated · 이전 판은 그대로 정식).
    if let (Some(mm), Some(app)) = (lic.max_major, crate::version::major(product.version)) {
        if app > mm {
            return Verdict::Outdated(lic);
        }
    }
    if !lic.max_version.is_empty() && crate::version::at_or_after(product.version, &lic.max_version)
    {
        return Verdict::Outdated(lic);
    }
    if let Some(exp) = date::parse_days(&lic.expires) {
        if today > exp {
            return Verdict::Expired(lic);
        }
    }
    Verdict::Licensed(lic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    struct DalekV;
    impl SigVerifier for DalekV {
        fn verify(&self, _alg: Alg, pubkey: &[u8], msg: &[u8], sig: &[u8]) -> bool {
            let Ok(pk) =
                ed25519_dalek::VerifyingKey::from_bytes(pubkey.try_into().unwrap_or(&[0u8; 32]))
            else {
                return false;
            };
            let Ok(s) = ed25519_dalek::Signature::from_slice(sig) else {
                return false;
            };
            pk.verify_strict(msg, &s).is_ok()
        }
    }
    const P: Product = Product {
        id: "nexa-sql",
        build_date: "2026-09-27",
        version: "1.2.3",
    };
    fn keypair() -> (SigningKey, Vec<u8>) {
        let sk = SigningKey::generate(&mut rand_core::OsRng);
        let pk = sk.verifying_key().to_bytes().to_vec();
        (sk, pk)
    }
    fn signed(sk: &SigningKey, extra: &[(&str, &str)]) -> String {
        let mut d = Doc::default();
        for (k, v) in [
            ("format", "nxl1"),
            ("product", "nexa-sql"),
            ("id", "NSL-2026-000001"),
            ("licensee", "ACME"),
            ("kind", "user"),
            ("tier", "pro"),
            ("features", "*"),
            ("machine", &base32::encode(&[9u8; 20])),
            ("issued", "2026-09-27"),
            ("updates_until", "2027-09-27"),
        ] {
            d.set(k, v);
        }
        for (k, v) in extra {
            d.set(k, v);
        }
        let sig = sk.sign(&d.canonical(LICENSE_DOMAIN));
        d.set("sig", &base32::encode(&sig.to_bytes()));
        d.serialize()
    }
    fn roots(pk: &'static [u8]) -> Vec<RootKey> {
        vec![RootKey {
            id: "root-v1",
            alg: Alg::Ed25519,
            public: pk,
        }]
    }
    fn leak(v: Vec<u8>) -> &'static [u8] {
        Box::leak(v.into_boxed_slice())
    }
    const TODAY: i64 = 20_723; // 2026-09-27

    #[test]
    fn happy_path_and_normalization() {
        let (sk, pk) = keypair();
        let r = roots(leak(pk));
        let text = signed(&sk, &[]);
        match verify_license(&P, &r, &DalekV, &text, Some(&[9u8; 20]), TODAY) {
            Verdict::Licensed(l) => {
                assert!(l.allows("anything") && l.allows("export-xlsx"));
                assert_eq!(l.licensee, "ACME");
                assert_eq!(l.kind, Kind::User);
            }
            v => panic!("{v:?}"),
        }
        // 줄 순서 뒤집기 · CRLF · 주석 · 공백 → 그대로 통과(정규화).
        let mut lines: Vec<&str> = text.lines().collect();
        lines.reverse();
        let messy = format!("# hello\r\n{}\r\n", lines.join("\r\n  "));
        assert!(matches!(
            verify_license(&P, &r, &DalekV, &messy, Some(&[9u8; 20]), TODAY),
            Verdict::Licensed(_)
        ));
    }
    #[test]
    fn tamper_machine_key_product_and_dates() {
        let (sk, pk) = keypair();
        let r = roots(leak(pk));
        let text = signed(&sk, &[]);
        let tampered = text.replace("ACME", "ACMF");
        assert_eq!(
            verify_license(&P, &r, &DalekV, &tampered, Some(&[9u8; 20]), TODAY),
            Verdict::Invalid(Invalid::Signature)
        );
        assert_eq!(
            verify_license(&P, &r, &DalekV, &text, Some(&[8u8; 20]), TODAY),
            Verdict::Invalid(Invalid::Machine)
        );
        assert_eq!(
            verify_license(&P, &r, &DalekV, &text, None, TODAY),
            Verdict::Invalid(Invalid::Machine)
        );
        let (_, other) = keypair();
        assert_eq!(
            verify_license(
                &P,
                &roots(leak(other)),
                &DalekV,
                &text,
                Some(&[9u8; 20]),
                TODAY
            ),
            Verdict::Invalid(Invalid::Signature)
        );
        assert_eq!(
            verify_license(&P, &[], &DalekV, &text, Some(&[9u8; 20]), TODAY),
            Verdict::Invalid(Invalid::NoRootKey)
        );
        let clip = Product {
            id: "nexa-clip",
            build_date: "2026-09-27",
            version: "1.2.3",
        };
        assert_eq!(
            verify_license(&clip, &r, &DalekV, &text, Some(&[9u8; 20]), TODAY),
            Verdict::Invalid(Invalid::Product)
        );
        assert!(matches!(
            verify_license(
                &P,
                &r,
                &DalekV,
                &signed(&sk, &[("updates_until", "2026-01-01")]),
                Some(&[9u8; 20]),
                TODAY
            ),
            Verdict::Outdated(_)
        ));
        assert!(matches!(
            verify_license(
                &P,
                &r,
                &DalekV,
                &signed(&sk, &[("expires", "2026-09-26")]),
                Some(&[9u8; 20]),
                TODAY
            ),
            Verdict::Expired(_)
        ));
        assert!(
            matches!(
                verify_license(
                    &P,
                    &r,
                    &DalekV,
                    &signed(&sk, &[("expires", "2026-09-27")]),
                    Some(&[9u8; 20]),
                    TODAY
                ),
                Verdict::Licensed(_)
            ),
            "만료일 당일은 유효"
        );
        let dup = format!("{text}licensee=EVIL\n");
        assert_eq!(
            verify_license(&P, &r, &DalekV, &dup, Some(&[9u8; 20]), TODAY),
            Verdict::Invalid(Invalid::DuplicateKeys)
        );
        assert_eq!(
            verify_license(
                &P,
                &r,
                &DalekV,
                &text.replace("nxl1", "nxl9"),
                Some(&[9u8; 20]),
                TODAY
            ),
            Verdict::Invalid(Invalid::Format)
        );
    }
    #[test]
    fn features_list_and_org_without_machine() {
        let (sk, pk) = keypair();
        let r = roots(leak(pk));
        let text = signed(
            &sk,
            &[("features", "export-xlsx, ssh-tunnel"), ("key", "root-v1")],
        );
        let Verdict::Licensed(l) = verify_license(&P, &r, &DalekV, &text, Some(&[9u8; 20]), TODAY)
        else {
            panic!()
        };
        assert!(l.allows("ssh-tunnel") && !l.allows("compare"));
        // 기기 없는 조직 파일 = 기기 검사 생략.
        let mut d = Doc::parse(&signed(
            &sk,
            &[("kind", "org"), ("seats", "10"), ("seat_mode", "named")],
        ));
        d.pairs.retain(|(k, _)| k != "machine" && k != "sig");
        let sig = sk.sign(&d.canonical(LICENSE_DOMAIN));
        d.set("sig", &base32::encode(&sig.to_bytes()));
        let Verdict::Licensed(l) = verify_license(&P, &r, &DalekV, &d.serialize(), None, TODAY)
        else {
            panic!()
        };
        assert_eq!(
            (l.kind, l.seats, l.seat_mode),
            (Kind::Org, 10, Some(SeatMode::Named))
        );
    }

    /// 버전 조항(09-27): `max_major` 초과 · `max_version` 이상 = Outdated · 경계는 관대(같은 Major · 그 버전 미만 = 정식).
    #[test]
    fn version_clauses_major_and_cutoff() {
        let (sk, pk) = keypair();
        let r = roots(leak(pk));
        let ok = |extra: &[(&str, &str)]| {
            verify_license(
                &P,
                &r,
                &DalekV,
                &signed(&sk, extra),
                Some(&[9u8; 20]),
                TODAY,
            )
        };
        assert!(
            matches!(ok(&[("max_major", "1")]), Verdict::Licensed(_)),
            "같은 Major"
        );
        assert!(
            matches!(ok(&[("max_major", "0")]), Verdict::Outdated(_)),
            "Major 초과"
        );
        assert!(
            matches!(ok(&[("max_version", "1.3.0")]), Verdict::Licensed(_)),
            "1.2.3 < 1.3.0"
        );
        assert!(
            matches!(ok(&[("max_version", "1.2.3")]), Verdict::Outdated(_)),
            "그 버전부터 무효"
        );
        assert!(matches!(
            ok(&[("max_version", "1.2.0")]),
            Verdict::Outdated(_)
        ));
        assert!(
            matches!(ok(&[("max_version", "junk")]), Verdict::Licensed(_)),
            "못 읽으면 관대"
        );
        if let Verdict::Licensed(l) = ok(&[("max_major", "1"), ("max_version", "2.0.0")]) {
            assert_eq!(l.max_major, Some(1));
            assert_eq!(l.max_version, "2.0.0");
        } else {
            panic!();
        }
    }
}
