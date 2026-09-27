# nexa-license

Nexa 계열(nexa-sql · nexa-clip · nexa-beep · nexa-dir)과 비공개 인증 서버(`nexa-license-server`)가 함께 쓰는 **라이선스 라이브러리**입니다.
서명된 라이선스/리스 파일 형식, 서명 체인 검증, 기기 ID, 리스 프로토콜을 한 판으로 제공합니다.

> **설계 SSOT는 [SosomLab/nexa-sql](https://github.com/SosomLab/nexa-sql)의 문서입니다.**
> [docs/23 정품 인증·기능 게이트](https://github.com/SosomLab/nexa-sql/blob/main/docs/23-license-activation.md) ·
> [docs/25 라이선스 종류·인증 서버·저장소 분리(§10 범용성 점검)](https://github.com/SosomLab/nexa-sql/blob/main/docs/25-license-tiers-and-server.md).
> 결정은 nexa-sql `docs/10` DR-25 · DR-26.

## 원칙

- **라이브러리는 앱을 모른다.** `Feature` 열거형 · 사용자 문자열 · 저장 폴더 · 설정 키는 소비자가 주입합니다. 오류는 열거형만 돌려줍니다.
- **기본 의존 0.** 서명 어댑터 · 기기 ID · 파일 관리 · 프로토콜은 feature로 켭니다(`ed25519` · `machine-id` · `fs` · `protocol`).
- **한 형식.** 라이선스 · 리스 · 요청 코드 메타 · 서버 메시지가 같은 key=value 문서와 같은 정규화 규칙(정렬 · LF · `sig` 제외)을 씁니다.
- **앱은 검증 전용 · 비밀 없음.** 앱이 켜는 feature에는 공개키와 검증 코드만 들어갑니다. 서명 생성·키 생성·비밀키 봉투는 feature `issuer`(`sign` · `envelope`) 뒤에만 있고 **발급기 `crates/nexa-license-tool`만 켭니다**(09-27 · 인증 서버 보류로 비공개 저장소 대신 여기). 루트 비밀키는 어느 저장소에도 없습니다(발급 PC의 암호 봉투 `nxk1`). 정책의 안전은 코드 비밀이 아니라 비밀키 보관에 있습니다 — 다른 키로 서명한 파일은 공개키만 내장한 앱이 거부합니다(nexa-sql docs/91 §0).

## 계층(docs/25 §10-3)

| 모듈 | feature | 상태 |
|---|---|---|
| `format` key=value 파싱·직렬화·정규화 서명 대상 | 항상 | ✅ |
| `base32` Crockford(요청 코드·기기 코드·서명) | 항상 | ✅ |
| `types` `Product` · `Kind` · `SeatMode` · `Alg` | 항상 | ✅ |
| `date` `YYYY-MM-DD` ↔ 일 수 · `add_years`(외부 crate 0) | 항상 | ✅ 09-27 |
| `version` `MAJOR.MINOR.PATCH` 비교(`max_major`·`max_version` 조항) | 항상 | ✅ 09-27 |
| `verify` 체인 검증(`SigVerifier` 포트 · 기기 일치 · 시각 · **버전 조항 `max_major`/`max_version`** · `Verdict`) | 항상 | ✅ 09-27 |
| `ed25519` dalek 2.x 검증 어댑터 | `ed25519` | ✅ 09-27 |
| `machine` 3-OS 기기 ID(SHA-256[..20] · 도메인 태그) | `machine-id` | ✅ 09-27 |
| `request` 요청 코드 `NEXAREQ1` encode/decode | 항상 | ✅ 09-27 |
| `keys` SosomLab 루트 공개키(`ROOT_KEYS`) | 항상 | ✅ 09-27 `root-v1`(Ed25519 · 공개키 base32 `44W1RFJH…`) — `nexa-license-tool keys-rs`가 생성 · 손으로 고치지 않음 · 회전 = nexa-sql docs/91 §5 |
| `sign` 키 생성 · 문서 서명 · `.pub` 파일 · `keys.rs` 생성 | `issuer` | ✅ 09-27(발급기 전용) |
| `envelope` 비밀키 봉투 `nxk1`(PBKDF2-HMAC-SHA256 600k · HMAC 스트림 · MAC · RFC 벡터 시험) | `issuer` | ✅ 09-27 |
| `fs` 폴더를 받아 라이선스 파일 관리(`Store` · 폴더 순서 · 원자적 쓰기 · 변경 서명) | `fs` | ✅ 09-27 |
| `protocol` 리스 메시지 + 최소 HTTP 프레이밍 | `protocol` | ☐ |

시험 28개 + 발급기 E2E 1(`cargo test --workspace --all-features`) · 3-OS CI. 툴체인은 `rust-toolchain.toml`(stable · rustfmt/clippy)로 고정.

## 발급기 `nexa-license-tool`(nexa-sql docs/25 §12)

```
nexa-license-tool keygen --out root.key --id root-v1            # 봉투(nxk1 · 0600) + root.key.pub · 암호 = 프롬프트 | --pass-env | --pass-stdin
nexa-license-tool keys-rs root.key.pub --out <앱>/crates/nexa-license/src/keys.rs   # 앱에 공개키 내장(회전 = .pub 둘 이상)
nexa-license-tool rekey --key root.key --out root-v1b.key                        # 봉투 암호 변경(같은 키 · 옛 봉투는 백업 뒤 삭제)
nexa-license-tool decode-request NEXAREQ1....                    # 고객 요청 코드 보기
nexa-license-tool issue --key root.key --request NEXAREQ1.... --kind user --licensee "ACME" --tier pro   # ./issued/<id>/nexa-sql.license + mail.txt + ledger.tsv
#   기본(09-27): 유효기간 expires = 발급일 + 3년 · updates_until = expires · max_major = 요청 앱의 Major(Major 바뀌면 무효) · --max-version X.Y.Z = 그 버전부터 무효(Major 무관) · --expires none = 영구
nexa-license-tool reissue --key root.key --id NSL-2026-000001 --add-request NEXAREQ1....                   # 기기 추가(5대) · v2 · 옛 판 보존
nexa-license-tool verify ./issued/NSL-2026-000001/nexa-sql.license --pub root.key.pub --machine <base32>   # 앱과 같은 검증 코드
nexa-license-tool ledger list | find acme
```
종료 코드 0 · 1 · 2(인자) · 3(요청 코드) · 4(봉투 = 암호/손상). 비밀키는 메모리에만 · 대장에는 기기 ID 접두 8자만. 운영 절차 = nexa-sql docs/91.

## 소비자

```toml
# 형제 저장소 path 의존(nexa-ui 방식) — clone: git@github.com:SosomLab/nexa-license.git 을 ../nexa-license 에
nexa-license = { path = "../nexa-license/crates/nexa-license", features = ["ed25519", "machine-id", "fs", "protocol"] }
```

- nexa-sql → `nsql-license`(얇은 층: `Feature` 매핑 · 상태 · 게이트)
- nexa-license-server(비공개) → `nexa-licensed`(서버) · `nexa-license-tool`(발급기 · 서명·키 생성은 그쪽에서 구현) — `protocol`
- nexa-dir2 → `ed25519` feature를 끄고 `alg=p256` CNG 어댑터를 그쪽 저장소에서 구현(외부 crate 0 유지)

## 개발

```
cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features
```

CI는 Windows · macOS · Linux 3-OS. 문서·커밋 규약은 nexa-sql `docs/16`을 따릅니다.

## 라이선스

PolyForm Noncommercial 1.0.0 — [LICENSE.md](LICENSE.md)(영문 정본) · [LICENSE.ko.md](LICENSE.ko.md). 상업적 사용은 별도 유료 라이선스(kiros33@sosomlab.com).
