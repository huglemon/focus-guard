# Focus Guard

<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" height="128" alt="Focus Guard Logo">
</p>

<p align="center">
  <strong>Developer Health Assistant - Smart Break Reminder for AI CLI Users</strong>
</p>

<p align="center">
  <a href="https://github.com/huglemon/focus-guard/releases">
    <img src="https://img.shields.io/github/v/release/huglemon/focus-guard" alt="Release">
  </a>
  <a href="https://github.com/huglemon/focus-guard/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/huglemon/focus-guard" alt="License">
  </a>
  <a href="https://github.com/huglemon/focus-guard/stargazers">
    <img src="https://img.shields.io/github/stars/huglemon/focus-guard" alt="Stars">
  </a>
</p>

<p align="center">
  English | <a href="./README.md">中文</a>
</p>

---

## Introduction

Focus Guard is a macOS menu bar application designed for developers using AI CLI tools (such as Claude Code, Gemini CLI, Codex, etc.). It intelligently monitors your work status and reminds you to take breaks at the right moments, protecting your health.

## Features

- **CLI Process Monitoring** - Automatically detects AI CLI tools like Claude Code, Gemini CLI, Codex
- **Smart Break Reminders** - Reminds you to rest when CLI is waiting for input, without interrupting your workflow
- **Sound Notifications** - Plays alert sound when CLI awaits input
- **Auto Window Focus** - Intelligently identifies and brings the correct terminal/IDE window to front
- **Multi-language Support** - Supports English and Chinese interfaces
- **Auto Updates** - Built-in auto-update functionality

## Installation

### System Requirements

- macOS 10.15+
- Apple Silicon (M1/M2/M3)

### Download

1. Go to [Releases](https://github.com/huglemon/focus-guard/releases) page
2. Download the latest `.dmg` file
3. Open the DMG and drag the app to Applications folder
4. On first launch, right-click the app and select "Open"

## Usage

### Initial Setup

1. **Launch the app** - Focus Guard will appear in the menu bar
2. **Grant permissions** - Allow the following permissions when prompted:
   - **Notifications** - For sending break reminders
   - **Input Monitoring** - For detecting keyboard/mouse activity

### Menu Bar Icon

| Icon Color | Status |
|------------|--------|
| 🟢 Green | CLI is working |
| 🔴 Red | CLI is waiting for your input |
| ⚪ Gray | No CLI process detected |

### Settings

Click the menu bar icon to access these options:

- **Show Time** - Display sitting duration in menu bar
- **Sound Notification** - Play sound when CLI is waiting
- **Auto Focus Terminal** - Automatically bring terminal window to front
- **Smart Break Reminder** - Enable/disable break reminders
- **Reminder Interval** - Set interval (20/30/40/50/60 minutes)
- **Language** - Switch between English/Chinese

### Smart Break Reminder Logic

1. The app tracks your continuous sitting time
2. When the set interval is reached, it sends a reminder when CLI is waiting for input
3. After receiving a reminder:
   - No keyboard/mouse activity for 2 minutes → Considered as rested, timer resets
   - Still active → Continue accumulating time

## Privacy Policy

Focus Guard takes your privacy seriously:

### Data Collection

- **No personal data collected** - The app runs entirely locally
- **No data uploaded** - All data is stored only on your device
- **No user tracking** - No analytics or tracking code

### Permission Usage

| Permission | Purpose | Data Handling |
|------------|---------|---------------|
| Notifications | Send break reminders | Local display only, not uploaded |
| Input Monitoring | Detect activity to determine rest status | Only detects presence of activity, not specific content |

### Network Access

- The app only accesses GitHub for update checks
- No other servers are contacted
- No user data is transmitted

## Development

```bash
# Install dependencies
bun install

# Development mode
bunx tauri dev

# Build
bunx tauri build
```

## Tech Stack

- **Frontend**: TypeScript + Vite
- **Backend**: Rust + Tauri 2
- **Package Manager**: Bun

## License

This project is licensed under the [MIT License](./LICENSE).

## Author

**Huglemon**

- Blog: [https://www.huglemon.com](https://www.huglemon.com)
- Twitter: [@huglemon520](https://x.com/huglemon520)
- Jike: [huglemon](https://okjk.co/zV01gI)

## Acknowledgments

Thanks to all developers who use and support Focus Guard!

If this project helps you, please give it a ⭐ Star!

---

<p align="center">
  Made with ❤️ by <a href="https://www.huglemon.com">Huglemon</a>
</p>
