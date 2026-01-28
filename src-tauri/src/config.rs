use crate::i18n::Language;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri_plugin_store::StoreExt;

const CONFIG_FILE: &str = "config.json";
const KEY_SHOW_TIME: &str = "show_time_in_tray";
const KEY_POLL_INTERVAL: &str = "poll_interval_secs";
const KEY_SOUND_ENABLED: &str = "sound_enabled";
const KEY_AUTO_BRING_TO_FRONT: &str = "auto_bring_to_front";
const KEY_SITTING_REMINDER_ENABLED: &str = "sitting_reminder_enabled";
const KEY_SITTING_REMINDER_INTERVAL: &str = "sitting_reminder_interval_minutes";
const KEY_LANGUAGE: &str = "language";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub show_time_in_tray: bool,
    pub poll_interval_secs: u64,      // 监听间隔时间（秒）
    pub sound_enabled: bool,          // 是否启用声音通知
    pub auto_bring_to_front: bool,    // CLI等待时自动置顶终端
    pub sitting_reminder_enabled: bool,       // 是否启用智能久坐提醒
    pub sitting_reminder_interval_minutes: u32, // 提醒间隔（分钟）
    pub language: Language,           // 界面语言
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            show_time_in_tray: true,
            poll_interval_secs: 5,
            sound_enabled: false,          // 默认关闭，需要用户授权
            auto_bring_to_front: false,    // 默认关闭，需要辅助功能权限
            sitting_reminder_enabled: true, // 默认开启
            sitting_reminder_interval_minutes: 40, // 默认40分钟
            language: Language::default(), // 默认英文
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
            let mut config = self.config.lock().unwrap();

            if let Some(value) = store.get(KEY_SHOW_TIME) {
                if let Some(v) = value.as_bool() {
                    config.show_time_in_tray = v;
                }
            }
            if let Some(value) = store.get(KEY_POLL_INTERVAL) {
                if let Some(v) = value.as_u64() {
                    config.poll_interval_secs = v.max(1); // 最小1秒
                }
            }
            if let Some(value) = store.get(KEY_SOUND_ENABLED) {
                if let Some(v) = value.as_bool() {
                    config.sound_enabled = v;
                }
            }
            if let Some(value) = store.get(KEY_AUTO_BRING_TO_FRONT) {
                if let Some(v) = value.as_bool() {
                    config.auto_bring_to_front = v;
                }
            }
            if let Some(value) = store.get(KEY_SITTING_REMINDER_ENABLED) {
                if let Some(v) = value.as_bool() {
                    config.sitting_reminder_enabled = v;
                }
            }
            if let Some(value) = store.get(KEY_SITTING_REMINDER_INTERVAL) {
                if let Some(v) = value.as_u64() {
                    config.sitting_reminder_interval_minutes = v.max(1) as u32;
                }
            }
            if let Some(value) = store.get(KEY_LANGUAGE) {
                if let Some(v) = value.as_str() {
                    config.language = match v {
                        "Chinese" => Language::Chinese,
                        _ => Language::English,
                    };
                }
            }
        }
    }

    pub fn save(&self, app: &tauri::AppHandle) {
        if let Ok(store) = app.store(CONFIG_FILE) {
            let config = self.config.lock().unwrap();
            let _ = store.set(KEY_SHOW_TIME, config.show_time_in_tray);
            let _ = store.set(KEY_POLL_INTERVAL, config.poll_interval_secs);
            let _ = store.set(KEY_SOUND_ENABLED, config.sound_enabled);
            let _ = store.set(KEY_AUTO_BRING_TO_FRONT, config.auto_bring_to_front);
            let _ = store.set(KEY_SITTING_REMINDER_ENABLED, config.sitting_reminder_enabled);
            let _ = store.set(
                KEY_SITTING_REMINDER_INTERVAL,
                config.sitting_reminder_interval_minutes,
            );
            let lang_str = match config.language {
                Language::English => "English",
                Language::Chinese => "Chinese",
            };
            let _ = store.set(KEY_LANGUAGE, lang_str);
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

    #[allow(dead_code)]
    pub fn get_poll_interval(&self) -> u64 {
        self.config.lock().unwrap().poll_interval_secs
    }

    #[allow(dead_code)]
    pub fn set_poll_interval(&self, secs: u64) {
        self.config.lock().unwrap().poll_interval_secs = secs.max(1);
    }

    pub fn get_sound_enabled(&self) -> bool {
        self.config.lock().unwrap().sound_enabled
    }

    pub fn toggle_sound(&self) -> bool {
        let mut config = self.config.lock().unwrap();
        config.sound_enabled = !config.sound_enabled;
        config.sound_enabled
    }

    pub fn get_auto_bring_to_front(&self) -> bool {
        self.config.lock().unwrap().auto_bring_to_front
    }

    pub fn toggle_auto_bring_to_front(&self) -> bool {
        let mut config = self.config.lock().unwrap();
        config.auto_bring_to_front = !config.auto_bring_to_front;
        config.auto_bring_to_front
    }

    pub fn get_sitting_reminder_enabled(&self) -> bool {
        self.config.lock().unwrap().sitting_reminder_enabled
    }

    pub fn toggle_sitting_reminder(&self) -> bool {
        let mut config = self.config.lock().unwrap();
        config.sitting_reminder_enabled = !config.sitting_reminder_enabled;
        config.sitting_reminder_enabled
    }

    pub fn get_sitting_reminder_interval(&self) -> u32 {
        self.config.lock().unwrap().sitting_reminder_interval_minutes
    }

    /// 循环切换提醒间隔：20 -> 30 -> 40 -> 50 -> 60 -> 20
    pub fn cycle_sitting_reminder_interval(&self) -> u32 {
        let mut config = self.config.lock().unwrap();
        config.sitting_reminder_interval_minutes = match config.sitting_reminder_interval_minutes {
            20 => 30,
            30 => 40,
            40 => 50,
            50 => 60,
            _ => 20,
        };
        config.sitting_reminder_interval_minutes
    }

    pub fn get_language(&self) -> Language {
        self.config.lock().unwrap().language
    }

    pub fn toggle_language(&self) -> Language {
        let mut config = self.config.lock().unwrap();
        config.language = config.language.toggle();
        config.language
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
