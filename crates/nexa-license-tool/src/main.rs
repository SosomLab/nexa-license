//! `nexa-license-tool` — SosomLab **오프라인 발급기**(nexa-sql docs/25 §12 · T-241). 대화식 프롬프트 없이 스크립트 가능 · 종료 코드로 결과.
//!
//! ```text
//! nexa-license-tool keygen  --out <봉투> [--id root-v1] [--pass-env NAME | --pass-stdin] [--iter 600000]
//! nexa-license-tool keys-rs <a.pub> [<b.pub>…] [--out keys.rs]            # 앱 라이브러리 keys.rs 본문(회전 = 둘 이상)
//! nexa-license-tool decode-request <NEXAREQ1.…>                          # 기기 코드 · 메타 보기
//! nexa-license-tool issue   --key <봉투> [--pass-env NAME|--pass-stdin] --request <코드> [--request …] --kind device|user|team-seat|org
//!                           --licensee "<이름>" [--email <e>] (--tier trial|pro|org | --features a,b) [--seats N] [--seat-mode named|device|concurrent]
//!                           [--updates-until YYYY-MM-DD] [--expires YYYY-MM-DD] [--max-major N] [--id NSL-2026-000001] [--product nexa-sql]
//!                           [--out ./issued] [--ledger ./issued/ledger.tsv] [--note "…"] [--no-mail]
//! nexa-license-tool reissue --key <봉투> [--pass-…] --id <ID> [--add-request <코드>]… [--drop-machine <base32 접두>]… [--out …] [--ledger …] [--note "…"]
//! nexa-license-tool verify  <파일> (--pub <a.pub>… | --key <봉투>) [--machine <base32>] [--build-date YYYY-MM-DD] [--product nexa-sql]
//! nexa-license-tool ledger  list | find <text> [--ledger …]
//! ```
//! 종료 코드: 0 · 1(실패) · 2(인자) · 3(요청 코드 무효) · 4(키 봉투 실패 = 암호·손상).
//! 비밀키는 봉투에서 풀어 **메모리에만** · 대장에는 기기 ID **접두 8자**만(docs/25 §12-4).

mod ledger;
mod presets;

use std::{
    fs, io,
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

use nexa_license::{
    base32, date, envelope,
    format::Doc,
    keys::RootKey,
    request,
    sign::{self, Keypair},
    types::{Alg, Kind, Product, SeatMode},
    verify::{verify_license, Verdict},
    FORMAT,
};
use rand_core::RngCore;

const EXIT_ARGS: i32 = 2;
const EXIT_REQUEST: i32 = 3;
const EXIT_ENVELOPE: i32 = 4;

/// 옵션 파서 — `--name value` · `--flag` · 위치 인자. 같은 이름 반복 = 여러 값(`--request`).
#[derive(Debug, Default)]
struct Args {
    cmd: String,
    positional: Vec<String>,
    opts: Vec<(String, String)>,
    flags: Vec<String>,
}

const VALUE_OPTS: &[&str] = &[
    "--out",
    "--id",
    "--pass-env",
    "--iter",
    "--key",
    "--request",
    "--kind",
    "--licensee",
    "--email",
    "--tier",
    "--features",
    "--seats",
    "--seat-mode",
    "--updates-until",
    "--expires",
    "--max-major",
    "--product",
    "--ledger",
    "--note",
    "--add-request",
    "--drop-machine",
    "--pub",
    "--machine",
    "--build-date",
    "--id-prefix",
];

impl Args {
    fn parse(argv: &[String]) -> Result<Args, String> {
        let mut a = Args::default();
        let mut it = argv.iter();
        a.cmd = it.next().cloned().unwrap_or_default();
        while let Some(x) = it.next() {
            if let Some(name) = x.strip_prefix("--") {
                let name = format!("--{name}");
                if VALUE_OPTS.contains(&name.as_str()) {
                    let v = it.next().ok_or_else(|| format!("{name} needs a value"))?;
                    a.opts.push((name, v.clone()));
                } else {
                    a.flags.push(name);
                }
            } else {
                a.positional.push(x.clone());
            }
        }
        Ok(a)
    }
    fn get(&self, name: &str) -> Option<&str> {
        self.opts
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
    fn all(&self, name: &str) -> Vec<&str> {
        self.opts
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }
    fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }
}

fn usage() -> i32 {
    eprintln!(
        "nexa-license-tool — SosomLab offline license issuer\n\n\
         keygen  --out <envelope> [--id root-v1] [--pass-env NAME | --pass-stdin] [--iter 600000]\n\
         keys-rs <a.pub> [<b.pub>...] [--out keys.rs]\n\
         decode-request <NEXAREQ1....>\n\
         issue   --key <envelope> [--pass-env NAME|--pass-stdin] --request <code>... --kind device|user|team-seat|org\n\
                 --licensee <name> [--email <e>] (--tier trial|pro|org | --features a,b) [--seats N] [--seat-mode named|device|concurrent]\n\
                 [--updates-until D] [--expires D] [--max-major N] [--id ID] [--product nexa-sql] [--out ./issued] [--ledger ./issued/ledger.tsv] [--note ...] [--no-mail]\n\
         reissue --key <envelope> [--pass-...] --id <ID> [--add-request <code>]... [--drop-machine <prefix>]... [--out ...] [--ledger ...] [--note ...]\n\
         verify  <file> (--pub <a.pub>... | --key <envelope>) [--machine <base32>] [--build-date D] [--product nexa-sql]\n\
         ledger  list | find <text> [--ledger ...]\n\n\
         exit codes: 0 ok · 1 failed · 2 usage · 3 bad request code · 4 key envelope (password/corrupt)"
    );
    EXIT_ARGS
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let a = match Args::parse(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(EXIT_ARGS);
        }
    };
    let code = match a.cmd.as_str() {
        "keygen" => cmd_keygen(&a),
        "keys-rs" => cmd_keys_rs(&a),
        "decode-request" => cmd_decode_request(&a),
        "issue" => cmd_issue(&a),
        "reissue" => cmd_reissue(&a),
        "verify" => cmd_verify(&a),
        "ledger" => cmd_ledger(&a),
        _ => usage(),
    };
    std::process::exit(code);
}

// ─────────────────────────────────────────────────────────────── 공통

fn random16() -> [u8; 16] {
    let mut b = [0u8; 16];
    rand_core::OsRng.fill_bytes(&mut b);
    b
}

/// 암호 — `--pass-env` → `--pass-stdin`(첫 줄) → 터미널 프롬프트(유닉스 = 에코 끔).
fn password(a: &Args, confirm: bool) -> Result<Vec<u8>, i32> {
    if let Some(name) = a.get("--pass-env") {
        return std::env::var(name).map(String::into_bytes).map_err(|_| {
            eprintln!("environment variable {name} is not set");
            EXIT_ENVELOPE
        });
    }
    if a.flag("--pass-stdin") {
        let mut line = String::new();
        io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|_| EXIT_ENVELOPE)?;
        return Ok(line.trim_end_matches(['\r', '\n']).as_bytes().to_vec());
    }
    let p1 = prompt_secret("passphrase: ")?;
    if confirm {
        let p2 = prompt_secret("again: ")?;
        if p1 != p2 {
            eprintln!("passphrases differ");
            return Err(EXIT_ENVELOPE);
        }
    }
    if p1.is_empty() {
        eprintln!("empty passphrase refused");
        return Err(EXIT_ENVELOPE);
    }
    Ok(p1)
}

fn prompt_secret(label: &str) -> Result<Vec<u8>, i32> {
    eprint!("{label}");
    let _ = io::stderr().flush();
    #[cfg(unix)]
    let _ = std::process::Command::new("stty").arg("-echo").status();
    let mut line = String::new();
    let r = io::stdin().lock().read_line(&mut line);
    #[cfg(unix)]
    let _ = std::process::Command::new("stty").arg("echo").status();
    eprintln!();
    r.map_err(|_| EXIT_ENVELOPE)?;
    Ok(line.trim_end_matches(['\r', '\n']).as_bytes().to_vec())
}

/// 봉투를 열어 키쌍 + key id.
fn open_key(a: &Args) -> Result<(Keypair, String), i32> {
    let Some(path) = a.get("--key") else {
        eprintln!("--key <envelope> is required");
        return Err(EXIT_ARGS);
    };
    let text = fs::read_to_string(path).map_err(|e| {
        eprintln!("cannot read {path}: {e}");
        EXIT_ENVELOPE
    })?;
    let (id, _) = envelope::peek(&text).ok_or_else(|| {
        eprintln!("{path}: not a key envelope");
        EXIT_ENVELOPE
    })?;
    let pw = password(a, false)?;
    let secret = envelope::open(&text, &pw).map_err(|e| {
        eprintln!("cannot open key envelope: {e:?}");
        EXIT_ENVELOPE
    })?;
    Ok((Keypair::from_secret(secret), id))
}

fn today() -> String {
    date::format_days(date::today_days())
}

fn check_date(label: &str, v: &str) -> Result<(), i32> {
    if v.is_empty() || date::parse_days(v).is_some() {
        Ok(())
    } else {
        eprintln!("{label}: not a date (YYYY-MM-DD): {v}");
        Err(EXIT_ARGS)
    }
}

fn machine_prefix(b32: &str) -> String {
    b32.chars().take(8).collect()
}

// ─────────────────────────────────────────────────────────────── keygen · keys-rs

fn cmd_keygen(a: &Args) -> i32 {
    let Some(out) = a.get("--out") else {
        return usage();
    };
    let id = a.get("--id").unwrap_or("root-v1");
    let iter: u32 = match a
        .get("--iter")
        .map_or(Ok(envelope::DEFAULT_ITER), str::parse)
    {
        Ok(n) if n >= 1000 => n,
        _ => {
            eprintln!("--iter must be a number ≥ 1000");
            return EXIT_ARGS;
        }
    };
    let out = PathBuf::from(out);
    if out.exists() {
        eprintln!(
            "{} exists — refusing to overwrite a key envelope",
            out.display()
        );
        return 1;
    }
    let pw = match password(a, true) {
        Ok(p) => p,
        Err(c) => return c,
    };
    let kp = Keypair::generate();
    let env = envelope::seal(
        &kp.secret,
        &kp.public,
        id,
        &pw,
        iter,
        &random16(),
        &random16(),
    );
    if let Some(dir) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("{}: {e}", dir.display());
            return 1;
        }
    }
    if let Err(e) = write_private(&out, env.as_bytes()) {
        eprintln!("{}: {e}", out.display());
        return 1;
    }
    let pub_path = pub_path_of(&out);
    if let Err(e) = fs::write(&pub_path, sign::public_key_file(id, &kp.public)) {
        eprintln!("{}: {e}", pub_path.display());
        return 1;
    }
    println!("key id      {id}");
    println!("public      {}", kp.public_b32());
    println!("envelope    {}", out.display());
    println!("public file {}", pub_path.display());
    println!("next        1) back up the envelope + passphrase offline, twice (docs/91 §3)");
    println!(
        "            2) nexa-license-tool keys-rs {} --out <app>/crates/nexa-license/src/keys.rs",
        pub_path.display()
    );
    0
}

fn pub_path_of(envelope: &Path) -> PathBuf {
    let mut p = envelope.as_os_str().to_owned();
    p.push(".pub");
    PathBuf::from(p)
}

fn write_private(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut o = fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let mut f = o.open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

fn cmd_keys_rs(a: &Args) -> i32 {
    if a.positional.is_empty() {
        return usage();
    }
    let mut keys = Vec::new();
    for p in &a.positional {
        let Ok(text) = fs::read_to_string(p) else {
            eprintln!("cannot read {p}");
            return 1;
        };
        let Some(k) = sign::parse_public_key_file(&text) else {
            eprintln!("{p}: not a public key file (nxp1)");
            return 1;
        };
        keys.push(k);
    }
    let src = sign::keys_rs_source(&keys);
    match a.get("--out") {
        Some(o) => {
            if let Err(e) = fs::write(o, &src) {
                eprintln!("{o}: {e}");
                return 1;
            }
            println!("wrote {o} ({} key(s))", keys.len());
        }
        None => print!("{src}"),
    }
    0
}

// ─────────────────────────────────────────────────────────────── request

fn decode_request(code: &str) -> Result<request::Request, i32> {
    request::decode(code).ok_or_else(|| {
        eprintln!(
            "invalid request code: {}",
            code.chars().take(24).collect::<String>()
        );
        EXIT_REQUEST
    })
}

fn cmd_decode_request(a: &Args) -> i32 {
    let Some(code) = a.positional.first() else {
        return usage();
    };
    let r = match decode_request(code) {
        Ok(r) => r,
        Err(c) => return c,
    };
    println!("machine  {}", base32::encode(&r.machine));
    for (k, v) in &r.meta.pairs {
        println!("{k:<8} {v}");
    }
    0
}

// ─────────────────────────────────────────────────────────────── issue

struct IssueSpec {
    product: String,
    id: String,
    kind: Kind,
    licensee: String,
    email: String,
    tier: String,
    features: String,
    machines: Vec<String>,
    seats: Option<u32>,
    seat_mode: Option<SeatMode>,
    updates_until: String,
    expires: String,
    max_major: String,
    note: String,
    req_meta: String,
}

fn build_doc(s: &IssueSpec, issued: &str) -> Doc {
    let mut d = Doc::default();
    d.set("format", FORMAT);
    d.set("product", &s.product);
    d.set("id", &s.id);
    d.set("licensee", &s.licensee);
    if !s.email.is_empty() {
        d.set("email", &s.email);
    }
    d.set("kind", &s.kind.to_string());
    d.set("tier", &s.tier);
    d.set("features", &s.features);
    for (i, m) in s.machines.iter().enumerate() {
        if i == 0 {
            d.set("machine", m);
        } else {
            d.set(&format!("machine.{}", i + 1), m);
        }
    }
    if let Some(n) = s.seats {
        d.set("seats", &n.to_string());
    }
    if let Some(m) = s.seat_mode {
        d.set("seat_mode", &m.to_string());
    }
    d.set("issued", issued);
    d.set("updates_until", &s.updates_until);
    if !s.expires.is_empty() {
        d.set("expires", &s.expires);
    }
    if !s.max_major.is_empty() {
        d.set("max_major", &s.max_major);
    }
    d
}

fn out_dir(a: &Args) -> PathBuf {
    PathBuf::from(a.get("--out").unwrap_or("./issued"))
}
fn ledger_path(a: &Args) -> PathBuf {
    a.get("--ledger")
        .map_or_else(|| out_dir(a).join("ledger.tsv"), PathBuf::from)
}

fn cmd_issue(a: &Args) -> i32 {
    let product = a.get("--product").unwrap_or("nexa-sql").to_string();
    let Some(kind) = a.get("--kind").and_then(Kind::parse) else {
        eprintln!("--kind device|user|team-seat|org is required");
        return EXIT_ARGS;
    };
    let Some(licensee) = a.get("--licensee").filter(|s| !s.trim().is_empty()) else {
        eprintln!("--licensee is required");
        return EXIT_ARGS;
    };
    let features = match (a.get("--tier"), a.get("--features")) {
        (Some(t), None) => match presets::features_for(&product, t) {
            Some(f) => f,
            None => {
                eprintln!("unknown tier {t} for {product} (trial|pro|org)");
                return EXIT_ARGS;
            }
        },
        (None, Some(f)) => f
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(","),
        _ => {
            eprintln!("exactly one of --tier or --features is required");
            return EXIT_ARGS;
        }
    };
    let tier = a.get("--tier").unwrap_or("custom").to_string();
    let requests = a.all("--request");
    let max_machines = kind.max_machines();
    if requests.is_empty() && max_machines > 0 {
        eprintln!("--request <code> is required for kind {kind} (up to {max_machines})");
        return EXIT_ARGS;
    }
    if max_machines > 0 && requests.len() > max_machines {
        eprintln!(
            "kind {kind} allows at most {max_machines} machine(s), got {}",
            requests.len()
        );
        return EXIT_ARGS;
    }
    let mut machines = Vec::new();
    let mut metas = Vec::new();
    for code in &requests {
        let r = match decode_request(code) {
            Ok(r) => r,
            Err(c) => return c,
        };
        let m = base32::encode(&r.machine);
        if machines.contains(&m) {
            eprintln!("duplicate machine in requests: {}", machine_prefix(&m));
            return EXIT_REQUEST;
        }
        machines.push(m);
        metas.push(format!(
            "{}/{}",
            r.meta.get("os").unwrap_or("?"),
            r.meta.get("app").unwrap_or("?")
        ));
    }
    let seats = match a.get("--seats").map(str::parse::<u32>) {
        None => None,
        Some(Ok(n)) => Some(n),
        Some(Err(_)) => {
            eprintln!("--seats must be a number");
            return EXIT_ARGS;
        }
    };
    let seat_mode = match a.get("--seat-mode") {
        None => None,
        Some(s) => match SeatMode::parse(s) {
            Some(m) => Some(m),
            None => {
                eprintln!("--seat-mode named|device|concurrent");
                return EXIT_ARGS;
            }
        },
    };
    if matches!(kind, Kind::Org) && seats.is_none() {
        eprintln!("--seats N is required for kind {kind}");
        return EXIT_ARGS;
    }
    let issued = today();
    let today_days = date::today_days();
    let updates_until = a
        .get("--updates-until")
        .map_or_else(|| date::format_days(today_days + 365), str::to_string);
    let expires = a.get("--expires").map_or_else(
        || {
            if tier == "trial" {
                date::format_days(today_days + presets::TRIAL_DAYS)
            } else {
                String::new()
            }
        },
        str::to_string,
    );
    for (l, v) in [("--updates-until", &updates_until), ("--expires", &expires)] {
        if let Err(c) = check_date(l, v) {
            return c;
        }
    }
    let max_major = a.get("--max-major").unwrap_or("").to_string();
    if !max_major.is_empty() && max_major.parse::<u32>().is_err() {
        eprintln!("--max-major must be a number");
        return EXIT_ARGS;
    }
    let ledger = ledger_path(a);
    let id = match a.get("--id") {
        Some(id) => id.to_string(),
        None => ledger::next_id(&ledger, a.get("--id-prefix").unwrap_or("NSL"), &issued[..4]),
    };
    let spec = IssueSpec {
        product,
        id,
        kind,
        licensee: licensee.to_string(),
        email: a.get("--email").unwrap_or("").to_string(),
        tier,
        features,
        machines,
        seats,
        seat_mode,
        updates_until,
        expires,
        max_major,
        note: a.get("--note").unwrap_or("").to_string(),
        req_meta: metas.join(";"),
    };
    let (kp, key_id) = match open_key(a) {
        Ok(k) => k,
        Err(c) => return c,
    };
    let mut doc = build_doc(&spec, &issued);
    kp.sign_license(&mut doc, &key_id);
    write_issued(a, &spec, &doc, 1, !a.flag("--no-mail"))
}

/// 파일 쓰기 + 대장 1행 + (선택) mail.txt. 자기 검증(앱과 같은 코드)을 먼저 한다.
fn write_issued(a: &Args, s: &IssueSpec, doc: &Doc, version: u32, mail: bool) -> i32 {
    let text = doc.serialize();
    let product = Product {
        id: Box::leak(s.product.clone().into_boxed_str()),
        build_date: Box::leak(today().into_boxed_str()),
    };
    let pk: Vec<u8> = base32::decode(doc.get("key").unwrap_or("")).unwrap_or_default();
    let _ = pk;
    // 자기 검증: 문서의 key id에 맞는 공개키는 봉투에서 얻는다(--key 필수였으므로 존재).
    if let Some(path) = a.get("--key") {
        if let Some((kid, public)) = fs::read_to_string(path)
            .ok()
            .and_then(|t| envelope::peek(&t))
        {
            let roots = [RootKey {
                id: Box::leak(kid.into_boxed_str()),
                alg: Alg::Ed25519,
                public: Box::leak(public.to_vec().into_boxed_slice()),
            }];
            let m = s.machines.first().and_then(|m| base32::decode(m));
            let v = verify_license(
                &product,
                &roots,
                &nexa_license::ed25519::Ed25519Verifier,
                &text,
                m.as_deref(),
                date::today_days(),
            );
            if !matches!(v, Verdict::Licensed(_)) {
                eprintln!("self-verify failed: {v:?} — nothing written");
                return 1;
            }
        }
    }
    let dir = out_dir(a).join(&s.id);
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("{}: {e}", dir.display());
        return 1;
    }
    let file = dir.join(format!("{}.license", s.product));
    if version > 1 && file.exists() {
        let prev = dir.join(format!("{}.license.v{}", s.product, version - 1));
        let _ = fs::rename(&file, &prev);
    }
    if let Err(e) = fs::write(&file, &text) {
        eprintln!("{}: {e}", file.display());
        return 1;
    }
    let row = ledger::Row {
        id: s.id.clone(),
        version,
        issued: doc.get("issued").unwrap_or("").to_string(),
        kind: s.kind.to_string(),
        tier: s.tier.clone(),
        licensee: s.licensee.clone(),
        email: s.email.clone(),
        features: s.features.clone(),
        machines: s
            .machines
            .iter()
            .map(|m| machine_prefix(m))
            .collect::<Vec<_>>()
            .join(","),
        updates_until: s.updates_until.clone(),
        expires: s.expires.clone(),
        note: s.note.clone(),
        file: file.display().to_string(),
        req_meta: s.req_meta.clone(),
    };
    if let Err(e) = ledger::append(&ledger_path(a), &row) {
        eprintln!("ledger: {e}");
        return 1;
    }
    if mail {
        let _ = fs::write(
            dir.join("mail.txt"),
            presets::mail_text(s.licensee.as_str(), &s.product, &s.id),
        );
    }
    println!("id             {}", s.id);
    println!("version        {version}");
    println!("kind / tier    {} / {}", s.kind, s.tier);
    println!("licensee       {}", s.licensee);
    println!("features       {}", s.features);
    println!("machines       {}", row.machines);
    println!("updates_until  {}", s.updates_until);
    println!(
        "expires        {}",
        if s.expires.is_empty() {
            "-"
        } else {
            &s.expires
        }
    );
    println!("file           {}", file.display());
    println!("ledger         {}", ledger_path(a).display());
    0
}

// ─────────────────────────────────────────────────────────────── reissue

fn cmd_reissue(a: &Args) -> i32 {
    let Some(id) = a.get("--id") else {
        eprintln!("--id <ID> is required");
        return EXIT_ARGS;
    };
    let ledger = ledger_path(a);
    let Some(last) = ledger::find_latest(&ledger, id) else {
        eprintln!("{id}: not in ledger {}", ledger.display());
        return 1;
    };
    let Ok(text) = fs::read_to_string(&last.file) else {
        eprintln!("cannot read previous file {}", last.file);
        return 1;
    };
    let prev = Doc::parse(&text);
    let kind = prev.get("kind").and_then(Kind::parse).unwrap_or_default();
    let mut machines: Vec<String> = prev
        .get_numbered("machine")
        .iter()
        .map(|s| s.to_string())
        .collect();
    for pfx in a.all("--drop-machine") {
        let before = machines.len();
        machines.retain(|m| !m.starts_with(&pfx.to_ascii_uppercase()));
        if machines.len() == before {
            eprintln!("--drop-machine {pfx}: no such machine");
            return 1;
        }
    }
    let mut metas = Vec::new();
    for code in a.all("--add-request") {
        let r = match decode_request(code) {
            Ok(r) => r,
            Err(c) => return c,
        };
        let m = base32::encode(&r.machine);
        if !machines.contains(&m) {
            machines.push(m);
        }
        metas.push(format!(
            "{}/{}",
            r.meta.get("os").unwrap_or("?"),
            r.meta.get("app").unwrap_or("?")
        ));
    }
    let max = kind.max_machines();
    if max > 0 && machines.len() > max {
        eprintln!(
            "kind {kind} allows at most {max} machine(s), would have {}",
            machines.len()
        );
        return 1;
    }
    let spec = IssueSpec {
        product: prev.get("product").unwrap_or("nexa-sql").to_string(),
        id: id.to_string(),
        kind,
        licensee: prev.get("licensee").unwrap_or("").to_string(),
        email: prev.get("email").unwrap_or("").to_string(),
        tier: prev.get("tier").unwrap_or("").to_string(),
        features: prev.get("features").unwrap_or("").to_string(),
        machines,
        seats: prev.get("seats").and_then(|s| s.parse().ok()),
        seat_mode: prev.get("seat_mode").and_then(SeatMode::parse),
        updates_until: prev.get("updates_until").unwrap_or("").to_string(),
        expires: prev.get("expires").unwrap_or("").to_string(),
        max_major: prev.get("max_major").unwrap_or("").to_string(),
        note: a.get("--note").unwrap_or("reissue").to_string(),
        req_meta: metas.join(";"),
    };
    let (kp, key_id) = match open_key(a) {
        Ok(k) => k,
        Err(c) => return c,
    };
    let mut doc = build_doc(&spec, prev.get("issued").unwrap_or(&today()));
    doc.set("reissued", &today());
    kp.sign_license(&mut doc, &key_id);
    write_issued(a, &spec, &doc, last.version + 1, false)
}

// ─────────────────────────────────────────────────────────────── verify

fn cmd_verify(a: &Args) -> i32 {
    let Some(file) = a.positional.first() else {
        return usage();
    };
    let Ok(text) = fs::read_to_string(file) else {
        eprintln!("cannot read {file}");
        return 1;
    };
    let mut roots: Vec<RootKey> = Vec::new();
    for p in a.all("--pub") {
        let Some((id, pk)) = fs::read_to_string(p)
            .ok()
            .and_then(|t| sign::parse_public_key_file(&t))
        else {
            eprintln!("{p}: not a public key file");
            return 1;
        };
        roots.push(RootKey {
            id: Box::leak(id.into_boxed_str()),
            alg: Alg::Ed25519,
            public: Box::leak(pk.to_vec().into_boxed_slice()),
        });
    }
    if let Some(k) = a.get("--key") {
        if let Some((id, pk)) = fs::read_to_string(k).ok().and_then(|t| envelope::peek(&t)) {
            roots.push(RootKey {
                id: Box::leak(id.into_boxed_str()),
                alg: Alg::Ed25519,
                public: Box::leak(pk.to_vec().into_boxed_slice()),
            });
        }
    }
    if roots.is_empty() {
        eprintln!("--pub <file> or --key <envelope> is required");
        return EXIT_ARGS;
    }
    let doc = Doc::parse(&text);
    let product = Product {
        id: Box::leak(
            a.get("--product")
                .or_else(|| doc.get("product"))
                .unwrap_or("nexa-sql")
                .to_string()
                .into_boxed_str(),
        ),
        build_date: Box::leak(
            a.get("--build-date")
                .map_or_else(today, str::to_string)
                .into_boxed_str(),
        ),
    };
    let machine = match a.get("--machine") {
        None => None,
        Some(m) => match base32::decode(m) {
            Some(b) if b.len() == 20 => Some(b),
            _ => {
                eprintln!("--machine must be a base32 machine code (20 bytes)");
                return EXIT_ARGS;
            }
        },
    };
    let v = verify_license(
        &product,
        &roots,
        &nexa_license::ed25519::Ed25519Verifier,
        &text,
        machine.as_deref(),
        date::today_days(),
    );
    match v {
        Verdict::Licensed(l) => {
            println!("verdict        licensed");
            println!("id             {}", l.id);
            println!("licensee       {}", l.licensee);
            println!("kind / tier    {} / {}", l.kind, l.tier);
            println!(
                "features       {}",
                l.features.iter().cloned().collect::<Vec<_>>().join(",")
            );
            println!("machines       {}", l.machines.len());
            println!("updates_until  {}", l.updates_until);
            println!(
                "expires        {}",
                if l.expires.is_empty() {
                    "-"
                } else {
                    &l.expires
                }
            );
            println!("key            {}", l.key_id);
            0
        }
        Verdict::Outdated(l) => {
            println!(
                "verdict        outdated (updates_until {} < build)",
                l.updates_until
            );
            1
        }
        Verdict::Expired(l) => {
            println!("verdict        expired ({})", l.expires);
            1
        }
        Verdict::Invalid(i) => {
            println!("verdict        invalid({i:?})");
            1
        }
    }
}

// ─────────────────────────────────────────────────────────────── ledger

fn cmd_ledger(a: &Args) -> i32 {
    let path = ledger_path(a);
    let rows = ledger::read(&path);
    match a.positional.first().map(String::as_str) {
        Some("list") => {
            ledger::print(&rows);
            0
        }
        Some("find") => {
            let Some(q) = a.positional.get(1) else {
                return usage();
            };
            let q = q.to_ascii_lowercase();
            let hits: Vec<ledger::Row> = rows
                .into_iter()
                .filter(|r| {
                    [&r.id, &r.licensee, &r.email, &r.note, &r.machines]
                        .iter()
                        .any(|f| f.to_ascii_lowercase().contains(&q))
                })
                .collect();
            ledger::print(&hits);
            i32::from(hits.is_empty())
        }
        _ => usage(),
    }
}
