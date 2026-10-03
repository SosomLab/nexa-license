# CLAUDE.md — nexa-license

**Nexa 계열 공용 라이선스 라이브러리.** 설계 SSOT = `../nexa-sql/docs/23-license-activation.md` · `../nexa-sql/docs/25-license-tiers-and-server.md`(§9 저장소 분리 · §10 범용성 — **§10-3 계층이 정본**). 결정 = nexa-sql `docs/10` DR-25 · DR-26 · 열린 D-23 · D-24 · D-40.

- 조직 SosomLab · 개발자 Sangyong Bae · **공개 저장소**(DR-26). 서버·발급기는 비공개 `../nexa-license-server`.
- 형제 clone = `git@kiros33.github.com:SosomLab/<repo>.git`(SSH 별칭). 소비자는 path 의존 `../nexa-license/crates/nexa-license`.
- 라이브러리(`nexa-license`) = 검증 · 요청 코드 · 파일 자리. 발급기(`nexa-license-tool` · keygen/issue/reissue/verify)는 **이 저장소**에 있다(09-27 이후 — 종전 "비공개 서버 저장소에만"은 옛 기록). 루트 **개인키**만 발급 PC 밖으로 나가지 않는다.
- 규약: 라이브러리는 앱을 모른다(`Feature`·문자열·폴더·설정 키 주입 · 오류는 열거형) · 기본 의존 0(feature로만) · 서명 알고리즘은 포트(`alg=ed25519|p256`) · 도메인 상수는 `lib.rs`(계열 공통).
- 문서·커밋 규약 = nexa-sql `docs/16`(Conventional Commits · `git add <파일>`만 · push는 사용자 요청 시). 진행 기록은 **nexa-sql의 journal/STATUS에 남긴다**(이 저장소는 코드 + README).
- 검증 = `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`(CI 3-OS).
