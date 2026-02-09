#!/usr/bin/env bash
# Quick stats for a single rule file
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
RULEFILE="${1:?Usage: rule-stats.sh <rulefile>}"
FILE="$RULES_DIR/$RULEFILE"

if [[ ! -f "$FILE" ]]; then
  echo "Error: $FILE not found" >&2
  exit 1
fi

lines=$(count_lines "$FILE")
tokens=$(estimate_tokens "$FILE")
has_paths=$(has_paths_frontmatter "$FILE")
ref_count=$(count_at_refs "$FILE")
refs=$(get_at_refs "$FILE" | tr '\n' ',' | sed 's/,$//')

# Determine health status
health="healthy"
[[ $lines -gt 400 ]] && health="oversized"
[[ "$has_paths" == "false" ]] && health="needs_scoping"
[[ $ref_count -gt 0 ]] && health="has_redundant_refs"

# Get current paths if any
paths=""
if [[ "$has_paths" == "true" ]]; then
  paths=$(get_paths "$FILE" | tr '\n' ',' | sed 's/,$//')
fi

# Check who references this file
referenced_by=$(grep -rl "@\.claude/rules/$RULEFILE" "$(get_project_root)" 2>/dev/null | \
  sed "s|$(get_project_root)/||" | tr '\n' ',' | sed 's/,$//' || echo "")

cat <<EOF
{
  "file": "$RULEFILE",
  "lines": $lines,
  "tokens": $tokens,
  "has_paths": $has_paths,
  "paths": "$paths",
  "ref_count": $ref_count,
  "refs": "$refs",
  "referenced_by": "$referenced_by",
  "health": "$health"
}
EOF
