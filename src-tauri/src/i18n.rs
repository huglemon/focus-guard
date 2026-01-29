use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Language {
    English,
    Chinese,
}

impl Default for Language {
    fn default() -> Self {
        Language::Chinese
    }
}

impl Language {
    pub fn toggle(&self) -> Self {
        match self {
            Language::English => Language::Chinese,
            Language::Chinese => Language::English,
        }
    }
}

pub struct Strings {
    // Menu items
    pub no_cli_running: &'static str,
    pub sitting_time_hours: &'static str,  // "已坐 {}小时{}分钟" / "Sitting: {}h {}m"
    pub sitting_time_minutes: &'static str, // "已坐 {}分钟" / "Sitting: {}m"
    pub show_time: &'static str,
    pub sound_notification: &'static str,
    pub auto_bring_to_front: &'static str,
    pub auto_start: &'static str,
    pub smart_sitting_reminder: &'static str,
    pub reminder_interval: &'static str,  // "提醒间隔: {}分钟" / "Interval: {}m"
    pub reset_timer: &'static str,
    pub quit: &'static str,
    pub language: &'static str,

    // Update related
    pub check_update: &'static str,
    pub version: &'static str,  // "版本: {}" / "Version: {}"
    pub checking_update: &'static str,
    pub update_available: &'static str,  // "发现新版本: {}" / "New version: {}"
    pub no_update: &'static str,
    pub update_error: &'static str,
    pub downloading: &'static str,
    pub install_restart: &'static str,

    // Notifications
    pub app_name: &'static str,
    pub cli_waiting: &'static str,
    pub sitting_reminder_title: &'static str,
    pub sitting_reminder_body: &'static str,  // "你已经坐了{}分钟了，起来活动一下吧！"
    pub smart_reminder_title: &'static str,
    pub smart_reminder_body: &'static str,    // "你已经连续工作{}分钟了！\n休息2分钟后自动重置计时"
    pub sound_enabled_msg: &'static str,
}

const ENGLISH: Strings = Strings {
    // Menu items
    no_cli_running: "No CLI running",
    sitting_time_hours: "Sitting: {}h {}m",
    sitting_time_minutes: "Sitting: {}m",
    show_time: "Show Time",
    sound_notification: "Sound Notification",
    auto_bring_to_front: "Auto Bring to Front",
    auto_start: "Launch at Login",
    smart_sitting_reminder: "Smart Sitting Reminder",
    reminder_interval: "Interval: {}m",
    reset_timer: "Reset Timer",
    quit: "Quit",
    language: "中文",

    // Update related
    check_update: "Check for Updates",
    version: "Version: {}",
    checking_update: "Checking...",
    update_available: "New version: {}",
    no_update: "Already up to date",
    update_error: "Update check failed",
    downloading: "Downloading...",
    install_restart: "Install & Restart",

    // Notifications
    app_name: "Focus Guard",
    cli_waiting: "CLI is waiting for your input!",
    sitting_reminder_title: "Sitting Reminder",
    sitting_reminder_body: "You've been sitting for {} minutes. Time to stretch!",
    smart_reminder_title: "Time for a Break",
    smart_reminder_body: "You've been working for {} minutes!\nTimer resets after 2 min of inactivity",
    sound_enabled_msg: "Sound notification enabled",
};

const CHINESE: Strings = Strings {
    // Menu items
    no_cli_running: "无CLI运行",
    sitting_time_hours: "已坐 {}小时{}分钟",
    sitting_time_minutes: "已坐 {}分钟",
    show_time: "显示时间",
    sound_notification: "声音通知",
    auto_bring_to_front: "自动置顶终端",
    auto_start: "开机自动启动",
    smart_sitting_reminder: "智能久坐提醒",
    reminder_interval: "提醒间隔: {}分钟",
    reset_timer: "重置计时",
    quit: "退出",
    language: "English",

    // Update related
    check_update: "检查更新",
    version: "版本: {}",
    checking_update: "检查中...",
    update_available: "发现新版本: {}",
    no_update: "已是最新版本",
    update_error: "检查更新失败",
    downloading: "下载中...",
    install_restart: "安装并重启",

    // Notifications
    app_name: "Focus Guard",
    cli_waiting: "CLI正在等待你的输入，请查看终端！",
    sitting_reminder_title: "久坐提醒",
    sitting_reminder_body: "你已经坐了{}分钟了，起来活动一下吧！",
    smart_reminder_title: "该休息了",
    smart_reminder_body: "你已经连续工作{}分钟了！\n休息2分钟后自动重置计时",
    sound_enabled_msg: "声音通知已开启",
};

pub fn get_strings(lang: Language) -> &'static Strings {
    match lang {
        Language::English => &ENGLISH,
        Language::Chinese => &CHINESE,
    }
}

/// Format sitting time string
pub fn format_sitting_time(lang: Language, minutes: u32) -> String {
    let s = get_strings(lang);
    if minutes >= 60 {
        s.sitting_time_hours
            .replace("{}", &(minutes / 60).to_string())
            .replacen("{}", &(minutes % 60).to_string(), 1)
    } else {
        s.sitting_time_minutes.replace("{}", &minutes.to_string())
    }
}

/// Format reminder interval string
pub fn format_interval(lang: Language, minutes: u32) -> String {
    get_strings(lang).reminder_interval.replace("{}", &minutes.to_string())
}

/// Format sitting reminder body
pub fn format_sitting_reminder(lang: Language, minutes: u32) -> String {
    get_strings(lang).sitting_reminder_body.replace("{}", &minutes.to_string())
}

/// Format smart reminder body
pub fn format_smart_reminder(lang: Language, minutes: u32) -> String {
    get_strings(lang).smart_reminder_body.replace("{}", &minutes.to_string())
}

/// Format version string
pub fn format_version(lang: Language, version: &str) -> String {
    get_strings(lang).version.replace("{}", version)
}

/// Format update available string
pub fn format_update_available(lang: Language, version: &str) -> String {
    get_strings(lang).update_available.replace("{}", version)
}
