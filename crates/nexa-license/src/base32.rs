//! Crockford base32 — 사람이 복사·이메일로 옮기는 값(요청 코드 · 기기 코드 · 서명)용.
//!
//! base64 대신인 이유(docs/23 §2): `+ / =` 오타 · 줄바꿈 사고가 잦고 대소문자를 구분한다. Crockford는 대소문자 무관 ·
//! `O→0` `I/L→1` 자동 교정 · 구분 `-` 허용. 패딩 없음.

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// 인코딩(대문자 · 패딩 없음). 5바이트 → 8글자.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 8 / 5 + 1);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for &b in bytes {
        acc = (acc << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// 4글자마다 `-`로 묶은 표시형(`6QJ3-KPN0-…`) — 기기 코드 · 라이선스 ID 표시용.
#[must_use]
pub fn encode_grouped(bytes: &[u8], group: usize) -> String {
    let raw = encode(bytes);
    if group == 0 {
        return raw;
    }
    let mut out = String::with_capacity(raw.len() + raw.len() / group);
    for (i, c) in raw.chars().enumerate() {
        if i > 0 && i % group == 0 {
            out.push('-');
        }
        out.push(c);
    }
    out
}

fn value_of(c: char) -> Option<u32> {
    let c = c.to_ascii_uppercase();
    Some(match c {
        '0' | 'O' => 0,
        '1' | 'I' | 'L' => 1,
        '2'..='9' => c as u32 - '0' as u32,
        'A'..='H' => c as u32 - 'A' as u32 + 10,
        'J' | 'K' => c as u32 - 'J' as u32 + 18,
        'M' | 'N' => c as u32 - 'M' as u32 + 20,
        'P'..='T' => c as u32 - 'P' as u32 + 22,
        'V'..='Z' => c as u32 - 'V' as u32 + 27,
        _ => return None,
    })
}

/// 디코딩 — 대소문자 무관 · `-`·공백 무시 · `O/I/L` 교정. 잘못된 글자 = `None`.
/// 끝의 남는 비트(패딩 없음이라 생기는 0~4비트)는 0이어야 한다(잘린 문자열 탐지).
#[must_use]
pub fn decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8 + 1);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in s.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        acc = (acc << 5) | value_of(c)?;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    if bits >= 5 || (acc & ((1 << bits) - 1)) != 0 {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_various_lengths() {
        for n in 0..70usize {
            let bytes: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            let enc = encode(&bytes);
            assert_eq!(decode(&enc).as_deref(), Some(bytes.as_slice()), "n={n}");
            assert_eq!(
                enc.len(),
                n.div_ceil(5) * 8 - ((5 - n % 5) % 5) * 8 / 5,
                "n={n}"
            );
        }
    }

    #[test]
    fn known_vectors() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(&[0]), "00");
        assert_eq!(encode(&[0xFF]), "ZW");
        assert_eq!(encode(b"hello"), "D1JPRV3F");
        assert_eq!(decode("d1jprv3f").as_deref(), Some(&b"hello"[..]));
    }

    #[test]
    fn forgiving_input() {
        let bytes = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let enc = encode_grouped(&bytes, 4);
        assert_eq!(enc.len(), 16 + 3, "16글자 + 구분자 3");
        assert_eq!(enc.matches('-').count(), 3);
        assert!(!enc.ends_with('-'));
        assert_eq!(decode(&enc).as_deref(), Some(&bytes[..]));
        let spaced = enc.replace('-', " ").to_ascii_lowercase();
        assert_eq!(decode(&spaced), decode(&enc.replace('-', "")));
        // O→0 · I/L→1 교정
        assert_eq!(decode("OO"), Some(vec![0]));
        assert_eq!(decode("Il"), decode("11"));
    }

    #[test]
    fn rejects_bad_chars_and_truncation() {
        assert_eq!(decode("U0"), None, "U는 알파벳에 없다");
        assert_eq!(decode(""), Some(vec![]));
        assert_eq!(
            decode("0"),
            None,
            "글자 1개 = 바이트가 안 된다(길이 ≡ 1 mod 8은 불가)"
        );
        assert_eq!(
            decode("D1JPRV3"),
            None,
            "hello에서 한 글자 잘림 → 잔여 비트가 0이 아니다"
        );
        assert_eq!(decode("D1JPRV3F0"), None, "여분 글자");
    }
}
