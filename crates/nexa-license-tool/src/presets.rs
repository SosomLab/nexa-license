//! 제품별 tier 프리셋(nexa-sql docs/25 §13-3 · D-41 · D-48). **초기 판 = `features=*`**(D-48: 이후 추가된 기능도 포함 · 낱개 구분은 새 발급분부터).
//! 낱개가 필요하면 `--features a,b`로 직접 준다(파일 형식은 그대로).

/// 체험 기간(D-28).
pub(crate) const TRIAL_DAYS: i64 = 14;
/// 기본 유효기간(년 · 사용자 09-27 "기본 3년") — `expires`·`updates_until` 기본.
pub(crate) const TERM_YEARS: i64 = 3;

pub(crate) fn features_for(product: &str, tier: &str) -> Option<String> {
    let _ = product; // 지금은 전 제품 같은 규칙(nexa-sql · nexa-clip · nexa-beep · nexa-dir).
    match tier {
        "trial" | "pro" | "org" | "team" => Some("*".to_string()),
        _ => None,
    }
}

/// 고객 전달용 본문(한/영 · docs/25 §12-4).
pub(crate) fn mail_text(
    licensee: &str,
    product: &str,
    id: &str,
    expires: &str,
    max_major: &str,
) -> String {
    let term = if expires.is_empty() {
        "무기한".to_string()
    } else {
        format!("{expires}까지")
    };
    let major = if max_major.is_empty() {
        String::new()
    } else {
        format!(" · 메이저 버전 {max_major}.x까지")
    };
    format!(
        "{licensee} 님, {product} 라이선스({id})를 보내 드립니다.\n\
         유효기간: {term}{major}(그 뒤 판은 새 라이선스가 필요합니다 · 이전 판은 계속 정식).\n\
         \n\
         적용 방법(3줄):\n\
         1) 첨부한 {product}.license 파일을 저장합니다.\n\
         2) 터미널: nsql license install <저장한 파일>   — 또는 앱 설정 ▸ 라이선스 ▸ 파일 열기\n\
         3) nsql license status 로 licensed 를 확인합니다.\n\
         \n\
         PC를 추가·교체하려면 그 PC에서 `nsql license request` 로 만든 한 줄 코드를 보내 주세요(연 5회 재발급).\n\
         \n\
         ---\n\
         Dear {licensee}, attached is your {product} license ({id}).\n\
         How to apply: 1) save the attached {product}.license  2) run `nsql license install <file>` (or Settings ▸ License ▸ Open file)\n\
         3) check with `nsql license status`. To add or replace a PC, send the one-line code from `nsql license request` on that PC.\n"
    )
}
