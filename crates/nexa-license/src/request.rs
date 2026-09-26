//! 요청 코드 `NEXAREQ1.<base32(기기 ID 20B)>.<base32(kv 메타)>` — 사용자가 복사해 이메일에 붙이는 한 줄(docs/23 §2-2).
//! 발급기는 기기 ID만 서명 대상에 넣고, 메타(os·app·d·n)는 표시·대장용이다.

use crate::{base32, format::Doc, REQUEST_PREFIX};

/// 디코드된 요청.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// 기기 ID 20바이트.
    pub machine: Vec<u8>,
    /// 메타 key=value(`os` · `app` · `d` · `n` · `e` …) — 없으면 빈 문서.
    pub meta: Doc,
}

/// 요청 코드 만들기(앱 쪽). `meta`는 `Doc` 그대로 직렬화해 base32로 싼다(값에 `.`·개행이 있어도 안전).
#[must_use]
pub fn encode(machine: &[u8], meta: &Doc) -> String {
    let m = base32::encode(machine);
    let meta_s = meta.serialize();
    if meta_s.is_empty() {
        format!("{REQUEST_PREFIX}.{m}")
    } else {
        format!("{REQUEST_PREFIX}.{m}.{}", base32::encode(meta_s.as_bytes()))
    }
}

/// 요청 코드 풀기(발급기 쪽). 접두·길이가 틀리면 None.
#[must_use]
pub fn decode(code: &str) -> Option<Request> {
    let code = code.trim();
    let mut parts = code.splitn(3, '.');
    if parts.next()? != REQUEST_PREFIX {
        return None;
    }
    let machine = base32::decode(parts.next()?)?;
    if machine.len() != 20 {
        return None;
    }
    let meta = match parts.next() {
        Some(m) if !m.is_empty() => Doc::parse(&String::from_utf8(base32::decode(m)?).ok()?),
        _ => Doc::default(),
    };
    Some(Request { machine, meta })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_with_and_without_meta() {
        let id = [7u8; 20];
        let mut meta = Doc::default();
        meta.set("os", "linux");
        meta.set("app", "0.0.1");
        meta.set("d", "2026-09-27");
        meta.set("n", "홍길동 · dots.in.name");
        let code = encode(&id, &meta);
        assert!(code.starts_with("NEXAREQ1."));
        let r = decode(&code).expect("decode");
        assert_eq!(r.machine, id);
        assert_eq!(r.meta.get("n"), Some("홍길동 · dots.in.name"));
        let bare = encode(&id, &Doc::default());
        assert_eq!(bare.matches('.').count(), 1);
        assert_eq!(decode(&bare).expect("bare").meta.pairs.len(), 0);
        assert!(decode("NEXASRV1.AAAA").is_none());
        assert!(decode("NEXAREQ1.AAAA").is_none(), "20바이트가 아니면 거부");
        assert!(decode(&format!(" {code} \n")).is_some(), "복사 공백 허용");
    }
}
