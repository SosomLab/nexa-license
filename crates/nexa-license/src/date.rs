//! 날짜 `YYYY-MM-DD` — 라이선스의 `issued`·`updates_until`·`expires`는 날짜 단위뿐이라 시간대·시각은 다루지 않는다(외부 crate 0).

/// `YYYY-MM-DD` → 1970-01-01부터의 일수(그레고리력 · 유효하지 않으면 None).
#[must_use]
pub fn parse_days(s: &str) -> Option<i64> {
    let s = s.trim();
    let mut it = s.splitn(3, '-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// 일수 → `YYYY-MM-DD`.
#[must_use]
pub fn format_days(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 오늘(UTC 기준 날짜 · 시스템 시계) — 만료형 판정에만 쓴다(영구 모델은 빌드일이 기준 · docs/23 §1-4).
#[must_use]
pub fn today_days() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86_400)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
    }
}

// Howard Hinnant의 days_from_civil / civil_from_days(공개 도메인 알고리즘).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_validation() {
        for s in [
            "1970-01-01",
            "2000-02-29",
            "2026-09-27",
            "2027-12-31",
            "1999-03-01",
        ] {
            let d = parse_days(s).expect(s);
            assert_eq!(format_days(d), s);
        }
        assert_eq!(parse_days("1970-01-01"), Some(0));
        assert_eq!(
            parse_days("2026-09-27").expect("27") - parse_days("2026-09-26").expect("26"),
            1
        );
        assert!(parse_days("2026-02-30").is_none());
        assert!(parse_days("2026-13-01").is_none());
        assert!(parse_days("abc").is_none());
        assert!(today_days() > parse_days("2026-01-01").expect("2026"));
    }
}
