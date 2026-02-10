#!/usr/bin/env bash
# Find rule files that may be stale or orphaned
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
PROJECT_ROOT="$(get_project_root)"

# Collect CLAUDE.md files and .claude/ contents (excluding rules dir itself) as search targets
SEARCH_TARGETS=()
while IFS= read -r f; do
  SEARCH_TARGETS+=("$f")
done < <(find "$PROJECT_ROOT" -name 'CLAUDE.md' -not -path '*/node_modules/*' -not -path '*/.git/*' 2>/dev/null)
while IFS= read -r f; do
  SEARCH_TARGETS+=("$f")
done < <(find "$PROJECT_ROOT/.claude" -type f -not -path "$RULES_DIR/*" 2>/dev/null)

echo "ORPHAN/STALE ANALYSIS"
echo "====================="
echo ""

for file in "$RULES_DIR"/*.md; do
  [[ -f "$file" ]] || continue
  [[ "$(basename "$file")" == .* ]] && continue
  name="$(basename "$file")"

  # Check if any CLAUDE.md or .claude/ file (outside rules/) references this rule
  ref_count=0
  if [[ ${#SEARCH_TARGETS[@]} -gt 0 ]]; then
    ref_count=$(grep -rl "$name" "${SEARCH_TARGETS[@]}" 2>/dev/null | wc -l | tr -d ' ')
  fi

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
