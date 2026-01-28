mod notification;
mod process_monitor;

use process_monitor::ProcessInfo;
use tauri::Emitter;

#[tauri::command]
fn get_cli_processes() -> Vec<ProcessInfo> {
    process_monitor::get_cli_processes()
}

#[tauri::command]
fn send_notification(app: tauri::AppHandle, title: String, body: String) -> Result<(), String> {
    notification::send_system_notification(&app, &title, &body)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_cli_processes,
            send_notification,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            // ���动后台监控任务
            std::thread::spawn(move || {
                let mut sitting_minutes: u32 = 0;
                let mut last_cli_waiting = false;

                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));

                    // 更新久坐时间
                    sitting_minutes += 1;

                    // 发送久坐时间更新事件
                    let _ = handle.emit("sitting-time-update", serde_json::json!({
                        "minutes": sitting_minutes
                    }));

                    // 每30分钟提醒一次
                    if sitting_minutes > 0 && sitting_minutes % 30 == 0 {
                        let _ = notification::notify_sitting_reminder(&handle, sitting_minutes);
                    }

                    // 检查CLI状态
                    let cli_waiting = process_monitor::has_waiting_cli();
                    if cli_waiting && !last_cli_waiting {
                        let _ = notification::notify_cli_waiting(&handle);
                    }
                    last_cli_waiting = cli_waiting;

                    // 发送CLI状态更新
                    let processes = process_monitor::get_cli_processes();
                    let _ = handle.emit("cli-status-update", processes);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
