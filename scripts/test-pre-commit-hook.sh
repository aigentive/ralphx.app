#!/usr/bin/env bash
# Tests for .githooks/pre-commit
# Verifies the hook rejects node_modules paths, rejects staged Tauri version drift,
# and allows normal files.

set -uo pipefail

HOOK_DIR="$(cd "$(dirname "$0")/.." && pwd)/.githooks"
PASS=0
FAIL=0

# ─── helpers ────────────────────────────────────────────────────────────────

setup_repo() {
  local dir
  dir=$(mktemp -d)
  git -C "$dir" init -q
  git -C "$dir" config user.email "test@test.com"
  git -C "$dir" config user.name "Test"
  git -C "$dir" config core.hooksPath "$HOOK_DIR"
  echo "$dir"
}

pass() { echo "  PASS: $1"; ((PASS++)); }
fail() { echo "  FAIL: $1"; ((FAIL++)); }

write_aligned_tauri_files() {
  local dir="$1"

  mkdir -p "$dir/frontend" "$dir/src-tauri"

  cat > "$dir/frontend/package.json" <<'EOF'
{
  "dependencies": {
    "@tauri-apps/api": "^2.10.1",
    "@tauri-apps/plugin-dialog": "^2.7.0",
    "@tauri-apps/plugin-fs": "^2.5.0",
    "@tauri-apps/plugin-global-shortcut": "^2.3.1",
    "@tauri-apps/plugin-opener": "^2.5.3",
    "@tauri-apps/plugin-process": "^2.3.1",
    "@tauri-apps/plugin-updater": "^2.10.1"
  }
}
EOF

  cat > "$dir/frontend/package-lock.json" <<'EOF'
{
  "name": "test",
  "packages": {
    "": {
      "dependencies": {
        "@tauri-apps/api": "^2.10.1",
        "@tauri-apps/plugin-dialog": "^2.7.0",
        "@tauri-apps/plugin-fs": "^2.5.0",
        "@tauri-apps/plugin-global-shortcut": "^2.3.1",
        "@tauri-apps/plugin-opener": "^2.5.3",
        "@tauri-apps/plugin-process": "^2.3.1",
        "@tauri-apps/plugin-updater": "^2.10.1"
      }
    },
    "node_modules/@tauri-apps/api": { "version": "2.10.1" },
    "node_modules/@tauri-apps/plugin-dialog": { "version": "2.7.0" },
    "node_modules/@tauri-apps/plugin-fs": { "version": "2.5.0" },
    "node_modules/@tauri-apps/plugin-global-shortcut": { "version": "2.3.1" },
    "node_modules/@tauri-apps/plugin-opener": { "version": "2.5.3" },
    "node_modules/@tauri-apps/plugin-process": { "version": "2.3.1" },
    "node_modules/@tauri-apps/plugin-updater": { "version": "2.10.1" }
  }
}
EOF

  cat > "$dir/src-tauri/Cargo.toml" <<'EOF'
[build-dependencies]
tauri-build = { version = "2.5.6", features = [] }

[dependencies]
tauri = { version = "2.10.3", features = ["devtools"] }
tauri-plugin-dialog = "2.7.0"
tauri-plugin-fs = "2.5.0"
tauri-plugin-global-shortcut = "2.3.1"
tauri-plugin-opener = "2.5.3"
tauri-plugin-updater = "2.10.1"
tauri-plugin-window-state = "2.4.1"
EOF

  cat > "$dir/src-tauri/Cargo.lock" <<'EOF'
[[package]]
name = "tauri"
version = "2.10.3"

[[package]]
name = "tauri-build"
version = "2.5.6"

[[package]]
name = "tauri-plugin-dialog"
version = "2.7.0"

[[package]]
name = "tauri-plugin-fs"
version = "2.5.0"

[[package]]
name = "tauri-plugin-global-shortcut"
version = "2.3.1"

[[package]]
name = "tauri-plugin-opener"
version = "2.5.3"

[[package]]
name = "tauri-plugin-updater"
version = "2.10.1"

[[package]]
name = "tauri-plugin-window-state"
version = "2.4.1"
EOF
}

# ─── Test 1: REJECT — top-level node_modules/package.json ───────────────────

T1=$(setup_repo)
mkdir -p "$T1/node_modules"
echo '{"name":"pkg"}' > "$T1/node_modules/package.json"
git -C "$T1" add -f node_modules/package.json
if git -C "$T1" commit -m "test" 2>/dev/null; then
  fail "Test 1: should have rejected node_modules/package.json"
else
  pass "Test 1: rejects top-level node_modules/package.json"
fi
rm -rf "$T1"

# ─── Test 2: REJECT — nested node_modules path ───────────────────────────────

T2=$(setup_repo)
mkdir -p "$T2/packages/lib/node_modules"
echo "// bar" > "$T2/packages/lib/node_modules/bar.js"
git -C "$T2" add -f packages/lib/node_modules/bar.js
if git -C "$T2" commit -m "test" 2>/dev/null; then
  fail "Test 2: should have rejected packages/lib/node_modules/bar.js"
else
  pass "Test 2: rejects nested packages/lib/node_modules/bar.js"
fi
rm -rf "$T2"

# ─── Test 3: ALLOW — normal source file ──────────────────────────────────────

T3=$(setup_repo)
mkdir -p "$T3/src"
echo "export const x = 1;" > "$T3/src/index.ts"
git -C "$T3" add src/index.ts
if git -C "$T3" commit -m "test" 2>/dev/null; then
  pass "Test 3: allows normal src/index.ts"
else
  fail "Test 3: should have allowed src/index.ts"
fi
rm -rf "$T3"

# ─── Test 4: ALLOW — file with 'node_modules' in non-directory context ───────

T4=$(setup_repo)
mkdir -p "$T4/docs"
echo "# Guide to node_modules" > "$T4/docs/node_modules_guide.md"
git -C "$T4" add docs/node_modules_guide.md
if git -C "$T4" commit -m "test" 2>/dev/null; then
  pass "Test 4: allows docs/node_modules_guide.md (no false positive)"
else
  fail "Test 4: should have allowed node_modules_guide.md"
fi
rm -rf "$T4"

# ─── Test 5: REJECT — staged Tauri version drift ────────────────────────────

T5=$(setup_repo)
write_aligned_tauri_files "$T5"
python3 - <<'PY' "$T5/frontend/package.json"
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
data = json.loads(path.read_text())
data["dependencies"]["@tauri-apps/api"] = "^2.11.0"
path.write_text(json.dumps(data, indent=2) + "\n")
PY
git -C "$T5" add frontend/package.json frontend/package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock
if git -C "$T5" commit -m "test" 2>/dev/null; then
  fail "Test 5: should have rejected staged Tauri version drift"
else
  pass "Test 5: rejects staged Tauri version drift"
fi
rm -rf "$T5"

# ─── Test 6: ALLOW — aligned staged Tauri version files ─────────────────────

T6=$(setup_repo)
write_aligned_tauri_files "$T6"
git -C "$T6" add frontend/package.json frontend/package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock
if git -C "$T6" commit -m "test" 2>/dev/null; then
  pass "Test 6: allows aligned staged Tauri version files"
else
  fail "Test 6: should have allowed aligned staged Tauri version files"
fi
rm -rf "$T6"

# ─── Results ─────────────────────────────────────────────────────────────────

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
