//! 3-OS 기기 ID(docs/23 §1-1 · D-24): OS 기기 식별자 **원문은 절대 내보내지 않고** `SHA-256(MACHINE_DOMAIN ‖ 원문)[..20]`만.
//! Windows = `HKLM\\SOFTWARE\\Microsoft\\Cryptography\\MachineGuid` · macOS = `IOPlatformUUID` · Linux = `/etc/machine-id`(없으면 dbus).
//! 외부 crate 0 — Windows·macOS는 OS 도구(`reg` · `ioreg`)의 출력을 읽는다.

use crate::MACHINE_DOMAIN;
use sha2::{Digest, Sha256};

/// 이 PC의 기기 ID 20바이트 — 원천을 못 읽으면 None(앱은 "요청 코드를 만들 수 없음"으로 안내).
#[must_use]
pub fn machine_id() -> Option<Vec<u8>> {
    raw_identifier().map(|raw| hash_identifier(&raw))
}

/// 원문 → 기기 ID(순수 · 시험용 공개).
#[must_use]
pub fn hash_identifier(raw: &str) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(MACHINE_DOMAIN.as_bytes());
    h.update(b"\0");
    h.update(raw.trim().as_bytes());
    h.finalize()[..20].to_vec()
}

#[cfg(target_os = "linux")]
fn raw_identifier() -> Option<String> {
    let read = |p: &str| {
        std::fs::read_to_string(p)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    read("/etc/machine-id").or_else(|| read("/var/lib/dbus/machine-id"))
}

#[cfg(target_os = "macos")]
fn raw_identifier() -> Option<String> {
    let out = std::process::Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("IOPlatformUUID"))?;
    let v = line.split('"').nth(3)?; // "IOPlatformUUID" = "XXXX"
    (!v.is_empty()).then(|| v.to_string())
}

#[cfg(target_os = "windows")]
fn raw_identifier() -> Option<String> {
    // GUI 앱(windows subsystem)에서 콘솔 프로그램을 띄우면 창이 잠깐 뜬다 → CREATE_NO_WINDOW(nexa-sql 102차 win 09-27 지적).
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("MachineGuid"))?;
    let v = line.split_whitespace().last()?;
    (v.len() >= 32).then(|| v.to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn raw_identifier() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_is_stable_20_bytes_and_domain_separated() {
        let a = hash_identifier("abc-123");
        assert_eq!(a.len(), 20);
        assert_eq!(a, hash_identifier("  abc-123\n"), "공백·개행 무시");
        assert_ne!(a, hash_identifier("abc-124"));
        let plain = Sha256::digest(b"abc-123");
        assert_ne!(&plain[..20], &a[..], "도메인 태그가 섞인다");
    }
    #[test]
    fn this_machine_yields_an_id_or_none() {
        if let Some(id) = machine_id() {
            assert_eq!(id.len(), 20);
            assert_eq!(id, machine_id().expect("second call"), "두 번 호출 동일");
        }
    }
}
