use std::process::Command;

/// 已知的终端和 IDE 应用
const TERMINAL_APPS: &[&str] = &["Warp", "iTerm", "iTerm2", "Terminal", "Alacritty", "kitty"];
const IDE_APPS: &[&str] = &["Cursor", "Code", "Antigravity"];

/// 已知的 CLI 进程名
const CLI_PROCESS_NAMES: &[&str] = &["claude", "codex", "gemini"];

/// 通过 CWD 查找 CLI 进程的 PID
/// 因为 hooks 传来的 PID 是脚本进程（已退出），需要通过 CWD 找到真正的 CLI 进程
fn find_cli_pid_by_cwd(cwd: &str) -> Option<u32> {
    println!("通过 CWD 查找 CLI 进程: {}", cwd);

    for cli_name in CLI_PROCESS_NAMES {
        // 使用 pgrep 查找进程
        let output = Command::new("pgrep")
            .args(["-x", cli_name])
            .output()
            .ok()?;

        let pids_str = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids_str.lines() {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // 使用 lsof 获取进程的工作目录
                let lsof_output = Command::new("lsof")
                    .args(["-p", &pid.to_string()])
                    .output()
                    .ok();

                if let Some(lsof) = lsof_output {
                    let lsof_str = String::from_utf8_lossy(&lsof.stdout);
                    // 查找 cwd 行
                    for line in lsof_str.lines() {
                        if line.contains("cwd") && line.contains(cwd) {
                            println!("找到匹配的 CLI 进程: {} (PID={})", cli_name, pid);
                            return Some(pid);
                        }
                    }
                }
            }
        }
    }

    println!("未找到匹配 CWD 的 CLI 进程");
    None
}

/// 通过 PID 获取进程的父应用名称
/// 沿着进程树向上查找，直到找到已知的终端或 IDE 应用
pub fn get_parent_app_for_pid(pid: u32) -> Option<String> {
    println!("开始查找进程树: 起始 PID={}", pid);
    let mut current_pid = pid;
    let mut visited = std::collections::HashSet::new();

    // 最多向上查找 10 层，防止无限循环
    for i in 0..10 {
        if visited.contains(&current_pid) {
            println!("检测到循环，停止查找");
            break;
        }
        visited.insert(current_pid);

        // 获取进程信息
        println!("第{}层: 查询 PID={}", i + 1, current_pid);
        if let Some((cmd_path, ppid)) = get_process_info(current_pid) {
            let cmd_lower = cmd_path.to_lowercase();
            println!("检查进程: PID={}, CMD={}", current_pid, cmd_path);

            // 检查是否是已知的终端应用（通过路径匹配）
            for terminal in TERMINAL_APPS {
                let terminal_lower = terminal.to_lowercase();
                // 匹配 .app 路径或进程名
                if cmd_lower.contains(&format!("{}.app", terminal_lower))
                    || cmd_lower.contains(&format!("/{}", terminal_lower))
                    || cmd_lower == terminal_lower
                {
                    println!("找到终端应用: {}", terminal);
                    return Some(terminal.to_string());
                }
            }

            // 检查是否是已知的 IDE 应用
            for ide in IDE_APPS {
                let ide_lower = ide.to_lowercase();
                if cmd_lower.contains(&format!("{}.app", ide_lower))
                    || cmd_lower.contains(&format!("/{}", ide_lower))
                    || cmd_lower == ide_lower
                {
                    println!("找到 IDE 应用: {}", ide);
                    return Some(ide.to_string());
                }
            }

            // 继续向上查找
            if ppid == 0 || ppid == 1 {
                println!("到达 init/launchd (ppid={}), 停止查找", ppid);
                break; // 到达 init/launchd
            }
            current_pid = ppid;
        } else {
            println!("无法获取 PID={} 的进程信息，进程可能已退出", current_pid);
            break;
        }
    }

    println!("进程树查找完成，未找到已知应用");
    None
}

/// 获取进程信息：(进程名/路径, 父进程 PID)
fn get_process_info(pid: u32) -> Option<(String, u32)> {
    // 分开获取 args 和 ppid，避免 ps 输出被截断
    // 注意：ps -o args=,ppid= 会截断长路径，所以需要分开调用

    // 获取完整命令路径（使用 -ww 确保不截断）
    let args_output = Command::new("ps")
        .args(["-ww", "-p", &pid.to_string(), "-o", "args="])
        .output()
        .ok()?;

    let cmd = String::from_utf8_lossy(&args_output.stdout)
        .trim()
        .to_string();

    if cmd.is_empty() {
        return None;
    }

    // 获取父进程 PID
    let ppid_output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "ppid="])
        .output()
        .ok()?;

    let ppid = String::from_utf8_lossy(&ppid_output.stdout)
        .trim()
        .parse::<u32>()
        .ok()?;

    Some((cmd, ppid))
}

/// 激活指定的应用
fn activate_app(app_name: &str) -> Result<(), String> {
    let script = format!(
        r#"
        tell application "System Events"
            if exists (process "{}") then
                tell application "{}" to activate
                return "activated"
            end if
        end tell
        return "not found"
        "#,
        app_name, app_name
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| e.to_string())?;

    let result = String::from_utf8_lossy(&output.stdout);
    if result.trim() == "activated" {
        println!("成功激活应用: {}", app_name);
        Ok(())
    } else {
        Err(format!("未找到应用: {}", app_name))
    }
}

/// 激活指定应用的特定窗口（通过窗口标题匹配）
fn activate_app_window(app_name: &str, cwd: Option<&str>) -> Result<(), String> {
    // 如果有工作目录，尝试匹配窗口标题
    if let Some(dir) = cwd {
        // 提取目录名作为匹配关键词
        let dir_name = std::path::Path::new(dir)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if !dir_name.is_empty() {
            // 尝试激活包含目录名的窗口
            let script = format!(
                r#"
                tell application "System Events"
                    if exists (process "{}") then
                        tell application "{}"
                            activate
                            -- 尝试找到包含目录名的窗口
                            set targetWindow to missing value
                            repeat with w in windows
                                if name of w contains "{}" then
                                    set targetWindow to w
                                    exit repeat
                                end if
                            end repeat
                            if targetWindow is not missing value then
                                set index of targetWindow to 1
                            end if
                        end tell
                        return "activated"
                    end if
                end tell
                return "not found"
                "#,
                app_name, app_name, dir_name
            );

            let output = Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()
                .map_err(|e| e.to_string())?;

            let result = String::from_utf8_lossy(&output.stdout);
            if result.trim() == "activated" {
                println!("成功激活应用窗口: {} (匹配: {})", app_name, dir_name);
                return Ok(());
            }
        }
    }

    // 如果没有 cwd 或匹配失败，直接激活应用
    activate_app(app_name)
}

/// 智能置顶：根据 CLI 的 PID 和工作目录激活正确的应用和窗口
pub fn bring_cli_to_front(pid: Option<u32>, cwd: Option<&str>) -> Result<(), String> {
    println!("智能置顶: PID={:?}, CWD={:?}", pid, cwd);

    // 策略1: 如果有 CWD，通过 CWD 查找真正的 CLI 进程
    // （因为 hooks 传来的 PID 是脚本进程，已经退出）
    if let Some(dir) = cwd {
        if let Some(cli_pid) = find_cli_pid_by_cwd(dir) {
            if let Some(app_name) = get_parent_app_for_pid(cli_pid) {
                println!("通过 CWD 找到父应用: {}", app_name);
                return activate_app_window(&app_name, cwd);
            }
        }
    }

    // 策略2: 如果有 PID，尝试通过进程树找到父应用（兜底）
    if let Some(p) = pid {
        if let Some(app_name) = get_parent_app_for_pid(p) {
            println!("通过 PID 进程树找到父应用: {}", app_name);
            return activate_app_window(&app_name, cwd);
        }
    }

    // 策略3: 使用传统的优先级方式
    println!("使用传统优先级方式激活");
    bring_terminal_to_front_legacy(cwd)
}

/// 传统的置顶方式（按优先级尝试）
fn bring_terminal_to_front_legacy(cwd: Option<&str>) -> Result<(), String> {
    // 按优先级尝试激活终端
    for terminal in TERMINAL_APPS {
        if activate_app_window(terminal, cwd).is_ok() {
            return Ok(());
        }
    }

    // 如果没有找到终端，尝试激活 IDE
    for ide in IDE_APPS {
        if activate_app_window(ide, cwd).is_ok() {
            return Ok(());
        }
    }

    println!("未找到任何终端或 IDE");
    Err("No terminal or IDE found".to_string())
}

/// 将终端应用置于最前（兼容旧接口）
pub fn bring_terminal_to_front() -> Result<(), String> {
    bring_cli_to_front(None, None)
}
