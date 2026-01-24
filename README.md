<p align="center">
  <img src="assets/logo.png" alt="RalphX" width="120" />
</p>

<h1 align="center">RalphX</h1>

<p align="center">
  <strong>The control room for autonomous AI development.</strong>
</p>

<p align="center">
  <a href="#the-core-insight">Core Insight</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#features">Features</a> •
  <a href="#getting-started">Get Started</a>
</p>

<p align="center">
  <img src="assets/hero.gif" alt="RalphX in action" width="800" />
</p>

---

**RalphX** is a native Mac desktop application for orchestrating autonomous AI development. It evolved from the [Ralph Wiggum Loop](https://github.com/anthropics/claude-code/discussions/1574) pattern—fresh context per task, specialized agents, human checkpoints—but serves anyone running Claude for serious development work.

### Core Concept

You can run Claude CLI in multiple terminal tabs, but you're managing each session manually—no unified view, no coordination, no checkpoints. RalphX gives you a proper control room:

- Orchestrates Claude agents via the **Claude Agent SDK**
- Stores project state in a **local database** (not scattered files)
- Provides **real-time visibility** across all running agents
- Supports **multiple concurrent projects** in one window
- Enables **human-in-the-loop checkpoints** and task injection mid-execution
- **Extensible architecture** supporting custom workflows, methodologies (BMAD, GSD), and Claude Code plugins

### The Problem It Solves

| Terminal Tabs | RalphX |
|---------------|--------|
| Editing specs and task lists by hand | Visual task management with Kanban |
| Multiple terminals, mental overhead | Unified dashboard for all projects |
| No visibility into what Claude is doing | Real-time activity stream |
| Can't inject tasks mid-execution | Task injection without stopping |
| Review diffs after the fact | Review gates before merge |
| Manual git worktree setup | Automatic branch isolation |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     RalphX (Tauri Application)                   │
├─────────────────────────────────────────────────────────────────┤
│  Frontend (React + TypeScript)                                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐    │
│  │ Project     │ │ Task Board  │ │ Agent Activity Stream   │    │
│  │ Selector    │ │ (Kanban)    │ │                         │    │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘    │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐    │
│  │ Ideation    │ │ Review      │ │ Settings                │    │
│  │ Chat        │ │ Panel       │ │                         │    │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│  Tauri IPC Bridge                                                │
├─────────────────────────────────────────────────────────────────┤
│  Backend (Rust)                                                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐    │
│  │ Agent       │ │ State       │ │ Database                │    │
│  │ Scheduler   │ │ Machine     │ │ (SQLite)                │    │
│  └─────────────┘ └─────────────┘ └─────────────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│  Claude Agent SDK (via Claude CLI)                               │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌──────────┐  │
│  │ Worker      │ │ Reviewer    │ │ Supervisor  │ │ Orchestr │  │
│  │ (Sonnet)    │ │ (Sonnet)    │ │ (Haiku)     │ │ (Opus)   │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └──────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**Design decisions:**

| Aspect | Choice | Why |
|--------|--------|-----|
| Desktop | Tauri 2.0 | 10MB bundle, 30MB RAM (vs Electron's 100MB/300MB) |
| Frontend | React 19 + TypeScript | Type-safe, fast iteration |
| Backend | Rust | Performance, safety |
| Database | SQLite | Local-first, portable, no server |
| State | 14-status state machine | Predictable task lifecycle |
| Updates | Event-driven | Real-time UI, no polling |

---

## Multi-Agent System

**Specialized agents. Not one model doing everything.**

| Agent | Role | Model | When It Runs |
|-------|------|-------|--------------|
| **Worker** | Writes code, runs tests, commits | Sonnet | Task execution |
| **Reviewer** | Code review, security checks | Sonnet | After task completion |
| **Supervisor** | Watchdog for infinite loops | Haiku | Continuous monitoring |
| **Orchestrator** | Plans tasks, answers questions | Opus | Chat interface |
| **QA Prep** | Generates acceptance criteria | Sonnet | Background, parallel with execution |
| **QA Executor** | Browser testing, visual verification | Sonnet | After implementation |

Each agent has its own tools, guardrails, and context. The Worker writes. The Reviewer critiques. The Supervisor intervenes if something goes wrong.

---

## Features

### Task Board (Kanban)

**Drag to Planned = auto-executes**

| Column | What Happens |
|--------|--------------|
| Draft | Ideas from brainstorming |
| Backlog | Confirmed but deferred |
| To-do | Ready when you are |
| **Planned** | Queued for auto-execution |
| In Progress | Running now (live status) |
| In Review | AI verifying the work |
| Done | Approved and complete |

Move a task to "Planned" and it executes automatically. No buttons to click. Priority determined by position—drag to top means "do next."

<p align="center">
  <img src="assets/kanban.png" alt="Kanban board" width="700" />
</p>

---

### 14-State Task Lifecycle

Every task moves through a predictable state machine:

```
backlog → ready → executing → execution_done
                      ↓
              qa_refining → qa_testing → qa_passed
                                ↓
                           qa_failed → revision_needed
                                            ↓
                                       (retry execution)
                                            ↓
              pending_review → approved (terminal)
                    ↓
               revision_needed → executing (rework)
```

**Each state = one operation:**
- `executing` = Worker agent running
- `qa_testing` = Browser tests only
- `pending_review` = AI reviewer only
- No compound states. Full observability.

---

### Ideation System

**From conversation to executable tasks**

Open the chat panel (`⌘K`) and talk to the Orchestrator:

> "Let's add user authentication with OAuth"

The Orchestrator breaks it down into **task proposals** with:

| Field | Description |
|-------|-------------|
| Priority score | 0-100 based on dependencies, critical path, business value |
| Dependencies | Which tasks must complete first |
| Complexity | trivial / simple / moderate / complex / very_complex |
| Acceptance criteria | Auto-generated steps |

Review, adjust, apply to Kanban with one click.

<p align="center">
  <img src="assets/ideation.png" alt="Ideation panel" width="700" />
</p>

---

### Review System

**Multi-level verification before anything reaches Done**

| Stage | Reviewer | Action |
|-------|----------|--------|
| AI Review | Sonnet | Auto-triggered after execution |
| Human Review | You | Escalated for security/architecture |
| QA Testing | Browser agent | Visual verification against criteria |

**Review outcomes:**
- **Approve** → Task moves to Done
- **Request Changes** → Auto-creates fix task, re-executes
- **Escalate** → Requires human decision

Max 3 automatic fix attempts before requiring human intervention.

---

### Supervisor (Watchdog)

**Lightweight monitoring, heavy intervention**

The Supervisor doesn't run constantly. It monitors events and escalates only when needed:

| Pattern | Trigger | Response |
|---------|---------|----------|
| Loop detected | Same tool called 3+ times | Inject guidance or pause |
| Stuck | No file changes for 5+ minutes | Alert + analysis |
| Runaway | Token usage > 50k without progress | Kill task |
| Error loop | Same error repeating | Pause for investigation |

Uses Haiku for analysis—fast and cheap. Only invoked when anomaly detected.

---

### Git Worktree Isolation

**Your branch stays untouched**

RalphX creates an isolated worktree for each project:

```
Your repo:        ~/projects/my-app (your branch)
RalphX worktree:  ~/ralphx-worktrees/my-app (ralphx/feature-auth)
```

AI commits go to a separate branch. When done:
- View the diff
- Merge, cherry-pick, or discard
- Your choice

---

### Activity Stream

**Watch Claude think**

Every tool call, file read, and decision—streamed live. Events batched at 50ms for smooth UI. Expand any action to see full context.

<p align="center">
  <img src="assets/activity.gif" alt="Activity stream" width="700" />
</p>

---

## Extensibility

### Workflows

Define your own Kanban columns while RalphX handles the underlying state machine. Want a simple 4-column board? A complex review pipeline? Your columns, your labels—the execution engine adapts.

### Methodologies

Plug in structured development approaches:

| Methodology | What It Brings |
|-------------|----------------|
| **BMAD** | 8 specialized agents, 4-phase delivery, document-driven workflow |
| **GSD** | 11 agents, wave-based parallel execution, checkpoint protocols |
| **Custom** | Define your own agent roles, phases, and artifacts |

### Claude Code Plugin

RalphX ships as a Claude Code plugin—agents, skills, and hooks you can use standalone or extend:
- **Agents**: Worker, Reviewer, Supervisor, Orchestrator, QA
- **Skills**: Coding standards, testing patterns, git workflow, review checklists
- **Hooks**: Event-driven automation for your own workflows

---

## Tech Stack

| Layer | Technology | Version |
|-------|------------|---------|
| Desktop | Tauri | 2.0 |
| Frontend | React | 19 |
| Language | TypeScript | 5.8 |
| State | Zustand + TanStack Query | Latest |
| Backend | Rust | 1.83+ |
| Database | SQLite (rusqlite) | 3.x |
| AI | Claude Agent SDK | Latest |
| Styling | Tailwind CSS | 4.x |

---

## Who This Is For

**Engineers who already:**
- Run Claude in autonomous loops
- Understand context engineering
- Use git worktrees for parallel work
- Want to scale from one project to many

**And want:**
- Real-time visibility without tailing logs
- Task injection without restarting loops
- Review gates that don't break flow
- A proper interface instead of shell scripts

**Not for you if:**
- Looking for a Copilot replacement (this is orchestration, not autocomplete)
- Want zero human oversight
- On Windows/Linux (Mac-only for now)

---

## Getting Started

### Prerequisites

- macOS 13+ (Ventura or later)
- [Claude CLI](https://claude.ai/code) installed and authenticated

### Installation

```bash
git clone https://github.com/anthropics/ralphx
cd ralphx
npm install
npm run tauri dev
```

### First Run

1. Open RalphX
2. Create a new project (select any folder)
3. Press `⌘K` to open chat
4. Describe what you want to build
5. Apply generated proposals to Kanban
6. Drag tasks to "Planned" and watch them execute

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

## Building RalphX

**This app is built autonomously by the Ralph loop itself.**

The specification lives in [`specs/plan.md`](specs/plan.md) (9,000+ lines). Tasks are ordered so progress is incremental and visible at each phase.

---

## Links

- [Full Specification](specs/plan.md) — Complete technical spec
- [Changelog](CHANGELOG.md) — Release notes
- [License](LICENSE) — MIT

---

<p align="center">
  <sub>Built with Claude. Controlled by humans.</sub>
</p>
