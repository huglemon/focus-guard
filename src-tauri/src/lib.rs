mod activity_monitor;
mod config;
mod i18n;
mod ipc_server;
mod notification;
mod process_monitor;
mod state_manager;
mod updater;
mod window_manager;

use activity_monitor::ActivityMonitor;
use config::ConfigManager;
use i18n::{format_interval, format_sitting_time, format_update_available, format_version, get_strings};
use process_monitor::ProcessInfo;
use state_manager::{CliState, CliStatus, StateChangeEvent, StateManager};
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
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

/// 智能久坐提醒状态
struct SittingReminderState {
    awaiting_standup: bool,              // 是否等待用户站起来
    reminder_sent_at: Option<Instant>,   // 发送提醒的时间
}

impl Default for SittingReminderState {
    fn default() -> Self {
        Self {
            awaiting_standup: false,
            reminder_sent_at: None,
        }
    }
}

#[derive(Clone)]
struct AppState {
    sitting_minutes: Arc<Mutex<u32>>,
    tray_state: Arc<Mutex<TrayState>>,
    config: Arc<ConfigManager>,
    cli_states: Arc<Mutex<HashMap<String, CliStatus>>>,
    activity_monitor: Arc<ActivityMonitor>,
    sitting_reminder: Arc<Mutex<SittingReminderState>>,
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

    // 创建活动监听器
    let activity_monitor = Arc::new(ActivityMonitor::new());

    let state = AppState {
        sitting_minutes: Arc::new(Mutex::new(0)),
        tray_state: Arc::new(Mutex::new(TrayState::Gray)),
        config: Arc::new(ConfigManager::new()),
        cli_states: cli_states.clone(),
        activity_monitor: activity_monitor.clone(),
        sitting_reminder: Arc::new(Mutex::new(SittingReminderState::default())),
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![get_cli_processes])
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_clone = state.clone();

            // 加载配置
            state.config.load(&handle);

            // 初始检测CLI状态，并添加到状态列表
            let initial_processes = process_monitor::get_cli_processes();
            let initial_tray_state = determine_tray_state_from_processes(&initial_processes);
            *state.tray_state.lock().unwrap() = initial_tray_state;

            // 把检测到的进程添加到 cli_states
            {
                let mut cli_states = state.cli_states.lock().unwrap();
                for process in &initial_processes {
                    let key = process.name.to_lowercase();
                    let status = CliStatus::with_details(
                        process.name.clone(),
                        None, // 没有 session_id
                        process.cwd.clone(),
                    );
                    // 设置为 Working 状态（因为进程正在运行）
                    let mut status = status;
                    status.state = CliState::Working;
                    status.pid = Some(process.pid);
                    cli_states.insert(key, status);
                }
            }

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
                                let lang = state_clone.config.get_language();
                                // 先请求权限
                                match notification::request_notification_permission(app) {
                                    Ok(granted) => {
                                        println!("Notification permission: {}", if granted { "granted" } else { "denied" });
                                        if granted {
                                            let _ = notification::notify_sound_enabled(app, lang);
                                        }
                                    }
                                    Err(e) => {
                                        println!("Request notification permission failed: {}", e);
                                        // 即使请求失败也尝试发送通知（可能会触发系统权限弹窗）
                                        let _ = notification::notify_sound_enabled(app, lang);
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
                        "toggle_sitting_reminder" => {
                            let new_enabled = state_clone.config.toggle_sitting_reminder();
                            state_clone.config.save(app);

                            // 如果开启，启动活动监听器
                            if new_enabled {
                                state_clone.activity_monitor.start();
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
                        "cycle_interval" => {
                            let _new_interval = state_clone.config.cycle_sitting_reminder_interval();
                            state_clone.config.save(app);

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
                        "toggle_lang" => {
                            let _new_lang = state_clone.config.toggle_language();
                            state_clone.config.save(app);

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
                        "check_update" => {
                            let app_handle = app.clone();
                            let state_for_update = state_clone.clone();
                            tauri::async_runtime::spawn(async move {
                                let lang = state_for_update.config.get_language();
                                let s = get_strings(lang);

                                match updater::check_for_update(&app_handle).await {
                                    Ok(Some(version)) => {
                                        // 有新版本，发送通知
                                        let msg = format_update_available(lang, &version);
                                        let _ = notification::send_system_notification(&app_handle, s.app_name, &msg, false);
                                    }
                                    Ok(None) => {
                                        // 已是最新版本
                                        let _ = notification::send_system_notification(&app_handle, s.app_name, s.no_update, false);
                                    }
                                    Err(e) => {
                                        // 检查失败
                                        println!("Update check error: {}", e);
                                        let _ = notification::send_system_notification(&app_handle, s.app_name, s.update_error, false);
                                    }
                                }
                            });
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

            state_manager.start(ipc_receiver, move |event: StateChangeEvent| {
                let new_tray_state: TrayState = event.state.into();
                let mut current = state_for_manager.tray_state.lock().unwrap();
                let old_state = *current;

                // 更新托盘状态
                if new_tray_state != old_state {
                    *current = new_tray_state;
                }
                drop(current);

                // 只有聚合状态变化时才发送通知和置顶（从非红变红时）
                if event.state_changed
                    && new_tray_state == TrayState::Red
                    && old_state != TrayState::Red
                {
                    // 只有开启声音通知时才发送通知
                    if state_for_manager.config.get_sound_enabled() {
                        let lang = state_for_manager.config.get_language();
                        let _ = notification::notify_cli_waiting(&handle_state, lang, true);
                    }

                    // 智能置顶：使用 PID 和 CWD 激活正确的应用和窗口
                    if state_for_manager.config.get_auto_bring_to_front() {
                        let _ = window_manager::bring_cli_to_front(
                            event.pid,
                            event.cwd.as_deref(),
                        );
                    }
                }

                // 智能久坐提醒：在 CLI Working 事件时检查是否需要提醒
                if event.state == CliState::Working
                    && state_for_manager.config.get_sitting_reminder_enabled()
                {
                    let mut reminder = state_for_manager.sitting_reminder.lock().unwrap();
                    if !reminder.awaiting_standup {
                        let minutes = *state_for_manager.sitting_minutes.lock().unwrap();
                        let threshold = state_for_manager.config.get_sitting_reminder_interval();
                        if minutes >= threshold {
                            // 发送久坐提醒
                            let sound_enabled = state_for_manager.config.get_sound_enabled();
                            let lang = state_for_manager.config.get_language();
                            let _ = notification::notify_smart_sitting_reminder(
                                &handle_state,
                                lang,
                                minutes,
                                sound_enabled,
                            );
                            reminder.awaiting_standup = true;
                            reminder.reminder_sent_at = Some(Instant::now());
                            // 重置活动监听器的时间戳
                            state_for_manager.activity_monitor.reset_activity();
                        }
                    }
                }

                // 每次收到事件都更新图标和菜单（确保 CLI 列表实时更新）
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
                // 如果启用了智能久坐提醒，启动活动监听器
                if state_sit.config.get_sitting_reminder_enabled() {
                    state_sit.activity_monitor.start();
                }

                loop {
                    std::thread::sleep(std::time::Duration::from_secs(60));

                    // 检查智能久坐提醒状态
                    {
                        let mut reminder = state_sit.sitting_reminder.lock().unwrap();
                        if reminder.awaiting_standup {
                            if let Some(sent_at) = reminder.reminder_sent_at {
                                // 检查是否已过2分钟
                                if sent_at.elapsed().as_secs() >= 120 {
                                    // 检查用户是否在这2分钟内无活动
                                    if state_sit.activity_monitor.is_inactive_for(120) {
                                        // 用户站起来了，重置计时
                                        let mut m = state_sit.sitting_minutes.lock().unwrap();
                                        *m = 0;
                                        println!("用户已休息，重置久坐计时");
                                    } else {
                                        // 用户仍在活动，继续计时
                                        println!("用户仍在活动，继续计时");
                                    }
                                    // 清除等待状态
                                    reminder.awaiting_standup = false;
                                    reminder.reminder_sent_at = None;
                                }
                            }
                        }
                    }

                    let minutes = {
                        let mut m = state_sit.sitting_minutes.lock().unwrap();
                        *m += 1;
                        *m
                    };

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
    let lang = config.get_language();
    let s = get_strings(lang);

    // 显示各个 CLI 的状态
    let active_clis: Vec<&CliStatus> = cli_states
        .iter()
        .filter(|s| s.state != CliState::Offline)
        .collect();

    if active_clis.is_empty() {
        // 无 CLI 运行
        let status = MenuItem::new(app, s.no_cli_running, false, None::<&str>).unwrap();
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
    let time_str = format_sitting_time(lang, minutes);
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
        s.show_time,
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
        s.sound_notification,
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
        s.auto_bring_to_front,
        true,
        auto_front,
        None::<&str>,
    )
    .unwrap();
    let _ = menu.append(&toggle_front);

    let sitting_reminder_enabled = config.get_sitting_reminder_enabled();
    let toggle_sitting_reminder = CheckMenuItem::with_id(
        app,
        "toggle_sitting_reminder",
        s.smart_sitting_reminder,
        true,
        sitting_reminder_enabled,
        None::<&str>,
    )
    .unwrap();
    let _ = menu.append(&toggle_sitting_reminder);

    let interval = config.get_sitting_reminder_interval();
    let interval_label = format_interval(lang, interval);
    let cycle_interval = MenuItem::with_id(
        app,
        "cycle_interval",
        interval_label,
        sitting_reminder_enabled, // 只有开启久坐提醒时才可点击
        None::<&str>,
    )
    .unwrap();
    let _ = menu.append(&cycle_interval);

    // 分隔线
    let separator3 = PredefinedMenuItem::separator(app).unwrap();
    let _ = menu.append(&separator3);

    // 语言切换
    let toggle_lang = MenuItem::with_id(app, "toggle_lang", s.language, true, None::<&str>).unwrap();
    let _ = menu.append(&toggle_lang);

    // 分隔线
    let separator4 = PredefinedMenuItem::separator(app).unwrap();
    let _ = menu.append(&separator4);

    // 版本信息
    let version = app.package_info().version.to_string();
    let version_label = format_version(lang, &version);
    let version_item = MenuItem::new(app, version_label, false, None::<&str>).unwrap();
    let _ = menu.append(&version_item);

    // 检查更新
    let check_update = MenuItem::with_id(app, "check_update", s.check_update, true, None::<&str>).unwrap();
    let _ = menu.append(&check_update);

    // 重置计时
    let reset = MenuItem::with_id(app, "reset", s.reset_timer, true, None::<&str>).unwrap();
    let _ = menu.append(&reset);

    // 退出
    let quit = MenuItem::with_id(app, "quit", s.quit, true, None::<&str>).unwrap();
    let _ = menu.append(&quit);

    menu
}
