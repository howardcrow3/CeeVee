//! Resumable, verified model download (TECHNICAL_PLAN section 5).
//!
//! Writes to `<file>.partial`, resumes with an HTTP range request, checks the
//! SHA-256 of the finished file, and only then renames it into place, so a
//! half-downloaded or corrupted model is never loaded.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use ceevee_core::{Error, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::catalog::ModelEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub downloaded: u64,
    /// Total size in bytes if the server told us, else the catalog's estimate.
    pub total: u64,
}

fn net(e: impl std::fmt::Display) -> Error {
    Error::Other(format!("download failed: {e}"))
}

/// Streaming SHA-256 of a file, as lowercase hex.
pub async fn sha256_file(path: &Path) -> Result<String> {
    use tokio::io::AsyncReadExt;
    let mut f = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Download `entry` into `dir` and return the verified file path.
///
/// `cancel` is checked between chunks; cancelling keeps the partial file so the
/// next call resumes. Returns immediately if a verified copy already exists.
pub async fn download(
    entry: &ModelEntry,
    dir: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(Progress),
) -> Result<PathBuf> {
    let (Some(url), Some(want)) = (entry.url.as_deref(), entry.sha256.as_deref()) else {
        return Err(Error::Invalid(format!(
            "model {} is not published yet",
            entry.id
        )));
    };
    let want = want.to_ascii_lowercase();
    tokio::fs::create_dir_all(dir).await?;
    let final_path = dir.join(&entry.file);
    if final_path.exists() {
        if sha256_file(&final_path).await? == want {
            return Ok(final_path);
        }
        tokio::fs::remove_file(&final_path).await?; // damaged: fetch again
    }
    let partial = dir.join(format!("{}.partial", entry.file));
    let client = reqwest::Client::new();

    // At most one restart from zero: a server that ignores Range, or a stale partial.
    for attempt in 0..2 {
        let offset = match tokio::fs::metadata(&partial).await {
            Ok(m) if attempt == 0 => m.len(),
            _ => 0,
        };
        let mut req = client.get(url);
        if offset > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        let resp = req.send().await.map_err(net)?;
        let status = resp.status();

        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            // The partial is already the whole file, or longer than it. Let the hash decide.
            if sha256_file(&partial).await? == want {
                tokio::fs::rename(&partial, &final_path).await?;
                return Ok(final_path);
            }
            tokio::fs::remove_file(&partial).await?;
            continue;
        }
        if !status.is_success() {
            return Err(net(format!("server answered {status}")));
        }
        let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT && offset > 0;
        let start = if resumed { offset } else { 0 };
        let total = resp
            .content_length()
            .map_or(entry.size_bytes, |n| n + start);

        let mut file = if resumed {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&partial)
                .await?
        } else {
            tokio::fs::File::create(&partial).await?
        };
        let mut downloaded = start;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                file.flush().await?;
                return Err(Error::Cancelled);
            }
            let chunk = chunk.map_err(net)?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            on_progress(Progress {
                downloaded,
                total: total.max(downloaded),
            });
        }
        file.flush().await?;
        drop(file);

        if sha256_file(&partial).await? == want {
            tokio::fs::rename(&partial, &final_path).await?;
            return Ok(final_path);
        }
        tokio::fs::remove_file(&partial).await?;
        if resumed {
            continue; // the resumed bytes may have been stale; retry once from zero
        }
        return Err(Error::Invalid(format!(
            "{}: checksum does not match, file discarded",
            entry.file
        )));
    }
    Err(Error::Invalid(format!(
        "{}: checksum does not match, file discarded",
        entry.file
    )))
}

#[cfg(test)]
mod tests;
