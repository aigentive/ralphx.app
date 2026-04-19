> **Maintainer note:** Keep this file compact. Prefer one-line rules, links to source docs, and explicit non-negotiables over prose.

# Runtime Root Vs Target Project

## Purpose

Prevent regressions where agents or backend code assume the active user project checkout contains RalphX's own canonical agent/runtime assets.

## Non-Negotiables

| Rule | Detail |
|---|---|
| Target project != RalphX runtime root | The active `working_directory` can be any repo and usually does not contain RalphX `agents/`, `plugins/app`, or runtime config. |
| Canonical agent metadata is RalphX-owned | Load `agents/*/agent.yaml`, prompt metadata, and delegation policy from the RalphX source/runtime root, not from the target project checkout. |
| Generated plugin dirs are not source roots | Generated Claude plugin dirs may be repo-local in debug, temp in tests/CI, or app-support paths in the desktop app; resolve them back to the RalphX root before loading canonical agents. |
| Symlink-aware root resolution is required | Generated/runtime plugin bundles may point back to the RalphX root via symlinked runtime entries; root resolution must follow those links instead of trusting the bundle path itself. |
| Missing `agents/` in the target project is normal | Do not treat absence of RalphX canonical files in the user project as misconfiguration. |
| Root-resolution changes need environment coverage | Tests must cover repo-local generated dirs, external generated dirs, and symlinked plugin-root layouts. |

## First Places To Check

| Concern | Files |
|---|---|
| Canonical agent lookup | `src-tauri/src/infrastructure/agents/harness_agent_catalog.rs` |
| Generated plugin materialization | `src-tauri/src/infrastructure/agents/claude/generated_plugin.rs` |
| Delegation policy enforcement | `src-tauri/src/http_server/handlers/coordination/mod.rs` |
| Regression coverage | `src-tauri/src/infrastructure/agents/harness_agent_catalog_tests.rs`, `src-tauri/tests/delegation_handlers.rs` |
