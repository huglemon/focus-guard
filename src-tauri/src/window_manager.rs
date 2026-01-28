use std::process::Command;

/// 将终端应用置于最前
/// 支持常见的终端应用：Terminal、iTerm2、Warp、Alacritty、kitty
pub fn bring_terminal_to_front() -> Result<(), String> {
    // 使用 AppleScript 激活终端应用
    // 按优先级尝试激活不同的终端
    let terminals = [
        "iTerm",      // iTerm2
        "Terminal",   // macOS Terminal
        "Warp",       // Warp
        "Alacritty",  // Alacritty
        "kitty",      // kitty
    ];

    for terminal in &terminals {
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
            terminal, terminal
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| e.to_string())?;

        let result = String::from_utf8_lossy(&output.stdout);
        if result.trim() == "activated" {
            return Ok(());
        }
    }

    // 如果没有找到任何终端，尝试激活 Cursor 或 VS Code
    let ides = ["Cursor", "Code"];
    for ide in &ides {
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
            ide, ide
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| e.to_string())?;

        let result = String::from_utf8_lossy(&output.stdout);
        if result.trim() == "activated" {
            return Ok(());
        }
    }

    Err("No terminal or IDE found".to_string())
}
