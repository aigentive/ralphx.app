#!/usr/bin/env bash
# check-design-tokens.sh
#
# Fail the build if components leak design-system rules. Run from the frontend
# directory (or via `npm run check:tokens`).
#
# Guards:
#   1. No primitive tokens in component files (tier-1 leaks).
#   2. No Tailwind default palette utilities (palette leaks).
#   3. No hardcoded brand hex / rgba / rgb in component files.
#
# Excluded paths (documented in specs/design/styleguide.md §12):
#   - src/components/WelcomeScreen/** — marketing splash
#   - src/components/TaskGraph/battle-v2/BattleModeV2Overlay.tsx — game canvas
#   - *.test.tsx / *.test.ts — tests pin class/token names
#   - src/styles/** — token sources

set -euo pipefail

FRONTEND_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$FRONTEND_DIR"

EXCLUDE_PATHS=(
  --exclude='*.test.tsx'
  --exclude='*.test.ts'
  --exclude='*.snap'
  --exclude-dir='__snapshots__'
  --exclude-dir='WelcomeScreen'
  --exclude-dir='battle-v2'
)

FAIL=0

print_section() { printf "\n\033[1m%s\033[0m\n" "$1"; }
print_fail()    { printf "\033[31mFAIL\033[0m %s\n" "$1"; FAIL=1; }
print_pass()    { printf "\033[32mPASS\033[0m %s\n" "$1"; }

# ----------------------------------------------------------------------------
# Guard 1 — primitive-tier token leaks in components
# ----------------------------------------------------------------------------
print_section "Guard 1: primitive tokens in components"

PRIMITIVE_PATTERN='var\(--(gray|orange|amber|yellow-[0-9]|blue-[0-9]|cvd|hc|alpha-)'
hits=$(grep -rEn "${PRIMITIVE_PATTERN}" src/components "${EXCLUDE_PATHS[@]}" || true)

if [ -n "${hits}" ]; then
  print_fail "Primitive tokens must not appear in components (use the semantic or component tier)."
  echo "${hits}"
else
  print_pass "No primitive-tier leaks."
fi

# ----------------------------------------------------------------------------
# Guard 2 — Tailwind default palette utilities in components
# ----------------------------------------------------------------------------
print_section "Guard 2: Tailwind default palette in components"

PALETTE_PATTERN='\b(bg|text|border|ring|from|to|via)-(red|green|blue|amber|emerald|rose|yellow|indigo|purple|pink|sky|slate|zinc|neutral|stone)-[0-9]{2,3}\b'
hits=$(grep -rEn "${PALETTE_PATTERN}" src/components "${EXCLUDE_PATHS[@]}" || true)

if [ -n "${hits}" ]; then
  print_fail "Tailwind default palette classes bypass the theme system. Use text-status-error / bg-accent-primary / etc."
  echo "${hits}"
else
  print_pass "No Tailwind default-palette leaks."
fi

# ----------------------------------------------------------------------------
# Guard 3 — inline rgba/rgb literals in components
# ----------------------------------------------------------------------------
print_section "Guard 3: inline rgba/rgb in components"

# Allow `color-mix(...)` calls — they're the canonical dynamic-alpha pattern.
RGBA_HITS=$(grep -rEn 'rgba\(|rgb\(' src/components "${EXCLUDE_PATHS[@]}" \
  | grep -v 'color-mix' || true)

if [ -n "${RGBA_HITS}" ]; then
  print_fail "Inline rgba/rgb literals must be tokenised (use var(--token) or withAlpha())."
  echo "${RGBA_HITS}"
else
  print_pass "No inline rgba/rgb literals."
fi

# ----------------------------------------------------------------------------
# Guard 4 — brand-hex literals in live code (comments/docstrings exempt)
# ----------------------------------------------------------------------------
print_section "Guard 4: brand hex literals in live component code"

BRAND_HEX='#(ff6b35|FF6B35|34c759|34C759|64d2ff|64D2FF|0a84ff|0A84FF|ff9f0a|FF9F0A|ff3b30|FF3B30|30d158|30D158|ffd60a|FFD60A)'
BRAND_HITS=$(grep -rEn "${BRAND_HEX}" src/components "${EXCLUDE_PATHS[@]}" \
  | grep -vE ':[0-9]+:\s*\*' \
  | grep -vE ':[0-9]+:\s*//' || true)

if [ -n "${BRAND_HITS}" ]; then
  print_fail "Brand hex literals in live code — use var(--accent-primary) / var(--status-*)."
  echo "${BRAND_HITS}"
else
  print_pass "No brand hex in live code (docstrings exempt)."
fi

# ----------------------------------------------------------------------------
# Result
# ----------------------------------------------------------------------------
echo ""
if [ "$FAIL" -ne 0 ]; then
  echo -e "\033[31mDesign-token guards failed.\033[0m Fix the leaks above or update specs/design/styleguide.md §12 exclusions."
  exit 1
fi
echo -e "\033[32mAll design-token guards passed.\033[0m"
