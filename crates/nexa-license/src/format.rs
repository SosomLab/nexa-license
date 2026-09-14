//! key=value 문서 — 라이선스 · 리스 · 서버 메시지의 공통 형식(nexa-sql docs/23 §2-1).
//!
//! 자체 구현인 이유: 소비자마다 `nexa-conf`를 따로 vendored(beep · clip)하거나 아예 안 쓴다(dir2) — 어느 한 판에 의존하면
//! 트리가 겹친다. 형식이 단순해 파서·직렬화·정규화가 100줄이면 끝난다(docs/25 §10-2 #3).
//!
//! 규칙
//! - 한 줄 = `key=value`. `#` 주석 · 빈 줄 · `=` 없는 줄은 건너뛴다. CRLF 허용. 키·값 양끝 공백 제거.
//! - **정규화 서명 대상** = `domain` ‖ `\n` ‖ (`sig` 제외 전 쌍을 `(key, value)` 순 정렬 · `key=value\n` 결합).
//!   줄 순서 · 공백 · 줄끝 형식이 달라도 같은 바이트 — 이메일·복사를 거쳐도 검증이 깨지지 않는다.
//! - 값에 개행은 없다(한 줄 = 한 키 불변식). 직렬화는 `\n` 개행 · 마지막 줄 개행.

/// 파싱된 문서. 쌍은 파일 순서를 보존한다(정규화는 [`Doc::canonical`]이 따로 정렬).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Doc {
    pub pairs: Vec<(String, String)>,
}

/// 서명 줄 키 — 정규화 대상에서 제외된다.
pub const SIG_KEY: &str = "sig";

impl Doc {
    /// 관용 파싱 — 실패하지 않는다(손상 줄은 건너뛴다 · 유효성은 상위가 판단).
    #[must_use]
    pub fn parse(text: &str) -> Doc {
        let mut d = Doc::default();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            let Some((k, v)) = t.split_once('=') else {
                continue;
            };
            let k = k.trim();
            if k.is_empty() {
                continue;
            }
            d.pairs.push((k.to_string(), v.trim().to_string()));
        }
        d
    }

    /// 첫 값.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// `key` · `key.2` · `key.3` … 순서대로(다중 기기 `machine=`/`machine.2=` 규칙).
    #[must_use]
    pub fn get_numbered(&self, key: &str) -> Vec<&str> {
        let mut out: Vec<(u32, &str)> = self
            .pairs
            .iter()
            .filter_map(|(k, v)| {
                if k == key {
                    Some((1, v.as_str()))
                } else {
                    let rest = k.strip_prefix(key)?.strip_prefix('.')?;
                    rest.parse::<u32>().ok().map(|n| (n, v.as_str()))
                }
            })
            .collect();
        out.sort_by_key(|(n, _)| *n);
        out.into_iter().map(|(_, v)| v).collect()
    }

    /// 같은 키가 두 번 이상 있는가(검증측은 거부한다 — 어느 값이 서명됐는지 모호).
    #[must_use]
    pub fn has_duplicate_keys(&self) -> bool {
        let mut keys: Vec<&str> = self.pairs.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        keys.windows(2).any(|w| w[0] == w[1])
    }

    /// 값 설정(있으면 첫 것을 바꾸고, 없으면 끝에 붙인다).
    pub fn set(&mut self, key: &str, value: &str) {
        match self.pairs.iter_mut().find(|(k, _)| k == key) {
            Some(p) => p.1 = value.to_string(),
            None => self.pairs.push((key.to_string(), value.to_string())),
        }
    }

    /// 정규화 서명 대상 바이트(`sig` 제외 · 정렬 · LF).
    #[must_use]
    pub fn canonical(&self, domain: &str) -> Vec<u8> {
        let mut rows: Vec<(&str, &str)> = self
            .pairs
            .iter()
            .filter(|(k, _)| k != SIG_KEY)
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        rows.sort_unstable();
        let mut out = Vec::with_capacity(domain.len() + 1 + rows.len() * 24);
        out.extend_from_slice(domain.as_bytes());
        out.push(b'\n');
        for (k, v) in rows {
            out.extend_from_slice(k.as_bytes());
            out.push(b'=');
            out.extend_from_slice(v.as_bytes());
            out.push(b'\n');
        }
        out
    }

    /// 파일 표현(순서 보존 · LF · 값의 개행은 공백으로 — 한 줄 = 한 키).
    #[must_use]
    pub fn serialize(&self) -> String {
        let mut s = String::new();
        for (k, v) in &self.pairs {
            s.push_str(k);
            s.push('=');
            s.extend(
                v.chars()
                    .map(|c| if c == '\n' || c == '\r' { ' ' } else { c }),
            );
            s.push('\n');
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Nexa license — do not edit\r\nformat=nxl1\r\nproduct=nexa-sql\nmachine=AAAA\nmachine.2=BBBB\n\nbroken line\n  tier = pro  \nsig=ZZZZ\n";

    #[test]
    fn parse_is_lenient_and_trims() {
        let d = Doc::parse(SAMPLE);
        assert_eq!(d.get("format"), Some("nxl1"));
        assert_eq!(d.get("tier"), Some("pro"));
        assert_eq!(d.get("nope"), None);
        assert_eq!(d.pairs.len(), 6);
        assert_eq!(d.get_numbered("machine"), vec!["AAAA", "BBBB"]);
        assert!(!d.has_duplicate_keys());
    }

    #[test]
    fn canonical_ignores_order_crlf_comments_and_sig() {
        let a = Doc::parse(SAMPLE);
        let b = Doc::parse("tier=pro\nmachine.2=BBBB\nproduct=nexa-sql\nsig=DIFFERENT\nmachine=AAAA\nformat=nxl1\n");
        assert_eq!(
            a.canonical("nexa/license/v1"),
            b.canonical("nexa/license/v1")
        );
        assert_ne!(a.canonical("nexa/license/v1"), a.canonical("nexa/lease/v1"));
        let s = String::from_utf8(a.canonical("d")).expect("utf8");
        assert_eq!(
            s,
            "d\nformat=nxl1\nmachine=AAAA\nmachine.2=BBBB\nproduct=nexa-sql\ntier=pro\n"
        );
    }

    #[test]
    fn value_change_changes_canonical() {
        let mut a = Doc::parse(SAMPLE);
        let before = a.canonical("d");
        a.set("tier", "team");
        assert_ne!(before, a.canonical("d"));
        assert_eq!(a.get("tier"), Some("team"));
    }

    #[test]
    fn serialize_roundtrips_and_keeps_one_line_per_key() {
        let mut d = Doc::parse(SAMPLE);
        d.set("licensee", "ACME\nCorp");
        let text = d.serialize();
        assert!(text.contains("licensee=ACME Corp\n"));
        let again = Doc::parse(&text);
        assert_eq!(again.get("licensee"), Some("ACME Corp"));
        let twice = Doc::parse(&again.serialize());
        assert_eq!(again.pairs, twice.pairs);
        assert_eq!(again.canonical("d"), twice.canonical("d"));
    }

    #[test]
    fn duplicates_detected() {
        let d = Doc::parse("a=1\nb=2\na=3\n");
        assert!(d.has_duplicate_keys());
        assert_eq!(d.get("a"), Some("1"));
    }

    #[test]
    fn numbered_sorts_numerically() {
        let d = Doc::parse("machine.10=J\nmachine.2=B\nmachine=A\nmachine.x=bad\nmachinery=no\n");
        assert_eq!(d.get_numbered("machine"), vec!["A", "B", "J"]);
    }
}
