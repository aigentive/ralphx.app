# RalphX Release Automation Plan

## Overview

Implement complete release automation for RalphX macOS application:
- DMG distribution via GitHub Releases
- Code signing & notarization for Gatekeeper
- Auto-update functionality
- GitHub Actions automation

---

## Phase 1: Prerequisites (User Action)

> **Note:** This phase contains user actions only (no code tasks). These must be completed before Phase 2.

### 1.1 Enroll in Apple Developer Program

1. Go to https://developer.apple.com/programs/enroll/
2. Sign in with Apple ID (or create one)
3. Complete enrollment ($99/year)
4. Wait for approval (typically 24-48 hours)

### 1.2 Create Developer ID Certificate

After enrollment:
1. Open Keychain Access on your Mac
2. Keychain Access → Certificate Assistant → Request Certificate from CA
3. Enter email, select "Saved to disk"
4. Go to https://developer.apple.com/account/resources/certificates
5. Click "+" → Select "Developer ID Application"
6. Upload the certificate request
7. Download and double-click to install in Keychain

### 1.3 Create App-Specific Password

For notarization:
1. Go to https://appleid.apple.com/account/manage
2. Sign In & Security → App-Specific Passwords
3. Generate password, save it securely

### 1.4 Export Certificate for CI

```bash
# Export from Keychain as .p12
security find-identity -v -p codesigning
# Note the "Developer ID Application: Your Name (TEAM_ID)"

# Export via Keychain Access → right-click cert → Export
# Save as certificate.p12 with a strong password

# Base64 encode for GitHub secret
base64 -i certificate.p12 | pbcopy
# Paste this into APPLE_CERTIFICATE secret
```

---

## Phase 2: Tauri Configuration

### Task 2.1: Update tauri.conf.json with macOS bundle config (BLOCKING)
**Dependencies:** Phase 1 complete (user action)
**Atomic Commit:** `feat(bundle): add macOS DMG and signing configuration`

**File:** `src-tauri/tauri.conf.json`

Add to `bundle` section:
```json
{
  "bundle": {
    "macOS": {
      "minimumSystemVersion": "13.0",
      "signingIdentity": "-",
      "dmg": {
        "appPosition": { "x": 180, "y": 170 },
        "applicationFolderPosition": { "x": 480, "y": 170 },
        "windowSize": { "width": 660, "height": 400 }
      }
    }
  }
}
```

Note: `signingIdentity: "-"` means use environment variable. In CI, we set `APPLE_SIGNING_IDENTITY`.

### Task 2.2: Create entitlements file
**Dependencies:** None
**Atomic Commit:** `feat(bundle): add hardened runtime entitlements`

**File:** `src-tauri/entitlements.plist` (new)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
    <key>com.apple.security.automation.apple-events</key>
    <true/>
</dict>
</plist>
```

Required for `macOSPrivateApi: true` and hardened runtime.

### Task 2.3: Add release profile to Cargo.toml
**Dependencies:** None
**Atomic Commit:** `feat(bundle): add optimized release profile`

**File:** `src-tauri/Cargo.toml`

Add at end:
```toml
[profile.release]
lto = true
opt-level = "s"
strip = true
codegen-units = 1
```

---

## Phase 3: Auto-Update Configuration

### Task 3.1: Add updater plugin dependency (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(updater): add tauri-plugin-updater dependency`

**File:** `src-tauri/Cargo.toml`

Add to dependencies:
```toml
tauri-plugin-updater = "2"
```

### Task 3.2: Configure updater in tauri.conf.json
**Dependencies:** Task 3.3 (needs pubkey from key generation)
**Atomic Commit:** `feat(updater): configure updater endpoints and pubkey`

Add to `plugins` section:
```json
{
  "plugins": {
    "updater": {
      "pubkey": "GENERATE_THIS",
      "endpoints": [
        "https://github.com/lazabogdan/ralphx/releases/latest/download/latest.json"
      ]
    }
  }
}
```

### Task 3.3: Generate update signing keys (User Action)
**Dependencies:** None
**Note:** User action - no commit

```bash
# Run once, save keys securely
npx @tauri-apps/cli signer generate -w ~/.tauri/ralphx.key

# Output:
# - Private key: ~/.tauri/ralphx.key (keep secret, add to CI)
# - Public key: (paste into tauri.conf.json pubkey field)
```

### Task 3.4: Register updater plugin in Rust
**Dependencies:** Task 3.1
**Atomic Commit:** `feat(updater): register updater plugin`

**File:** `src-tauri/src/lib.rs`

Add to plugin registration:
```rust
.plugin(tauri_plugin_updater::Builder::new().build())
```

### Task 3.5: Add update checker component to frontend
**Dependencies:** Task 3.4
**Atomic Commit:** `feat(updater): add UpdateChecker component`

**File:** `src/components/UpdateChecker.tsx` (new)

Simple component that checks for updates on app start and shows notification if available.

---

## Phase 4: GitHub Actions Workflow

### Task 4.1: Create release workflow (BLOCKING)
**Dependencies:** Phase 2, Phase 3 complete
**Atomic Commit:** `feat(ci): add GitHub Actions release workflow`

**File:** `.github/workflows/release.yml` (new)

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to release (e.g., 0.2.0)'
        required: true

permissions:
  contents: write

jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'

      - name: Setup Rust
        uses: dtolnay/rust-action@stable

      - name: Install dependencies
        run: npm ci

      - name: Import Apple Certificate
        env:
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
        run: |
          echo $APPLE_CERTIFICATE | base64 --decode > certificate.p12
          security create-keychain -p "" build.keychain
          security default-keychain -s build.keychain
          security unlock-keychain -p "" build.keychain
          security import certificate.p12 -k build.keychain -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
          security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "" build.keychain
          rm certificate.p12

      - name: Build Tauri App
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: v__VERSION__
          releaseName: 'RalphX v__VERSION__'
          releaseBody: 'See CHANGELOG.md for details.'
          releaseDraft: true
          prerelease: false
          args: --verbose
```

### Task 4.2: Configure GitHub secrets (User Action)
**Dependencies:** Task 4.1, Phase 1.4 complete
**Note:** User action - no commit

Go to Repository → Settings → Secrets and variables → Actions

| Secret Name | Value |
|-------------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded .p12 file |
| `APPLE_CERTIFICATE_PASSWORD` | Password for .p12 |
| `APPLE_SIGNING_IDENTITY` | "Developer ID Application: Name (TEAMID)" |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | App-specific password |
| `APPLE_TEAM_ID` | Team ID from developer.apple.com |
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of ~/.tauri/ralphx.key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for signing key |

---

## Phase 5: Local Build & Scripts

### Task 5.1: Create local build script
**Dependencies:** Phase 2 complete
**Atomic Commit:** `feat(scripts): add local release build script`

**File:** `scripts/build-release.sh` (new)

```bash
#!/bin/bash
set -e

echo "Building RalphX release..."

# Build frontend
npm run build

# Build Tauri (release mode)
cd src-tauri
cargo tauri build

echo ""
echo "Build complete!"
echo "App: src-tauri/target/release/bundle/macos/RalphX.app"
echo "DMG: src-tauri/target/release/bundle/dmg/RalphX_*.dmg"
echo ""
echo "To test: open src-tauri/target/release/bundle/macos/RalphX.app"
```

### Task 5.2: Create version bump script
**Dependencies:** None
**Atomic Commit:** `feat(scripts): add version bump script`

**File:** `scripts/bump-version.sh` (new)

```bash
#!/bin/bash
set -e

VERSION=$1

if [ -z "$VERSION" ]; then
  echo "Usage: ./scripts/bump-version.sh <version>"
  echo "Example: ./scripts/bump-version.sh 0.2.0"
  exit 1
fi

echo "Bumping version to $VERSION..."

# Update package.json
npm version $VERSION --no-git-tag-version

# Update Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" src-tauri/Cargo.toml

# Update tauri.conf.json
cd src-tauri
cat tauri.conf.json | jq ".version = \"$VERSION\"" > tauri.conf.json.tmp
mv tauri.conf.json.tmp tauri.conf.json
cd ..

echo "Version updated to $VERSION"
echo ""
echo "To release:"
echo "  git add -A && git commit -m 'chore: bump version to $VERSION'"
echo "  git tag v$VERSION"
echo "  git push origin main --tags"
```

---

## Phase 6: Documentation

### Task 6.1: Create release process documentation
**Dependencies:** All phases complete
**Atomic Commit:** `docs: add release process documentation`

**File:** `docs/release-process.md` (new)

Complete documentation covering:
- Prerequisites and setup
- Local build testing
- Creating releases
- Troubleshooting

---

## Files Summary

| File | Action | Purpose |
|------|--------|---------|
| `src-tauri/tauri.conf.json` | Modify | DMG config, updater config |
| `src-tauri/Cargo.toml` | Modify | Release profile, updater plugin |
| `src-tauri/src/lib.rs` | Modify | Register updater plugin |
| `src-tauri/entitlements.plist` | Create | Hardened runtime entitlements |
| `.github/workflows/release.yml` | Create | CI/CD automation |
| `scripts/build-release.sh` | Create | Local build testing |
| `scripts/bump-version.sh` | Create | Version management |
| `src/components/UpdateChecker.tsx` | Create | UI for update notifications |
| `docs/release-process.md` | Create | Full documentation |

---

## Task Dependency Graph

```
Phase 1 (User Actions)
    │
    ├─► Task 2.1 (tauri.conf.json bundle config) ─┐
    │                                              │
    ├─► Task 2.2 (entitlements.plist)              ├─► Task 5.1 (build script)
    │                                              │
    └─► Task 2.3 (Cargo.toml release profile) ─────┘

Task 3.1 (updater dependency) ─► Task 3.4 (register plugin) ─► Task 3.5 (UpdateChecker)
                                        │
Task 3.3 (generate keys) ─► Task 3.2 (updater config) ─────────┘

Phase 2 + Phase 3 ─► Task 4.1 (release workflow) ─► Task 4.2 (GitHub secrets)

All Phases ─► Task 6.1 (documentation)
```

---

## Verification Checklist

After implementation:

- [ ] `./scripts/build-release.sh` produces DMG
- [ ] Local .app runs without Gatekeeper warnings (after signing)
- [ ] GitHub secrets are configured
- [ ] Push tag triggers GitHub Action
- [ ] Action builds, signs, notarizes, and publishes release
- [ ] Auto-update check works in app
- [ ] Downloaded DMG installs and runs on fresh Mac

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Compilation Unit Notes

All tasks in this plan are additive (new files or new config sections) and can compile independently:
- Adding a dependency without using it is valid (Task 3.1)
- New config sections don't break existing functionality
- New files are standalone compilation units
- The only cross-file dependency is Task 3.4 which requires Task 3.1's dependency to be present
