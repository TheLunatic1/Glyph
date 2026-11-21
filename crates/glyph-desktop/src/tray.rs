use crate::theme::LOGO_PNG;
use glyph_core::models::ServerSummary;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[derive(Debug, Clone)]
pub enum TrayCommand {
    ToggleMain,
    OpenServer(String),
    Exit,
}

pub struct TrayManager {
    _tray: TrayIcon,
    rx: std_mpsc::Receiver<TrayCommand>,
    servers_submenu: Submenu,
    menu_to_server: std::sync::Arc<std::sync::Mutex<HashMap<String, String>>>,
}

impl TrayManager {
    pub fn init(servers: &[ServerSummary]) -> Option<Self> {
        let img = image::load_from_memory(LOGO_PNG).ok()?.resize(32, 32, image::imageops::FilterType::Lanczos3);
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let icon = Icon::from_rgba(rgba.into_raw(), w, h).ok()?;

        let menu = Menu::new();
        let toggle_item = MenuItem::with_id("glyph_toggle", "Show / Hide Glyph", true, None);
        let servers_submenu = Submenu::new("Servers", true);
        let exit_item = MenuItem::with_id("glyph_exit", "Exit", true, None);

        let _ = menu.append(&toggle_item);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&servers_submenu);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&exit_item);

        let menu_to_server = std::sync::Arc::new(std::sync::Mutex::new(HashMap::new()));
        populate_submenu(&servers_submenu, servers, &menu_to_server);

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Glyph")
            .with_icon(icon)
            .build()
            .ok()?;

        let (tx, rx) = std_mpsc::channel();
        let tx_click = tx.clone();
        TrayIconEvent::set_event_handler(Some(move |ev: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                let _ = tx_click.send(TrayCommand::ToggleMain);
            }
        }));

        let map_ref = std::sync::Arc::clone(&menu_to_server);
        MenuEvent::set_event_handler(Some(move |ev: MenuEvent| {
            let id = ev.id().0.clone();
            if id == "glyph_toggle" {
                let _ = tx.send(TrayCommand::ToggleMain);
            } else if id == "glyph_exit" {
                let _ = tx.send(TrayCommand::Exit);
            } else if let Ok(guard) = map_ref.lock() {
                if let Some(server_id) = guard.get(&id) {
                    let _ = tx.send(TrayCommand::OpenServer(server_id.clone()));
                }
            }
        }));

        Some(Self {
            _tray: tray,
            rx,
            servers_submenu,
            menu_to_server,
        })
    }

    pub fn update_servers(&mut self, servers: &[ServerSummary]) {
        while self.servers_submenu.remove_at(0).is_some() {}
        populate_submenu(&self.servers_submenu, servers, &self.menu_to_server);
    }

    pub fn drain_commands(&self) -> Vec<TrayCommand> {
        let mut cmds = Vec::new();
        while let Ok(cmd) = self.rx.try_recv() {
            cmds.push(cmd);
        }
        cmds
    }
}

fn populate_submenu(
    submenu: &Submenu,
    servers: &[ServerSummary],
    map: &std::sync::Arc<std::sync::Mutex<HashMap<String, String>>>,
) {
    if let Ok(mut guard) = map.lock() {
        guard.clear();
        for s in servers {
            let item_id = format!("srv_{}", s.id);
            let label = if s.name.is_empty() { &s.host } else { &s.name };
            let item = MenuItem::with_id(&item_id, label, true, None);
            let _ = submenu.append(&item);
            guard.insert(item_id, s.id.clone());
        }
    }
}
