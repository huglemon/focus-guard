use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub status: String,
}

/// 获取正在运行的CLI进程（Claude Code、Gemini CLI、Codex CLI、终端等）
/// 作为 hooks 系统的兜底检测
pub fn get_cli_processes() -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // 我们关心的CLI/IDE进程
    let cli_names = [
        "claude",      // Claude Code CLI
        "gemini",      // Gemini CLI
        "codex",       // Codex CLI
        "cursor",      // Cursor IDE
        "code",        // VS Code
        "Terminal",    // macOS Terminal
        "iTerm2",      // iTerm2
        "Warp",        // Warp terminal
        "Alacritty",   // Alacritty
        "kitty",       // Kitty terminal
    ];

    let mut processes = Vec::new();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();

        // 检查是否是我们关心的CLI进程
        let is_cli = cli_names.iter().any(|&cli| {
            name.to_lowercase().contains(&cli.to_lowercase())
        });

        if is_cli {
            // 简化状态：只区分运行中和其他
            let status = match process.status() {
                sysinfo::ProcessStatus::Run => "running",
                _ => "idle",
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
