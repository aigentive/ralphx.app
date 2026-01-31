# Web Target and Browser Testing Guide

This document covers how to run RalphX in web mode (without Tauri) for browser automation testing with Playwright.

## Overview

RalphX can run in two modes:

| Mode | Command | Backend | Events | Use Case |
|------|---------|---------|--------|----------|
| **Tauri Mode** | `npm run dev` | Real Rust backend | Tauri IPC events | Normal development |
| **Web Mode** | `npm run dev:web` | Mock API (read-only) | In-memory events | Visual testing, Playwright |

Web mode enables browser automation testing by mocking the Tauri backend, allowing the React frontend to render with mock data in any browser.

---

## Running Web Mode

### Start the Dev Server

```bash
npm run dev:web
```

This starts Vite with `--mode web`, which:
- Uses mock API implementations from `src/api-mock/`
- Aliases Tauri plugins to mock implementations in `src/mocks/`
- Runs on port 5173 (separate from Tauri's port 1420)

### Build for Web

```bash
npm run build:web
```

Outputs to `dist-web/` directory for deployment or static testing.

### Access in Browser

Navigate to `http://localhost:5173` in any browser. The app will:
1. Detect it's not in Tauri (no `window.__TAURI_INTERNALS__`)
2. Use mock API for all backend calls
3. Use in-memory event bus for events
4. Load mock data (projects, tasks, proposals, etc.)

---

## What Works in Web Mode

### Fully Functional

| Feature | Notes |
|---------|-------|
| **UI Rendering** | All components render with mock data |
| **Navigation** | All routes and views accessible |
| **Kanban Board** | Task cards, columns, drag-drop (visual only) |
| **Ideation View** | Sessions, proposals, chat display |
| **Activity View** | Activity events with mock data |
| **Form Submission** | Forms submit without error (no-op) |
| **Theme Switching** | Light/dark mode works |
| **Animations** | All Framer Motion animations work |

### Mock Data Available

The mock API provides factory-generated data for:
- Projects (with default selection)
- Tasks (multiple statuses, mock content)
- Ideation sessions and proposals
- Plan artifacts
- Activity events
- Execution status
- Reviews

### Read-Only Behavior

| Action | Behavior |
|--------|----------|
| Create task | Returns mock success, no persistence |
| Update task | Returns mock success, state may update locally |
| Delete task | Returns mock success, no actual deletion |
| Move task | Visual only, no backend update |
| Send chat message | UI shows sending, mock response |

---

## Limitations

### Not Available in Web Mode

| Feature | Reason |
|---------|--------|
| **Real data persistence** | No SQLite database |
| **Agent execution** | No Claude Code backend |
| **File system access** | Mocked plugin returns empty |
| **Native dialogs** | Mocked to return defaults |
| **Auto-updates** | Mocked updater plugin |
| **Global shortcuts** | Mocked to no-op |

### Known Differences

1. **Data is reset on refresh** - Mock data regenerates each load
2. **No real-time updates** - Mock event bus doesn't receive backend events
3. **Forms appear to work but don't persist** - Good for visual testing, not functional testing

---

## Running Playwright Tests

### Prerequisites

Install Playwright (already in devDependencies):

```bash
npm install -D @playwright/test
npx playwright install
```

### Run All Tests

```bash
npx playwright test
```

This will:
1. Start the web mode dev server automatically
2. Run all tests in `tests/visual/`
3. Generate HTML report

### Run Specific Test

```bash
npx playwright test kanban
```

### Interactive Mode

```bash
npx playwright test --ui
```

Opens the Playwright Test UI for interactive debugging.

### Debug Mode

```bash
npx playwright test --debug
```

Runs tests with step-by-step debugging.

---

## Visual Regression Testing

### How It Works

1. Tests take screenshots of the app in specific states
2. Screenshots are compared against baseline images in `tests/visual/snapshots/`
3. Differences above threshold (1% pixel diff) fail the test

### Update Baseline Screenshots

When UI intentionally changes:

```bash
npx playwright test --update-snapshots
```

This regenerates all snapshot files. Review the changes before committing.

### Snapshot Location

Snapshots are stored in:
```
tests/visual/snapshots/
└── kanban-board.png          # Kanban board snapshot
└── [test-name]-[browser].png # Other snapshots
```

### Threshold Configuration

Configured in `playwright.config.ts`:

```typescript
expect: {
  toHaveScreenshot: {
    maxDiffPixelRatio: 0.01, // 1% tolerance
  },
},
```

---

## Writing New Tests

### Test File Location

Create tests in `tests/visual/`:

```
tests/visual/
├── kanban.spec.ts     # Kanban board tests
├── ideation.spec.ts   # Ideation view tests (add as needed)
└── snapshots/         # Auto-generated snapshot directory
```

### Basic Test Structure

```typescript
import { test, expect } from "@playwright/test";

test.describe("Feature Name", () => {
  test("renders correctly", async ({ page }) => {
    await page.goto("/");

    // Wait for app to load
    await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });

    // Your assertions
    const element = page.locator('[data-testid="my-element"]');
    await expect(element).toBeVisible();
  });

  test("matches snapshot", async ({ page }) => {
    await page.goto("/path");
    await page.waitForSelector('[data-testid="content"]', { timeout: 10000 });

    // Wait for animations
    await page.waitForTimeout(500);

    await expect(page).toHaveScreenshot("my-feature.png");
  });
});
```

### Testing Tips

1. **Use data-testid attributes** - Add `data-testid="..."` to components for reliable selection
2. **Wait for hydration** - The app header is a good indicator: `[data-testid="app-header"]`
3. **Allow animation time** - Use `waitForTimeout(500)` before screenshots
4. **Test specific elements** - Use `.locator()` for targeted assertions

---

## Architecture Overview

### Environment Detection

`src/lib/tauri-detection.ts`:

```typescript
export function isWebMode(): boolean {
  return typeof window !== "undefined" && !window.__TAURI_INTERNALS__;
}
```

### API Switching

The API layer automatically switches based on environment:

- **Tauri mode**: Real API calls via `invoke()`
- **Web mode**: Mock API from `src/api-mock/`

### Event Bus

`src/lib/event-bus.ts` provides:

- **TauriEventBus**: Real Tauri `listen()`/`emit()` in native mode
- **MockEventBus**: In-memory event emitter in web mode

The `EventProvider` component in `src/providers/EventProvider.tsx` automatically selects the appropriate bus.

### Plugin Mocks

Vite aliases Tauri plugins to mocks in `src/mocks/` when in web mode:

| Plugin | Mock Location |
|--------|---------------|
| `@tauri-apps/plugin-dialog` | `src/mocks/tauri-plugin-dialog.ts` |
| `@tauri-apps/plugin-fs` | `src/mocks/tauri-plugin-fs.ts` |
| `@tauri-apps/plugin-process` | `src/mocks/tauri-plugin-process.ts` |
| `@tauri-apps/plugin-updater` | `src/mocks/tauri-plugin-updater.ts` |
| `@tauri-apps/plugin-global-shortcut` | `src/mocks/tauri-plugin-global-shortcut.ts` |

---

## Troubleshooting

### Dev Server Won't Start

**Port already in use:**
```bash
# Kill process on port 5173
lsof -ti:5173 | xargs kill -9

# Retry
npm run dev:web
```

### Tests Fail to Start

**Playwright browsers not installed:**
```bash
npx playwright install
```

**Web server timeout:**
- Increase timeout in `playwright.config.ts` (default: 120s)
- Check if `npm run dev:web` works manually

### Screenshots Don't Match

**Font rendering differences:**
- Different OS/browser versions render fonts slightly differently
- Use `maxDiffPixelRatio` to allow small differences

**Animation timing:**
- Add `await page.waitForTimeout(500)` before screenshot
- Ensure CSS animations have completed

**Dynamic content:**
- Mock data is deterministic but timestamps change
- Use `seededRandom` patterns for consistent mock data

### Mock Data Issues

**Missing mock implementation:**
- Check `src/api-mock/` for the API function
- Add mock implementation if missing

**Type errors with mocks:**
- Ensure mock return types match real API types
- Check `src/types/` for expected interfaces

### Console Errors in Web Mode

**"Failed to invoke command":**
- Expected when Tauri APIs are called directly without going through the mock layer
- Check if the code path is using the abstracted API

**"__TAURI_INTERNALS__ is undefined":**
- Expected in web mode, indicates detection is working
- Should not cause functional issues

---

## File Reference

| File | Purpose |
|------|---------|
| `playwright.config.ts` | Playwright configuration |
| `tests/visual/` | Visual regression test files |
| `tests/visual/snapshots/` | Baseline screenshot images |
| `src/api-mock/` | Mock API implementations |
| `src/mocks/` | Tauri plugin mocks |
| `src/lib/tauri-detection.ts` | Environment detection |
| `src/lib/event-bus.ts` | Event bus abstraction |
| `src/providers/EventProvider.tsx` | Event provider component |
| `vite.config.ts` | Vite config with web mode handling |
