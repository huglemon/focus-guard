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

    // 我们关心的CLI/IDE进程 - 使用精确匹配
    // (进程名, 是否精确匹配)
    let cli_patterns: &[(&str, bool)] = &[
        ("claude", true),    // Claude Code CLI - 精确匹配
        ("gemini", true),    // Gemini CLI - 精确匹配
        ("codex", true),     // Codex CLI - 精确匹配
        ("Cursor", true),    // Cursor IDE - 精确匹配（注意大小写）
        ("Code", true),      // VS Code - 精确匹配（注意大小写）
        ("Terminal", true),  // macOS Terminal - 精确匹配
        ("iTerm2", true),    // iTerm2 - 精确匹配
        ("Warp", true),      // Warp terminal - 精确匹配
        ("Alacritty", true), // Alacritty - 精确匹配
        ("kitty", true),     // Kitty terminal - 精确匹配
    ];

    let mut processes = Vec::new();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_string();

        // 检查是否是我们关心的CLI进程
        let is_cli = cli_patterns.iter().any(|(pattern, exact)| {
            if *exact {
                // 精确匹配：进程名必须完全等于 pattern
                name == *pattern || name.to_lowercase() == pattern.to_lowercase()
            } else {
                // 模糊匹配（目前未使用）
                name.to_lowercase().contains(&pattern.to_lowercase())
            }
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
