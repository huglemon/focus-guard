use crate::ipc_server::{CliEvent, CliMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// CLI 状态
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CliState {
    Working,       // AI 正在处理
    WaitingInput,  // 等待用户输入
    Idle,          // 空闲
    Offline,       // 未运行
}

/// 单个 CLI 的状态信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CliStatus {
    pub cli_name: String,
    pub state: CliState,
    pub pid: Option<u32>,
    pub last_event: Option<CliEvent>,
    pub last_update: Instant,
}

impl CliStatus {
    pub fn new(cli_name: String) -> Self {
        Self {
            cli_name,
            state: CliState::Offline,
            pid: None,
            last_event: None,
            last_update: Instant::now(),
        }
    }
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
    pub fn start(self, receiver: Receiver<CliMessage>, on_state_change: impl Fn(CliState) + Send + 'static) {
        let states = self.cli_states.clone();
        let timeout = self.waiting_timeout;

        std::thread::spawn(move || {
            loop {
                // 非阻塞接收消息，超时 1 秒
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(msg) => {
                        let mut states_guard = states.lock().unwrap();
                        let status = states_guard
                            .entry(msg.cli.clone())
                            .or_insert_with(|| CliStatus::new(msg.cli.clone()));

                        status.last_event = Some(msg.event.clone());
                        status.last_update = Instant::now();
                        status.pid = msg.pid;

                        status.state = match msg.event {
                            CliEvent::SessionStart => CliState::Working,
                            CliEvent::SessionEnd => CliState::Offline,
                            CliEvent::Working => CliState::Working,
                            CliEvent::Stop => CliState::WaitingInput,
                            CliEvent::IdlePrompt => CliState::Idle,
                            CliEvent::PermissionPrompt => CliState::WaitingInput,
                        };

                        drop(states_guard);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // 超时检查：将长时间 WaitingInput 的状态转为 Idle
                        let mut states_guard = states.lock().unwrap();
                        for status in states_guard.values_mut() {
                            if status.state == CliState::WaitingInput
                                && status.last_update.elapsed() > timeout * 6
                            {
                                status.state = CliState::Idle;
                            }
                        }
                        drop(states_guard);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        eprintln!("State manager channel disconnected");
                        break;
                    }
                }

                // 计算聚合状态并通知
                let aggregate_state = Self::calculate_aggregate_state(&states);
                on_state_change(aggregate_state);
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
