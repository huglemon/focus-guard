mod notification;
mod process_monitor;

use process_monitor::ProcessInfo;
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

// 内嵌三种状态的图标
const ICON_GRAY: &[u8] = include_bytes!("../icons/tray_gray.png");
const ICON_GREEN: &[u8] = include_bytes!("../icons/tray_green.png");
const ICON_RED: &[u8] = include_bytes!("../icons/tray_red.png");

#[derive(Clone, Copy, PartialEq)]
enum TrayState {
    Gray,   // 无CLI运行
    Green,  // CLI运行中，用户交互中
    Red,    // CLI等待用户输入
}

#[derive(Clone)]
struct AppState {
    sitting_minutes: Arc<Mutex<u32>>,
    tray_state: Arc<Mutex<TrayState>>,
}

fn get_tray_icon(state: TrayState) -> Image<'static> {
    let data = match state {
        TrayState::Gray => ICON_GRAY,
        TrayState::Green => ICON_GREEN,
        TrayState::Red => ICON_RED,
    };
    Image::from_bytes(data).expect("Failed to load tray icon")
}

fn format_title(minutes: u32) -> String {
    if minutes >= 60 {
        format!("{}h{}m", minutes / 60, minutes % 60)
    } else {
        format!("{}m", minutes)
    }
}

/// 根据CLI进程状态判断托盘状态
fn determine_tray_state(processes: &[ProcessInfo]) -> TrayState {
    if processes.is_empty() {
        TrayState::Gray  // 无CLI运行
    } else {
        TrayState::Green // 有CLI运行
    }
    // 注意：红色状态（等待用户输入）需要更复杂的检测机制
    // 目前通过进程状态无法准确判断，后续可通过以下方式实现：
    // 1. 监控Claude Code的特定输出/状态文件
    // 2. 使用终端模拟器的API
    // 3. 监控stdin/stdout活动
}

#[tauri::command]
fn get_cli_processes() -> Vec<ProcessInfo> {
    process_monitor::get_cli_processes()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        sitting_minutes: Arc::new(Mutex::new(0)),
        tray_state: Arc::new(Mutex::new(TrayState::Gray)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![get_cli_processes])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_clone = state.clone();

            // 初始检测CLI状态
            let initial_processes = process_monitor::get_cli_processes();
            let initial_tray_state = determine_tray_state(&initial_processes);
            *state.tray_state.lock().unwrap() = initial_tray_state;

            let menu = build_menu(&handle, 0, initial_tray_state);

            let _tray = TrayIconBuilder::with_id("main")
                .icon(get_tray_icon(initial_tray_state))
                .title("0m")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "reset" => {
                            let mut minutes = state_clone.sitting_minutes.lock().unwrap();
                            *minutes = 0;
                            if let Some(tray) = app.tray_by_id("main") {
                                let current_state = *state_clone.tray_state.lock().unwrap();
                                let _ = tray.set_title(Some("0m"));
                                let _ = tray.set_menu(Some(build_menu(app, 0, current_state)));
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let handle_bg = handle.clone();
            let state_bg = state.clone();

            std::thread::spawn(move || {
                let mut last_tray_state = initial_tray_state;

                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5)); // 每5秒检查一次

                    // 检查CLI状态
                    let processes = process_monitor::get_cli_processes();
                    let new_tray_state = determine_tray_state(&processes);

                    // 更新状态
                    {
                        let mut ts = state_bg.tray_state.lock().unwrap();
                        *ts = new_tray_state;
                    }

                    // 状态变化时发送通知（从非红变红时）
                    if new_tray_state == TrayState::Red && last_tray_state != TrayState::Red {
                        let _ = notification::notify_cli_waiting(&handle_bg);
                    }

                    // 更新图标（状态变化时）
                    if new_tray_state != last_tray_state {
                        if let Some(tray) = handle_bg.tray_by_id("main") {
                            let _ = tray.set_icon(Some(get_tray_icon(new_tray_state)));
                            let minutes = *state_bg.sitting_minutes.lock().unwrap();
                            let _ = tray.set_menu(Some(build_menu(&handle_bg, minutes, new_tray_state)));
                        }
                    }

                    last_tray_state = new_tray_state;
                }
            });

            // 单独的久坐计时线程
            let handle_sit = handle.clone();
            let state_sit = state.clone();

            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));

                    let minutes = {
                        let mut m = state_sit.sitting_minutes.lock().unwrap();
                        *m += 1;
                        *m
                    };

                    // 每30分钟提醒久坐
                    if minutes > 0 && minutes % 30 == 0 {
                        let _ = notification::notify_sitting_reminder(&handle_sit, minutes);
                    }

                    // 更新标题
                    if let Some(tray) = handle_sit.tray_by_id("main") {
                        let _ = tray.set_title(Some(&format_title(minutes)));
                        let current_state = *state_sit.tray_state.lock().unwrap();
                        let _ = tray.set_menu(Some(build_menu(&handle_sit, minutes, current_state)));
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>, minutes: u32, tray_state: TrayState) -> Menu<R> {
    let menu = Menu::new(app).unwrap();

    // 状态说明
    let status_text = match tray_state {
        TrayState::Gray => "无CLI运行",
        TrayState::Green => "CLI运行中",
        TrayState::Red => "⚠️ CLI等待输入",
    };
    let status = MenuItem::new(app, status_text, false, None::<&str>).unwrap();

    // 久坐时间
    let time_str = if minutes >= 60 {
        format!("已坐 {}小时{}分钟", minutes / 60, minutes % 60)
    } else {
        format!("已坐 {}分钟", minutes)
    };
    let time_item = MenuItem::new(app, time_str, false, None::<&str>).unwrap();

    let reset = MenuItem::with_id(app, "reset", "重置计时", true, None::<&str>).unwrap();
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>).unwrap();

    let _ = menu.append(&status);
    let _ = menu.append(&time_item);
    let _ = menu.append(&reset);
    let _ = menu.append(&quit);

    menu
}
