<p align="center">
  <img src="assets/public/framed-welcome-2026-02-22.png" alt="RalphX.app — Describe it. Ship it." width="100%">
</p>

<p align="center">
  <strong>The AI development infrastructure you own. Open source. Local-first. Yours.</strong>
</p>

<p align="center">
  <a href="#what-it-is">What It Is</a> ·
  <a href="#install">Install</a> ·
  <a href="#how-it-works">How It Works</a> ·
  <a href="#who-its-for">Who It's For</a> ·
  <a href="#documentation">Docs</a> ·
  <a href="#origin">Origin</a>
</p>

---

## What It Is

RalphX.app is a native macOS desktop application for orchestrating AI development work across planning, implementation, review, QA, and merge workflows.

Describe what you want to build. RalphX.app turns that into structured tasks, routes each step to the right agent, runs work in isolated git worktrees, reviews the result, and prepares the merge or PR according to your project settings.

RalphX.app has no hosted backend. Project state and orchestration data live locally in SQLite. Agent actions are logged, scoped, and auditable; the AI runtimes you configure still receive the context needed to perform their work.

**Designed for provider-neutral orchestration, local ownership, and reviewable AI-generated code.**

<p align="center">
  <img src="assets/public/framed-graph-2026-02-22.png" alt="RalphX.app dependency graph — critical path highlighting, tier grouping, live execution status" width="100%">
</p>

---

## Getting Started

### Requirements

To run RalphX.app:

- macOS 13+ (Ventura or later)
- At least one supported agent runtime installed and authenticated:
  - [Claude CLI](https://docs.anthropic.com/en/docs/claude-code)
  - [Codex CLI](https://developers.openai.com/codex/cli)

To build from source:

- Node.js 18+ and npm
- Rust via [rustup.rs](https://rustup.rs); this repo pins its toolchain in `rust-toolchain.toml`
- Git

RalphX.app can route different workflow lanes through different harnesses. Claude remains the default, while Codex can be enabled incrementally for supported lanes. See [`docs/user-guides/agent-harnesses.md`](docs/user-guides/agent-harnesses.md).

Harness controls are exposed directly in the desktop app:
- `Settings -> General -> Execution Agents` for worker, reviewer, re-executor, and merger lanes
- `Settings -> Ideation -> Ideation Agents` for ideation, verifier, and specialist lanes

### Install

#### Homebrew

```bash
brew tap aigentive/ralphx
brew install --cask ralphx
```

This is the recommended path. The tap installs the signed DMG for your Mac and keeps RalphX.app available through normal Homebrew updates.

#### Direct Download

- [Apple Silicon DMG](https://github.com/aigentive/ralphx.app/releases/download/v0.1.0/RalphX_0.1.0_aarch64.dmg) for M-series Macs
- [Intel DMG](https://github.com/aigentive/ralphx.app/releases/download/v0.1.0/RalphX_0.1.0_x86_64.dmg) for Intel Macs

#### Build From Source

```bash
git clone https://github.com/aigentive/ralphx.app.git ralphx.app
cd ralphx.app
cd frontend
npm install
npm run tauri dev
```

First build compiles the Rust backend. Subsequent starts are faster.

For a fresh native dev start from the repo root:

```bash
./dev-fresh
```

### First Task

1. **Create a project** — Point RalphX.app at a git repository
2. **Open Ideation** — Describe what you want to build
3. **Apply proposals** — Review the generated tasks, apply to Kanban
4. **Watch it execute** — Workers write code, reviewers check it, and RalphX.app prepares the merge or PR according to your project settings

You intervene when a review gate escalates or when your settings require human approval. Otherwise, the workflow keeps moving.

---

## How It Works

RalphX.app turns a request into planned work, creates isolated branches and worktrees, routes each step to the right agent, and keeps review, QA, merge, and PR gates explicit.

Tool access is controlled at the runtime and MCP server layers, then reinforced by agent-specific prompts:

1. **Rust spawn config** — which tools the process can call
2. **MCP server filter** — which API endpoints the agent can reach
3. **Agent system prompt** — role guidance and escalation expectations

For example: reviewers run read-only, workers cannot approve their own output, and merge flows must pass the configured validation or PR gates.

---

## Tech Stack

| Layer | Technology | Why |
|-------|------------|-----|
| **Desktop** | Tauri 2.0 | Native macOS app shell without Electron. |
| **Backend** | Rust | Memory-safe. Compile-time guarantees. No GC pauses. |
| **Frontend** | React 19 + TypeScript | Strict types. Responsive Kanban, graph view, real-time activity stream. |
| **Database** | SQLite (local) | No hosted database. Project state and orchestration history stay local. |
| **AI Runtime** | Claude + Codex via lane-based harnesses | Provider-neutral orchestration with per-lane routing, recovery, and chat lineage. |
| **State Machine** | Rust enum + exhaustive match | Runtime-enforced transitions. Compile-time exhaustiveness checking. |
| **Git** | Worktree isolation | Parallel execution. AI never touches your working directory. |

---

## Who It's For

**Solo developers** — One board for all your AI agents. Review diffs before merge. Stop managing terminal tabs.

**Solopreneurs** — AI agents are your engineering team. Describe what you want, get shipped features with review gates that catch bugs at 2 AM.

**Team leads** — Encode standards as methodology plugins. Review gates catch routine issues before human review.

**Staff+ engineers** — Methodology plugins help encode architectural standards and team practices into agent workflows.

### Not for you (yet) if

- You're on Linux or Windows (macOS only, for now)
- You don't want to install an external agent runtime (RalphX.app currently targets Claude CLI and Codex CLI)
- You need fully offline AI execution
- You need multi-user collaboration (single-developer orchestration)

---

## Documentation

| Guide | What It Covers |
|-------|----------------|
| [Getting Started](docs/user-guides/getting-started.md) | Installation, first project, first workflow |
| [Design Systems](docs/user-guides/design-systems.md) | Generate, review, export, import, and reuse source-grounded design systems |
| [Ideation Studio](docs/user-guides/ideation-studio.md) | Session modes, team configuration, plan artifacts |
| [Kanban Board](docs/user-guides/kanban.md) | Board layout, task cards, drag-and-drop, filtering |
| [Graph View](docs/user-guides/graph-view.md) | Dependency graph, critical path, timeline, Battle Mode |
| [Execution Pipeline](docs/user-guides/execution.md) | Worker/coder/reviewer agents, concurrency, recovery |
| [Merge Pipeline](docs/user-guides/merge.md) | Merge strategies, validation, conflict resolution |
| [Agent Harnesses](docs/user-guides/agent-harnesses.md) | Claude/Codex lane routing, execution agent settings, chat lineage |
| [Task State Machine](docs/user-guides/task-state-machine.md) | Task lifecycle, transitions, and invariants |
| [Agent Orchestration](docs/user-guides/agent-orchestration.md) | Agent roles, permissions, and tool scoping |
| [Configuration](docs/user-guides/configuration.md) | Project settings, model config, methodology plugins |

---

## Origin

RalphX.app began as a shell script running a Ralph Wiggum loop for orchestrating agent sessions and grew into a native macOS control room for planning, executing, reviewing, and merging AI-assisted software work.

Built independently by [one person](https://www.linkedin.com/in/laza-bogdan/) and a fleet of AI agents.

The tool was built by the thing it builds.

---

## License

Apache 2.0. See [LICENSE](LICENSE).

Use it however you want. Build commercial products with it. Modify it. Distribute it. The patent grant means your legal team can approve it.

---

<p align="center">
  <strong>RalphX.app</strong> — Describe it. Ship it.
  <br>
  <sub>Open source. Local-first. Yours.</sub>
  <br><br>
  <a href="#getting-started">Get Started</a> &middot;
  <a href="#documentation">Documentation</a>
</p>
