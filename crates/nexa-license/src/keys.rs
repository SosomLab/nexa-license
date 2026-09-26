//! SosomLab 루트 **공개키**(검증용 · 공개 무방). 비밀키는 어느 저장소에도 없다 — 발급 PC의 암호 봉투에만(nexa-sql docs/25 §12-6 · docs/91).
//! 회전(D-27): 새 키를 `ROOT_V2` 자리에 더해 두 키를 동시에 검증하다가, 모든 앱이 새 판이 된 뒤 옛 키를 뺀다.

use crate::types::Alg;

/// 루트 키 한 개 — `id`는 라이선스의 `key=` 값과 맞춘다(없으면 같은 `alg`의 모든 키를 시도).
#[derive(Debug, Clone, Copy)]
pub struct RootKey {
    pub id: &'static str,
    pub alg: Alg,
    pub public: &'static [u8],
}

/// 현재 유효한 루트 공개키 목록. **초기 버전 09-27**: `nexa-license-tool keygen`으로 만든 첫 키(발급 PC로 옮긴 뒤 회전 예정 · docs/91).
pub const ROOT_KEYS: &[RootKey] = &[];
