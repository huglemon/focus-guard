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
        Ok(_) => println!("通知发送成功: {} - {}", title, body),
        Err(e) => println!("通知发送失败: {}", e),
    }
    result
}

/// 发送CLI等待通知
pub fn notify_cli_waiting(app: &tauri::AppHandle, with_sound: bool) -> Result<(), String> {
    send_system_notification(
        app,
        "Focus Guard",
        "CLI正在等待你的输入，请查看终端！",
        with_sound,
    )
}

/// 发送久坐提醒
#[allow(dead_code)]
pub fn notify_sitting_reminder(
    app: &tauri::AppHandle,
    minutes: u32,
    with_sound: bool,
) -> Result<(), String> {
    let body = format!("你已经坐了{}分钟了，起来活动一下吧！", minutes);
    send_system_notification(app, "久坐提醒", &body, with_sound)
}

/// 发送智能久坐提醒（CLI交互时触发）
pub fn notify_smart_sitting_reminder(
    app: &tauri::AppHandle,
    minutes: u32,
    with_sound: bool,
) -> Result<(), String> {
    let body = format!(
        "你已经连续工作{}分钟了！\n休息2分钟后自动重置计时",
        minutes
    );
    send_system_notification(app, "该休息了", &body, with_sound)
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
