# RalphX Release Process

This document covers the complete release workflow for RalphX, including prerequisites, local testing, and automated releases via GitHub Actions.

## Prerequisites

### Required Accounts & Certificates

| Requirement | Purpose | Where to Get |
|-------------|---------|--------------|
| Apple Developer Program | Code signing | [developer.apple.com/programs/enroll](https://developer.apple.com/programs/enroll) ($99/year) |
| Developer ID Application certificate | Gatekeeper-approved distribution | Keychain Access + developer.apple.com |
| App-specific password | Notarization authentication | [appleid.apple.com](https://appleid.apple.com/account/manage) |
| Tauri signing keys | Update signature verification | `npx @tauri-apps/cli signer generate` |

### One-Time Setup

#### 1. Create Developer ID Certificate

1. Open **Keychain Access** on your Mac
2. Go to Keychain Access → Certificate Assistant → **Request Certificate from CA**
3. Enter your email, select "Saved to disk"
4. Go to [developer.apple.com/account/resources/certificates](https://developer.apple.com/account/resources/certificates)
5. Click "+" → Select **Developer ID Application**
6. Upload the certificate request
7. Download and double-click to install in Keychain

#### 2. Generate App-Specific Password

1. Go to [appleid.apple.com/account/manage](https://appleid.apple.com/account/manage)
2. Navigate to Sign In & Security → App-Specific Passwords
3. Generate a new password
4. Save it securely (you'll need it for GitHub secrets)

#### 3. Export Certificate for CI

```bash
# Find your signing identity
security find-identity -v -p codesigning
# Note: "Developer ID Application: Your Name (TEAM_ID)"

# Export via Keychain Access:
# - Right-click the certificate → Export
# - Save as certificate.p12 with a strong password

# Base64 encode for GitHub secret
base64 -i certificate.p12 | pbcopy
# Paste this into APPLE_CERTIFICATE secret
```

#### 4. Generate Tauri Signing Keys

```bash
# Generate keys for update signature verification
npx @tauri-apps/cli signer generate -w ~/.tauri/ralphx.key

# Output:
# - Private key: ~/.tauri/ralphx.key (add to GitHub secrets)
# - Public key: (displayed in terminal - update tauri.conf.json)
```

Update `src-tauri/tauri.conf.json` with the public key:
```json
{
  "plugins": {
    "updater": {
      "pubkey": "YOUR_PUBLIC_KEY_HERE"
    }
  }
}
```

#### 5. Configure GitHub Secrets

Go to Repository → Settings → Secrets and variables → Actions

Add these secrets:

| Secret Name | Value |
|-------------|-------|
| `APPLE_CERTIFICATE` | Base64-encoded .p12 file (from step 3) |
| `APPLE_CERTIFICATE_PASSWORD` | Password for .p12 |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Your Name (TEAM_ID)` |
| `APPLE_ID` | Your Apple ID email |
| `APPLE_PASSWORD` | App-specific password (from step 2) |
| `APPLE_TEAM_ID` | Team ID from developer.apple.com |
| `TAURI_SIGNING_PRIVATE_KEY` | Contents of `~/.tauri/ralphx.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password for signing key |

---

## Local Build Testing

### Build Without Signing (Development)

```bash
# Quick build for local testing (no signing)
npm run tauri build
```

Output:
- App: `src-tauri/target/release/bundle/macos/RalphX.app`
- DMG: `src-tauri/target/release/bundle/dmg/RalphX_*.dmg`

### Build With Signing (Release)

Use the provided script:

```bash
./scripts/build-release.sh
```

This builds the frontend and Tauri app in release mode. For signed builds, ensure your environment has:

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
- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

### Step 2: Commit and Tag

```bash
git add -A
git commit -m "chore: bump version to 0.2.0"
git tag v0.2.0
git push origin main --tags
```

### Step 3: GitHub Actions

Pushing the tag triggers the release workflow automatically:

1. **Build**: Compiles frontend and Tauri app
2. **Sign**: Applies Developer ID certificate
3. **Notarize**: Submits to Apple for notarization
4. **Package**: Creates DMG with update manifest
5. **Release**: Creates draft GitHub release with artifacts

### Step 4: Publish Release

1. Go to GitHub → Releases
2. Find the draft release created by the workflow
3. Review the artifacts:
   - `RalphX_x.x.x_aarch64.dmg` - Apple Silicon
   - `RalphX_x.x.x_x64.dmg` - Intel (if configured)
   - `latest.json` - Update manifest
4. Edit release notes as needed
5. Click **Publish release**

---

## Manual Workflow Dispatch

For releases without a version tag:

1. Go to GitHub → Actions → Release workflow
2. Click **Run workflow**
3. Enter the version number (e.g., `0.2.0`)
4. Click **Run workflow**

---

## Auto-Update Flow

Once published:

1. Existing RalphX installations check `latest.json` on startup
2. If a newer version exists, a toast notification appears
3. User clicks "Update Now" to download
4. Progress is displayed during download
5. App relaunches with the new version

The update endpoint is configured in `src-tauri/tauri.conf.json`:
```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/lazabogdan/ralphx/releases/latest/download/latest.json"
      ]
    }
  }
}
```

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
- Verify `APPLE_ID` and `APPLE_PASSWORD` (app-specific password) are correct
- Ensure `APPLE_TEAM_ID` matches your developer account
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

**"Certificate import failed"**
- Re-export the certificate and base64 encode it
- Verify the password matches `APPLE_CERTIFICATE_PASSWORD`

**Workflow doesn't trigger**
- Ensure tag follows pattern `v*` (e.g., `v0.2.0`)
- Check Actions tab for workflow run status

### Update Issues

**"Update check failed"**
- Verify `latest.json` is accessible at the endpoint URL
- Check network connectivity
- Ensure the pubkey in `tauri.conf.json` matches the private key used to sign

**"Signature verification failed"**
- Regenerate signing keys and update both:
  - `TAURI_SIGNING_PRIVATE_KEY` secret
  - `pubkey` in `tauri.conf.json`

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
| `.github/workflows/release.yml` | CI/CD workflow for automated releases |
| `scripts/build-release.sh` | Local release build script |
| `scripts/bump-version.sh` | Version management script |
| `src-tauri/tauri.conf.json` | Bundle config, updater config |
| `src-tauri/Cargo.toml` | Release profile, updater dependency |
| `src-tauri/entitlements.plist` | Hardened runtime entitlements |
| `src/components/UpdateChecker.tsx` | Update notification UI |
