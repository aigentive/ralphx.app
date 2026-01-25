# Plan: Tauri Visual Testing Infrastructure

## Status: COMPLETE

Using **hybrid approach**: native macOS tools for screenshots/keyboard + tauri-mcp for introspection.

**Skill:** `.claude/skills/tauri-visual-test/SKILL.md`
**MCP Config:** `.mcp.json` (tauri-mcp via Node.js wrapper)
**tauri-mcp Location:** `~/Code/tauri-mcp/`

---

## What Works

| Task | Tool | Notes |
|------|------|-------|
| Screenshots | `screencapture -l$WID` | Clean window-only captures |
| Window ID | `GetWindowID ralphx "RalphX"` | Case-sensitive title |
| Keyboard | `osascript keystroke` | Reliable with focus |
| Window info | `mcp get_window_info` | Dimensions, position |
| Run JS | `mcp execute_js` | Webview access |
| Tauri IPC | `mcp call_ipc_command` | Backend commands |

## What Doesn't Work Reliably

| Task | Tool | Issue |
|------|------|-------|
| Screenshots | `mcp take_screenshot` | Captures whole screen, not just window |
| Keyboard | `mcp send_keyboard_input` | Doesn't reach focused app |

---

## Quick Start

```bash
# 1. Get process and window ID
PID=$(pgrep -f "ralphx")
WID=$(GetWindowID ralphx "RalphX")

# 2. Capture screenshot
screencapture -l$WID -o screenshot.png

# 3. Send keyboard shortcut
osascript -e "tell application \"System Events\"
  set frontmost of (first process whose unix id is $PID) to true
  keystroke \"2\" using command down
end tell"
```

---

## Setup (Already Done)

### 1. tauri-mcp Installation

```bash
# Rust binary (for Node.js wrapper to spawn)
cargo install tauri-mcp

# Node.js wrapper (for Claude Code compatibility)
git clone https://github.com/dirvine/tauri-mcp.git ~/Code/tauri-mcp
cd ~/Code/tauri-mcp && cargo build --release  # Build 0.1.5 with 'tool' subcommand
cd server && npm install
```

### 2. MCP Configuration

`.mcp.json`:
```json
{
  "mcpServers": {
    "tauri-mcp": {
      "command": "node",
      "args": ["/Users/lazabogdan/Code/tauri-mcp/server/index.js"],
      "env": {
        "TAURI_MCP_PATH": "/Users/lazabogdan/Code/tauri-mcp/target/release/tauri-mcp",
        "TAURI_MCP_LOG_LEVEL": "info"
      }
    }
  }
}
```

### 3. macOS Prerequisites

- `brew install smokris/getwindowid/getwindowid`
- Screen Recording permission (System Settings > Privacy)
- Accessibility permission (System Settings > Privacy)

### 4. Directory Structure

```
screenshots/visual-audit/
├── kanban/
├── ideation/
├── extensibility/
├── activity/
├── settings/
├── modals/
└── panels/
```

---

## Full Workflow Script

```bash
#!/bin/bash
set -e

# Check if app running
PID=$(pgrep -f "ralphx" || pgrep -f "tauri dev" || echo "")
if [ -z "$PID" ]; then
  npm run tauri dev &
  sleep 15
  PID=$(pgrep -f "ralphx")
fi

WID=$(GetWindowID ralphx "RalphX")
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Capture all views
for i in 1 2 3 4 5; do
  osascript -e "tell application \"System Events\"
    set frontmost of (first process whose unix id is $PID) to true
    keystroke \"$i\" using command down
  end tell"
  sleep 0.5

  case $i in
    1) VIEW="kanban" ;;
    2) VIEW="ideation" ;;
    3) VIEW="extensibility" ;;
    4) VIEW="activity" ;;
    5) VIEW="settings" ;;
  esac

  screencapture -l$WID -o "screenshots/visual-audit/$VIEW/default_$TIMESTAMP.png"
  echo "Captured: $VIEW"
done
```

---

## References

- [Tauri MCP Server](https://github.com/dirvine/tauri-mcp)
- [GetWindowID](https://github.com/smokris/GetWindowID)
- Skill: `.claude/skills/tauri-visual-test/SKILL.md`
