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

/// 제품별 라이선스 ID 접두(대장 `next_id` · nexa-dir3 LIC-166): nexa-sql `NSL` · nexa-dir `NDL` · 그 외 = 제품 머리글자 대문자 3자.
pub(crate) fn id_prefix_for(product: &str) -> String {
    match product {
        "nexa-sql" => "NSL".to_string(),
        "nexa-dir" => "NDL".to_string(),
        other => {
            let mut s: String = other
                .split(['-', '_'])
                .filter_map(|w| w.chars().next())
                .map(|c| c.to_ascii_uppercase())
                .collect();
            s.truncate(3);
            if s.len() < 3 {
                s.push('L');
            }
            s
        }
    }
}

/// 제품별 적용 안내(한 · 영) — CLI가 없는 제품(nexa-dir)은 GUI 경로만 · nexa-beep은 GUI + `--license install`(nexa-dir3 LIC-165 · 종전 `nsql license …` 하드코딩이 틀린 안내였다).
fn apply_steps(product: &str) -> (String, String) {
    match product {
        "nexa-dir" => (
            format!(
                "적용 방법(3줄):\n\
                 1) 첨부한 {product}.license 파일을 저장합니다.\n\
                 2) Nexa Dir ▸ 도움말 ▸ 라이선스… ▸ [라이선스 파일 열기…]에서 그 파일을 고릅니다.\n\
                 3) 같은 창의 상태 줄이 licensed 로 바뀌면 끝입니다.\n\
                 \n\
                 PC를 추가·교체하려면 그 PC에서 도움말 ▸ 라이선스… ▸ [요청 코드 복사]로 만든 한 줄 코드를 보내 주세요(연 5회 재발급).\n"
            ),
            format!(
                "How to apply: 1) save the attached {product}.license  2) Nexa Dir ▸ Help ▸ License… ▸ [Open license file…]\n\
                 3) the status line in that window turns to licensed. To add or replace a PC, send the one-line code from Help ▸ License… ▸ [Copy request code] on that PC.\n"
            ),
        ),
        // nexa-beep(10-09 · beep 설정 개편 docs/50 P1-b): GUI = 도움말 ▸ 라이선스… · CLI = `nexa-beep --license install`.
        "nexa-beep" => (
            format!(
                "적용 방법(3줄):\n\
                 1) 첨부한 {product}.license 파일을 저장합니다.\n\
                 2) Nexa Beep ▸ 도움말 ▸ 라이선스… ▸ [라이선스 파일 열기…]에서 그 파일을 고릅니다.\n\
                    (터미널: nexa-beep --license install <저장한 파일>)\n\
                 3) 같은 창의 상태 줄이 licensed 로 바뀌면 끝입니다.\n\
                 \n\
                 PC를 추가·교체하려면 그 PC에서 도움말 ▸ 라이선스… ▸ [요청 코드 복사]로 만든 한 줄 코드를 보내 주세요(연 5회 재발급).\n"
            ),
            format!(
                "How to apply: 1) save the attached {product}.license  2) Nexa Beep ▸ Help ▸ License… ▸ [Open license file…]\n\
                 (terminal: `nexa-beep --license install <file>`)  3) the status line in that window turns to licensed.\n\
                 To add or replace a PC, send the one-line code from Help ▸ License… ▸ [Copy request code] on that PC.\n"
            ),
        ),
        // nexa-clip(10-10 · clip 설정 개편 = beep docs/50 동일 절차): GUI = 설정 ▸ 정보 ▸ 라이선스… · CLI = `nexa-clip --license install`.
        "nexa-clip" => (
            format!(
                "적용 방법(3줄):\n\
                 1) 첨부한 {product}.license 파일을 저장합니다.\n\
                 2) Nexa Clip ▸ 설정 ▸ 정보 ▸ 라이선스… ▸ [라이선스 파일 열기…]에서 그 파일을 고릅니다.\n\
                    (터미널: nexa-clip --license install <저장한 파일>)\n\
                 3) 같은 창의 상태 줄이 licensed 로 바뀌면 끝입니다.\n\
                 \n\
                 PC를 추가·교체하려면 그 PC에서 설정 ▸ 정보 ▸ 라이선스… ▸ [요청 코드 복사]로 만든 한 줄 코드를 보내 주세요(연 5회 재발급).\n"
            ),
            format!(
                "How to apply: 1) save the attached {product}.license  2) Nexa Clip ▸ Settings ▸ About ▸ License… ▸ [Open license file…]\n\
                 (terminal: `nexa-clip --license install <file>`)  3) the status line in that window turns to licensed.\n\
                 To add or replace a PC, send the one-line code from Settings ▸ About ▸ License… ▸ [Copy request code] on that PC.\n"
            ),
        ),
        _ => (
            format!(
                "적용 방법(3줄):\n\
                 1) 첨부한 {product}.license 파일을 저장합니다.\n\
                 2) 터미널: nsql license install <저장한 파일>   — 또는 앱 설정 ▸ 라이선스 ▸ 파일 열기\n\
                 3) nsql license status 로 licensed 를 확인합니다.\n\
                 \n\
                 PC를 추가·교체하려면 그 PC에서 `nsql license request` 로 만든 한 줄 코드를 보내 주세요(연 5회 재발급).\n"
            ),
            format!(
                "How to apply: 1) save the attached {product}.license  2) run `nsql license install <file>` (or Settings ▸ License ▸ Open file)\n\
                 3) check with `nsql license status`. To add or replace a PC, send the one-line code from `nsql license request` on that PC.\n"
            ),
        ),
    }
}

/// 고객 전달용 본문(한/영 · docs/25 §12-4 · 적용 안내는 제품별).
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
    let (ko, en) = apply_steps(product);
    format!(
        "{licensee} 님, {product} 라이선스({id})를 보내 드립니다.\n\
         유효기간: {term}{major}(그 뒤 판은 새 라이선스가 필요합니다 · 이전 판은 계속 정식).\n\
         \n\
         {ko}\
         \n\
         ---\n\
         Dear {licensee}, attached is your {product} license ({id}).\n\
         {en}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_prefix_per_product() {
        assert_eq!(id_prefix_for("nexa-sql"), "NSL");
        assert_eq!(id_prefix_for("nexa-dir"), "NDL");
        assert_eq!(id_prefix_for("nexa-clip"), "NCL");
        assert_eq!(id_prefix_for("nexa-beep"), "NBL", "자동 머리글자(N·B + L)");
        assert_eq!(id_prefix_for("x"), "XL");
    }

    #[test]
    fn mail_text_branches_by_product() {
        let dir = mail_text("홍길동", "nexa-dir", "NDL-2026-000001", "2029-10-03", "0");
        assert!(dir.contains("도움말 ▸ 라이선스…") && dir.contains("Help ▸ License…"));
        assert!(
            !dir.contains("nsql license"),
            "dir 제품에 없는 CLI를 안내하지 않는다"
        );
        assert!(dir.contains("nexa-dir.license") && dir.contains("2029-10-03까지"));
        let beep = mail_text("홍길동", "nexa-beep", "NBL-2026-000001", "2029-10-09", "");
        assert!(
            beep.contains("Nexa Beep ▸ 도움말 ▸ 라이선스…")
                && beep.contains("Nexa Beep ▸ Help ▸ License…")
        );
        assert!(beep.contains("nexa-beep --license install"));
        assert!(
            !beep.contains("nsql license"),
            "beep 제품에 nexa-sql CLI를 안내하지 않는다"
        );
        assert!(beep.contains("nexa-beep.license") && beep.contains("NBL-2026-000001"));
        let clip = mail_text("홍길동", "nexa-clip", "NCL-2026-000001", "2029-10-10", "");
        assert!(
            clip.contains("Nexa Clip ▸ 설정 ▸ 정보 ▸ 라이선스…")
                && clip.contains("Nexa Clip ▸ Settings ▸ About ▸ License…")
        );
        assert!(clip.contains("nexa-clip --license install"));
        assert!(
            !clip.contains("nsql license"),
            "clip 제품에 nexa-sql CLI를 안내하지 않는다"
        );
        assert!(clip.contains("nexa-clip.license") && clip.contains("NCL-2026-000001"));
        let sql = mail_text("ACME", "nexa-sql", "NSL-2026-000001", "", "");
        assert!(sql.contains("nsql license install") && sql.contains("무기한"));
    }
}
