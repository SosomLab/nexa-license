//! `nexa-license` — Nexa 계열 공용 라이선스 라이브러리.
//!
//! **설계 SSOT는 `SosomLab/nexa-sql`의 `docs/23-license-activation.md`(형식 · 판정)와
//! `docs/25-license-tiers-and-server.md`(티어 · 인증 서버 · 저장소 분리 · §10 범용성 점검)다.** 이 crate는 그 문서의
//! "라이브러리" 칸을 구현한다 — 앱(nexa-sql · nexa-clip · nexa-beep · nexa-dir2)과 비공개 인증 서버가 같은 판을 쓴다.
//!
//! 원칙(docs/25 §10)
//! - **앱을 모른다.** `Feature` 열거형 · 사용자 문자열 · 저장 폴더 규칙 · 설정 키는 호출측이 준다. 오류는 열거형만.
//! - **기본 의존 0.** 서명 어댑터(`ed25519` · dir2용 `p256`은 그쪽 저장소의 CNG 어댑터) · 기기 ID · 파일 · 프로토콜은 feature.
//! - **한 형식.** 라이선스 · 리스 · 요청 코드 메타 · 서버 메시지가 전부 같은 key=value 문서([`format`])와 같은 정규화 규칙을 쓴다.
//! - 제품 구분은 서명 본문의 `product=`가 한다 — 서명 도메인·기기 ID 도메인은 계열 공통(한 루트 키 · 한 PC = 한 기기 코드).
//!
//! - **앱은 검증 전용.** 서명 생성·키 생성·봉투(발급)는 feature `issuer` 뒤에만 있고(`sign` · `envelope`) 발급기 `nexa-license-tool`만 켠다
//!   (09-27 · 서버 보류로 비공개 저장소 대신 이 워크스페이스 `crates/nexa-license-tool`). 정책의 안전은 코드 비밀이 아니라
//!   **루트 비밀키 보관**에 있다 — 공개키만 내장된 앱은 다른 키로 서명한 파일을 거부한다(docs/91 §0).
//!
//! 층(09-27 초기 버전): [`format`] · [`base32`] · [`types`] · [`date`] · [`request`] · [`verify`](SigVerifier 포트 · D-40) · [`keys`](루트 공개키) ·
//! `ed25519`(feature) · `machine`(feature) · [`fs`](feature · 폴더 주입 · 원자적 설치). 다음: `protocol`(리스).

pub mod base32;
pub mod date;
#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "issuer")]
pub mod envelope;
pub mod format;
#[cfg(feature = "fs")]
pub mod fs;
pub mod keys;
#[cfg(feature = "machine-id")]
pub mod machine;
pub mod request;
#[cfg(feature = "issuer")]
pub mod sign;
pub mod types;
pub mod verify;
pub mod version;

/// 라이선스·리스 파일 `format=` 값. 바뀌면 `nxl2`로 올리고 두 판을 동시에 검증한다.
pub const FORMAT: &str = "nxl1";
/// 클라이언트 요청 코드 접두(`NEXAREQ1.<기기 코드>.<메타>`).
pub const REQUEST_PREFIX: &str = "NEXAREQ1";
/// 인증 서버 요청 코드 접두(`NEXASRV1.<서버 공개키>.<메타>`).
pub const SERVER_REQUEST_PREFIX: &str = "NEXASRV1";
/// 기기 ID 해시 도메인 — 계열 공통(제품별로 다르면 사용자가 앱마다 다른 코드를 보내야 한다).
pub const MACHINE_DOMAIN: &str = "nexa/machine-v1";
/// 라이선스 서명 도메인(루트 키 → 라이선스).
pub const LICENSE_DOMAIN: &str = "nexa/license/v1";
/// 리스 서명 도메인(서버 키 → 리스).
pub const LEASE_DOMAIN: &str = "nexa/lease/v1";
