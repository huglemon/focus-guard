mod config;
mod ipc_server;
mod notification;
mod process_monitor;
mod state_manager;
mod window_manager;

use config::ConfigManager;
use process_monitor::ProcessInfo;
use state_manager::{CliState, CliStatus, StateManager};
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, IconMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

// 内嵌三种状态的图标
const ICON_GRAY: &[u8] = include_bytes!("../icons/tray_gray.png");
const ICON_GREEN: &[u8] = include_bytes!("../icons/tray_green.png");
const ICON_RED: &[u8] = include_bytes!("../icons/tray_red.png");

#[derive(Clone, Copy, PartialEq)]
enum TrayState {
    Gray,  // 无CLI运行
    Green, // CLI运行中，用户交互中
    Red,   // CLI等待用户输入
}

impl From<CliState> for TrayState {
    fn from(state: CliState) -> Self {
        match state {
            CliState::Working => TrayState::Green,
            CliState::WaitingInput => TrayState::Red,
            CliState::Idle => TrayState::Red, // Idle 也显示红色提醒用户
            CliState::Offline => TrayState::Gray,
        }
    }
}

#[derive(Clone)]
struct AppState {
    sitting_minutes: Arc<Mutex<u32>>,
    tray_state: Arc<Mutex<TrayState>>,
    config: Arc<ConfigManager>,
    cli_states: Arc<Mutex<HashMap<String, CliStatus>>>,
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

/// 根据CLI进程状态判断托盘状态（兜底检测）
fn determine_tray_state_from_processes(processes: &[ProcessInfo]) -> TrayState {
    if processes.is_empty() {
        TrayState::Gray // 无CLI运行
    } else {
        TrayState::Green // 有CLI运行
    }
}

#[tauri::command]
fn get_cli_processes() -> Vec<ProcessInfo> {
    process_monitor::get_cli_processes()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 创建状态管理器并获取共享状态
    let state_manager = StateManager::new();
    let cli_states = state_manager.get_states();

    let state = AppState {
        sitting_minutes: Arc::new(Mutex::new(0)),
        tray_state: Arc::new(Mutex::new(TrayState::Gray)),
        config: Arc::new(ConfigManager::new()),
        cli_states: cli_states.clone(),
    };

    // 创建 IPC 通道
    let (ipc_sender, ipc_receiver) = mpsc::channel();

    // 启动 IPC 服务器
    let ipc_sender_clone = ipc_sender.clone();
    std::thread::spawn(move || {
        ipc_server::start_ipc_server(ipc_sender_clone);
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![get_cli_processes])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_clone = state.clone();

            // 加载配置
            state.config.load(&handle);

            // 初始检测CLI状态
            let initial_processes = process_monitor::get_cli_processes();
            let initial_tray_state = determine_tray_state_from_processes(&initial_processes);
            *state.tray_state.lock().unwrap() = initial_tray_state;

            let show_time = state.config.get_show_time();
            let cli_states_snapshot: Vec<CliStatus> =
                state.cli_states.lock().unwrap().values().cloned().collect();
            let menu = build_menu(
                &handle,
                0,
                initial_tray_state,
                &cli_states_snapshot,
                &state.config,
            );

            let initial_title = if show_time {
                Some("0m".to_string())
            } else {
                None
            };

            let _tray = TrayIconBuilder::with_id("main")
                .icon(get_tray_icon(initial_tray_state))
                .title(initial_title.as_deref().unwrap_or(""))
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "reset" => {
                            let mut minutes = state_clone.sitting_minutes.lock().unwrap();
                            *minutes = 0;
                            if let Some(tray) = app.tray_by_id("main") {
                                let current_state = *state_clone.tray_state.lock().unwrap();
                                let show_time = state_clone.config.get_show_time();
                                if show_time {
                                    let _ = tray.set_title(Some("0m"));
                                }
                                let cli_states_snapshot: Vec<CliStatus> = state_clone
                                    .cli_states
                                    .lock()
                                    .unwrap()
                                    .values()
                                    .cloned()
                                    .collect();
                                let _ = tray.set_menu(Some(build_menu(
                                    app,
                                    0,
                                    current_state,
                                    &cli_states_snapshot,
                                    &state_clone.config,
                                )));
                            }
                        }
                        "toggle_time" => {
                            let new_show_time = state_clone.config.toggle_show_time();
                            state_clone.config.save(app);
                            if let Some(tray) = app.tray_by_id("main") {
                                let minutes = *state_clone.sitting_minutes.lock().unwrap();
                                let current_state = *state_clone.tray_state.lock().unwrap();
                                if new_show_time {
                                    let _ = tray.set_title(Some(&format_title(minutes)));
                                } else {
                                    let _ = tray.set_title(Some(""));
                                }
                                let cli_states_snapshot: Vec<CliStatus> = state_clone
                                    .cli_states
                                    .lock()
                                    .unwrap()
                                    .values()
                                    .cloned()
                                    .collect();
                                let _ = tray.set_menu(Some(build_menu(
                                    app,
                                    minutes,
                                    current_state,
                                    &cli_states_snapshot,
                                    &state_clone.config,
                                )));
                            }
                        }
                        "toggle_sound" => {
                            let new_enabled = state_clone.config.toggle_sound();
                            state_clone.config.save(app);

                            // 如果开启，请求通知权限并发送测试通知
                            if new_enabled {
                                // 先请求权限
                                match notification::request_notification_permission(app) {
                                    Ok(granted) => {
                                        println!("通知权限状态: {}", if granted { "已授权" } else { "未授权" });
                                        if granted {
                                            let _ = notification::send_system_notification(
                                                app,
                                                "Focus Guard",
                                                "声音通知已开启",
                                                true,
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        println!("请求通知权限失败: {}", e);
                                        // 即使请求失败也尝试发送通知（可能会触发系统权限弹窗）
                                        let _ = notification::send_system_notification(
                                            app,
                                            "Focus Guard",
                                            "声音通知已开启",
                                            true,
                                        );
                                    }
                                }
                            }

                            if let Some(tray) = app.tray_by_id("main") {
                                let minutes = *state_clone.sitting_minutes.lock().unwrap();
                                let current_state = *state_clone.tray_state.lock().unwrap();
                                let cli_states_snapshot: Vec<CliStatus> = state_clone
                                    .cli_states
                                    .lock()
                                    .unwrap()
                                    .values()
                                    .cloned()
                                    .collect();
                                let _ = tray.set_menu(Some(build_menu(
                                    app,
                                    minutes,
                                    current_state,
                                    &cli_states_snapshot,
                                    &state_clone.config,
                                )));
                            }
                        }
                        "toggle_front" => {
                            let new_enabled = state_clone.config.toggle_auto_bring_to_front();
                            state_clone.config.save(app);

                            // 如果开启，测试置顶功能
                            if new_enabled {
                                let _ = window_manager::bring_terminal_to_front();
                            }

                            if let Some(tray) = app.tray_by_id("main") {
                                let minutes = *state_clone.sitting_minutes.lock().unwrap();
                                let current_state = *state_clone.tray_state.lock().unwrap();
                                let cli_states_snapshot: Vec<CliStatus> = state_clone
                                    .cli_states
                                    .lock()
                                    .unwrap()
                                    .values()
                                    .cloned()
                                    .collect();
                                let _ = tray.set_menu(Some(build_menu(
                                    app,
                                    minutes,
                                    current_state,
                                    &cli_states_snapshot,
                                    &state_clone.config,
                                )));
                            }
                        }
                        "quit" => {
                            // 清理 IPC socket
                            ipc_server::cleanup();
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // 启动状态管理器
            let handle_state = handle.clone();
            let state_for_manager = state.clone();

            state_manager.start(ipc_receiver, move |cli_state| {
                let new_tray_state: TrayState = cli_state.into();
                let mut current = state_for_manager.tray_state.lock().unwrap();
                let old_state = *current;

                if new_tray_state != old_state {
                    *current = new_tray_state;
                    drop(current);

                    // 状态变化时发送通知和置顶（从非红变红时）
                    if new_tray_state == TrayState::Red && old_state != TrayState::Red {
                        // 只有开启声音通知时才发送通知
                        if state_for_manager.config.get_sound_enabled() {
                            let _ = notification::notify_cli_waiting(&handle_state, true);
                        }

                        // 自动置顶终端
                        if state_for_manager.config.get_auto_bring_to_front() {
                            let _ = window_manager::bring_terminal_to_front();
                        }
                    }

                    // 更新图标和菜单
                    if let Some(tray) = handle_state.tray_by_id("main") {
                        let _ = tray.set_icon(Some(get_tray_icon(new_tray_state)));
                        let minutes = *state_for_manager.sitting_minutes.lock().unwrap();
                        let cli_states_snapshot: Vec<CliStatus> = state_for_manager
                            .cli_states
                            .lock()
                            .unwrap()
                            .values()
                            .cloned()
                            .collect();
                        let _ = tray.set_menu(Some(build_menu(
                            &handle_state,
                            minutes,
                            new_tray_state,
                            &cli_states_snapshot,
                            &state_for_manager.config,
                        )));
                    }
                }
            });

            // 兜底进程检测线程
            // 对于没有配置 hooks 的 CLI（如 Codex），通过进程检测来补充状态
            let _handle_bg = handle.clone();
            let _state_bg = state.clone();
            let cli_states_bg = cli_states.clone();
            let ipc_sender_bg = ipc_sender.clone();

            std::thread::spawn(move || {
                // 已知配置了 hooks 的 CLI
                let hooked_clis = ["claude", "gemini"];

                loop {
                    std::thread::sleep(std::time::Duration::from_secs(10)); // 每10秒检查一次

                    // 获取所有 CLI 进程
                    let processes = process_monitor::get_cli_processes();

                    // 检查哪些 CLI 没有通过 hooks 报告状态
                    let hook_states = cli_states_bg.lock().unwrap();

                    for process in &processes {
                        let cli_name = process.name.to_lowercase();

                        // 检查是否是没有配置 hooks 的 CLI
                        let is_hooked = hooked_clis.iter().any(|&h| cli_name.contains(h));

                        if !is_hooked {
                            // 对于没有 hooks 的 CLI（如 codex），检查是否已有状态
                            let has_state = hook_states.keys().any(|k| cli_name.contains(k));

                            if !has_state {
                                // 通过 IPC 通道模拟一个 Working 事件
                                let msg = ipc_server::CliMessage {
                                    cli: cli_name.clone(),
                                    event: ipc_server::CliEvent::Working,
                                    pid: Some(process.pid),
                                    timestamp: None,
                                    session_id: None,
                                    cwd: None,
                                };
                                let _ = ipc_sender_bg.send(msg);
                                println!("Fallback detection: {} (PID: {})", cli_name, process.pid);
                            }
                        }
                    }

                    drop(hook_states);

                    // 检查已记录但进程已退出的 CLI
                    let hook_states = cli_states_bg.lock().unwrap();
                    let active_process_names: Vec<String> =
                        processes.iter().map(|p| p.name.to_lowercase()).collect();

                    // 找出需要标记为 Offline 的 CLI
                    let offline_clis: Vec<String> = hook_states
                        .keys()
                        .filter(|cli| {
                            // 如果是没有 hooks 的 CLI，检查进程是否还在运行
                            let is_hooked = hooked_clis.iter().any(|&h| cli.contains(h));
                            if !is_hooked {
                                !active_process_names
                                    .iter()
                                    .any(|p| p.contains(cli.as_str()))
                            } else {
                                false
                            }
                        })
                        .cloned()
                        .collect();

                    drop(hook_states);

                    // 发送 SessionEnd 事件
                    for cli in offline_clis {
                        let msg = ipc_server::CliMessage {
                            cli: cli.clone(),
                            event: ipc_server::CliEvent::SessionEnd,
                            pid: None,
                            timestamp: None,
                            session_id: None,
                            cwd: None,
                        };
                        let _ = ipc_sender_bg.send(msg);
                        println!("Fallback detection: {} exited", cli);
                    }
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
                        let sound_enabled = state_sit.config.get_sound_enabled();
                        let _ = notification::notify_sitting_reminder(&handle_sit, minutes, sound_enabled);
                    }

                    // 更新标题和菜单
                    if let Some(tray) = handle_sit.tray_by_id("main") {
                        let show_time = state_sit.config.get_show_time();
                        if show_time {
                            let _ = tray.set_title(Some(&format_title(minutes)));
                        }
                        let current_state = *state_sit.tray_state.lock().unwrap();
                        let cli_states_snapshot: Vec<CliStatus> = state_sit
                            .cli_states
                            .lock()
                            .unwrap()
                            .values()
                            .cloned()
                            .collect();
                        let _ = tray.set_menu(Some(build_menu(
                            &handle_sit,
                            minutes,
                            current_state,
                            &cli_states_snapshot,
                            &state_sit.config,
                        )));
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    minutes: u32,
    _tray_state: TrayState,
    cli_states: &[CliStatus],
    config: &ConfigManager,
) -> Menu<R> {
    let menu = Menu::new(app).unwrap();

    // 显示各个 CLI 的状态
    let active_clis: Vec<&CliStatus> = cli_states
        .iter()
        .filter(|s| s.state != CliState::Offline)
        .collect();

    if active_clis.is_empty() {
        // 无 CLI 运行
        let status = MenuItem::new(app, "无CLI运行", false, None::<&str>).unwrap();
        let _ = menu.append(&status);
    } else {
        // 显示每个 CLI 的状态
        for cli_status in &active_clis {
            let icon_data = match cli_status.state {
                CliState::Working => ICON_GREEN,
                CliState::WaitingInput => ICON_RED,
                CliState::Idle => ICON_GRAY,
                CliState::Offline => continue,
            };
            let icon = Image::from_bytes(icon_data).ok();

            let cli_item = IconMenuItem::new(
                app,
                &cli_status.display_name,
                false,
                icon,
                None::<&str>,
            )
            .unwrap();
            let _ = menu.append(&cli_item);
        }
    }

    // 分隔线
    let separator1 = PredefinedMenuItem::separator(app).unwrap();
    let _ = menu.append(&separator1);

    // 久坐时间
    let time_str = if minutes >= 60 {
        format!("已坐 {}小时{}分钟", minutes / 60, minutes % 60)
    } else {
        format!("已坐 {}分钟", minutes)
    };
    let time_item = MenuItem::new(app, time_str, false, None::<&str>).unwrap();
    let _ = menu.append(&time_item);

    // 分隔线
    let separator2 = PredefinedMenuItem::separator(app).unwrap();
    let _ = menu.append(&separator2);

    // 设置选项
    let show_time = config.get_show_time();
    let toggle_time = CheckMenuItem::with_id(
        app,
        "toggle_time",
        "显示时间",
        true,
        show_time,
        None::<&str>,
    )
    .unwrap();
    let _ = menu.append(&toggle_time);

    let sound_enabled = config.get_sound_enabled();
    let toggle_sound = CheckMenuItem::with_id(
        app,
        "toggle_sound",
        "声音通知",
        true,
        sound_enabled,
        None::<&str>,
    )
    .unwrap();
    let _ = menu.append(&toggle_sound);

    let auto_front = config.get_auto_bring_to_front();
    let toggle_front = CheckMenuItem::with_id(
        app,
        "toggle_front",
        "自动置顶终端",
        true,
        auto_front,
        None::<&str>,
    )
    .unwrap();
    let _ = menu.append(&toggle_front);

    // 分隔线
    let separator3 = PredefinedMenuItem::separator(app).unwrap();
    let _ = menu.append(&separator3);

    // 重置计时
    let reset = MenuItem::with_id(app, "reset", "重置计时", true, None::<&str>).unwrap();
    let _ = menu.append(&reset);

    // 退出
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>).unwrap();
    let _ = menu.append(&quit);

    menu
}
