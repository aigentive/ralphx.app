# Development Guide

## Starting the Dev Server

```bash
# Full Tauri dev (frontend + Rust backend, hot reload)
npm run tauri dev

# Frontend-only web mode (mock Tauri backend)
npm run dev:web
```

| Mode | Command | Frontend Port | Backend | Use Case |
|------|---------|---------------|---------|----------|
| Native | `npm run tauri dev` | 1420 | Real Rust | Full-stack development |
| Web | `npm run dev:web` | 5173 | Mocked (`src/api-mock/`) | UI-only work |

The MCP HTTP server starts automatically on `http://127.0.0.1:3847` when the Tauri app initializes.

---

## Frontend Logging

### Logger Utility (`src/lib/logger.ts`)

A thin wrapper that gates debug output behind Vite's dev mode flag:

```typescript
import { logger } from "@/lib/logger";

logger.debug("operation details", data);  // dev only, prefixed with [debug]
logger.log("info message");               // dev only
logger.warn("non-fatal issue");           // always shown (dev + production)
logger.error("failure", err);             // always shown (dev + production)
```

| Method | Dev Mode | Production | Prefix |
|--------|----------|------------|--------|
| `logger.debug()` | console.debug | silent | `[debug]` |
| `logger.log()` | console.log | silent | none |
| `logger.warn()` | console.warn | console.warn | none |
| `logger.error()` | console.error | console.error | none |

### Browser DevTools Filtering

Open DevTools (`Cmd+Option+I` on macOS) and use the Console tab's log level filter:

- **Verbose** — shows `debug` + `log` + `warn` + `error`
- **Info** — shows `log` + `warn` + `error`
- **Warnings** — shows `warn` + `error`
- **Errors** — shows `error` only

Filter by prefix: type `[debug]` in the Console filter box to isolate logger.debug output.

---

## Backend Logging

### Tracing Setup (`src-tauri/src/lib.rs`)

The backend uses the `tracing` crate with `tracing-subscriber`. Initialization:

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("ralphx=info,warn")),
    )
    .init();
```

**Default:** `ralphx=info,warn` — RalphX modules at `info` level, everything else at `warn`.

### Controlling Log Verbosity with `RUST_LOG`

Set the `RUST_LOG` environment variable before starting the dev server:

```bash
RUST_LOG=ralphx=debug npm run tauri dev
```

### `RUST_LOG` Quick Reference

| Pattern | Effect |
|---------|--------|
| `ralphx=debug` | All RalphX modules at debug level |
| `ralphx=trace` | Maximum verbosity for RalphX |
| `warn` | Only warnings and errors (quiet) |
| `ralphx=info,warn` | **Default** — RalphX info, others warn |
| `ralphx::application::chat_service=debug` | Single module at debug |
| `ralphx=info,ralphx::application::git_service=debug` | Base info + one module debug |
| `ralphx=debug,ralphx::http_server=warn` | Debug everywhere except noisy HTTP |
| `debug` | Everything at debug (very verbose, includes deps) |

### Log Level Hierarchy

From most to least verbose:

| Level | Usage |
|-------|-------|
| `trace` | Line-by-line detail (rarely enabled) |
| `debug` | Operation details, intermediate values |
| `info` | Important events, status changes, startup |
| `warn` | Non-fatal issues, degraded operation |
| `error` | Failures requiring attention |

### Backend Log Output Format

Logs print to the terminal where `npm run tauri dev` is running:

```
2026-02-09T10:30:45Z  INFO ralphx: MCP HTTP server listening on http://127.0.0.1:3847
2026-02-09T10:30:46Z DEBUG ralphx::application::chat_service: stream processing started
```

---

## Common Debugging Workflows

### Chat Service / Streaming Issues

The chat service processes agent streams and is the most common source of debugging work.

```bash
RUST_LOG=ralphx::application::chat_service=debug npm run tauri dev
```

On parse failures, the streaming service writes debug output to a temp file:
```
/tmp/ralphx-stream-debug-{conversation_id}.log
```

Check these files if agent streams produce unexpected results.

### State Machine Transitions

To trace task status transitions through the state machine:

```bash
RUST_LOG=ralphx::domain::state_machine=debug npm run tauri dev
```

This shows transition dispatching, side effects, and auto-transition chains.

### Git Operations (Branch/Merge/Worktree)

To debug branch creation, merges, rebases, and worktree management:

```bash
RUST_LOG=ralphx::application::git_service=debug npm run tauri dev
```

### Startup Issues

If the app fails to start or initialize properly:

```bash
RUST_LOG=ralphx=debug npm run tauri dev
```

Check for:
- Database migration failures (logged at `error`)
- HTTP server bind failures on port 3847 (retries 5 times)
- Reconciliation runner output (logged at `info`/`warn`)

### MCP HTTP Server

To debug MCP tool calls from Claude agents:

```bash
RUST_LOG=ralphx::http_server=debug npm run tauri dev
```

This shows incoming requests to `127.0.0.1:3847` and handler responses.

### Combining Filters

Multiple modules can be combined with commas:

```bash
RUST_LOG="ralphx::application::chat_service=debug,ralphx::domain::state_machine=debug,ralphx::http_server=warn" npm run tauri dev
```

### Key Backend Modules

| Module Path | Covers |
|-------------|--------|
| `ralphx::application::chat_service` | Agent streaming, conversation management |
| `ralphx::application::git_service` | Branch, merge, worktree operations |
| `ralphx::application::task_scheduler_service` | Task scheduling, local-mode enforcement |
| `ralphx::application::task_transition_service` | Status transition orchestration |
| `ralphx::domain::state_machine` | State machine dispatch, side effects |
| `ralphx::http_server` | MCP proxy handlers on port 3847 |
| `ralphx::commands` | Tauri command layer |
| `ralphx::infrastructure` | Database, agents, file system |
