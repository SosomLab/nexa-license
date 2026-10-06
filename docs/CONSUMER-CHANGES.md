# CONSUMER-CHANGES — 소비자(nexa-sql · nexa-dir3 · 발급 도구 사용자)가 알아야 할 변경과 검증 상태

> **규칙**(사용자 2026-10-03 지시 "ui·license를 고치면 함께 쓰는 프로그램이 변경을 인식하고 검증할 수 있도록 기록을 남긴다"):
> 1. nexa-license를 고치는 **모든 커밋은 이 표에 한 줄을 더한다**(최신 위).
> 2. **종류**를 정직하게 적는다 — `추가` / **`동작 변경`**(검증 결과 · 파일 형식 · 기기 ID · 발급 출력이 달라짐) / `수정` / `문서`.
> 3. 라이브러리(`crates/nexa-license`)의 `동작 변경`·`수정`은 **소비자 시험까지**(nexa-sql `nsql-license` · nexa-dir3 `ndir-license` — `cargo test` · 별도 target-dir). 발급 도구(`crates/nexa-license-tool`) 변경은 도구 E2E + 발급물 확인. 못 한 검증은 **`미검증`**.
> 4. 소비자 세션은 pull 뒤 이 파일의 `동작 변경`·`미검증` 행부터 확인하고 상태 칸을 갱신한다.
> 검증 상태 어휘: `영향 없음(근거)` · `check` · `시험`(소비자 `cargo test`) · `실기`(앱에서 설치·표시 확인) · `미검증`.

| 커밋 | 크레이트 | 변경 요약 | 종류 | nexa-sql 영향 · 상태 | nexa-dir3 영향 · 상태 | 발급 도구 사용자 영향 · 상태 | 검증 방법 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 10-06(feat/per-product-issue) | nexa-license-tool | **제품 단위 발급**(사용자 10-06 "Nexa Dir과 Nexa SQL은 따로 발급 · 제품 단위로 파일 분리"): `issue`/`reissue --add-request`가 요청 코드 메타 `app=<제품>/…`이 `--product`(재발급 = 기존 파일 `product=`)와 다르면 **거부(종료 3)** · 한 `issue`에 제품이 섞인 요청 코드도 거부 · `--product` 생략 = 요청 코드의 앱 제품(없으면 종전대로 `nexa-sql`). 라이선스 형식·검증·기기 ID는 그대로 | 도구 = **동작 변경**(잘못된 제품 조합 발급 불가 · 기본 제품 추론) · 라이브러리 = 변경 없음 | 영향 없음(라이브러리 코드 변화 없음 · `app=nexa-sql/…` 요청은 종전과 같은 결과) | 영향 없음(라이브러리) · `app=nexa-dir/…` 요청을 `--product` 없이 내면 이제 `nexa-dir`·`NDL`로 나온다(종전 = 잘못된 `nexa-sql`) | 다른 제품 요청 코드를 섞던 발급은 실패로 바뀜 · 상태 = `cargo test --workspace` 통과(e2e 4 · 새 시험 `issue_is_per_product_and_rejects_foreign_request_codes`) | 같은 기기의 sql·dir 요청 코드로 각각 발급 → NSL·NDL · 파일 2개 · `--request <dir> --product nexa-sql` = 3 |
| 6e68aba(10-03) | nexa-license-tool · nexa-license | 발급 도구 `presets::mail_text`가 **제품별 적용 안내**를 쓴다(nexa-dir = 도움말 ▸ 라이선스… GUI 안내 · CLI 안내 제거) · 라이브러리는 **문서 주석만**(소비자 목록 nexa-dir2 → nexa-dir3 · p256 계획 폐기 문구) | 도구 = **동작 변경**(발급 메일 문구) · 라이브러리 = 문서 | 영향 없음(라이브러리 코드 변화 없음 · 주석만) · 도구 메일의 nexa-sql 분기는 시험 `mail_text_branches_by_product`로 고정 | 영향 없음(라이브러리) · 안내 문구가 dir3 실제 진입점(도움말 ▸ 라이선스…)과 맞는지 = **실기 미검증** | nexa-dir 발급 메일 문구가 바뀜 · 상태 = nexa-license `cargo test --workspace` **29 통과**(10-03 · 별도 target-dir · `mail_text_branches_by_product` · e2e 포함) | `nexa-license-tool issue … --product nexa-dir` 출력 메일에 "도움말 ▸ 라이선스…" 안내가 있는지 · nexa-sql 제품 메일이 전과 같은지 |

> 이전 변경(09-27 이전: rekey · reissue --renew · 유효기간 3년 · 버전 조항 · 루트 공개키 root-v1 · `reg query` CREATE_NO_WINDOW 등)은 이 파일 신설 전이라 소급하지 않았다 — 필요하면 `git log`로 확인하고, 다시 건드릴 때 이 표에 올린다.

## 검증 기록(소비자 세션이 덧붙임)

| 날짜 | 소비자 | 커밋 | 한 것 | 결과 |
| --- | --- | --- | --- | --- |
| 10-03 | nexa-license(보조 세션) | 6e68aba | `cargo test --workspace --target-dir target/dir3check` | 29 통과 · 실패 0 |
