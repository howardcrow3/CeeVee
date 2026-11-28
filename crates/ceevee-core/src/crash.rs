//! Crash reports. When CeeVee panics it writes a small text file into the log folder; the next
//! start tells the person and offers the folder so they can send the file to support. Nothing is
//! uploaded. A report holds the version, the system, the panic message and where it happened,
//! and never any job, resume or mail content.

use std::fs;
use std::path::{Path, PathBuf};

const PREFIX: &str = "crash-";
const NEW: &str = ".txt";
const SEEN: &str = ".seen";

/// The text of a report. `message` and `location` come from the panic.
pub fn format_report(
    version: &str,
    os: &str,
    unix_time: u64,
    message: &str,
    location: &str,
) -> String {
    format!(
        "CeeVee crash report\nversion: {version}\nsystem: {os}\ntime (unix): {unix_time}\nwhere: {location}\nmessage: {message}\n"
    )
}

/// Write a report into `dir` and return its path. Never panics; a failure to write is returned.
pub fn write_report(dir: &Path, unix_time: u64, text: &str) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{PREFIX}{unix_time}{NEW}"));
    fs::write(&path, text)?;
    Ok(path)
}

/// A report the person has not been told about yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unseen {
    pub path: PathBuf,
    pub summary: String,
}

/// The newest unseen report in `dir`, if any. The summary is its `message:` line.
pub fn latest_unseen(dir: &Path) -> Option<Unseen> {
    let mut found: Vec<(u64, PathBuf)> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stamp = name
                .strip_prefix(PREFIX)?
                .strip_suffix(NEW)?
                .parse::<u64>()
                .ok()?;
            Some((stamp, e.path()))
        })
        .collect();
    found.sort();
    let (_, path) = found.pop()?;
    let text = fs::read_to_string(&path).unwrap_or_default();
    let summary = text
        .lines()
        .find_map(|l| l.strip_prefix("message: "))
        .unwrap_or("CeeVee closed unexpectedly")
        .to_string();
    Some(Unseen { path, summary })
}

/// Mark every report in `dir` as seen so the notice is not shown again. The files stay.
pub fn mark_all_seen(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with(PREFIX) && name.ends_with(NEW) {
            let _ = fs::rename(e.path(), e.path().with_extension(&SEEN[1..]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ceevee-crash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn report_has_the_facts_and_no_more() {
        let t = format_report("1.0.0", "windows", 5, "boom", "src/x.rs:3");
        assert!(
            t.contains("version: 1.0.0") && t.contains("message: boom") && t.contains("src/x.rs:3")
        );
    }

    #[test]
    fn newest_unseen_report_is_found_then_cleared() {
        let d = tmp("seen");
        assert_eq!(latest_unseen(&d), None, "no folder yet");
        write_report(&d, 10, &format_report("1", "os", 10, "old", "a")).unwrap();
        write_report(&d, 20, &format_report("1", "os", 20, "new", "b")).unwrap();
        fs::write(d.join("ceevee.log"), "not a report").unwrap();
        assert_eq!(latest_unseen(&d).unwrap().summary, "new");
        mark_all_seen(&d);
        assert_eq!(latest_unseen(&d), None);
        assert!(d.join("crash-20.seen").exists(), "reports are kept");
        let _ = fs::remove_dir_all(&d);
    }
}
