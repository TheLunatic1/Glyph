#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod termius_handlers;
mod theme;
mod tray;
mod ui;

use single_instance::SingleInstance;
use std::io::Write;
use std::path::PathBuf;

fn crash_log_path() -> PathBuf {
    if let Some(base) = directories::BaseDirs::new() {
        let dir = base.home_dir().join(".glyph");
        let _ = std::fs::create_dir_all(&dir);
        return dir.join("crash.log");
    }
    std::env::temp_dir().join("glyph-crash.log")
}

fn append_crash_log(msg: &str) {
    let path = crash_log_path();
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let _ = writeln!(f, "[{ts}] {msg}");
    }
}

fn single_instance_id() -> String {
    #[cfg(target_os = "macos")]
    {
        let dir = directories::ProjectDirs::from("com", "glyph", "Glyph")
            .map(|p| p.data_local_dir().to_path_buf())
            .unwrap_or_else(std::env::temp_dir);
        let _ = std::fs::create_dir_all(&dir);
        dir.join("com.glyph.app.v3.lock")
            .to_string_lossy()
            .into_owned()
    }
    #[cfg(not(target_os = "macos"))]
    {
        "com.glyph.app.v3".to_string()
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        append_crash_log(&format!("PANIC: {info}"));
        prev_hook(info);
    }));

    let lock_id = single_instance_id();
    let _instance = match SingleInstance::new(&lock_id) {
        Ok(inst) => {
            if !inst.is_single() {
                return Ok(());
            }
            Some(inst)
        }
        Err(e) => {
            let msg = format!("SingleInstance lock warning ({lock_id}): {e}");
            log::warn!("{msg}");
            append_crash_log(&msg);
            None
        }
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _rt_guard = rt.enter();

    if let Err(e) = app::run() {
        let msg = format!("Glyph UI terminated with error: {e}");
        append_crash_log(&msg);
        anyhow::bail!("{msg}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_instance_id_is_valid() {
        let id = single_instance_id();
        assert!(!id.trim().is_empty());
        #[cfg(target_os = "macos")]
        {
            let p = std::path::Path::new(&id);
            assert!(p.is_absolute());
            assert!(p.parent().is_some_and(std::path::Path::exists));
        }
    }
}
