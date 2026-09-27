//! SosomLab 루트 **공개키**(검증용 · 공개 무방). 비밀키는 어느 저장소에도 없다 — 발급 PC의 암호 봉투에만(nexa-sql docs/25 §12-6 · docs/91).
//! 회전(D-27): 새 키를 다음 자리에 더해 두 키를 동시에 검증하다가, 모든 앱이 새 판이 된 뒤 옛 키를 뺀다.
//! ★ 이 파일은 `nexa-license-tool keys-rs <a.pub> [<b.pub>…]`가 만든다 — 손으로 고치지 말 것.

use crate::types::Alg;

/// 루트 키 한 개 — `id`는 라이선스의 `key=` 값과 맞춘다(없으면 같은 `alg`의 모든 키를 시도).
#[derive(Debug, Clone, Copy)]
pub struct RootKey {
    pub id: &'static str,
    pub alg: Alg,
    pub public: &'static [u8],
}

/// 현재 유효한 루트 공개키 목록.
#[rustfmt::skip] // 생성 파일 — 키 개수와 무관하게 CI `cargo fmt --check`를 통과하도록 모양을 고정한다.
pub const ROOT_KEYS: &[RootKey] = &[
    RootKey {
        id: "root-v1",
        alg: Alg::Ed25519,
        public: &[
            0x21, 0x38, 0x1c, 0x3e, 0x51, 0x40, 0x10, 0xe2,
            0xff, 0xfc, 0x53, 0x46, 0x82, 0xb2, 0x1c, 0x2c,
            0x09, 0xde, 0x3f, 0x85, 0xe2, 0xa9, 0x80, 0x80,
            0xf0, 0xc9, 0x42, 0x8d, 0x65, 0xab, 0x84, 0x06,
        ],
    },
];
