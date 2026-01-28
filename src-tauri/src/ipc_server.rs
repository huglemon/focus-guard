use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::mpsc::Sender;

pub const SOCKET_PATH: &str = "/tmp/focus-guard.sock";

/// CLI 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CliEvent {
    SessionStart,
    SessionEnd,
    Working,    // PreToolUse, BeforeAgent, BeforeTool
    Stop,       // Claude Stop, AfterAgent
    IdlePrompt, // Claude idle_prompt notification
    PermissionPrompt,
}

/// 从 CLI hooks 接收的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliMessage {
    pub cli: String, // "claude", "gemini", "codex"
    pub event: CliEvent,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub timestamp: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>, // 会话 ID，用于区分多实例
    #[serde(default)]
    pub cwd: Option<String>, // 工作目录
}

/// 启动 Unix Socket 服务器
pub fn start_ipc_server(sender: Sender<CliMessage>) {
    // 清理旧的 socket 文件
    let socket_path = Path::new(SOCKET_PATH);
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }

    // 创建 Unix Socket 监听器
    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind Unix socket: {}", e);
            return;
        }
    };

    println!("IPC server listening on {}", SOCKET_PATH);

    // 处理连接
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let sender = sender.clone();
                std::thread::spawn(move || {
                    handle_connection(stream, sender);
                });
            }
            Err(e) => {
                eprintln!("IPC connection error: {}", e);
            }
        }
    }
}

fn handle_connection(stream: std::os::unix::net::UnixStream, sender: Sender<CliMessage>) {
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        match line {
            Ok(data) => {
                if data.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<CliMessage>(&data) {
                    Ok(msg) => {
                        println!("Received CLI event: {:?}", msg);
                        if sender.send(msg).is_err() {
                            eprintln!("Failed to send message to state manager");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse CLI message: {} - data: {}", e, data);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                break;
            }
        }
    }
}

/// 清理 socket 文件
pub fn cleanup() {
    let socket_path = Path::new(SOCKET_PATH);
    if socket_path.exists() {
        let _ = std::fs::remove_file(socket_path);
    }
}
