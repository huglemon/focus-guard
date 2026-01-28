use tauri_plugin_notification::NotificationExt;

/// 发送系统通知
pub fn send_system_notification(
    app: &tauri::AppHandle,
    title: &str,
    body: &str,
) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

/// 发送CLI等待通知
pub fn notify_cli_waiting(app: &tauri::AppHandle) -> Result<(), String> {
    send_system_notification(app, "Focus Guard", "CLI正在等待你的输入，请查看终端！")
}

/// 发送久坐提醒
pub fn notify_sitting_reminder(app: &tauri::AppHandle, minutes: u32) -> Result<(), String> {
    let body = format!("你已经坐了{}分钟了，起来活动一下吧！", minutes);
    send_system_notification(app, "久坐提醒", &body)
}
