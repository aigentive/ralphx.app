# Memory Framework V2: Background Agents + SQLite + MCP + Rule Ingestion

## 1. Title
Memory Framework V2 for RalphX: decouple memory work from main execution agents, move canonical memory to SQLite, expose memory via MCP tools, preserve Claude rule auto-loading through scoped index files, and add automated rule ingestion + archival feedback loop.

## 2. Objective
Replace script/hook-enforced memory maintenance and knowledge capture with a backend-orchestrated, configurable, project-scoped memory system that:

1. Runs in background behind the scenes.
2. Uses dedicated memory agents on Haiku.
3. Stores canonical memory in SQLite.
4. Keeps `.claude/rules/*.md` as autoload triggers/indexes (not canonical full storage).
5. Supports rule-file-to-memory ingestion and deterministic archive snapshots for recovery.

## 3. Business / Product Goals

1. Reduce execution friction for worker/reviewer/orchestrator agents by removing stop-hook enforcement loops.
2. Improve memory quality and consistency with dedicated specialized memory workers.
3. Make memory retrieval explicit and tool-driven (MCP), reducing context bloat.
4. Preserve Claude path-trigger autoload behavior using scoped rule files with `paths:` globs.
5. Capture high-value planning/exploration discoveries, not only implementation findings.
6. Provide robust disaster recovery with filesystem archives derived from DB canonical state.

## 4. Out of Scope (V1)

1. Heartbeat-triggered maintenance jobs (deferred).
2. Bidirectional sync from archive back into DB.
3. Per-agent trigger selector UI (v1 is category-only by user decision).
4. Remote DB replication.

## 5. Key Decisions Locked

1. Trigger targeting model: category-only.
2. Rule file payload: index + retrieval guide (not full details).
3. Migration path: immediate hard switch (legacy hook enforcement removed in same release).
4. Planning capture: enabled by default but with stricter quality gates.
5. Buckets: exactly three.
6. Write permissions: specialized memory agents only.
7. Rule ingestion behavior: rewrite rules to index form and archive full reconstructed text programmatically.

## 6. Terminology

1. **Canonical Memory**: authoritative memory entries in SQLite.
2. **Rule Index File**: committed `.claude/rules/...` file with scoped summaries + memory IDs + retrieval instructions.
3. **Archive Snapshot**: full-text markdown snapshots in `.claude/memory-archive/` generated from DB.
4. **Memory Maintainer Agent**: background agent for ingestion, dedupe, optimization, index normalization.
5. **Memory Capture Agent**: background agent for extracting new high-value memory from conversations.

## 7. High-Level Architecture

### 7.1 Runtime Components

1. Tauri backend (orchestration + DB + HTTP endpoints).
2. MCP server (`ralphx-plugin/ralphx-mcp-server`) exposing memory read/write tools.
3. Dedicated memory agents:
   1. `ralphx:memory-maintainer` (Haiku)
   2. `ralphx:memory-capture` (Haiku)
4. Rule index sync service (backend).
5. Archive generation service (backend, deterministic, non-agent).

### 7.2 Trigger Flow

1. Primary trigger occurs after agent run completion in unified chat background processing (post queue drain).
2. Backend maps context to category and checks project memory settings.
3. Eligible pipelines spawn in parallel:
   1. maintenance
   2. capture
4. Each pipeline can be independently enabled/disabled by category.
5. Recursion guard prevents memory agents from triggering memory pipelines.

## 8. Category Model

Categories:

1. `planning`
2. `execution`
3. `review`
4. `merge`
5. `project_chat`

### 8.1 Context-to-Category Mapping

1. `ideation` and task/project planning-oriented chat => `planning`
2. `task_execution` => `execution`
3. `review` => `review`
4. `merge` => `merge`
5. project-level generic chat => `project_chat`

## 9. Memory Bucket Taxonomy

Exactly three buckets:

1. `architecture_patterns`
2. `implementation_discoveries`
3. `operational_playbooks`

### 9.1 Classification Guidance

1. `architecture_patterns`: subsystem relationships, state-machine behavior, invariant rules, complex data flows.
2. `implementation_discoveries`: non-obvious code-level findings, framework quirks, migration gotchas.
3. `operational_playbooks`: reproducible operational procedures, diagnostics, recovery tactics.

## 10. Settings / Configuration

## 10.1 Project Settings (new)

Add a new `memory` block in project settings persisted in DB:

```json
{
  "memory": {
    "enabled": true,
    "maintenance": {
      "categories": ["execution", "review", "merge"]
    },
    "capture": {
      "categories": ["planning", "execution", "review"]
    },
    "archive": {
      "enabled": true,
      "path": ".claude/memory-archive",
      "auto_commit": false,
      "retain_rule_snapshots": true
    }
  }
}
```

## 10.2 Defaults

1. Memory enabled by default.
2. Maintenance default categories: `execution`, `review`, `merge`.
3. Capture default categories: `planning`, `execution`, `review`.
4. Archive enabled by default.
5. Archive auto-commit disabled by default.

## 10.3 Runtime Config (`ralphx.yaml`)

Add:

1. Global memory defaults.
2. New agent definitions:
   1. `memory-maintainer` with `model: haiku`
   2. `memory-capture` with `model: haiku`
3. MCP tools allowlists for each memory agent.

## 11. API / Interface Changes

## 11.1 New MCP Read Tools (broadly available)

1. `search_memories`
2. `get_memory`
3. `get_memories_for_paths`
4. `get_conversation_transcript`

## 11.2 New MCP Write Tools (memory agents only)

1. `upsert_memories`
2. `mark_memory_obsolete`
3. `refresh_memory_rule_index`
4. `ingest_rule_file`
5. `rebuild_archive_snapshots`

## 11.3 New HTTP Endpoints in Tauri MCP Bridge

Add corresponding REST routes and handlers for all tools above in `src-tauri/src/http_server`.

## 11.4 Security / Access Control

All memory tools must pass through existing three-layer allowlist model:

1. Rust agent config / `ralphx.yaml`
2. MCP server tool allowlist (`tools.ts`)
3. Agent frontmatter `allowedTools`

Write tools are denied for non-memory agents.

## 12. Data Model Changes (SQLite)

Add migration `v22_memory_framework`.

## 12.1 Tables

### `project_memory_settings`

1. `project_id` TEXT PRIMARY KEY
2. `enabled` INTEGER NOT NULL DEFAULT 1
3. `maintenance_categories_json` TEXT NOT NULL
4. `capture_categories_json` TEXT NOT NULL
5. `archive_enabled` INTEGER NOT NULL DEFAULT 1
6. `archive_path` TEXT NOT NULL DEFAULT '.claude/memory-archive'
7. `archive_auto_commit` INTEGER NOT NULL DEFAULT 0
8. `retain_rule_snapshots` INTEGER NOT NULL DEFAULT 1
9. timestamps

### `memory_entries`

1. `id` TEXT PRIMARY KEY (UUID)
2. `project_id` TEXT NOT NULL
3. `bucket` TEXT NOT NULL
4. `title` TEXT NOT NULL
5. `summary` TEXT NOT NULL
6. `details_markdown` TEXT NOT NULL
7. `scope_paths_json` TEXT NOT NULL
8. `source_context_type` TEXT
9. `source_context_id` TEXT
10. `source_conversation_id` TEXT
11. `source_rule_file` TEXT
12. `quality_score` REAL
13. `status` TEXT NOT NULL DEFAULT 'active'
14. `content_hash` TEXT NOT NULL
15. timestamps

Indexes:

1. `(project_id, bucket, status)`
2. `(project_id, source_conversation_id)`
3. FTS index (if available) or LIKE-search index strategy.

### `memory_events`

1. `id` TEXT PRIMARY KEY
2. `project_id` TEXT NOT NULL
3. `event_type` TEXT NOT NULL
4. `actor_type` TEXT NOT NULL (`system|memory-maintainer|memory-capture`)
5. `details_json` TEXT NOT NULL
6. `created_at` TEXT NOT NULL

### `memory_rule_bindings`

1. `project_id` TEXT NOT NULL
2. `scope_key` TEXT NOT NULL
3. `rule_file_path` TEXT NOT NULL
4. `paths_json` TEXT NOT NULL
5. `last_synced_at` TEXT
6. `last_content_hash` TEXT
7. PRIMARY KEY `(project_id, scope_key)`

### `memory_archive_jobs`

1. `id` TEXT PRIMARY KEY
2. `project_id` TEXT NOT NULL
3. `job_type` TEXT NOT NULL (`memory_snapshot|rule_snapshot|full_rebuild`)
4. `payload_json` TEXT NOT NULL
5. `status` TEXT NOT NULL (`pending|running|done|failed`)
6. `error_message` TEXT
7. timestamps

## 13. Rule File Strategy (committed, path-scoped)

Rule files remain required for Claude auto-load.

## 13.1 Canonical Rule Index Format

Each managed memory rule file:

1. Must include YAML frontmatter with `paths:` globs.
2. Contains:
   1. concise, in-depth summary blocks
   2. memory ID references
   3. explicit MCP retrieval instructions
3. Must not store canonical full details.

Example shape:

```markdown
---
paths:
  - "src-tauri/src/application/**"
  - "src-tauri/src/domain/state_machine/**"
---

# Memory Index: Task Transition Semantics

## Summary
- Transition side effects must go through TransitionHandler.
- Startup reconciliation replays specific active states.

## Memory References
- `mem_01H...` (architecture_patterns)
- `mem_01J...` (implementation_discoveries)

## Retrieval
- Use `get_memories_for_paths` with affected paths.
- Use `get_memory` for full details by ID.
```

## 14. Maintainer Ingestion Loop (new)

The `memory-maintainer` agent/service must ingest user-authored `.claude/rules` continuously.

## 14.1 Ingestion Sources

1. New/changed files under `.claude/rules/`.
2. Existing unmanaged rule files discovered during periodic scans.

## 14.2 Ingestion Steps

1. Detect candidate rule files.
2. Parse frontmatter and `paths`.
3. Parse content into semantic chunks.
4. Classify chunk bucket.
5. Upsert to `memory_entries` (canonical DB).
6. Emit `memory_events`.
7. Rewrite source rule into canonical index format.
8. Enqueue archive jobs.

## 14.3 Rewriting Policy

1. Replace full rule details with compact index view.
2. Preserve and normalize `paths` globs.
3. Add memory IDs + retrieval instructions.
4. Deterministic formatting to minimize diff churn.

## 15. Archive Strategy (system-driven, non-agent)

Archive is produced by backend services from DB canonical state.

## 15.1 Why

1. Recovery if DB is corrupted/lost.
2. Auditable historical snapshots.
3. Avoid reliance on agent behavior for backups.

## 15.2 Snapshot Outputs

1. Per-memory snapshot files:
   1. `.claude/memory-archive/memories/<memory_id>.md`
2. Per-rule reconstruction snapshots:
   1. `.claude/memory-archive/rules/<scope_key>/<timestamp>.md`
3. Optional project-level consolidated snapshots:
   1. `.claude/memory-archive/projects/<project_id>/<timestamp>.md`

## 15.3 Snapshot Content Requirements

Each snapshot includes:

1. metadata header (project id, memory id, bucket, hash, generated time)
2. full reconstructed memory details
3. linkage to originating rule index file(s)

## 15.4 Sync Direction

1. DB => index files + archive files.
2. Rule authoring => ingested => DB canonical.
3. Archive is derived only; no direct archive-to-DB import in v1.

## 16. Background Orchestration Details

## 16.1 Trigger Hook Point

In `chat_service_send_background` post-completion sequence:

1. after stream outcome is persisted
2. after queued message processing finalization
3. before final housekeeping exit

## 16.2 Spawn Conditions

For each run context:

1. resolve project id
2. load `project_memory_settings`
3. if disabled -> exit
4. map to category
5. if category included in maintenance -> spawn `memory-maintainer`
6. if category included in capture -> spawn `memory-capture`
7. spawn in parallel, fire-and-forget

## 16.3 Recursion Guard

Do not trigger memory pipelines when current agent is:

1. `memory-maintainer`
2. `memory-capture`

## 16.4 Failure Handling

1. Failures logged to `memory_events`.
2. No blocking of primary user workflow.
3. Retry queue for archive and index sync jobs.

## 17. Memory Capture Quality Gates

Planning sessions are eligible but stricter:

1. Must include non-obvious, reusable system knowledge.
2. Must save re-exploration time (target >15 minutes).
3. Must not be generic advice or trivial summaries.
4. Must not duplicate existing memory entries (hash + semantic checks).

If threshold not met: emit "no_capture" event only.

## 18. Migration / Cutover Plan

Immediate hard switch in same release:

1. Remove/disable runtime reliance on:
   1. `ralphx-plugin/hooks/scripts/enforce-rule-manager.sh`
   2. `ralphx-plugin/hooks/scripts/commit-memory-files.sh`
   3. `ralphx-plugin/hooks/scripts/enforce-knowledge-capture.sh`
2. Update `ralphx-plugin/hooks/hooks.json` to remove memory stop-hook blocks.
3. Remove broad memory skill preapproval from non-memory agents in Rust agent config path.
4. Keep legacy scripts in repository for history only (not wired).

## 18.1 Optional One-Time Import

On first run post-migration:

1. Parse existing `.claude/memory/*.md` logs.
2. Parse `.claude/rules/.optimization-log.md`.
3. Insert import artifacts into `memory_events` and optionally low-priority entries.

## 19. MCP / Tooling Integration Requirements

For every new memory tool, update all required layers:

1. Tauri HTTP route + handler.
2. MCP tool definitions in `ralphx-mcp-server/src/tools.ts`.
3. MCP call dispatch in `ralphx-mcp-server/src/index.ts`.
4. Agent allowlists in:
   1. runtime config (`ralphx.yaml`)
   2. MCP `TOOL_ALLOWLIST`
   3. agent frontmatter `allowedTools`.

## 20. Frontend Work

## 20.1 Settings UI

Add `Memory` section in `SettingsView`:

1. Enable Memory toggle.
2. Maintenance category toggles.
3. Capture category toggles.
4. Archive toggles/settings (enabled/path/auto-commit).

## 20.2 API Layer

Add typed wrappers and schemas for:

1. `get_project_memory_settings`
2. `update_project_memory_settings`

## 20.3 Project Scoping

Settings remain per-project and use active project semantics.

## 21. Testing Strategy

## 21.1 Migration Tests

1. Fresh DB creates all new tables/indexes.
2. Upgrade from v21 to v22 succeeds.
3. Defaults inserted for existing projects.

## 21.2 Repository Tests

1. CRUD for memory entries.
2. Search and path filtering.
3. Rule binding upsert/idempotency.
4. Archive job queue transitions.

## 21.3 MCP Tests

1. Read tools accessible by standard agents.
2. Write tools denied for non-memory agents.
3. Memory agents can write.
4. Project scope validation enforced.

## 21.4 Orchestration Tests

1. Post-run trigger maps category correctly.
2. Maintenance/capture spawn in parallel when both enabled.
3. Recursion guard works.

## 21.5 Ingestion Tests

1. New rule file ingested into DB.
2. Rule rewritten into index view.
3. `paths` preserved/normalized.
4. Re-ingest is idempotent.

## 21.6 Archive Tests

1. DB update enqueues archive jobs.
2. Snapshot files are deterministic.
3. Startup recovery replays failed/pending jobs.

## 21.7 End-to-End Scenario

1. User/agent adds detailed `.claude/rules` content.
2. Maintainer ingests -> DB canonical memory.
3. Rule rewritten to index + memory IDs.
4. Archive snapshots generated.
5. Future agent retrieves details via MCP by ID/path.

## 22. Operational / Rollout Notes

1. Feature flag gate for staged rollout in dev builds if needed.
2. Observability metrics:
   1. captures attempted/successful
   2. maintenance runs
   3. ingest conversions
   4. archive job success/failure
3. Logging into `memory_events` plus tracing spans.

## 23. Risks and Mitigations

1. **Risk**: Over-capture noise.
   1. Mitigation: quality gates + dedupe + maintain bucket discipline.
2. **Risk**: Rule rewrite causes unwanted churn.
   1. Mitigation: deterministic serializer + no-op hash check.
3. **Risk**: Archive growth.
   1. Mitigation: retention policy and optional compaction phase (future).
4. **Risk**: Tool permission drift.
   1. Mitigation: enforce three-layer allowlist checks and tests.

## 24. Implementation Work Packages

1. WP1: DB schema + repositories + migration tests.
2. WP2: Memory settings commands + frontend API + settings UI wiring.
3. WP3: Memory MCP tools + routes + handlers + allowlists.
4. WP4: Background trigger orchestration in chat completion flow.
5. WP5: Memory agents definitions/prompts/config.
6. WP6: Rule ingestion + rewrite pipeline.
7. WP7: Archive job service + snapshot writers + recovery job.
8. WP8: Legacy hook cutover removal.
9. WP9: E2E and regression tests.

## 25. Acceptance Criteria (Release Gate)

1. Memory logic no longer depends on stop-hook blocking scripts.
2. Project can disable memory entirely.
3. Category toggles control maintenance/capture execution.
4. Memory agents run on Haiku and in background.
5. Canonical memory stored in SQLite.
6. Rule files remain path-scoped auto-loadable indexes.
7. User-authored detailed rules are ingested and converted.
8. Archive snapshots are generated from DB automatically.
9. Non-memory agents cannot call write-memory MCP tools.
10. Full end-to-end retrieval by memory ID/path works.

## 26. File-Level Impact Map (expected)

### Backend (Rust)

1. `src-tauri/src/infrastructure/sqlite/migrations/mod.rs` (v22 registration)
2. `src-tauri/src/infrastructure/sqlite/migrations/v22_memory_framework.rs`
3. new repositories under `src-tauri/src/domain/repositories/` and `src-tauri/src/infrastructure/sqlite/`
4. `src-tauri/src/application/app_state.rs` (wire repos/services)
5. `src-tauri/src/http_server/mod.rs` + `handlers/` (memory endpoints)
6. `src-tauri/src/application/chat_service/chat_service_send_background.rs` (trigger insertion)
7. `src-tauri/src/lib.rs` (startup recovery job for archive queues)

### MCP Server (TS)

1. `ralphx-plugin/ralphx-mcp-server/src/tools.ts`
2. `ralphx-plugin/ralphx-mcp-server/src/index.ts`
3. `ralphx-plugin/ralphx-mcp-server/src/agentNames.ts` (if new constants needed)

### Plugin / Agents

1. `ralphx.yaml` (memory defaults + memory agent entries)
2. new `ralphx-plugin/agents/memory-maintainer.md`
3. new `ralphx-plugin/agents/memory-capture.md`
4. update affected existing agent frontmatter allowlists for read tools

### Frontend

1. `src/types/settings.ts`
2. `src/components/settings/SettingsView.tsx`
3. `src/api/*` settings wrappers for memory settings
4. `src/App.tsx` settings load/save wiring

### Legacy cleanup

1. `ralphx-plugin/hooks/hooks.json` (remove memory enforcement hooks)
2. `src-tauri/src/infrastructure/agents/claude/agent_config/mod.rs` (remove broad memory-skill preapproval behavior)

## 27. Final Notes for Implementer Agent

1. Preserve existing system behavior where unrelated to memory framework.
2. Keep changes strongly typed and migration-safe.
3. Do not introduce per-agent trigger UI in this phase.
4. Do not add heartbeat scheduler now.
5. Ensure all generated index/archive files are deterministic and idempotent.
6. Treat DB as single source of truth for memory content after ingestion.
