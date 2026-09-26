//! 발급 대장 `ledger.tsv`(발급 PC 로컬 · 비공개 저장소에도 올리지 않는다 · docs/25 §12-4 L-3). 기기 ID는 **접두 8자**만.
//! 열: id · version · issued · kind · tier · licensee · email · features · machines · updates_until · expires · note · file · req_meta

use std::{fs, io::Write, path::Path};

pub(crate) const HEADER: &str = "id\tversion\tissued\tkind\ttier\tlicensee\temail\tfeatures\tmachines\tupdates_until\texpires\tnote\tfile\treq_meta";

#[derive(Debug, Clone, Default)]
pub(crate) struct Row {
    pub id: String,
    pub version: u32,
    pub issued: String,
    pub kind: String,
    pub tier: String,
    pub licensee: String,
    pub email: String,
    pub features: String,
    pub machines: String,
    pub updates_until: String,
    pub expires: String,
    pub note: String,
    pub file: String,
    pub req_meta: String,
}

fn clean(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\t' || c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .collect()
}

impl Row {
    fn line(&self) -> String {
        [
            &self.id,
            &self.version.to_string(),
            &self.issued,
            &self.kind,
            &self.tier,
            &self.licensee,
            &self.email,
            &self.features,
            &self.machines,
            &self.updates_until,
            &self.expires,
            &self.note,
            &self.file,
            &self.req_meta,
        ]
        .iter()
        .map(|s| clean(s))
        .collect::<Vec<_>>()
        .join("\t")
    }
    fn parse(line: &str) -> Option<Row> {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 14 || f[0] == "id" {
            return None;
        }
        Some(Row {
            id: f[0].into(),
            version: f[1].parse().unwrap_or(1),
            issued: f[2].into(),
            kind: f[3].into(),
            tier: f[4].into(),
            licensee: f[5].into(),
            email: f[6].into(),
            features: f[7].into(),
            machines: f[8].into(),
            updates_until: f[9].into(),
            expires: f[10].into(),
            note: f[11].into(),
            file: f[12].into(),
            req_meta: f[13].into(),
        })
    }
}

pub(crate) fn read(path: &Path) -> Vec<Row> {
    fs::read_to_string(path)
        .map(|t| t.lines().filter_map(Row::parse).collect())
        .unwrap_or_default()
}

pub(crate) fn append(path: &Path, row: &Row) -> std::io::Result<()> {
    if let Some(d) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(d)?;
    }
    let fresh = !path.exists();
    let mut f = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    if fresh {
        writeln!(f, "{HEADER}")?;
    }
    writeln!(f, "{}", row.line())?;
    f.sync_all()
}

/// 같은 id의 마지막 판.
pub(crate) fn find_latest(path: &Path, id: &str) -> Option<Row> {
    read(path)
        .into_iter()
        .filter(|r| r.id == id)
        .max_by_key(|r| r.version)
}

/// `<prefix>-<year>-<6자리>` — 그 해의 최대 순번 + 1.
pub(crate) fn next_id(path: &Path, prefix: &str, year: &str) -> String {
    let head = format!("{prefix}-{year}-");
    let max = read(path)
        .iter()
        .filter_map(|r| r.id.strip_prefix(&head).and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);
    format!("{head}{:06}", max + 1)
}

pub(crate) fn print(rows: &[Row]) {
    let header = Row {
        id: "id".into(),
        issued: "issued".into(),
        kind: "kind".into(),
        tier: "tier".into(),
        licensee: "licensee".into(),
        updates_until: "until".into(),
        expires: "expires".into(),
        machines: "machines".into(),
        ..Row::default()
    };
    print_row(&header, "ver");
    for r in rows {
        print_row(r, &r.version.to_string());
    }
}

fn print_row(r: &Row, ver: &str) {
    println!(
        "{:<18} {:>3} {:<10} {:<6} {:<6} {:<24} {:<10} {:<10} {}",
        r.id,
        ver,
        r.issued,
        r.kind,
        r.tier,
        r.licensee.chars().take(24).collect::<String>(),
        r.updates_until,
        if r.expires.is_empty() {
            "-"
        } else {
            &r.expires
        },
        r.machines
    );
}
