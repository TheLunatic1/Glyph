#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod termius_handlers;
mod theme;
mod tray;
mod ui;

use single_instance::SingleInstance;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let _instance = SingleInstance::new("com.glyph.app.v3")?;
    if !_instance.is_single() {
        return Ok(());
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _rt_guard = rt.enter();

    if let Err(e) = app::run() {
        anyhow::bail!("Glyph UI terminated with error: {e}");
    }
    Ok(())
}
