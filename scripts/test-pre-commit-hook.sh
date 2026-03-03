#!/usr/bin/env bash
# Tests for .githooks/pre-commit
# Verifies the hook rejects node_modules paths and allows normal files.

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

# ─── Results ─────────────────────────────────────────────────────────────────

echo ""
echo "Results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]
