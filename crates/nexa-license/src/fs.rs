//! 라이선스 파일 관리(feature `fs`) — **폴더는 호출측이 준다**(라이브러리는 앱·OS 폴더 규칙을 모른다 · nexa-sql docs/25 §10).
//!
//! - 판정 순서 = 주어진 폴더 순(앱은 사용자 폴더 → 기기 공용 폴더 → … 로 넘긴다 · docs/25 §11-2). 첫 번째로 **존재하는** 파일을 쓴다.
//! - 설치·삭제는 **첫 폴더**에만(개인 라이선스 자리). 기기 공용 폴더 설치는 관리자 권한 문제라 호출측이 따로 한다.
//! - 쓰기는 **원자적**(같은 폴더의 임시 파일에 쓰고 `rename`) — 두 프로세스가 동시에 설치해도 읽는 쪽은 늘 온전한 파일을 본다(docs/23 §5).
//! - 파일 없음·폴더 없음은 오류가 아니라 `None`(= Free). I/O 오류만 `Err`.

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::types::Product;

/// 라이선스 파일 자리.
#[derive(Debug, Clone)]
pub struct Store {
    product: Product,
    dirs: Vec<PathBuf>,
}

/// 읽은 파일 — 어디서 · 무엇을 · 언제(변경 감지용 서명).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub text: String,
    pub stamp: Stamp,
}

/// 변경 감지 서명 — mtime 해상도가 거친 파일 시스템을 위해 길이도 함께 본다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub mtime: Option<SystemTime>,
    pub len: u64,
}

impl Store {
    /// `dirs` = 판정 순서. 비어 있으면 어떤 파일도 찾지 않는다(설치는 `Err(NotFound)`).
    #[must_use]
    pub fn new(product: Product, dirs: Vec<PathBuf>) -> Store {
        Store { product, dirs }
    }

    #[must_use]
    pub fn product(&self) -> &Product {
        &self.product
    }

    #[must_use]
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// `<dir>/<product>.license`.
    #[must_use]
    pub fn path_in(&self, dir: &Path) -> PathBuf {
        dir.join(self.product.license_file_name())
    }

    /// 설치·삭제 대상(첫 폴더의 파일).
    #[must_use]
    pub fn primary_path(&self) -> Option<PathBuf> {
        self.dirs.first().map(|d| self.path_in(d))
    }

    /// 첫 번째로 존재하는 파일.
    #[must_use]
    pub fn locate(&self) -> Option<PathBuf> {
        self.dirs
            .iter()
            .map(|d| self.path_in(d))
            .find(|p| p.is_file())
    }

    /// 현재 파일의 변경 서명(없으면 None) — 재판정 여부를 싸게 결정한다.
    #[must_use]
    pub fn stamp(&self) -> Option<(PathBuf, Stamp)> {
        let p = self.locate()?;
        let m = fs::metadata(&p).ok()?;
        Some((
            p,
            Stamp {
                mtime: m.modified().ok(),
                len: m.len(),
            },
        ))
    }

    /// 읽기 — 없으면 `Ok(None)`.
    pub fn read(&self) -> io::Result<Option<Found>> {
        let Some(path) = self.locate() else {
            return Ok(None);
        };
        let text = fs::read_to_string(&path)?;
        let m = fs::metadata(&path)?;
        Ok(Some(Found {
            path,
            text,
            stamp: Stamp {
                mtime: m.modified().ok(),
                len: m.len(),
            },
        }))
    }

    /// 본문을 첫 폴더에 원자적으로 설치(폴더가 없으면 만든다). 검증은 호출측이 **먼저** 한다.
    pub fn install_text(&self, text: &str) -> io::Result<PathBuf> {
        let Some(path) = self.primary_path() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no license directory",
            ));
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&path, text.as_bytes())?;
        Ok(path)
    }

    /// 파일을 읽어 설치(원본은 그대로).
    pub fn install_from(&self, src: &Path) -> io::Result<PathBuf> {
        let text = fs::read_to_string(src)?;
        self.install_text(&text)
    }

    /// 첫 폴더의 파일 삭제 — 없었으면 `Ok(false)`.
    pub fn remove(&self) -> io::Result<bool> {
        let Some(path) = self.primary_path() else {
            return Ok(false);
        };
        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
}

/// 원자적 쓰기 — 같은 폴더의 임시 파일(`<이름>.tmp-<pid>-<nanos>`)에 쓰고 `rename`(같은 파일 시스템 · 덮어쓰기).
/// 실패하면 임시 파일을 지운다. 유닉스는 0600(라이선스에 `licensee`가 있다).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "license".to_string());
    let tmp = dir.join(format!("{name}.tmp-{}-{nanos}", std::process::id()));
    let res = (|| -> io::Result<()> {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        io::Write::write_all(&mut f, bytes)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::Doc;

    const P: Product = Product {
        id: "nexa-test",
        build_date: "2026-09-27",
    };
    fn tmp(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let d = std::env::temp_dir().join(format!(
            "nexa-license-fs-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&d).expect("temp dir");
        d
    }

    #[test]
    fn missing_dir_and_file_are_none_not_errors() {
        let base = tmp("none");
        let s = Store::new(P, vec![base.join("no-such"), base.join("either")]);
        assert_eq!(s.read().expect("read"), None);
        assert_eq!(s.locate(), None);
        assert!(!s.remove().expect("remove"));
        let empty = Store::new(P, vec![]);
        assert_eq!(empty.read().expect("read"), None);
        assert!(empty.install_text("x").is_err(), "폴더 없음 = 설치 불가");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn install_read_order_and_remove() {
        let base = tmp("order");
        let user = base.join("user");
        let machine = base.join("machine");
        let s = Store::new(P, vec![user.clone(), machine.clone()]);
        // 기기 공용 폴더에만 있으면 그것을 본다.
        fs::create_dir_all(&machine).expect("mkdir");
        fs::write(s.path_in(&machine), "format=nxl1\nid=M\n").expect("write");
        let f = s.read().expect("read").expect("found");
        assert_eq!(f.path, s.path_in(&machine));
        assert!(f.text.contains("id=M"));
        // 사용자 폴더에 설치하면(폴더 자동 생성) 그것이 앞선다.
        let p = s.install_text("format=nxl1\nid=U\n").expect("install");
        assert_eq!(p, s.path_in(&user));
        let f = s.read().expect("read").expect("found");
        assert_eq!(f.path, p);
        assert!(f.text.contains("id=U"));
        assert_eq!(f.stamp.len, 17);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&p).expect("meta").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        // 임시 파일이 남지 않는다.
        let leftovers = fs::read_dir(&user)
            .expect("ls")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .count();
        assert_eq!(leftovers, 0);
        // 삭제 = 첫 폴더만 → 다시 기기 파일이 보인다.
        assert!(s.remove().expect("remove"));
        assert_eq!(s.locate(), Some(s.path_in(&machine)));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn install_from_file_and_stamp_changes() {
        let base = tmp("from");
        let s = Store::new(P, vec![base.join("u")]);
        let src = base.join("mail.license");
        fs::write(&src, "format=nxl1\nid=A\n").expect("src");
        s.install_from(&src).expect("install");
        assert!(src.is_file(), "원본 보존");
        let a = s.stamp().expect("stamp").1;
        s.install_text("format=nxl1\nid=ABCDEF\n")
            .expect("install 2");
        let b = s.stamp().expect("stamp").1;
        assert_ne!(a, b, "길이가 달라 mtime 해상도와 무관하게 변경을 본다");
        let _ = fs::remove_dir_all(&base);
    }

    /// docs/23 §5 "동시 두 프로세스 설치 → 원자적 교체 · 둘 다 유효 파일을 본다".
    #[test]
    fn concurrent_installs_never_expose_a_torn_file() {
        let base = tmp("race");
        let s = Store::new(P, vec![base.join("u")]);
        let mut big_a = Doc::default();
        let mut big_b = Doc::default();
        big_a.set("format", "nxl1");
        big_b.set("format", "nxl1");
        for i in 0..400 {
            big_a.set(&format!("k{i:03}"), &"a".repeat(64));
            big_b.set(&format!("k{i:03}"), &"b".repeat(64));
        }
        big_a.set("end", "A");
        big_b.set("end", "B");
        let (ta, tb) = (big_a.serialize(), big_b.serialize());
        s.install_text(&ta).expect("seed");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let writers: Vec<_> = [ta.clone(), tb.clone()]
            .into_iter()
            .map(|t| {
                let s = s.clone();
                let stop = stop.clone();
                std::thread::spawn(move || {
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        s.install_text(&t).expect("install");
                    }
                })
            })
            .collect();
        let mut seen = 0;
        for _ in 0..300 {
            if let Some(f) = s.read().expect("read") {
                assert!(
                    f.text == ta || f.text == tb,
                    "찢긴 파일을 봤다(len {})",
                    f.text.len()
                );
                let d = Doc::parse(&f.text);
                assert!(d.get("end").is_some());
                seen += 1;
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        for w in writers {
            w.join().expect("join");
        }
        assert!(seen > 0);
        let _ = fs::remove_dir_all(&base);
    }
}
