//! 앱 버전 비교(외부 crate 0 · `MAJOR.MINOR.PATCH` 숫자만 · 뒤에 붙은 `-pre`/`+meta`는 무시).
//! 라이선스의 `max_major=`(Major가 바뀌면 무효 · 기본) · `max_version=`(그 버전부터 무효 · Major 무관 · 선택) 판정에 쓴다(nexa-sql 09-27).

/// `"1.4.2-beta"` → `(1, 4, 2)`. 숫자가 아니면 None.
#[must_use]
pub fn parse(s: &str) -> Option<(u64, u64, u64)> {
    let core = s.trim().split(['-', '+']).next()?;
    let mut it = core.split('.');
    let major = it.next()?.trim().parse().ok()?;
    let minor = it.next().map_or(Some(0), |m| m.trim().parse().ok())?;
    let patch = it.next().map_or(Some(0), |p| p.trim().parse().ok())?;
    Some((major, minor, patch))
}

#[must_use]
pub fn major(s: &str) -> Option<u64> {
    parse(s).map(|v| v.0)
}

/// `app`이 `bound` **이상**인가(= `max_version` 조항에 걸리는가). 둘 중 하나라도 못 읽으면 `false`(관대).
#[must_use]
pub fn at_or_after(app: &str, bound: &str) -> bool {
    match (parse(app), parse(bound)) {
        (Some(a), Some(b)) => a >= b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_and_compare() {
        assert_eq!(parse("1.4.2"), Some((1, 4, 2)));
        assert_eq!(parse("2"), Some((2, 0, 0)));
        assert_eq!(parse("0.0.1-beta+7"), Some((0, 0, 1)));
        assert_eq!(parse("x.y"), None);
        assert_eq!(major("3.9.9"), Some(3));
        assert!(at_or_after("1.4.0", "1.4.0"));
        assert!(at_or_after("1.10.0", "1.9.9"));
        assert!(!at_or_after("1.3.9", "1.4.0"));
        assert!(!at_or_after("garbage", "1.0.0"), "못 읽으면 관대");
    }
}
