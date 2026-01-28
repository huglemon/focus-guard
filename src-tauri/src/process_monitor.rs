use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
    pub status: String,
}

/// 获取正在运行的 AI CLI 进程（Claude Code、Gemini CLI、Codex CLI）
/// 作为 hooks 系统的兜底检测
pub fn get_cli_processes() -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // 只检测 AI CLI 工具，不检测终端应用
    let cli_patterns: &[&str] = &[
        "claude", // Claude Code CLI
        "gemini", // Gemini CLI
        "codex",  // Codex CLI
    ];

    let mut processes = Vec::new();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();
        let name_lower = name.to_lowercase();

        // 精确匹配 AI CLI 工具
        let is_cli = cli_patterns
            .iter()
            .any(|pattern| name_lower == *pattern);

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
    processes.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());

    processes
}
