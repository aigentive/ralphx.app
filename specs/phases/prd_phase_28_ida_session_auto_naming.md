# RalphX - Phase 28: IDA Session Auto-Naming

## Overview

Add auto-generated session titles for IDA conversations using a lightweight Haiku agent, with real-time streaming updates and manual rename capability. When a user sends their first message in an IDA session, a background agent generates a concise title based on the message content and updates the session in real-time via MCP tools and Tauri events.

**Reference Plan:**
- `specs/plans/ida-session-auto-naming.md` - Detailed implementation plan with component specifications and event flow

## Goals

1. Auto-generate meaningful session titles from the user's first message using Haiku
2. Provide real-time title updates via Tauri events without page refresh
3. Enable manual session rename via three-dot menu in SessionBrowser

## Dependencies

### Phase 27 (Chat Architecture Refactor) - Required

| Dependency | Why Needed |
|------------|------------|
| Ideation chat hooks | Session auto-naming triggers from chat send logic |
| useIdeationEvents | Extends existing event listener pattern for title updates |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/ida-session-auto-naming.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/ida-session-auto-naming.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Add update_ideation_session_title Tauri command that updates session title and emits event",
    "plan_section": "1. Backend - New Tauri Command",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '1. Backend'",
      "Add update_ideation_session_title command in ideation_commands_session.rs",
      "Call existing session_repo.update_title()",
      "Emit ideation:session_title_updated event with session_id and title",
      "Register command in lib.rs",
      "Run cargo clippy && cargo test",
      "Commit: feat(ideation): add update_ideation_session_title command"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add HTTP endpoint POST /api/update_session_title for MCP server to call",
    "plan_section": "1. Backend - New HTTP Endpoint",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '1. Backend'",
      "Add UpdateSessionTitleRequest struct in http_server/types.rs",
      "Add update_session_title handler in handlers/ideation.rs",
      "Handler calls session_repo.update_title() and emits ideation:session_title_updated event",
      "Add route .route('/api/update_session_title', post(update_session_title)) in http_server/mod.rs",
      "Run cargo clippy && cargo test",
      "Commit: feat(http): add update_session_title endpoint"
    ],
    "passes": true
  },
  {
    "category": "mcp",
    "description": "Add update_session_title MCP tool with session-namer agent allowlist",
    "plan_section": "2. MCP Server",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '2. MCP Server'",
      "Add tool definition to ALL_TOOLS array in ralphx-mcp-server/src/tools.ts",
      "Add session-namer to TOOL_ALLOWLIST with access to update_session_title",
      "No index.ts changes needed - uses default POST routing",
      "Test tool registration with RALPHX_AGENT_TYPE=session-namer",
      "Commit: feat(mcp): add update_session_title tool"
    ],
    "passes": true
  },
  {
    "category": "agent",
    "description": "Create session-namer agent that generates titles from first message",
    "plan_section": "3. Plugin Agent",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '3. Plugin Agent'",
      "Create ralphx-plugin/agents/session-namer.md",
      "Set model: haiku in frontmatter",
      "Write system prompt for generating 3-6 word titles",
      "Include instruction to call update_session_title with session_id and title",
      "Test with CLI: claude --plugin-dir ./ralphx-plugin --agent session-namer",
      "Commit: feat(plugin): add session-namer agent"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add spawn_session_namer Tauri command to spawn naming agent in background",
    "plan_section": "5. Triggering Mechanism",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '5. Triggering Mechanism'",
      "Add spawn_session_namer command in ideation_commands_session.rs",
      "Use AgenticClientSpawner to spawn session-namer agent",
      "Pass session_id and first_message as context in prompt",
      "Return immediately (fire-and-forget background task)",
      "Register command in lib.rs",
      "Run cargo clippy && cargo test",
      "Commit: feat(ideation): add spawn_session_namer command"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add API wrappers and event listener for session title updates",
    "plan_section": "4. Frontend",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '4. Frontend'",
      "Add updateTitle wrapper in src/api/ideation.ts",
      "Add spawnSessionNamer wrapper in src/api/ideation.ts",
      "Add ideation:session_title_updated listener in useIdeationEvents.ts",
      "Listener should call ideationStore.updateSession() with new title",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add title update API and event listener"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Trigger session namer on first message send in IDA chat",
    "plan_section": "5. Triggering Mechanism - Frontend Call",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '5. Triggering Mechanism'",
      "Find chat send logic in useIdaChat or IntegratedChatPanel",
      "After first message sent (message count 0->1), call spawnSessionNamer",
      "Pass sessionId and firstMessage content",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): trigger auto-naming on first message"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add three-dot menu with rename option to SessionBrowser items",
    "plan_section": "4. Frontend - UI Changes",
    "steps": [
      "Read specs/plans/ida-session-auto-naming.md section '4. Frontend'",
      "Add DropdownMenu to session items in SessionBrowser.tsx",
      "Options: Rename, Archive, Delete",
      "Implement inline edit mode for rename",
      "Call updateTitle API on rename confirm",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(ideation): add session context menu with rename"
    ],
    "passes": true
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Haiku model for naming** | Lightweight, fast, cost-effective for simple title generation |
| **Fire-and-forget spawning** | Non-blocking UX - user doesn't wait for title to generate |
| **Event-based title updates** | Real-time UI updates without polling or page refresh |
| **MCP tool for title update** | Consistent with existing agent-to-backend communication pattern |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] update_ideation_session_title command updates DB and emits event
- [ ] spawn_session_namer spawns agent without blocking
- [ ] HTTP endpoint validates request and updates session

### Frontend - Run `npm run test`
- [ ] Event listener updates store on title change
- [ ] API wrappers correctly invoke Tauri commands

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Create new IDA session, send "I want to build a task management app", verify title appears
- [ ] Click three-dot menu on session, rename to custom title, verify persists after refresh
- [ ] Open DevTools, watch for ideation:session_title_updated event on first message

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] First message send triggers spawnSessionNamer call
- [ ] Agent spawns and calls MCP tool update_session_title
- [ ] MCP tool calls HTTP endpoint which updates DB
- [ ] Tauri event emitted and received by frontend listener
- [ ] Store updated and SessionBrowser re-renders with new title

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
