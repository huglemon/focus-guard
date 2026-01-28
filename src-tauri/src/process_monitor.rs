use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub status: String,
}

/// 获取正在运行的CLI进程（Claude Code、终端等）
pub fn get_cli_processes() -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cli_names = [
        "claude",      // Claude Code
        "node",        // Node.js (可能是Claude Code)
        "Terminal",    // macOS Terminal
        "iTerm2",      // iTerm2
        "Warp",        // Warp terminal
        "Alacritty",   // Alacritty
        "kitty",       // Kitty terminal
        "zsh",         // Zsh shell
        "bash",        // Bash shell
        "fish",        // Fish shell
    ];

    let mut processes = Vec::new();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();

        // 检查是否是我们关心的CLI进程
        let is_cli = cli_names.iter().any(|&cli| {
            name.to_lowercase().contains(&cli.to_lowercase())
        });

        if is_cli {
            // 检查进程状态
            let status = match process.status() {
                sysinfo::ProcessStatus::Run => "running",
                sysinfo::ProcessStatus::Sleep => "waiting", // 可能在等待输入
                sysinfo::ProcessStatus::Idle => "idle",
                _ => "unknown",
            };

            processes.push(ProcessInfo {
                name: name.clone(),
                pid: pid.as_u32(),
                status: status.to_string(),
            });
        }
    }

    // 去重并只保留主要进程
    processes.sort_by(|a, b| a.name.cmp(&b.name));
    processes.dedup_by(|a, b| a.name == b.name);

    processes
}

/// 检查是否有CLI进程在等待用户输入
pub fn has_waiting_cli() -> bool {
    let processes = get_cli_processes();
    processes.iter().any(|p| p.status == "waiting")
}
