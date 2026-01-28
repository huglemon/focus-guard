use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri_plugin_store::StoreExt;

const CONFIG_FILE: &str = "config.json";
const KEY_SHOW_TIME: &str = "show_time_in_tray";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub show_time_in_tray: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            show_time_in_tray: true,
        }
    }
}

pub struct ConfigManager {
    config: Arc<Mutex<AppConfig>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(AppConfig::default())),
        }
    }

    pub fn load(&self, app: &tauri::AppHandle) {
        if let Ok(store) = app.store(CONFIG_FILE) {
            if let Some(value) = store.get(KEY_SHOW_TIME) {
                if let Some(show_time) = value.as_bool() {
                    let mut config = self.config.lock().unwrap();
                    config.show_time_in_tray = show_time;
                }
            }
        }
    }

    pub fn save(&self, app: &tauri::AppHandle) {
        if let Ok(store) = app.store(CONFIG_FILE) {
            let config = self.config.lock().unwrap();
            let _ = store.set(KEY_SHOW_TIME, config.show_time_in_tray);
            let _ = store.save();
        }
    }

    pub fn get_show_time(&self) -> bool {
        self.config.lock().unwrap().show_time_in_tray
    }

    #[allow(dead_code)]
    pub fn set_show_time(&self, show: bool) {
        self.config.lock().unwrap().show_time_in_tray = show;
    }

    pub fn toggle_show_time(&self) -> bool {
        let mut config = self.config.lock().unwrap();
        config.show_time_in_tray = !config.show_time_in_tray;
        config.show_time_in_tray
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
