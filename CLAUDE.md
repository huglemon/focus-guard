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
