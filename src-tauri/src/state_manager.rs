use crate::ipc_server::{CliEvent, CliMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// CLI 状态
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CliState {
    Working,      // AI 正在处理
    WaitingInput, // 等待用户输入
    Idle,         // 空闲
    Offline,      // 未运行
}

/// 单个 CLI 的状态信息
#[derive(Debug, Clone)]
pub struct CliStatus {
    pub cli_name: String,
    pub state: CliState,
    pub pid: Option<u32>,
    pub last_event: Option<CliEvent>,
    pub last_update: Instant,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub display_name: String, // 如 "Claude - my-project"
    pub stop_received_at: Option<Instant>, // Stop 事件接收时间，用于延迟判断
}

impl CliStatus {
    #[allow(dead_code)]
    pub fn new(cli_name: String) -> Self {
        let display_name = capitalize_first(&cli_name);
        Self {
            cli_name,
            state: CliState::Offline,
            pid: None,
            last_event: None,
            last_update: Instant::now(),
            session_id: None,
            cwd: None,
            display_name,
            stop_received_at: None,
        }
    }

    pub fn with_details(cli_name: String, session_id: Option<String>, cwd: Option<String>) -> Self {
        let display_name = Self::format_display_name(&cli_name, cwd.as_deref());
        Self {
            cli_name,
            state: CliState::Offline,
            pid: None,
            last_event: None,
            last_update: Instant::now(),
            session_id,
            cwd,
            display_name,
            stop_received_at: None,
        }
    }

    fn format_display_name(cli_name: &str, cwd: Option<&str>) -> String {
        let cli_display = capitalize_first(cli_name);
        match cwd {
            Some(path) => {
                let project_name = Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);
                format!("{} - {}", cli_display, project_name)
            }
            None => cli_display,
        }
    }

    pub fn update_display_name(&mut self) {
        self.display_name = Self::format_display_name(&self.cli_name, self.cwd.as_deref());
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// 生成状态 key，支持多实例
fn make_state_key(cli: &str, session_id: Option<&str>) -> String {
    match session_id {
        Some(sid) => format!("{}:{}", cli, sid),
        None => cli.to_string(),
    }
}

/// 状态变化事件，包含详细信息
#[derive(Debug, Clone)]
pub struct StateChangeEvent {
    pub state: CliState,
    pub pid: Option<u32>,
    pub cwd: Option<String>,
    pub cli_name: String,
    pub state_changed: bool, // 聚合状态是否变化（用于判断是否需要通知）
}

/// 状态管理器
pub struct StateManager {
    /// 各 CLI 的状态
    cli_states: Arc<Mutex<HashMap<String, CliStatus>>>,
    /// 等待输入超时时间（秒）
    waiting_timeout: Duration,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            cli_states: Arc::new(Mutex::new(HashMap::new())),
            waiting_timeout: Duration::from_secs(10), // Stop 后 10 秒无活动视为等待输入
        }
    }

    /// 获取状态的共享引用
    pub fn get_states(&self) -> Arc<Mutex<HashMap<String, CliStatus>>> {
        self.cli_states.clone()
    }

    /// 启动状态管理循环
    pub fn start(
        self,
        receiver: Receiver<CliMessage>,
        on_state_change: impl Fn(StateChangeEvent) + Send + 'static,
    ) {
        let states = self.cli_states.clone();
        let timeout = self.waiting_timeout;

        std::thread::spawn(move || {
            let mut last_aggregate_state = CliState::Offline;

            loop {
                // 非阻塞接收消息，超时 1 秒
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(msg) => {
                        let mut states_guard = states.lock().unwrap();
                        let key = make_state_key(&msg.cli, msg.session_id.as_deref());
                        let status = states_guard.entry(key).or_insert_with(|| {
                            CliStatus::with_details(
                                msg.cli.clone(),
                                msg.session_id.clone(),
                                msg.cwd.clone(),
                            )
                        });

                        status.last_event = Some(msg.event.clone());
                        status.last_update = Instant::now();
                        status.pid = msg.pid;

                        // 更新 session_id 和 cwd（如果有新值）
                        if msg.session_id.is_some() {
                            status.session_id = msg.session_id.clone();
                        }
                        if msg.cwd.is_some() {
                            status.cwd = msg.cwd.clone();
                            status.update_display_name();
                        }

                        // 处理 Stop 事件的延迟逻辑
                        let new_state = match msg.event {
                            CliEvent::SessionStart => {
                                status.stop_received_at = None;
                                CliState::Working
                            }
                            CliEvent::SessionEnd => {
                                status.stop_received_at = None;
                                CliState::Offline
                            }
                            CliEvent::Working => {
                                // 收到 Working 事件，清除 Stop 记录
                                status.stop_received_at = None;
                                CliState::Working
                            }
                            CliEvent::Stop => {
                                // Stop 事件：记录时间，但保持 Working 状态
                                // 延迟判断是否真的需要用户输入
                                status.stop_received_at = Some(Instant::now());
                                // 保持当前状态（如果是 Working 就保持 Working）
                                if status.state == CliState::Offline {
                                    CliState::Working
                                } else {
                                    status.state
                                }
                            }
                            CliEvent::IdlePrompt => {
                                status.stop_received_at = None;
                                CliState::Idle
                            }
                            CliEvent::PermissionPrompt => {
                                // 权限提示需要立即响应
                                status.stop_received_at = None;
                                CliState::WaitingInput
                            }
                        };
                        status.state = new_state;

                        // 保存当前 CLI 的信息用于回调
                        let current_pid = status.pid;
                        let current_cwd = status.cwd.clone();
                        let current_cli = status.cli_name.clone();

                        drop(states_guard);

                        // 计算聚合状态
                        let aggregate_state = Self::calculate_aggregate_state(&states);
                        let state_changed = aggregate_state != last_aggregate_state;

                        if state_changed {
                            last_aggregate_state = aggregate_state;
                        }

                        // 每次收到事件都通知（用于更新菜单），但标记是否需要通知/置顶
                        on_state_change(StateChangeEvent {
                            state: aggregate_state,
                            pid: current_pid,
                            cwd: current_cwd,
                            cli_name: current_cli,
                            state_changed,
                        });
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // 超时检查
                        let mut states_guard = states.lock().unwrap();
                        let mut state_updated = false;

                        for status in states_guard.values_mut() {
                            // 检查 Stop 延迟：如果收到 Stop 超过 3 秒没有新事件，转为 WaitingInput
                            if let Some(stop_time) = status.stop_received_at {
                                if stop_time.elapsed() > Duration::from_secs(3) {
                                    status.state = CliState::WaitingInput;
                                    status.stop_received_at = None;
                                    state_updated = true;
                                }
                            }

                            // 将长时间 WaitingInput 的状态转为 Idle
                            if status.state == CliState::WaitingInput
                                && status.last_update.elapsed() > timeout * 6
                            {
                                status.state = CliState::Idle;
                                state_updated = true;
                            }
                        }
                        drop(states_guard);

                        // 检查聚合状态是否变化
                        let aggregate_state = Self::calculate_aggregate_state(&states);
                        if aggregate_state != last_aggregate_state || state_updated {
                            let changed = aggregate_state != last_aggregate_state;
                            if changed {
                                last_aggregate_state = aggregate_state;
                            }
                            on_state_change(StateChangeEvent {
                                state: aggregate_state,
                                pid: None,
                                cwd: None,
                                cli_name: String::new(),
                                state_changed: changed,
                            });
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        eprintln!("State manager channel disconnected");
                        break;
                    }
                }
            }
        });
    }

    /// 计算聚合状态（用于托盘图标）
    fn calculate_aggregate_state(states: &Arc<Mutex<HashMap<String, CliStatus>>>) -> CliState {
        let states_guard = states.lock().unwrap();

        if states_guard.is_empty() {
            return CliState::Offline;
        }

        // 优先级：WaitingInput > Working > Idle > Offline
        let mut has_working = false;
        let mut has_waiting = false;
        let mut has_idle = false;

        for status in states_guard.values() {
            match status.state {
                CliState::WaitingInput => has_waiting = true,
                CliState::Working => has_working = true,
                CliState::Idle => has_idle = true,
                CliState::Offline => {}
            }
        }

        if has_waiting {
            CliState::WaitingInput
        } else if has_working {
            CliState::Working
        } else if has_idle {
            CliState::Idle
        } else {
            CliState::Offline
        }
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}
