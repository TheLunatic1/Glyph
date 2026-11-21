use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use directories::BaseDirs;
use serde_json::json;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub fn default_logs_dir() -> Result<PathBuf> {
    if let Some(base) = BaseDirs::new() {
        return Ok(base.home_dir().join(".glyph").join("logs"));
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        return Ok(PathBuf::from(home).join(".glyph").join("logs"));
    }
    Err(anyhow!("Unable to resolve home directory for ~/.glyph/logs"))
}

fn sanitize_label(label: &str) -> String {
    let s: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(48)
        .collect();
    if s.is_empty() {
        "glyph_session".to_string()
    } else {
        s
    }
}

/// Live terminal session recorder writing standard **asciinema v2** `.cast` recordings.
pub struct SessionRecorder {
    path: PathBuf,
    writer: BufWriter<File>,
    started_at: Instant,
}

impl SessionRecorder {
    pub fn start(session_label: &str, cols: u32, rows: u32) -> Result<Self> {
        let dir = default_logs_dir()?;
        Self::start_in_dir(&dir, session_label, cols, rows)
    }

    pub fn start_in_dir(dir: &Path, session_label: &str, cols: u32, rows: u32) -> Result<Self> {
        fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create log directory {}", dir.display()))?;
        let ts = Utc::now().format("%Y%m%d_%H%M%S");
        let safe = sanitize_label(session_label);
        let path = dir.join(format!("{safe}_{ts}.cast"));
        let file = File::create(&path)
            .with_context(|| format!("Failed to create recording {}", path.display()))?;
        let mut writer = BufWriter::new(file);

        let header = json!({
            "version": 2,
            "width": cols.max(10),
            "height": rows.max(4),
            "timestamp": Utc::now().timestamp(),
            "title": session_label,
        });
        writeln!(writer, "{header}")?;
        writer.flush()?;

        Ok(Self {
            path,
            writer,
            started_at: Instant::now(),
        })
    }

    pub fn record_output(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let elapsed = self.started_at.elapsed().as_secs_f64();
        let text = String::from_utf8_lossy(bytes);
        let ev = json!([elapsed, "o", text]);
        if writeln!(self.writer, "{ev}").is_ok() {
            let _ = self.writer.flush();
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn export_plain_log(session_label: &str, content: &str) -> Result<PathBuf> {
    let dir = default_logs_dir()?;
    export_plain_log_in_dir(&dir, session_label, content)
}

pub fn export_plain_log_in_dir(dir: &Path, session_label: &str, content: &str) -> Result<PathBuf> {
    fs::create_dir_all(dir)
        .with_context(|| format!("Failed to create log directory {}", dir.display()))?;
    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let safe = sanitize_label(session_label);
    let path = dir.join(format!("{safe}_{ts}.log"));
    fs::write(&path, content.as_bytes())
        .with_context(|| format!("Failed to write plain transcript {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asciinema_cast_and_plain_log_export_work() {
        let dir = std::env::temp_dir().join(format!(
            "glyph_rec_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let mut rec = SessionRecorder::start_in_dir(&dir, "prod@web:22", 100, 32).unwrap();
        rec.record_output(b"hello glyph\r\n");
        let cast_path = rec.path().to_path_buf();
        drop(rec);

        let cast_txt = fs::read_to_string(&cast_path).unwrap();
        let mut lines = cast_txt.lines();
        let header: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(header["version"], 2);
        assert_eq!(header["width"], 100);
        assert_eq!(header["height"], 32);

        let event: serde_json::Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(event[1], "o");
        assert_eq!(event[2], "hello glyph\r\n");

        let log_path = export_plain_log_in_dir(&dir, "prod@web", "line1\nline2").unwrap();
        assert_eq!(fs::read_to_string(log_path).unwrap(), "line1\nline2");

        let _ = fs::remove_dir_all(dir);
    }
}
