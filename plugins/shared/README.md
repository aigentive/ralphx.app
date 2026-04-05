# ralphx-shared-plugin

A Claude Code plugin providing behavioral skills for autonomous agents interacting with RalphX development pipelines.

## What is this?

This plugin teaches external Claude Code agents **judgment** for navigating the RalphX pipeline: when to act vs observe, how to handle edge cases, what events mean for decision-making. It supplements the `v1_get_agent_guide` API reference (tool schemas, sequencing rules) with operational playbook content.

**Audience:** External autonomous agents interacting with RalphX via the External MCP API.

**Naming note:**
- examples in this plugin use canonical RalphX method names: `v1_*`
- Claude/Codex MCP wrappers in some environments are typically `mcp__ralphx__v1_*`
- ReefBot integration uses `ralphx__v1_*`

Never derive tool names from the skill name.

**Not for:** Internal RalphX agents (those use `plugins/app/` inside the RalphX app).

## Installation

### Method 1: Local path (development/testing)

Clone or access the RalphX repository, then load the plugin directly:

```bash
claude --plugin-dir ./plugins/shared
```

Load alongside other plugins:

```bash
claude --plugin-dir ./plugins/shared --plugin-dir ./other-plugin
```

To pick up changes without restarting:

```shell
/reload-plugins
```

### Method 2: Marketplace installation

If a marketplace has been configured for this plugin, add the marketplace and install:

```shell
# Add the marketplace (GitHub format)
/plugin marketplace add <org>/<repo>

# Install the plugin
/plugin install ralphx-shared-plugin@<marketplace-name>
```

After installing, run `/reload-plugins` to activate the plugin.

See [Discover and install plugins](https://code.claude.com/docs/en/discover-plugins) for full marketplace instructions.

## Usage

### Basic invocation

After loading the plugin, the skill is available as:

```shell
/ralphx-shared-plugin:ralphx-swe
```

This loads the full playbook: bootstrap steps, core principles, do's and don'ts, quick decision guide, and reference navigation.

### Section arguments

Pass a section name to load specific content:

```shell
/ralphx-shared-plugin:ralphx-swe quick-start
```

| Argument | Loads |
|----------|-------|
| `quick-start` | Bootstrap (2 startup steps) + Quick Decision Guide |
| `state-machine` | Full 24-state pipeline reference |
| `decisions` | 6 ASCII decision trees for common scenarios |
| `events` | All 20 event types and recommended reactions |
| `recovery` | 5 step-by-step failure recovery procedures |
| `dos-donts` | Full Do's and Don'ts table |
| `cross-project` | Cross-project orchestration reference |

### Verify the skill is loaded

After loading the plugin, run `/help` — the skill appears under the `ralphx-shared-plugin` namespace.

### Validate plugin structure

```bash
claude plugin validate .
```

Or from within Claude Code:

```shell
/plugin validate .
```

## Skill Content Overview

```
skills/ralphx-swe/
├── SKILL.md                    # Core judgment + bootstrap + dispatch
└── reference/
    ├── state-machine.md        # All 24 pipeline states + transition table
    ├── decision-trees.md       # 6 ASCII decision trees for common scenarios
    ├── event-catalog.md        # 20 event types with recommended reactions
    ├── failure-playbooks.md    # 5 step-by-step recovery procedures
    └── cross-project.md        # Cross-project orchestration reference
```

### SKILL.md

The main skill file (~180 lines). Contains:
- **Bootstrap** — 2 startup steps every session: check attention items, load tool guide
- **Core Principles** — observe before act, event-driven (passive), annotate before intervene, separate review approval from merge progression
- **Do's and Don'ts** — situations with correct and incorrect actions
- **Quick Decision Guide** — 10 most common `if X then Y` decision points with exact tool calls
- **Reference Navigation** — table pointing to the 5 reference files
- **Section Dispatch** — argument-based routing to reference files

### reference/state-machine.md

All 24 pipeline states grouped by category (Idle, Active, Transient, Waiting, Suspended, Done), with transition semantics, behavioral patterns, and why auto-transitions happen. Essential reading before handling `task:status_changed` events.

### reference/decision-trees.md

6 ASCII decision trees covering: escalated reviews, merge conflicts, blocked tasks, failed tasks, ideation verification not converging, and capacity exhaustion. Each tree uses real `v1_*` tool calls.

### reference/event-catalog.md

All 20 event types from the `RalphXEvent` discriminated union, grouped by category (task, review, merge, ideation, system), with field descriptions and recommended agent reactions per event type.

### reference/failure-playbooks.md

5 recovery procedures with step-by-step `v1_*` tool calls: `accept_plan_and_schedule` saga failure, task stuck in blocked, rate limit 429, ideation agent unexpectedly idle, and webhook delivery failure anti-patterns.

### reference/cross-project.md

Cross-project orchestration reference: what cross-project orchestration is and what it means for your external agent — how tasks appear across multiple projects, how events arrive from each project, and what (if anything) agents should do.

## Contributing / Syncing

Reference files are derived from source material in the RalphX repo. When source material changes, update accordingly:

| Reference file | Source |
|----------------|--------|
| `reference/state-machine.md` | `.claude/rules/task-state-machine.md` |
| `reference/event-catalog.md` | `plugins/app/ralphx-external-mcp/src/tools/events.ts` |
| `reference/decision-trees.md` | Cross-references states + events; verify tool names against external MCP tool list |
| `reference/failure-playbooks.md` | Verify tool names + parameters against external MCP tool list |
| `reference/cross-project.md` | `docs/features/active-plan.md` + cross-project orchestration flow |

Each reference file has a `<!-- Source: path | Last synced: date -->` comment at the top for tracking.

**Sync checklist:**
1. Check state count in `task-state-machine.md` matches `reference/state-machine.md`
2. Check event count in `events.ts` (`RalphXEvent` discriminated union) matches `reference/event-catalog.md`
3. Verify all `v1_*` tool references in decision trees and playbooks still exist
4. Keep examples canonical: `v1_*` in skill content; wrapper names only in mapping notes
5. Run `claude plugin validate .` to check structure
6. Test locally: `claude --plugin-dir ./plugins/shared`, run `/ralphx-shared-plugin:ralphx-swe`, verify skill appears in `/help`
