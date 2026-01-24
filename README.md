<p align="center">
  <img src="assets/logo.png" alt="RalphX" width="120" />
</p>

<h1 align="center">RalphX</h1>

<p align="center">
  <strong>The control room for autonomous AI development.</strong>
  <br />
  <sub>Born from the Ralph Wiggum Loop. Built for engineers who run AI in production.</sub>
</p>

<p align="center">
  <a href="#youve-outgrown-the-terminal">Why</a> •
  <a href="#features">Features</a> •
  <a href="#how-it-works">How It Works</a> •
  <a href="#getting-started">Get Started</a> •
  <a href="#architecture">Architecture</a>
</p>

<p align="center">
  <img src="assets/hero.gif" alt="RalphX in action" width="800" />
</p>

---

## You've Outgrown the Terminal

You're already running Claude in loops. Fresh context per iteration.
Tasks completing while you sleep. No context debt.

But the interface hasn't caught up:

- **Editing JSON by hand** — `manifest.json`, `prd.md`, task lists
- **Tailing logs** — `tail -f logs/iteration_42.json` to see what's happening
- **No mid-loop control** — Can't inject a task without stopping everything
- **Parallel projects = chaos** — Multiple terminals, mental overhead
- **Review after the fact** — Reading git diffs when it's already committed

The loop works. The tooling doesn't.

---

## Your Loop, With a Dashboard

RalphX wraps your autonomous workflows in a native Mac app:

| Instead of... | You get... |
|---------------|------------|
| `tail -f logs/` | Real-time activity stream |
| Editing JSON task lists | Drag-and-drop Kanban |
| Stopping the loop to add a task | Task injection mid-execution |
| One project per terminal | Parallel projects, one window |
| Reading diffs after commit | Review gates before merge |
| Manual worktree setup | Automatic isolation per project |

Same principles. Proper tooling.

---

## Why "RalphX"?

RalphX evolved from the [Ralph Wiggum Loop](https://github.com/anthropics/claude-code/discussions/1574) — an autonomous development pattern that runs Claude iteratively with fresh context windows until all tasks complete.

The loop proved that AI can build entire systems autonomously.
RalphX is the interface that makes it practical.

---

## Features

### Task Management

**Drag-and-drop Kanban with auto-execution**

Move a task to "Planned" and it executes automatically. No buttons to click.

| Column | What Happens |
|--------|--------------|
| Draft | Ideas from brainstorming |
| Backlog | Confirmed but deferred |
| To-do | Ready when you are |
| **Planned** | Queued for auto-execution |
| In Progress | Running now (live status) |
| In Review | AI verifying the work |
| Done | Approved and complete |

<p align="center">
  <img src="assets/kanban.png" alt="Kanban board" width="700" />
</p>

---

### Multi-Agent System

**Specialized agents. Not one model doing everything.**

| Agent | Job | Model |
|-------|-----|-------|
| **Worker** | Writes code, runs tests, commits | Sonnet |
| **Reviewer** | Code review, security checks | Sonnet |
| **Supervisor** | Watchdog for infinite loops | Haiku |
| **Orchestrator** | Plans tasks, answers questions | Opus |

Each agent has its own tools, guardrails, and context. The Worker writes. The Reviewer critiques. The Supervisor intervenes if something goes wrong.

---

### Ideation System

**From conversation to executable tasks**

Open the chat panel (`⌘K`) and talk to the Orchestrator:

> "Let's add user authentication with OAuth"

The Orchestrator breaks it down into **proposals** with:
- Priority scores
- Dependency analysis
- Effort estimates

Review, adjust, then apply to your Kanban with one click.

<p align="center">
  <img src="assets/ideation.png" alt="Ideation panel" width="700" />
</p>

---

### Real-Time Activity Stream

**Watch Claude think**

Every tool call, file read, and decision—streamed live. Expand any action to see the full context. Search through history.

<p align="center">
  <img src="assets/activity.gif" alt="Activity stream" width="700" />
</p>

---

### Git Worktree Isolation

**Your branch stays untouched**

RalphX creates an isolated worktree for each project. AI commits go to a separate branch while you keep working on yours.

```
Your repo:        ~/projects/my-app (your branch)
RalphX worktree:  ~/ralphx-worktrees/my-app (ralphx/feature-auth)
```

When done: review the diff, merge, cherry-pick, or discard. Your choice.

---

### QA & Review System

**Multi-level verification before anything reaches "Done"**

1. **AI Review** — Automatic code review after each task
2. **Human Review** — Escalation for architecture or security decisions
3. **QA Testing** — Acceptance criteria validation with test steps

Tasks can't slip through. Every state change is logged with timestamps and reasons.

---

### Extensible Workflows

**Your methodology, your rules**

Built-in support for:
- **BMAD** — Breakthrough Method for Agile AI-Driven Development
- **GSD** — Get Shit Done spec-driven workflows
- **Custom** — Define your own columns, agents, and artifact flows

Swap methodologies per project. The internal state machine handles the mapping.

---

## How It Works

```
1. Create a project     →  Point RalphX at any folder
2. Add tasks            →  Chat, import, or create manually
3. Drag to Planned      →  Task queues for execution
4. Watch it work        →  Real-time activity stream
5. Review and approve   →  AI or human checkpoints
6. Merge when ready     →  Git handles the rest
```

### Under the Hood

```
┌─────────────────────────────────────────────────────────────────┐
│                      RalphX (Native Mac App)                     │
├─────────────────────────────────────────────────────────────────┤
│  React UI           │  Tauri Bridge      │  Rust Backend        │
│  • Kanban board     │  • IPC events      │  • SQLite database   │
│  • Activity stream  │  • Commands        │  • State machine     │
│  • Chat interface   │  • Real-time sync  │  • Agent scheduler   │
└─────────────────────┴────────────────────┴──────────────────────┘
                               │
                    ┌──────────┴──────────┐
                    │    Claude CLI       │
                    │  (your credentials) │
                    └─────────────────────┘
```

**Key design decisions:**
- **Fresh context per task** — No accumulated context debt
- **14-state lifecycle** — Predictable task progression
- **Event-driven UI** — Real-time updates, no polling
- **Local SQLite** — Your data stays on your machine

---

## Who It's For

**Engineers who already:**
- Run Claude in autonomous loops
- Understand fresh context windows and iteration patterns
- Use git worktrees for parallel work
- Want to scale from one project to many

**And want:**
- Real-time visibility without tailing logs
- Task injection without restarting loops
- Review gates that don't break flow
- A proper interface instead of shell scripts

**Not for you if:**
- You're looking for a Copilot replacement (this is orchestration, not autocomplete)
- You want zero human oversight
- You're on Windows/Linux (Mac-only for now)

---

## Getting Started

### Prerequisites

- macOS 13+ (Ventura or later)
- [Claude CLI](https://claude.ai/code) installed and authenticated

### Installation

```bash
# Clone the repository
git clone https://github.com/anthropics/ralphx
cd ralphx

# Install dependencies
npm install

# Build and run
npm run tauri dev
```

### First Run

1. Open RalphX
2. Create a new project (select any folder)
3. Press `⌘K` to open chat
4. Describe what you want to build
5. Apply the generated proposals to your Kanban
6. Drag tasks to "Planned" and watch them execute

---

## Architecture

| Layer | Technology | Why |
|-------|------------|-----|
| Desktop | **Tauri 2.0** | 10MB bundle, 30MB RAM |
| Frontend | **React 19 + TypeScript** | Type-safe, fast iteration |
| State | **Zustand + TanStack Query** | Minimal, reactive |
| Backend | **Rust** | Performance, safety |
| Database | **SQLite** | Local-first, portable |
| AI | **Claude Agent SDK** | Native streaming |
| Styling | **Tailwind CSS** | Utility-first |

For the complete 9,000+ line specification, see [specs/plan.md](specs/plan.md).

---

## Roadmap

| Phase | Status |
|-------|--------|
| Core Kanban + Task Execution | Complete |
| Multi-Agent Orchestration | Complete |
| QA & Review System | Complete |
| Ideation System | Complete |
| Workflows & Methodologies | In Progress |
| VM Isolation | Planned |

---

## Links

- [Full Specification](specs/plan.md) — Complete technical spec
- [Changelog](CHANGELOG.md) — Release notes
- [License](LICENSE) — MIT

---

<p align="center">
  <sub>Built with Claude. Controlled by humans.</sub>
</p>
