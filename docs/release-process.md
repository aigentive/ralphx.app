# RalphX Release Process

This document covers the RalphX release workflow, from local build testing through public GitHub Releases and in-app updater publication.

---

## Local Build Testing

### Build Without Signing (Development)

```bash
# Quick build for local testing (no signing)
cd frontend && npm run tauri build
```

Output:
- App: `src-tauri/target/release/bundle/macos/RalphX.app`
- DMG: `src-tauri/target/release/bundle/dmg/RalphX_*.dmg`

### Local Release-Like Build

Use the local helper when you want a release-mode build that still syncs local app data for internal testing:

```bash
./scripts/build-local-release.sh
```

This helper may seed the app-data DB from the dev DB and refresh plugin runtime into Application Support.

### Production Release Build

Use the production entrypoint for distributable artifacts and CI/release automation:

```bash
./scripts/build-prod-release.sh
```

This path does not mutate local Application Support state. For signed builds, ensure your environment has:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
```

### Testing the Build

```bash
# Open the built app
open src-tauri/target/release/bundle/macos/RalphX.app

# Or mount and test the DMG
open src-tauri/target/release/bundle/dmg/RalphX_*.dmg
```

For signed builds, verify there are no Gatekeeper warnings when opening the app.

---

## Creating a Release

### Step 1: Bump Version

```bash
./scripts/bump-version.sh 0.2.0
```

This updates version in:
- `frontend/package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

### Step 2: Commit Release Prep

```bash
git add -A
git commit -m "chore: bump version to 0.2.0"
```

### Step 3: Draft And Review Release Notes

Run this after the release code is finalized and local regression is green, but before you push the release tag if you want the reviewed notes committed into `release-notes/vX.Y.Z.md` and picked up automatically by the release workflow.

```bash
./scripts/generate-release-notes.sh 0.2.0
```

Then:

1. Review and edit the draft from:
   - `release-notes/v0.2.0.md`
2. If draft generation fails or you want to inspect the Codex run, check the logs in:
   - `.artifacts/release-notes/logs/`
3. Commit that curated notes file before tagging if you want the workflow-created draft GitHub release to use it automatically:
   - `git add release-notes/v0.2.0.md`
   - `git commit -m "docs: add release notes for v0.2.0"`
4. If you decide not to keep the draft in git, leave it uncommitted or remove it locally:
   - `rm -f release-notes/v0.2.0.md`

### Step 4: Create And Push The Release Tag

```bash
git tag v0.2.0
git push origin main --tags
```

### Step 5: GitHub Actions

Pushing the tag triggers the release workflow automatically:

1. **Build**: Compiles frontend and Tauri app
2. **Sign**: Applies Developer ID certificate
3. **Notarize**: Submits to Apple for notarization
4. **Package**: Creates per-architecture DMGs, signed updater bundles, `latest.json`, and checksums
5. **Release**: Publishes the assets to the public binaries repo `aigentive/ralphx-releases`

### Step 6: Publish Release

1. Go to `aigentive/ralphx-releases` → Releases
2. Find the release created by the workflow
3. Review the artifacts:
   - `RalphX_x.x.x_aarch64.dmg` - Apple Silicon
   - `RalphX_x.x.x_x86_64.dmg` - Intel
   - `RalphX_x.x.x_aarch64.app.tar.gz` - Apple Silicon updater bundle
   - `RalphX_x.x.x_aarch64.app.tar.gz.sig` - Apple Silicon updater signature
   - `RalphX_x.x.x_x86_64.app.tar.gz` - Intel updater bundle
   - `RalphX_x.x.x_x86_64.app.tar.gz.sig` - Intel updater signature
   - `latest.json`
   - `checksums.txt`
4. Edit release notes as needed
5. If you dispatched the workflow with `draft=true`, click **Publish release**

## Manual Workflow Dispatch

For releases without a version tag:

1. Go to `aigentive/ralphx` → Actions → Release workflow
2. Click **Run workflow**
3. Enter the version number (e.g., `0.2.0`)
4. Choose whether the public release should stay a draft
5. Click **Run workflow**

---

## In-App Updates

The release workflow now publishes Tauri updater artifacts to the public binaries repo.

Current release contract:
- updater endpoint: `https://github.com/aigentive/ralphx-releases/releases/latest/download/latest.json`
- published releases include per-architecture `.app.tar.gz` updater bundles and `.sig` files
- `latest.json` points the app at those public updater bundles
- the updater follows GitHub's `latest` endpoint, so only the latest published non-draft release is visible automatically

---

## Troubleshooting

### Build Failures

**"No signing identity found"**
```bash
# Verify certificate is installed
security find-identity -v -p codesigning

# Should show:
# "Developer ID Application: Your Name (TEAM_ID)"
```

**"Unable to notarize"**
- Verify `APPLE_API_ISSUER`, `APPLE_API_KEY`, and `APPLE_API_KEY_P8`
- Ensure `APPLE_TEAM_ID` matches the signing team
- Check Apple's notarization service status at [developer.apple.com/system-status](https://developer.apple.com/system-status/)

**Cargo build errors**
```bash
# Clean and rebuild
cd src-tauri
cargo clean
cargo tauri build
```

### GitHub Actions Issues

**"Secret not found"**
- Verify all secrets are configured in repository settings
- Secret names are case-sensitive
- The public release job also requires `RELEASES_REPO_TOKEN`

**"Certificate import failed"**
- Re-export the certificate and base64 encode it
- Verify the password matches `APPLE_CERTIFICATE_PASSWORD`

**Workflow doesn't trigger**
- Ensure tag follows pattern `v*` (e.g., `v0.2.0`)
- Check Actions tab for workflow run status

**Public release upload failed**
- Verify `RELEASES_REPO_TOKEN` has `Contents: Read and write` on `aigentive/ralphx-releases`
- Confirm the target repo exists and the token owner has write access to it

**Updater assets missing**
- Confirm `src-tauri/tauri.conf.json` still has `"bundle.createUpdaterArtifacts": true`
- Confirm the build produced `.app.tar.gz` and `.app.tar.gz.sig` files under `src-tauri/target/release/bundle/macos/`

### Gatekeeper Issues

**"App is damaged and can't be opened"**
- App wasn't properly signed or notarized
- Check the release workflow logs for signing/notarization errors
- For local testing, temporarily allow: `xattr -cr /path/to/RalphX.app`

**"Developer cannot be verified"**
- Notarization may not have completed
- Check [developer.apple.com/system-status](https://developer.apple.com/system-status/)
- Wait a few minutes and try again

---

## File Reference

| File | Purpose |
|------|---------|
| `.github/workflows/release.yml` | CI/CD workflow that publishes public release assets and updater metadata |
| `scripts/build-local-release.sh` | Local internal release-like build script |
| `scripts/build-prod-release.sh` | Production release artifact entrypoint |
| `scripts/bump-version.sh` | Version management script |
| `scripts/generate-release-notes.sh` | Codex-assisted release notes draft generator |
| `release-notes/` | Curated release notes consumed automatically by the release workflow when present |
| `src-tauri/tauri.conf.json` | Bundle config, updater config |
| `src-tauri/Cargo.toml` | Release profile, updater dependency |
| `src-tauri/entitlements.plist` | Hardened runtime entitlements |
| `src/components/UpdateChecker.tsx` | Update notification UI |
