//! 공용 타입 — 제품 기술자 · 라이선스 종류 · 좌석 모드 · 서명 알고리즘. 전부 앱 무관(docs/25 §10-2).

use std::fmt;

/// 제품 기술자 — 호출측(앱·서버·발급기)이 상수로 넘긴다. 라이브러리에 제품 이름이 박히지 않는 유일한 장치.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Product {
    /// `product=` 값 · 설정 폴더 하위 이름과 같은 소문자 id(`nexa-sql` · `nexa-clip` · `nexa-beep` · `nexa-dir`).
    pub id: &'static str,
    /// 빌드 날짜 `YYYY-MM-DD`(각 앱 `build.rs`가 박는다) — 영구 라이선스의 `updates_until` 판정 기준.
    pub build_date: &'static str,
}

impl Product {
    /// 라이선스 파일 이름(`<id>.license`).
    #[must_use]
    pub fn license_file_name(&self) -> String {
        format!("{}.license", self.id)
    }

    /// 리스 파일 이름(`<id>.lease`).
    #[must_use]
    pub fn lease_file_name(&self) -> String {
        format!("{}.lease", self.id)
    }

    /// `product=` 값이 이 제품을 허용하는가 — 정확히 같거나, 쉼표 목록에 포함되거나, `*`(전 제품 번들).
    #[must_use]
    pub fn accepts(&self, product_field: &str) -> bool {
        product_field
            .split(',')
            .map(str::trim)
            .any(|p| p == "*" || p == self.id)
    }
}

/// 라이선스 종류(`kind=` · docs/25 §2-1). 없으면 `User`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    Device,
    #[default]
    User,
    TeamSeat,
    Org,
}

impl Kind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Device => "device",
            Kind::User => "user",
            Kind::TeamSeat => "team-seat",
            Kind::Org => "org",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Kind> {
        match s.trim() {
            "device" => Some(Kind::Device),
            "user" | "" => Some(Kind::User),
            "team-seat" => Some(Kind::TeamSeat),
            "org" => Some(Kind::Org),
            _ => None,
        }
    }

    /// 파일에 나열할 수 있는 기기 수 상한(DR-26: Device 1 · User 5 · Team 좌석 = User와 같음 · Org는 리스가 정한다).
    #[must_use]
    pub const fn max_machines(self) -> usize {
        match self {
            Kind::Device => 1,
            Kind::User | Kind::TeamSeat => 5,
            Kind::Org => 0,
        }
    }
}

/// 조직 좌석 모드(`seat_mode=` · 조직 라이선스에 고정).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeatMode {
    #[default]
    Named,
    Device,
    Concurrent,
}

impl SeatMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SeatMode::Named => "named",
            SeatMode::Device => "device",
            SeatMode::Concurrent => "concurrent",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<SeatMode> {
        match s.trim() {
            "named" | "" => Some(SeatMode::Named),
            "device" => Some(SeatMode::Device),
            "concurrent" => Some(SeatMode::Concurrent),
            _ => None,
        }
    }
}

/// 서명 알고리즘(`alg=` · D-40 포트). 라이브러리는 어댑터를 호출측에서 받는다 — dalek은 `ed25519` feature, dir2는 CNG P-256.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alg {
    #[default]
    Ed25519,
    P256,
}

impl Alg {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Alg::Ed25519 => "ed25519",
            Alg::P256 => "p256",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Alg> {
        match s.trim() {
            "ed25519" | "" => Some(Alg::Ed25519),
            "p256" => Some(Alg::P256),
            _ => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl fmt::Display for SeatMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
impl fmt::Display for Alg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQL: Product = Product {
        id: "nexa-sql",
        build_date: "2026-09-14",
    };

    #[test]
    fn product_names_and_bundles() {
        assert_eq!(SQL.license_file_name(), "nexa-sql.license");
        assert_eq!(SQL.lease_file_name(), "nexa-sql.lease");
        assert!(SQL.accepts("nexa-sql"));
        assert!(SQL.accepts("nexa-clip, nexa-sql"));
        assert!(SQL.accepts("*"));
        assert!(!SQL.accepts("nexa-clip"));
        assert!(!SQL.accepts("nexa-sql-pro"));
    }

    #[test]
    fn enums_roundtrip_with_defaults() {
        for k in [Kind::Device, Kind::User, Kind::TeamSeat, Kind::Org] {
            assert_eq!(Kind::parse(k.as_str()), Some(k));
        }
        assert_eq!(Kind::parse(""), Some(Kind::User));
        assert_eq!(Kind::parse("site"), None);
        assert_eq!(Kind::Device.max_machines(), 1);
        assert_eq!(Kind::User.max_machines(), 5);
        for m in [SeatMode::Named, SeatMode::Device, SeatMode::Concurrent] {
            assert_eq!(SeatMode::parse(m.as_str()), Some(m));
        }
        for a in [Alg::Ed25519, Alg::P256] {
            assert_eq!(Alg::parse(a.as_str()), Some(a));
        }
        assert_eq!(Alg::parse(""), Some(Alg::Ed25519));
        assert_eq!(format!("{}", Kind::TeamSeat), "team-seat");
    }
}
