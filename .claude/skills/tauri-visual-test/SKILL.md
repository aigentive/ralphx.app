---
name: tauri-visual-test
description: Visual testing and screenshot capture for RalphX Tauri app. Use for visual audits, screenshot capture, and UI verification.
---

# Tauri Visual Testing

Visual testing for RalphX using a hybrid approach: native macOS tools for screenshots/keyboard + tauri-mcp for app introspection.

## Prerequisites

1. **GetWindowID**: `brew install smokris/getwindowid/getwindowid`
2. **Screen Recording permission**: System Settings > Privacy & Security > Screen Recording
3. **Accessibility permission**: System Settings > Privacy & Security > Accessibility
4. **tauri-mcp MCP server**: Already configured in `.mcp.json`

## Quick Reference

### Screenshot (Clean Window-Only)

**Naming Convention**: Use timestamp prefix for chronological ordering:
```
screenshots/YYYY-MM-DD_HH-MM-SS_[task-name].png
```
Example: `screenshots/2026-01-25_14-30-45_kanban-default.png`

```bash
# Get window ID and capture with timestamp
WID=$(GetWindowID ralphx "RalphX")
screencapture -l$WID -o "screenshots/$(date +%Y-%m-%d_%H-%M-%S)_kanban-default.png"
```

### Keyboard Navigation

```bash
# Focus app and send shortcut
osascript -e 'tell application "System Events"
  set frontmost of (first process whose unix id is '$PID') to true
  delay 0.3
  keystroke "2" using command down
end tell'
```

| Shortcut | Action |
|----------|--------|
| Cmd+1 | Kanban view |
| Cmd+2 | Ideation view |
| Cmd+3 | Extensibility view |
| Cmd+4 | Activity view |
| Cmd+5 | Settings view |
| Cmd+K | Toggle Chat |

### Test Data Setup

Test data is seeded programmatically via Tauri commands. Before visual audits, ensure test data exists.

#### Test Data Profiles

| Profile | Creates | Use For |
|---------|---------|---------|
| `minimal` | Project only | Empty state testing |
| `kanban` | Project + 5 tasks | Kanban board audits (default) |
| `ideation` | Kanban + sessions/proposals | Ideation view audits (TODO) |
| `full` | All data types | Comprehensive testing |

#### Seeding Test Data

**Option 1: Via Tauri invoke (from browser console when app is running)**
```javascript
await window.__TAURI__.core.invoke('seed_test_data', { profile: 'kanban' });
```

**Option 2: Direct database check/seed script**
```bash
# Check if data exists
sqlite3 src-tauri/ralphx.db "SELECT COUNT(*) FROM projects;"

# If empty (0), the visual audit task should call the seed command
```

**Option 3: From frontend code (in tests or hooks)**
```typescript
await api.testData.seed("kanban");  // Specific profile
await api.testData.seedVisualAudit(); // Alias for kanban
await api.testData.clear();  // Remove all test data
```

#### What Kanban Profile Creates

- 1 test project ("Visual Audit Test")
- 5 sample tasks:
  - 1 backlog task
  - 2 ready tasks (To Do)
  - 1 executing task (In Progress)
  - 1 approved task (Done)

### MCP Tools (for introspection)

```
get_window_info(process_id)    - Window dimensions, position, visibility
execute_js(process_id, code)   - Run JS in webview (requires debug port)
call_ipc_command(process_id, command_name, args) - Call Tauri commands (limited)
monitor_resources(process_id)  - CPU/memory usage
```

**Note**: MCP `call_ipc_command` returns stub responses - use keyboard shortcuts for reliable interactions.

## Complete Workflow

```bash
#!/bin/bash
set -e

# 1. Check if app running (DO NOT start multiple instances)
PID=$(pgrep -f "ralphx" || pgrep -f "tauri dev" || echo "")
if [ -z "$PID" ]; then
  echo "Starting RalphX..."
  npm run tauri dev &
  sleep 15  # Wait for compile
  PID=$(pgrep -f "ralphx")
fi

# 2. Get window ID
WID=$(GetWindowID ralphx "RalphX")
if [ -z "$WID" ]; then
  echo "ERROR: Could not find RalphX window"
  exit 1
fi

# 3. Capture each view (using YYYY-MM-DD_HH-MM-SS_task-name.png convention)

# Kanban (Cmd+1)
osascript -e "tell application \"System Events\"
  set frontmost of (first process whose unix id is $PID) to true
  delay 0.3
  keystroke \"1\" using command down
end tell"
sleep 0.5
screencapture -l$WID -o "screenshots/$(date +%Y-%m-%d_%H-%M-%S)_kanban-default.png"

# Ideation (Cmd+2)
osascript -e "tell application \"System Events\" to keystroke \"2\" using command down"
sleep 0.5
screencapture -l$WID -o "screenshots/$(date +%Y-%m-%d_%H-%M-%S)_ideation-default.png"

# Extensibility (Cmd+3)
osascript -e "tell application \"System Events\" to keystroke \"3\" using command down"
sleep 0.5
screencapture -l$WID -o "screenshots/$(date +%Y-%m-%d_%H-%M-%S)_extensibility-default.png"

# Activity (Cmd+4)
osascript -e "tell application \"System Events\" to keystroke \"4\" using command down"
sleep 0.5
screencapture -l$WID -o "screenshots/$(date +%Y-%m-%d_%H-%M-%S)_activity-default.png"

# Settings (Cmd+5)
osascript -e "tell application \"System Events\" to keystroke \"5\" using command down"
sleep 0.5
screencapture -l$WID -o "screenshots/$(date +%Y-%m-%d_%H-%M-%S)_settings-default.png"

echo "Visual audit complete!"
ls -la screenshots/visual-audit/*/
```

## Screenshot Naming Convention

**Format**: `screenshots/YYYY-MM-DD_HH-MM-SS_[task-name].png`

This ensures screenshots sort oldest→newest when browsing the folder.

**Examples**:
- `screenshots/2026-01-25_14-30-45_kanban-default.png`
- `screenshots/2026-01-25_14-31-02_kanban-task-hover.png`
- `screenshots/2026-01-25_14-31-15_ideation-chat-open.png`
- `screenshots/2026-01-25_14-31-30_settings-model-section.png`

**Command**:
```bash
screencapture -l$WID -o "screenshots/$(date +%Y-%m-%d_%H-%M-%S)_[task-name].png"
```

## Verification Checklist

For each screenshot, verify against `specs/DESIGN.md`:
- [ ] No purple/blue gradients (warm orange #ff6b35 only)
- [ ] SF Pro font (not Inter)
- [ ] Layered shadows on elevated elements
- [ ] Proper spacing and alignment
- [ ] Accent color follows 5% rule

## Tool Comparison

| Task | Use This | NOT This |
|------|----------|----------|
| Screenshot | `screencapture -l$WID` | `mcp take_screenshot` (captures full screen) |
| Keyboard | `osascript keystroke` | `mcp send_keyboard_input` (returns "sent" but doesn't work, even when focused) |
| Window info | `mcp get_window_info` | - |
| Run JS | `mcp execute_js` | - |
| Call Tauri cmd | `mcp call_ipc_command` | - |

## Troubleshooting

**"Could not find window"**
- Ensure app is running: `pgrep -f ralphx`
- Check window title: `GetWindowID ralphx "RalphX"` (title is case-sensitive)

**Black/empty screenshot**
- Grant Screen Recording permission to Terminal/IDE

**Keyboard shortcuts not working**
- Grant Accessibility permission
- Ensure app is focused before sending keys

**Multiple app instances**
- Kill all: `pkill -f ralphx; pkill -f "tauri dev"`
- Start fresh: `npm run tauri dev`
