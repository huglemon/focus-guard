#!/bin/bash

# Focus Guard - CLI Hooks 配置脚本
# 自动配置 Claude Code、Gemini CLI、Codex CLI 的 hooks

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NOTIFY_SCRIPT="$SCRIPT_DIR/focus-guard-notify"

echo "🔧 Focus Guard Hooks 配置脚本"
echo "=============================="
echo ""

# 检查 notify 脚本是否存在
if [ ! -f "$NOTIFY_SCRIPT" ]; then
    echo "❌ 错误: focus-guard-notify 脚本不存在"
    echo "   请确保 $NOTIFY_SCRIPT 文件存在"
    exit 1
fi

# 确保 notify 脚本可执行
chmod +x "$NOTIFY_SCRIPT"

# 配置 Claude Code hooks
setup_claude_hooks() {
    echo "📦 配置 Claude Code hooks..."

    CLAUDE_SETTINGS="$HOME/.claude/settings.json"
    CLAUDE_DIR="$HOME/.claude"

    # 创建目录
    mkdir -p "$CLAUDE_DIR"

    # 生成 hooks 配置
    # 使用 bash -c 来传递 stdin JSON，包含 session_id 和 cwd
    HOOKS_CONFIG=$(cat <<EOF
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo '{\"session_id\":\"'\$CLAUDE_SESSION_ID'\",\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT claude session_start"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo '{\"session_id\":\"'\$CLAUDE_SESSION_ID'\",\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT claude session_end"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo '{\"session_id\":\"'\$CLAUDE_SESSION_ID'\",\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT claude stop"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "echo '{\"session_id\":\"'\$CLAUDE_SESSION_ID'\",\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT claude working"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "echo '{\"session_id\":\"'\$CLAUDE_SESSION_ID'\",\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT claude idle_prompt"
          }
        ]
      },
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "echo '{\"session_id\":\"'\$CLAUDE_SESSION_ID'\",\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT claude permission_prompt"
          }
        ]
      }
    ]
  }
}
EOF
)

    # 如果配置文件存在，合并配置；否则创建新文件
    if [ -f "$CLAUDE_SETTINGS" ]; then
        echo "   发现现有配置，正在合并..."
        # 备份原配置
        cp "$CLAUDE_SETTINGS" "$CLAUDE_SETTINGS.backup"
        # 使用 jq 合并配置（如果有 jq）
        if command -v jq &> /dev/null; then
            jq -s '.[0] * .[1]' "$CLAUDE_SETTINGS" <(echo "$HOOKS_CONFIG") > "$CLAUDE_SETTINGS.tmp"
            mv "$CLAUDE_SETTINGS.tmp" "$CLAUDE_SETTINGS"
        else
            echo "   ⚠️  未安装 jq，将覆盖现有配置"
            echo "   原配置已备份到 $CLAUDE_SETTINGS.backup"
            echo "$HOOKS_CONFIG" > "$CLAUDE_SETTINGS"
        fi
    else
        echo "$HOOKS_CONFIG" > "$CLAUDE_SETTINGS"
    fi

    echo "   ✅ Claude Code hooks 配置完成"
}

# 配置 Gemini CLI hooks
setup_gemini_hooks() {
    echo "📦 配置 Gemini CLI hooks..."

    GEMINI_SETTINGS="$HOME/.gemini/settings.json"
    GEMINI_DIR="$HOME/.gemini"

    # 创建目录
    mkdir -p "$GEMINI_DIR"

    # 生成 hooks 配置
    HOOKS_CONFIG=$(cat <<EOF
{
  "hooks": {
    "SessionStart": [
      {
        "command": "echo '{\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT gemini session_start"
      }
    ],
    "SessionEnd": [
      {
        "command": "echo '{\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT gemini session_end"
      }
    ],
    "BeforeAgent": [
      {
        "command": "echo '{\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT gemini working"
      }
    ],
    "AfterAgent": [
      {
        "command": "echo '{\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT gemini stop"
      }
    ],
    "BeforeTool": [
      {
        "command": "echo '{\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT gemini working"
      }
    ],
    "AfterTool": [
      {
        "command": "echo '{\"cwd\":\"'\$PWD'\"}' | $NOTIFY_SCRIPT gemini stop"
      }
    ]
  }
}
EOF
)

    # 如果配置文件存在，合并配置；否则创建新文件
    if [ -f "$GEMINI_SETTINGS" ]; then
        echo "   发现现有配置，正在合并..."
        cp "$GEMINI_SETTINGS" "$GEMINI_SETTINGS.backup"
        if command -v jq &> /dev/null; then
            jq -s '.[0] * .[1]' "$GEMINI_SETTINGS" <(echo "$HOOKS_CONFIG") > "$GEMINI_SETTINGS.tmp"
            mv "$GEMINI_SETTINGS.tmp" "$GEMINI_SETTINGS"
        else
            echo "   ⚠️  未安装 jq，将覆盖现有配置"
            echo "   原配置已备份到 $GEMINI_SETTINGS.backup"
            echo "$HOOKS_CONFIG" > "$GEMINI_SETTINGS"
        fi
    else
        echo "$HOOKS_CONFIG" > "$GEMINI_SETTINGS"
    fi

    echo "   ✅ Gemini CLI hooks 配置完成"
}

# 主菜单
echo "请选择要配置的 CLI 工具:"
echo "  1) Claude Code"
echo "  2) Gemini CLI"
echo "  3) 全部配置"
echo "  4) 退出"
echo ""
read -p "请输入选项 [1-4]: " choice

case $choice in
    1)
        setup_claude_hooks
        ;;
    2)
        setup_gemini_hooks
        ;;
    3)
        setup_claude_hooks
        echo ""
        setup_gemini_hooks
        ;;
    4)
        echo "退出"
        exit 0
        ;;
    *)
        echo "无效选项"
        exit 1
        ;;
esac

echo ""
echo "=============================="
echo "✅ 配置完成！"
echo ""
echo "请重启相应的 CLI 工具以使配置生效。"
echo ""
echo "测试方法:"
echo "  echo '{\"cli\":\"claude\",\"event\":\"stop\",\"session_id\":\"test\",\"cwd\":\"/tmp\"}' | nc -U /tmp/focus-guard.sock"
