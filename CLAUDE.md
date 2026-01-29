# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Focus Guard is a developer health assistant desktop app built with Tauri 2 (Rust backend + TypeScript frontend). It monitors CLI processes, tracks sitting time, and sends reminders.

## Development Commands

```bash
# Install dependencies
bun install

# Run in development mode (starts Vite + Tauri)
bunx tauri dev

# Build for production
bunx tauri build

# Check Rust code
cd src-tauri && cargo check

# Format Rust code
cd src-tauri && cargo fmt
```

## Architecture

### Tech Stack
- **Frontend**: TypeScript + Vite (vanilla, no framework)
- **Backend**: Rust + Tauri 2
- **Package Manager**: Bun

### Key Directories
- `src/` - Frontend TypeScript code
- `src-tauri/src/` - Rust backend code
- `src-tauri/icons/` - App icons for all platforms

### Rust Modules (src-tauri/src/)
- `lib.rs` - Main entry, Tauri setup, background monitoring thread
- `process_monitor.rs` - Detects CLI processes (Terminal, iTerm, Claude Code, etc.)
- `notification.rs` - macOS system notifications via tauri-plugin-notification

### Frontend-Backend Communication
- **Commands**: Frontend calls Rust via `invoke()` → `#[tauri::command]` functions
- **Events**: Rust emits to frontend via `handle.emit()` → frontend listens with `listen()`

### Current Events
- `sitting-time-update` - Emitted every minute with sitting duration
- `cli-status-update` - Emitted with CLI process status changes
- `user-presence-update` - (Planned) User presence from camera detection

## Git Workflow

Create feature branches before development:
```bash
git checkout -b feature/your-feature-name
# ... develop and test ...
git checkout main
git merge feature/your-feature-name
git push
```

## Planned Features
- Pure menu bar app (no main window)
- Camera-based user presence detection
- Sitting time display in menu bar with status indicator

## Release Process (重要!)

发布新版本的完整流程，**必须按顺序执行**：

```bash
# 1. 构建
bunx tauri build

# 2. 签名 App (ad-hoc 签名，解决"已损坏"问题)
codesign --force --deep --sign - "src-tauri/target/release/bundle/macos/Focus Guard.app"

# 3. 重新打包 DMG (关键! 否则 DMG 里的 App 没有签名)
mkdir -p /tmp/dmg-temp
cp -R "src-tauri/target/release/bundle/macos/Focus Guard.app" /tmp/dmg-temp/
ln -s /Applications /tmp/dmg-temp/Applications
hdiutil create -volname "Focus Guard" -srcfolder /tmp/dmg-temp -ov -format UDZO "src-tauri/target/release/bundle/dmg/Focus Guard_VERSION_aarch64.dmg"
rm -rf /tmp/dmg-temp

# 4. Tauri 更新签名
bunx tauri signer sign -f ~/.tauri/focus-guard.key -p "inwind3181515" "src-tauri/target/release/bundle/dmg/Focus Guard_VERSION_aarch64.dmg"

# 5. 复制文件到项目根目录 (注意文件名用点号)
cp "src-tauri/target/release/bundle/dmg/Focus Guard_VERSION_aarch64.dmg" "Focus.Guard_VERSION_aarch64.dmg"
cp "src-tauri/target/release/bundle/dmg/Focus Guard_VERSION_aarch64.dmg.sig" "Focus.Guard_VERSION_aarch64.dmg.sig"

# 6. 更新 latest.json (version, signature, url)

# 7. 提交推送
git add -A && git commit -m "chore: 发布 vVERSION" && git push

# 8. 创建 GitHub Release
gh release create vVERSION --title "vVERSION" --notes "更新说明" Focus.Guard_VERSION_aarch64.dmg Focus.Guard_VERSION_aarch64.dmg.sig latest.json
```

### 签名密钥信息
- **私钥路径**: `~/.tauri/focus-guard.key`
- **私钥密码**: `inwind3181515`
- **公钥路径**: `~/.tauri/focus-guard.key.pub`

### 注意事项
- **必须先签名 App 再打包 DMG**，否则用户打开会提示"已损坏"
- DMG 文件名在 URL 中用点号 (Focus.Guard)，本地生成的用空格 (Focus Guard)
