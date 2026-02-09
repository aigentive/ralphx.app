#!/usr/bin/env bash
# Find rule files that may be stale or orphaned
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
PROJECT_ROOT="$(get_project_root)"

echo "ORPHAN/STALE ANALYSIS"
echo "====================="
echo ""

for file in "$RULES_DIR"/*.md; do
  [[ -f "$file" ]] || continue
  [[ "$(basename "$file")" == .* ]] && continue
  name="$(basename "$file")"

  # Check if any non-rule file references this rule (disable pipefail for grep chain)
  ref_count=$(set +o pipefail; grep -rl "$name" "$PROJECT_ROOT" 2>/dev/null \
    | grep -v '.claude/rules/' \
    | grep -v '.git/' \
    | grep -v 'node_modules/' \
    | grep -v '.optimization-log' \
    | wc -l | tr -d ' ')

  if [[ "$ref_count" -eq 0 ]]; then
    lines=$(count_lines "$file")
    echo "ORPHANED: $name ($lines lines) — never referenced outside .claude/rules/"
  fi
done

echo ""
echo "STALE CHECK (references to non-existent files):"

for file in "$RULES_DIR"/*.md; do
  [[ -f "$file" ]] || continue
  [[ "$(basename "$file")" == .* ]] && continue
  name="$(basename "$file")"

  # Extract file paths from backticks and check existence
  paths=$(set +o pipefail; grep -oE '`[a-zA-Z0-9_./-]+\.(rs|ts|tsx|js)`' "$file" 2>/dev/null | tr -d '`' | sort -u)
  stale_count=0
  while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    if [[ ! -e "$PROJECT_ROOT/$path" ]]; then
      stale_count=$((stale_count + 1))
    fi
  done <<< "$paths"

  if [[ $stale_count -gt 0 ]]; then
    echo "  $name: $stale_count references to non-existent files"
  fi
done
