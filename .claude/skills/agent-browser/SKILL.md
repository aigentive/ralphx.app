---
name: agent-browser
description: Browser automation for visual testing and verification
---

# Agent Browser Skill

Headless browser automation for visual verification of UI implementations.

## Quick Reference

### Navigation
- `agent-browser open <url>` — Open URL
- `agent-browser close` — Close browser
- `agent-browser reload` — Refresh page

### Page Analysis
- `agent-browser snapshot` — Full DOM snapshot with element refs (@e1, @e2...)
- `agent-browser snapshot -i` — Interactive elements only (recommended)
- `agent-browser snapshot -c` — Compact output
- `agent-browser snapshot -i -c` — Interactive + compact (best for verification)

### Screenshots
- `agent-browser screenshot <path.png>` — Capture viewport
- `agent-browser screenshot --full <path.png>` — Full page screenshot

### Interactions
- `agent-browser click @e1` — Click element by reference
- `agent-browser fill @e1 "text"` — Fill input field
- `agent-browser type @e1 "text"` — Type character by character
- `agent-browser press Enter` — Press key
- `agent-browser hover @e1` — Hover over element
- `agent-browser scroll @e1` — Scroll element into view

### Data Extraction
- `agent-browser get text @e1` — Get text content
- `agent-browser get value @e1` — Get input value
- `agent-browser get attr @e1 href` — Get attribute

### State Verification
- `agent-browser is visible @e1` — Check visibility
- `agent-browser is enabled @e1` — Check if enabled
- `agent-browser is checked @e1` — Check checkbox state

### Wait Conditions
- `agent-browser wait @e1` — Wait for element
- `agent-browser wait 2000` — Wait milliseconds
- `agent-browser wait --load` — Wait for page load

## Verification Workflow

1. Start app: `npm run tauri dev`
2. Open browser: `agent-browser open http://localhost:1420`
3. Analyze page: `agent-browser snapshot -i -c`
4. Capture proof: `agent-browser screenshot screenshots/[task-name].png`
5. Test interactions if applicable
6. Close: `agent-browser close`
