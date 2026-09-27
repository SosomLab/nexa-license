//! 발급기 E2E — 바이너리를 실제로 실행(docs/23 §5 · 25 §12-8 ②): keygen → issue → verify → reissue → ledger · 오류 종료 코드.

use std::{path::PathBuf, process::Command};

use nexa_license::{base32, format::Doc, request};

fn tool() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nexa-license-tool"))
}
fn tmp() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let d = std::env::temp_dir().join(format!("nlt-e2e-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).expect("tmp");
    d
}
fn req(machine: [u8; 20], name: &str) -> String {
    let mut m = Doc::default();
    m.set("os", "linux");
    m.set("app", "nexa-sql/0.0.1");
    m.set("n", name);
    request::encode(&machine, &m)
}
fn run(c: &mut Command) -> (i32, String, String) {
    let o = c.output().expect("run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn keygen_issue_verify_reissue_ledger() {
    let base = tmp();
    let env_path = base.join("root.key");
    // keygen(암호 = env · 반복 낮춰 빠르게)
    let (rc, out, err) = run(tool()
        .args(["keygen", "--out"])
        .arg(&env_path)
        .args([
            "--id",
            "root-t1",
            "--pass-env",
            "NLT_PASS",
            "--iter",
            "1000",
        ])
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    assert!(out.contains("key id      root-t1"));
    let pub_path = base.join("root.key.pub");
    assert!(pub_path.is_file());
    // 같은 경로 keygen = 거부(덮어쓰기 금지)
    let (rc, _, _) = run(tool()
        .args(["keygen", "--out"])
        .arg(&env_path)
        .args(["--pass-env", "NLT_PASS"])
        .env("NLT_PASS", "x"));
    assert_eq!(rc, 1);
    // rekey: 새 암호 봉투 = 같은 공개키 · 옛 암호로는 못 연다 · 새 암호로 발급 가능
    let env2 = base.join("root2.key");
    let (rc, out, err) = run(tool()
        .args(["rekey", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--new-pass-env",
            "NLT_NEW",
            "--iter",
            "1000",
            "--out",
        ])
        .arg(&env2)
        .env("NLT_PASS", "s3cret")
        .env("NLT_NEW", "n3w-pass"));
    assert_eq!(rc, 0, "{err}");
    let pub1 = std::fs::read_to_string(&pub_path).expect("pub1");
    let pub2 = std::fs::read_to_string(base.join("root2.key.pub")).expect("pub2");
    assert_eq!(pub1, pub2, "같은 키");
    assert!(out.contains("key id      root-t1"));
    let (rc, _, _) = run(tool().args(["decode-request", "x"]).env("NLT_PASS", "x"));
    assert_eq!(rc, 3);
    // keys-rs
    let (rc, out, _) = run(tool().arg("keys-rs").arg(&pub_path));
    assert_eq!(rc, 0);
    assert!(out.contains("id: \"root-t1\"") && out.contains("ROOT_KEYS"));
    // decode-request
    let m1 = [7u8; 20];
    let r1 = req(m1, "홍길동");
    let (rc, out, _) = run(tool().args(["decode-request", &r1]));
    assert_eq!(rc, 0);
    assert!(out.contains(&base32::encode(&m1)) && out.contains("홍길동"));
    let (rc, _, _) = run(tool().args(["decode-request", "NEXAREQ1.ZZ"]));
    assert_eq!(rc, 3, "요청 코드 무효 = 3");
    // issue(user · 기기 1)
    let issued = base.join("issued");
    let (rc, out, err) = run(tool()
        .args(["issue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--request",
            &r1,
            "--kind",
            "user",
            "--licensee",
            "ACME Corp",
            "--email",
            "a@b.c",
            "--tier",
            "pro",
            "--note",
            "order#1",
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    assert!(out.contains("id             NSL-"), "{out}");
    let id = out
        .lines()
        .find_map(|l| l.strip_prefix("id             "))
        .expect("id")
        .trim()
        .to_string();
    let file = issued.join(&id).join("nexa-sql.license");
    assert!(file.is_file());
    assert!(issued.join(&id).join("mail.txt").is_file());
    let text = std::fs::read_to_string(&file).expect("read");
    let d = Doc::parse(&text);
    assert_eq!(d.get("features"), Some("*"));
    assert_eq!(d.get("key"), Some("root-t1"));
    assert!(d.get("sig").is_some());
    assert!(d.get("expires").is_none(), "pro = 영구");
    // 잘못된 암호 = 4 · 아무것도 안 씀
    let (rc, _, _) = run(tool()
        .args(["issue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--request",
            &r1,
            "--kind",
            "user",
            "--licensee",
            "X",
            "--tier",
            "pro",
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "wrong"));
    assert_eq!(rc, 4);
    // verify(--pub · 그 기기) = 0 · 다른 기기 = 1 · 변조 = 1
    let (rc, out, _) = run(tool()
        .arg("verify")
        .arg(&file)
        .arg("--pub")
        .arg(&pub_path)
        .args(["--machine", &base32::encode(&m1)]));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("verdict        licensed"));
    let (rc, out, _) = run(tool()
        .arg("verify")
        .arg(&file)
        .arg("--key")
        .arg(&env_path)
        .args(["--machine", &base32::encode(&[8u8; 20])]));
    assert_eq!(rc, 1);
    assert!(out.contains("invalid(Machine)"), "{out}");
    let tampered = base.join("t.license");
    std::fs::write(&tampered, text.replace("ACME Corp", "EVIL")).expect("w");
    let (rc, out, _) = run(tool()
        .arg("verify")
        .arg(&tampered)
        .arg("--pub")
        .arg(&pub_path));
    assert_eq!(rc, 1);
    assert!(out.contains("invalid(Signature)"));
    // reissue: 기기 추가(5대 안) → v2 · 옛 판 보존 · 두 기기 모두 통과
    let m2 = [9u8; 20];
    let r2 = req(m2, "홍길동 노트북");
    let (rc, out, err) = run(tool()
        .args(["reissue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--id",
            &id,
            "--add-request",
            &r2,
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    assert!(out.contains("version        2"));
    assert!(issued.join(&id).join("nexa-sql.license.v1").is_file());
    for m in [m1, m2] {
        let (rc, _, _) = run(tool()
            .arg("verify")
            .arg(&file)
            .arg("--pub")
            .arg(&pub_path)
            .args(["--machine", &base32::encode(&m)]));
        assert_eq!(rc, 0);
    }
    // ledger: 두 행 · 기기 접두만 · find
    let ledger = std::fs::read_to_string(issued.join("ledger.tsv")).expect("ledger");
    assert_eq!(ledger.lines().count(), 3, "헤더 + 2행:\n{ledger}");
    assert!(
        !ledger.contains(&base32::encode(&m1)),
        "기기 ID 전체가 대장에 없다"
    );
    assert!(ledger.contains(&base32::encode(&m1)[..8]));
    let (rc, out, _) = run(tool()
        .args(["ledger", "find", "acme", "--ledger"])
        .arg(issued.join("ledger.tsv")));
    assert_eq!(rc, 0);
    assert_eq!(out.lines().count(), 3);
    let (rc, _, _) = run(tool()
        .args(["ledger", "find", "nobody", "--ledger"])
        .arg(issued.join("ledger.tsv")));
    assert_eq!(rc, 1);
    // 다음 id = 순번 +1
    let (rc, out, _) = run(tool()
        .args(["issue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--kind",
            "org",
            "--seats",
            "10",
            "--licensee",
            "Org",
            "--tier",
            "org",
            "--no-mail",
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{out}");
    let id2 = out
        .lines()
        .find_map(|l| l.strip_prefix("id             "))
        .expect("id2")
        .trim()
        .to_string();
    assert_ne!(id, id2);
    assert!(id2.ends_with("000002"), "{id2}");
    // 인자 오류 = 2 · 체험 = expires 자동
    let (rc, _, _) = run(tool().args(["issue", "--kind", "user"]));
    assert_eq!(rc, 2);
    let (rc, out, _) = run(tool()
        .args(["issue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--request",
            &r1,
            "--kind",
            "user",
            "--licensee",
            "T",
            "--tier",
            "trial",
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0);
    assert!(
        !out.contains("expires        -"),
        "trial = 만료일 있음:\n{out}"
    );
    let _ = std::fs::remove_dir_all(&base);
}
