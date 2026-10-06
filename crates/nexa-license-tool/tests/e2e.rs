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
    // 유효기간 기본 3년 · Major 조항 = 요청 앱(nexa-sql/0.0.1)의 Major 0 · updates_until = expires.
    let exp = d.get("expires").expect("expires = 기본 3년");
    assert!(exp > "2028", "{exp}");
    assert_eq!(d.get("updates_until"), Some(exp));
    assert_eq!(d.get("max_major"), Some("0"));
    assert!(d.get("max_version").is_none());
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
    // Major가 바뀐 앱(1.0.0)에서는 outdated · 같은 Major(0.9.9)는 정식.
    let (rc, out, _) = run(tool()
        .arg("verify")
        .arg(&file)
        .arg("--pub")
        .arg(&pub_path)
        .args(["--machine", &base32::encode(&m1), "--app-version", "1.0.0"]));
    assert_eq!(rc, 1);
    assert!(out.contains("outdated"), "{out}");
    let (rc, _, _) = run(tool()
        .arg("verify")
        .arg(&file)
        .arg("--pub")
        .arg(&pub_path)
        .args(["--machine", &base32::encode(&m1), "--app-version", "0.9.9"]));
    assert_eq!(rc, 0);
    // 특정 버전 이후 무효(--max-version) · 영구(--expires none) · Major 제한 없음(none).
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
            "Cut",
            "--tier",
            "pro",
            "--max-version",
            "0.5.0",
            "--expires",
            "none",
            "--max-major",
            "none",
            "--no-mail",
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    assert!(
        out.contains("expires        -")
            && out.contains("max_version    0.5.0")
            && out.contains("max_major      -"),
        "{out}"
    );
    let id_cut = out
        .lines()
        .find_map(|l| l.strip_prefix("id             "))
        .expect("id")
        .trim()
        .to_string();
    let f_cut = issued.join(&id_cut).join("nexa-sql.license");
    let (rc, _, _) = run(tool()
        .arg("verify")
        .arg(&f_cut)
        .arg("--pub")
        .arg(&pub_path)
        .args(["--machine", &base32::encode(&m1), "--app-version", "0.4.9"]));
    assert_eq!(rc, 0, "0.4.9 < 0.5.0 = 정식");
    let (rc, out, _) = run(tool()
        .arg("verify")
        .arg(&f_cut)
        .arg("--pub")
        .arg(&pub_path)
        .args(["--machine", &base32::encode(&m1), "--app-version", "0.5.0"]));
    assert_eq!(rc, 1);
    assert!(out.contains("outdated"), "{out}");
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
    assert_eq!(
        ledger.lines().count(),
        4,
        "헤더 + 3행(발급 · Cut 발급 · 재발급):\n{ledger}"
    );
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
            "--max-major",
            "none",
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
    assert!(id2.ends_with("000003"), "{id2}");
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

/// 정책 전 발급분(만료 없음 · Major 조항 없음)을 같은 ID로 새 정책에 맞추기(09-27): `reissue --renew` → 3년 · max_major = 요청 앱 Major ·
/// mail.txt 다시 · 개별 옵션 · 기존 ID로 `issue` = 거부.
#[test]
fn reissue_renew_applies_current_terms() {
    let base = tmp();
    let key = base.join("root.key");
    let (rc, _, err) = run(tool()
        .args(["keygen", "--out"])
        .arg(&key)
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
    let pubk = base.join("root.key.pub");
    let issued = base.join("issued");
    let m = [5u8; 20];
    let r = req(m, "linux");
    let issue = |extra: &[&str]| {
        run(tool()
            .args(["issue", "--key"])
            .arg(&key)
            .args(["--pass-env", "NLT_PASS", "--request", &r, "--kind", "user"])
            .args(["--licensee", "linux", "--tier", "pro", "--no-mail"])
            .args(extra)
            .arg("--out")
            .arg(&issued)
            .env("NLT_PASS", "s3cret"))
    };
    let reissue = |extra: &[&str]| {
        run(tool()
            .args(["reissue", "--key"])
            .arg(&key)
            .args(["--pass-env", "NLT_PASS", "--id", "NSL-2026-000001"])
            .args(extra)
            .arg("--out")
            .arg(&issued)
            .env("NLT_PASS", "s3cret"))
    };
    // 정책 전 모양: 만료 없음 · Major 조항 없음
    let (rc, out, err) = issue(&[
        "--id",
        "NSL-2026-000001",
        "--expires",
        "none",
        "--updates-until",
        "2027-09-27",
        "--max-major",
        "none",
    ]);
    assert_eq!(rc, 0, "{err}");
    assert!(
        out.contains("expires        -") && out.contains("max_major      -"),
        "{out}"
    );
    // 같은 ID로 issue = 거부(파일·대장 보호)
    let (rc, _, err) = issue(&["--id", "NSL-2026-000001"]);
    assert_eq!(rc, 1);
    assert!(err.contains("reissue"), "{err}");
    // 옵션 없는 reissue = 조항 그대로
    let (rc, out, err) = reissue(&[]);
    assert_eq!(rc, 0, "{err}");
    assert!(out.contains("expires        -"), "{out}");
    let dir = issued.join("NSL-2026-000001");
    assert!(!dir.join("mail.txt").exists(), "조항 그대로 = mail 없음");
    // --renew = 오늘 + 3년 · updates_until = expires · max_major = 대장의 요청 앱 Major(0)
    let (rc, out, err) = reissue(&["--renew"]);
    assert_eq!(rc, 0, "{err}");
    let three = nexa_license::date::format_days(nexa_license::date::add_years(
        nexa_license::date::today_days(),
        3,
    ));
    assert!(out.contains("version        3"), "{out}");
    assert!(out.contains(&format!("expires        {three}")), "{out}");
    assert!(out.contains("max_major      0"), "{out}");
    assert!(dir.join("nexa-sql.license.v2").is_file());
    let mail = std::fs::read_to_string(dir.join("mail.txt")).expect("mail");
    assert!(mail.contains(&three) && mail.contains("0.x"), "{mail}");
    let lic = std::fs::read_to_string(dir.join("nexa-sql.license")).expect("lic");
    assert!(lic.contains("reissued="), "{lic}");
    assert!(lic.contains(&format!("updates_until={three}")), "{lic}");
    // 검증: 0.x = 정식 · 1.0.0 = outdated
    let verify = |ver: &str| {
        run(tool()
            .arg("verify")
            .arg(dir.join("nexa-sql.license"))
            .arg("--pub")
            .arg(&pubk)
            .args(["--machine", &base32::encode(&m), "--app-version", ver]))
    };
    assert_eq!(verify("0.9.0").0, 0);
    assert_eq!(verify("1.0.0").0, 1);
    // 개별 옵션이 앞선다 · 잘못된 값 = 2
    let (rc, out, err) = reissue(&["--max-version", "0.5.0", "--expires", "2030-01-01"]);
    assert_eq!(rc, 0, "{err}");
    assert!(out.contains("expires        2030-01-01") && out.contains("max_version    0.5.0"));
    assert_eq!(verify("0.4.9").0, 0);
    assert_eq!(verify("0.5.0").0, 1);
    assert_eq!(reissue(&["--max-version", "x"]).0, 2);
    assert_eq!(reissue(&["--expires", "2030-13-01"]).0, 2);
    let _ = std::fs::remove_dir_all(&base);
}

/// nexa-dir 제품(nexa-dir3 LIC-170): `--product nexa-dir` → `nexa-dir.license` · ID 접두 `NDL` · mail.txt = GUI 안내(CLI 없음) ·
/// `app=nexa-dir/<ver>` 메타에서 Major 추출(max_major 기본 = 요청 앱 Major) · verify 통과.
#[test]
fn issue_nexa_dir_product_prefix_and_mail() {
    let base = tmp();
    let env_path = base.join("root.key");
    let (rc, _, err) = run(tool()
        .args(["keygen", "--out"])
        .arg(&env_path)
        .args([
            "--id",
            "root-d1",
            "--pass-env",
            "NLT_PASS",
            "--iter",
            "1000",
        ])
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    let mut m = Doc::default();
    m.set("os", "windows");
    m.set("app", "nexa-dir/0.23.0");
    m.set("n", "Dir User");
    let r = request::encode(&[9u8; 20], &m);
    let issued = base.join("issued");
    let (rc, out, err) = run(tool()
        .args(["issue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--request",
            &r,
            "--product",
            "nexa-dir",
            "--kind",
            "user",
            "--licensee",
            "Dir User",
            "--email",
            "d@u.c",
            "--tier",
            "pro",
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    let id = out
        .lines()
        .find_map(|l| l.strip_prefix("id             "))
        .expect("id")
        .trim()
        .to_string();
    assert!(id.starts_with("NDL-"), "제품별 접두: {id}");
    let dir = issued.join(&id);
    let file = dir.join("nexa-dir.license");
    assert!(file.is_file(), "제품 이름 파일");
    let d = Doc::parse(&std::fs::read_to_string(&file).expect("read"));
    assert_eq!(d.get("product"), Some("nexa-dir"));
    assert_eq!(
        d.get("max_major"),
        Some("0"),
        "요청 앱 nexa-dir/0.23.0 → Major 0"
    );
    let mail = std::fs::read_to_string(dir.join("mail.txt")).expect("mail");
    assert!(
        mail.contains("도움말 ▸ 라이선스…") && mail.contains("Help ▸ License…"),
        "{mail}"
    );
    assert!(
        !mail.contains("nsql license"),
        "nexa-dir 안내에 nsql CLI 없음: {mail}"
    );
    // verify = 제품 nexa-dir · 요청 기기 → licensed.
    let (rc, out, err) = run(tool()
        .arg("verify")
        .arg(&file)
        .arg("--pub")
        .arg(base.join("root.key.pub"))
        .args([
            "--product",
            "nexa-dir",
            "--machine",
            &base32::encode(&[9u8; 20]),
        ]));
    assert_eq!(rc, 0, "{err}\n{out}");
    assert!(out.to_ascii_lowercase().contains("licensed"), "{out}");
    // `--id-prefix`는 기본을 이긴다.
    let (rc, out, err) = run(tool()
        .args(["issue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--request",
            &r,
            "--product",
            "nexa-dir",
            "--kind",
            "user",
            "--licensee",
            "Dir User",
            "--tier",
            "pro",
            "--id-prefix",
            "ZZZ",
            "--no-mail",
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    assert!(out.contains("id             ZZZ-"), "{out}");
}

/// 제품 단위 분리(사용자 10-06): 같은 PC라도 nexa-sql · nexa-dir 라이선스는 따로 — 요청 코드의 앱 제품과 `--product`가 다르면 거부 · 생략하면 요청 코드에서 추론.
#[test]
fn issue_is_per_product_and_rejects_foreign_request_codes() {
    let base = tmp();
    let env_path = base.join("root.key");
    let (rc, _, err) = run(tool()
        .args(["keygen", "--out"])
        .arg(&env_path)
        .args([
            "--id",
            "root-p1",
            "--pass-env",
            "NLT_PASS",
            "--iter",
            "1000",
        ])
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 0, "{err}");
    let code = |app: &str| {
        let mut m = Doc::default();
        m.set("os", "windows");
        m.set("app", app);
        request::encode(&[5u8; 20], &m)
    };
    let (sql, dir) = (code("nexa-sql/0.1.2"), code("nexa-dir/0.23.0"));
    let issued = base.join("issued");
    let issue = |extra: &[&str]| {
        run(tool()
            .args(["issue", "--key"])
            .arg(&env_path)
            .args([
                "--pass-env",
                "NLT_PASS",
                "--kind",
                "user",
                "--licensee",
                "Same PC",
                "--tier",
                "pro",
                "--no-mail",
            ])
            .args(extra)
            .arg("--out")
            .arg(&issued)
            .env("NLT_PASS", "s3cret"))
    };
    let id_of = |out: &str| {
        out.lines()
            .find_map(|l| l.strip_prefix("id             "))
            .expect("id")
            .trim()
            .to_string()
    };
    // 다른 제품의 요청 코드 = 3(요청 코드 무효) · 섞인 요청 = 3.
    let (rc, _, err) = issue(&["--request", &dir, "--product", "nexa-sql"]);
    assert_eq!(rc, 3, "{err}");
    assert!(err.contains("nexa-dir"), "{err}");
    let (rc, _, err) = issue(&["--request", &sql, "--request", &code("nexa-dir/0.1.0")]);
    assert_eq!(rc, 3, "{err}");
    // `--product` 생략 = 요청 코드의 제품 · 같은 기기라도 제품마다 다른 ID · 다른 파일.
    let (rc, out, err) = issue(&["--request", &dir]);
    assert_eq!(rc, 0, "{err}");
    let dir_id = id_of(&out);
    assert!(dir_id.starts_with("NDL-"), "{dir_id}");
    assert!(issued.join(&dir_id).join("nexa-dir.license").is_file());
    let (rc, out, err) = issue(&["--request", &sql]);
    assert_eq!(rc, 0, "{err}");
    let sql_id = id_of(&out);
    assert!(sql_id.starts_with("NSL-"), "{sql_id}");
    assert!(issued.join(&sql_id).join("nexa-sql.license").is_file());
    // 재발급 기기 추가도 제품이 맞아야 한다.
    let (rc, _, err) = run(tool()
        .args(["reissue", "--key"])
        .arg(&env_path)
        .args([
            "--pass-env",
            "NLT_PASS",
            "--id",
            &sql_id,
            "--add-request",
            &code("nexa-dir/0.23.0"),
            "--out",
        ])
        .arg(&issued)
        .env("NLT_PASS", "s3cret"));
    assert_eq!(rc, 3, "{err}");
}
