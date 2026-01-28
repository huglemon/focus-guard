use crate::i18n::{format_sitting_reminder, format_smart_reminder, get_strings, Language};
use tauri_plugin_notification::NotificationExt;

/// 发送系统通知（带声音选项）
pub fn send_system_notification(
    app: &tauri::AppHandle,
    title: &str,
    body: &str,
    with_sound: bool,
) -> Result<(), String> {
    let mut builder = app.notification().builder();
    builder = builder.title(title).body(body);

    if with_sound {
        builder = builder.sound("default");
    }

    let result = builder.show().map_err(|e| e.to_string());
    match &result {
        Ok(_) => println!("Notification sent: {} - {}", title, body),
        Err(e) => println!("Notification failed: {}", e),
    }
    result
}

/// 发送CLI等待通知
pub fn notify_cli_waiting(
    app: &tauri::AppHandle,
    lang: Language,
    with_sound: bool,
) -> Result<(), String> {
    let s = get_strings(lang);
    send_system_notification(app, s.app_name, s.cli_waiting, with_sound)
}

/// 发送久坐提醒
#[allow(dead_code)]
pub fn notify_sitting_reminder(
    app: &tauri::AppHandle,
    lang: Language,
    minutes: u32,
    with_sound: bool,
) -> Result<(), String> {
    let s = get_strings(lang);
    let body = format_sitting_reminder(lang, minutes);
    send_system_notification(app, s.sitting_reminder_title, &body, with_sound)
}

/// 发送智能久坐提醒（CLI交互时触发）
pub fn notify_smart_sitting_reminder(
    app: &tauri::AppHandle,
    lang: Language,
    minutes: u32,
    with_sound: bool,
) -> Result<(), String> {
    let s = get_strings(lang);
    let body = format_smart_reminder(lang, minutes);
    send_system_notification(app, s.smart_reminder_title, &body, with_sound)
}

/// 发送声音通知已开启的提示
pub fn notify_sound_enabled(app: &tauri::AppHandle, lang: Language) -> Result<(), String> {
    let s = get_strings(lang);
    send_system_notification(app, s.app_name, s.sound_enabled_msg, true)
}

/// 请求通知权限
pub fn request_notification_permission(app: &tauri::AppHandle) -> Result<bool, String> {
    app.notification()
        .request_permission()
        .map(|state| matches!(state, tauri_plugin_notification::PermissionState::Granted))
        .map_err(|e| e.to_string())
}

/// 检查通知权限状态
#[allow(dead_code)]
pub fn check_notification_permission(app: &tauri::AppHandle) -> Result<bool, String> {
    app.notification()
        .permission_state()
        .map(|state| matches!(state, tauri_plugin_notification::PermissionState::Granted))
        .map_err(|e| e.to_string())
}
