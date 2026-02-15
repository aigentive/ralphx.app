# Decision: Agent Teams Product Briefs Approved

**Date:** 2026-02-15
**Status:** APPROVED
**Approver:** Product Owner (Laza Bogdan)

## Summary

All 6 agent teams product briefs have been reviewed and approved for implementation planning.

## Approved Briefs

| # | Brief | Path | Version | Scope |
|---|-------|------|---------|-------|
| 1 | Ideation Integration | `docs/product-briefs/agent-teams-ideation-integration.md` | v5 | Agent teams at orchestrator-ideation level — Research & Debate team modes |
| 2 | Worker Integration | `docs/product-briefs/agent-teams-worker-integration.md` | v3 | Agent teams at worker level for parallel task execution |
| 3 | Configurable Agent Variants | `docs/product-briefs/configurable-agent-variants.md` | v5 | YAML process mapping, variant inheritance, runtime switching |
| 4 | Chat UI Extension (Timeline) | `docs/product-briefs/agent-teams-chat-ui-extension.md` | v1 | Extend existing chat UI with multi-agent timeline and team events |
| 5 | Split-Pane UI (tmux-inspired) | `docs/product-briefs/agent-teams-split-pane-ui.md` | v1 | Full-screen split-pane layout with coordinator left, teammates stacked right |
| 6 | UI Decision | `docs/product-briefs/agent-teams-ui-decision.md` | v2 | Phased Hybrid recommendation — timeline first, split-pane as power mode |

## Supporting Documentation

| Doc | Path | Purpose |
|-----|------|---------|
| Claude Spawning System | `docs/architecture/claude-spawning-system.md` | Technical reference for CLI spawning pipeline + agent teams CLI flags |
| Agent Catalog | `docs/architecture/agent-catalog.md` | All 20 RalphX agents documented with workflow diagrams |
| YAML Config Management | `docs/architecture/ralphx-yaml-config.md` | Configuration schema, profiles, tool allowlist generation |
| Agent Teams System Card | `docs/agent-teams-system-card.md` | Exhaustive reference: 15 sections + 3 appendices covering all team tools, CLI flags, communication patterns |

## Key Decisions Locked In

### Architecture
- Dynamic team composition by default; constrained mode opt-in via YAML
- Pre-flight MCP validation (`request_team_plan`) for v1; custom `spawn_teammate` MCP tool for endgame
- Teammates must be interactive (no `-p` flag); lead can use `-p` mode
- Both `CLAUDECODE=1` and `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` required
- New lightweight `ideation-team-lead` coordinator (not reusing orchestrator-ideation)
- Config provides constraints (ceilings), not rigid role definitions

### Artifact Model
- Extend existing artifact system with `parent_artifact_id` + `metadata_json` (no new tables)
- 3 new artifact types: TeamResearch, TeamAnalysis, TeamSummary
- 1 new bucket: `team-findings`
- 2 thin MCP wrappers: `create_team_artifact`, `get_team_artifacts`
- Artifacts persist indefinitely

### UI Strategy (Phased Hybrid — score 4.10/5.0)
- **Phase 1**: Chat timeline extension — ships first, lower risk, provides shared infrastructure
- **Phase 2**: Split-pane power mode — for 2-4 agent scenarios, reuses 70% of Phase 1
- **Phase 3**: Display mode toggle with smart defaults (split-pane for ideation debate, timeline for execution teams)

### Product Defaults
- 5 default specialists per team
- No budget cap (configurable via `team_constraints.budget_limit`)
- Per-teammate cost display
- Side-by-side debate UI (stacked cards for narrow viewports)
- Team-ideated plans tagged with `team_ideated: true` metadata
- Trust the lead fully on prompt quality (no validation)
- Show predefined roles before spawn in constrained mode
- Workers can document decisions via team artifacts
- Team resume supported in RECOVER phase
- Lead creates TeamSummary artifact (≤2000 tokens) for session resume

### Integration Points
- Both ideation and worker integration ship in Phase 1
- Additive only — agent teams are opt-in, current flows preserved as default
- Users can message team lead AND individual teammates directly
- No new ChatContextTypes — team mode is boolean flag on existing contexts

## Next Steps

1. Convert approved briefs into implementation PRDs with task breakdowns
2. Prioritize Phase 1 implementation order (likely: configurable variants → ideation teams → worker teams → chat timeline UI)
3. Validate `-p` mode + interactive teammate spawning in a prototype
4. Design database migration for artifact model extensions
