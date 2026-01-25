# Plan: Tauri Visual Testing Infrastructure

## Problem

The current visual audit tasks specify using `agent-browser` (Playwright), but:
1. Playwright runs in a headless browser, not the actual Tauri app
2. Tauri APIs only work within the Tauri webview context
3. The agent cannot interact with the real app or take meaningful screenshots

## Solution

Use **[Tauri MCP Server](https://github.com/dirvine/tauri-mcp)** - an MCP server that provides:
- Screenshot capture of Tauri windows
- Keyboard input simulation
- Mouse click simulation
- JavaScript execution in the webview
- Direct Tauri IPC command invocation

Since it's an MCP server, it integrates directly with Claude Code.

---

## Implementation Plan

### Phase 1: Install Tauri MCP Server

**Option A: Cargo Install (Simpler)**
```bash
cargo install tauri-mcp
```

**Option B: Node.js Wrapper (Better Claude Desktop compatibility)**
```bash
# Download from releases
# https://github.com/dirvine/tauri-mcp/releases
```

### Phase 2: Configure MCP Server for Claude Code

Add to `.claude/settings.json` or project MCP config:

```json
{
  "mcpServers": {
    "tauri-mcp": {
      "command": "tauri-mcp",
      "args": ["serve"],
      "env": {
        "TAURI_MCP_LOG_LEVEL": "info"
      }
    }
  }
}
```

### Phase 3: Create Visual Testing Skill

Create `ralphx-plugin/skills/tauri-visual-test/SKILL.md`:

```markdown
---
name: tauri-visual-test
description: Visual testing and screenshot capture for RalphX Tauri app. This is the ONLY way to visually test, capture screenshots, or interact with the running Tauri application on macOS. Do NOT use agent-browser or Playwright - they cannot access Tauri APIs. Use tauri-mcp MCP server tools instead.
---

# Tauri Visual Testing

This skill enables visual testing of the RalphX Tauri application using the tauri-mcp server.

**IMPORTANT:** This is the ONLY way to visual test the app on macOS. Do NOT use:
- agent-browser (cannot access Tauri APIs)
- Playwright/Puppeteer (runs in browser, not Tauri webview)
- WebDriver (no macOS support for WKWebView)

You MUST use the tauri-mcp MCP server tools for all visual testing.

## Available Tools (via MCP)

- `launch_app` - Start the RalphX app
- `take_screenshot` - Capture window screenshot
- `send_keyboard_input` - Simulate keyboard (Cmd+1-5 for views, Cmd+K for chat)
- `send_mouse_click` - Click at coordinates
- `execute_js` - Run JavaScript in webview
- `call_ipc_command` - Call Tauri commands directly

## Workflow

1. Build the app: `npm run tauri build`
2. Launch via MCP: `launch_app` with path to built app
3. Wait for window to be ready
4. Navigate using keyboard shortcuts
5. Capture screenshots
6. Verify against design docs
7. Stop app when done

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+1 | Kanban view |
| Cmd+2 | Ideation view |
| Cmd+3 | Extensibility view |
| Cmd+4 | Activity view |
| Cmd+5 | Settings view |
| Cmd+K | Toggle Chat panel |

## Screenshot Locations

Save screenshots to: `screenshots/visual-audit/`

Naming convention: `{view}-{state}-{timestamp}.png`
- `kanban-default-20260125.png`
- `kanban-task-hover-20260125.png`
- `ideation-chat-open-20260125.png`

## Verification Checklist

For each screenshot, verify:
- [ ] No purple/blue gradients
- [ ] SF Pro font (not Inter)
- [ ] Warm orange accent (#ff6b35)
- [ ] Layered shadows on elevated elements
- [ ] Proper spacing and alignment
```

### Phase 4: Grant macOS Permissions

The tauri-mcp server requires:
1. **Accessibility** permission (for input simulation)
2. **Screen Recording** permission (for screenshots)

Add to setup instructions in CLAUDE.md.

### Phase 5: Update Visual Audit Tasks

Update ALL visual audit tasks in `prd_phase_14_implementation.md` to:

1. **Add mandatory skill reference** at the start of steps
2. **Remove all agent-browser references**
3. **Use tauri-mcp tools explicitly**

**New visual audit task template:**

```json
{
  "category": "visual-audit",
  "description": "Visual audit: [Component Name]",
  "design_doc": "specs/design/pages/[component].md",
  "skill": "tauri-visual-test",
  "steps": [
    "MANDATORY: Use /tauri-visual-test skill for all visual testing (agent-browser does NOT work with Tauri)",
    "Ensure RalphX app is running (npm run tauri dev)",
    "Use tauri-mcp take_screenshot to capture current state",
    "Use tauri-mcp send_keyboard_input to navigate (Cmd+1 for Kanban, Cmd+2 for Ideation, etc.)",
    "Capture screenshot after navigation",
    "For hover states: use send_mouse_click to position, then take_screenshot",
    "For modals: use execute_js or send_mouse_click to trigger, then take_screenshot",
    "Save screenshots to screenshots/visual-audit/[component]/",
    "Verify against Acceptance Criteria in design doc",
    "Verify against Design Quality Checklist in design doc",
    "Check: NO purple/blue gradients, NO Inter font, warm orange accent",
    "If issues found: fix using /frontend-design skill",
    "Re-capture screenshots after fixes",
    "Commit fixes if any: fix: [component] visual polish"
  ],
  "passes": false
}
```

**Key changes:**
- First step explicitly says skill is MANDATORY and agent-browser does NOT work
- All screenshot/interaction steps use tauri-mcp tools by name
- Screenshots saved to organized directory structure

### Phase 6: Create Screenshot Directory Structure

```
screenshots/
└── visual-audit/
    ├── kanban/
    ├── ideation/
    ├── settings/
    ├── activity/
    ├── extensibility/
    ├── modals/
    └── panels/
```

---

## Alternative: Simpler Approach Without MCP

If MCP setup is too complex, use a shell-based approach:

### Script: `scripts/visual-test.sh`

```bash
#!/bin/bash

# Start the app
npm run tauri dev &
APP_PID=$!

# Wait for window
sleep 5

# Get window ID
WINDOW_ID=$(GetWindowID "RalphX")

# Capture screenshots
screencapture -l$WINDOW_ID -o screenshots/kanban.png

# Send keyboard shortcut for next view
osascript -e 'tell application "System Events" to keystroke "2" using command down'
sleep 1
screencapture -l$WINDOW_ID -o screenshots/ideation.png

# Continue for other views...

# Cleanup
kill $APP_PID
```

This requires:
- `brew install smokris/getwindowid/getwindowid`
- Screen Recording permission for Terminal

---

## Comparison

| Approach | Pros | Cons |
|----------|------|------|
| **Tauri MCP** | Full integration, Claude can drive directly, rich interaction | Setup complexity, macOS permissions |
| **Shell Script** | Simple, no dependencies | Manual, limited interaction, can't inspect DOM |
| **In-App Plugin** | Works from app context | Requires app modifications, harder to trigger externally |

---

## Recommendation

1. **Start with Tauri MCP** - It's the most powerful and integrates with Claude Code
2. **Fall back to shell script** if MCP proves problematic
3. **Document the setup** in CLAUDE.md for future sessions

---

## Tasks

- [ ] Install tauri-mcp: `cargo install tauri-mcp`
- [ ] Add MCP config to `.mcp.json` in project root
- [ ] Create `ralphx-plugin/skills/tauri-visual-test/SKILL.md`
- [ ] Grant macOS permissions (Accessibility + Screen Recording)
- [ ] Create `screenshots/visual-audit/` directory structure
- [ ] Update ALL 15 visual audit tasks in `prd_phase_14_implementation.md`:
  - Add `"skill": "tauri-visual-test"` field
  - Add MANDATORY skill instruction as first step
  - Replace agent-browser references with tauri-mcp tools
  - Update screenshot save paths
- [ ] Update final verification task similarly
- [ ] Test with Kanban visual audit
- [ ] Document setup in CLAUDE.md under new "Visual Testing" section

---

## References

- [Tauri MCP Server](https://github.com/dirvine/tauri-mcp)
- [MCP Protocol](https://modelcontextprotocol.io/)
- [Tauri Testing Docs](https://v2.tauri.app/develop/tests/)
- [macOS screencapture](https://ss64.com/mac/screencapture.html)
