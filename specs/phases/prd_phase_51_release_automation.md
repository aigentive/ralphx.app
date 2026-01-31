# RalphX - Phase 51: Release Automation

## Overview

Implement complete release automation for RalphX macOS application including DMG distribution via GitHub Releases, code signing and notarization for Gatekeeper, auto-update functionality, and GitHub Actions CI/CD automation.

This phase establishes the foundation for distributing RalphX to users with a professional, signed macOS application that can self-update.

**Reference Plan:**
- `specs/plans/ralphx_release_automation_plan.md` - Detailed implementation plan with code snippets and configuration

## Goals

1. Configure Tauri for macOS bundle with DMG output and code signing
2. Add auto-update functionality using tauri-plugin-updater
3. Create GitHub Actions workflow for automated releases
4. Provide local build scripts and documentation

## Dependencies

### Phase 50 (Confirmation Dialogs) - Required

| Dependency | Why Needed |
|------------|------------|
| Stable app | Release automation requires a stable, working application |

### External Prerequisites (User Action Required)

| Prerequisite | Why Needed |
|--------------|------------|
| Apple Developer Program enrollment | Required for code signing certificates |
| Developer ID Application certificate | Required for Gatekeeper-approved distribution |
| App-specific password | Required for notarization |
| Tauri signing keys | Required for update signature verification |

See `specs/plans/ralphx_release_automation_plan.md` Phase 1 for detailed setup instructions.

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/ralphx_release_automation_plan.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/ralphx_release_automation_plan.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Update tauri.conf.json with macOS bundle configuration",
    "plan_section": "Task 2.1: Update tauri.conf.json with macOS bundle config",
    "blocking": [8],
    "blockedBy": [],
    "atomic_commit": "feat(bundle): add macOS DMG and signing configuration",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 2.1'",
      "Add macOS section to bundle config in src-tauri/tauri.conf.json",
      "Configure minimumSystemVersion, signingIdentity, and DMG layout",
      "Verify JSON is valid",
      "Commit: feat(bundle): add macOS DMG and signing configuration"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Create entitlements.plist for hardened runtime",
    "plan_section": "Task 2.2: Create entitlements file",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(bundle): add hardened runtime entitlements",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 2.2'",
      "Create src-tauri/entitlements.plist with required entitlements",
      "Include JIT, unsigned executable memory, library validation, and Apple Events",
      "Commit: feat(bundle): add hardened runtime entitlements"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Add optimized release profile to Cargo.toml",
    "plan_section": "Task 2.3: Add release profile to Cargo.toml",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(bundle): add optimized release profile",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 2.3'",
      "Add [profile.release] section to src-tauri/Cargo.toml",
      "Configure lto, opt-level, strip, and codegen-units",
      "Run cargo check to verify",
      "Commit: feat(bundle): add optimized release profile"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Add tauri-plugin-updater dependency",
    "plan_section": "Task 3.1: Add updater plugin dependency",
    "blocking": [6],
    "blockedBy": [],
    "atomic_commit": "feat(updater): add tauri-plugin-updater dependency",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 3.1'",
      "Add tauri-plugin-updater = \"2\" to dependencies in src-tauri/Cargo.toml",
      "Run cargo check to verify dependency resolves",
      "Commit: feat(updater): add tauri-plugin-updater dependency"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Configure updater plugin in tauri.conf.json",
    "plan_section": "Task 3.2: Configure updater in tauri.conf.json",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "feat(updater): configure updater endpoints and pubkey",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 3.2'",
      "Add plugins.updater section to src-tauri/tauri.conf.json",
      "Configure pubkey placeholder and GitHub releases endpoint",
      "Add comment noting pubkey needs to be generated",
      "Verify JSON is valid",
      "Commit: feat(updater): configure updater endpoints and pubkey"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Register updater plugin in Rust",
    "plan_section": "Task 3.4: Register updater plugin in Rust",
    "blocking": [7],
    "blockedBy": [4],
    "atomic_commit": "feat(updater): register updater plugin",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 3.4'",
      "Add .plugin(tauri_plugin_updater::Builder::new().build()) to src-tauri/src/lib.rs",
      "Add after existing plugin registrations",
      "Run cargo clippy --all-targets --all-features -- -D warnings",
      "Commit: feat(updater): register updater plugin"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Add UpdateChecker component for auto-update UI",
    "plan_section": "Task 3.5: Add update checker component to frontend",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "feat(updater): add UpdateChecker component",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 3.5'",
      "Create src/components/UpdateChecker.tsx",
      "Implement component that checks for updates on mount",
      "Show toast notification when update is available",
      "Add to App.tsx or appropriate root component",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(updater): add UpdateChecker component"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "backend",
    "description": "Create GitHub Actions release workflow",
    "plan_section": "Task 4.1: Create release workflow",
    "blocking": [],
    "blockedBy": [1, 2, 3, 5, 6],
    "atomic_commit": "feat(ci): add GitHub Actions release workflow",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Task 4.1'",
      "Create .github/workflows directory if needed",
      "Create .github/workflows/release.yml with full workflow",
      "Include certificate import, Tauri build, and release creation",
      "Reference all required secrets",
      "Commit: feat(ci): add GitHub Actions release workflow"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "backend",
    "description": "Create local build and version bump scripts",
    "plan_section": "Phase 5: Local Build & Scripts",
    "blocking": [],
    "blockedBy": [1, 2, 3],
    "atomic_commit": "feat(scripts): add release build and version bump scripts",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md sections 'Task 5.1' and 'Task 5.2'",
      "Create scripts directory if needed",
      "Create scripts/build-release.sh for local DMG builds",
      "Create scripts/bump-version.sh for version management",
      "Make scripts executable with chmod +x",
      "Commit: feat(scripts): add release build and version bump scripts"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "documentation",
    "description": "Create release process documentation",
    "plan_section": "Task 6.1: Create release process documentation",
    "blocking": [],
    "blockedBy": [8, 9],
    "atomic_commit": "docs: add release process documentation",
    "steps": [
      "Read specs/plans/ralphx_release_automation_plan.md section 'Phase 6'",
      "Create docs/release-process.md",
      "Document prerequisites (Apple Developer, certificates, keys)",
      "Document local build testing process",
      "Document release creation workflow",
      "Include troubleshooting section",
      "Commit: docs: add release process documentation"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Environment-based signing identity** | Using `-` in tauri.conf.json allows CI to set `APPLE_SIGNING_IDENTITY` while local builds can skip signing |
| **GitHub Releases for updates** | Simple, free hosting for update manifests without additional infrastructure |
| **Separate signing key from Apple cert** | Tauri's update verification is separate from Apple's notarization - both are needed |
| **Draft releases by default** | Allows review before publishing, preventing accidental releases |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] All existing tests pass
- [ ] No clippy warnings

### Frontend - Run `npm run test`
- [ ] UpdateChecker component renders
- [ ] No TypeScript errors

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] `./scripts/build-release.sh` produces DMG in target/release/bundle/dmg/
- [ ] .app bundle runs without Gatekeeper warnings (after signing)
- [ ] Update checker shows in app (can mock update response)

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] UpdateChecker is imported AND rendered in App or root component
- [ ] Updater plugin is registered in lib.rs Builder chain
- [ ] GitHub workflow triggers on tag push

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.

---

## User Action Reminders

The following require manual user action and are NOT automated tasks:

1. **Apple Developer Program** - Enroll at developer.apple.com ($99/year)
2. **Developer ID Certificate** - Create via Keychain Access → developer.apple.com
3. **App-Specific Password** - Generate at appleid.apple.com
4. **Export Certificate** - Export .p12 and base64 encode for GitHub
5. **Generate Signing Keys** - Run `npx @tauri-apps/cli signer generate`
6. **Configure GitHub Secrets** - Add all secrets to repository settings

See `specs/plans/ralphx_release_automation_plan.md` Phase 1 for detailed instructions.
