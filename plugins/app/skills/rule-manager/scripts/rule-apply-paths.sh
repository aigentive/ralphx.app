#!/usr/bin/env bash
# Add or update paths: frontmatter on a rule file
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
RULEFILE="${1:?Usage: rule-apply-paths.sh <rulefile> <path1> [path2...] [--dry-run]}"
shift

# Collect paths and flags
PATHS=()
DRY_RUN=false
for arg in "$@"; do
  if [[ "$arg" == "--dry-run" ]]; then
    DRY_RUN=true
  else
    PATHS+=("$arg")
  fi
done

if [[ ${#PATHS[@]} -eq 0 ]]; then
  echo "Error: At least one path pattern required" >&2
  exit 1
fi

FILE="$RULES_DIR/$RULEFILE"
if [[ ! -f "$FILE" ]]; then
  echo "Error: $FILE not found" >&2
  exit 1
fi

# Build new frontmatter
new_fm="---
paths:"
for p in "${PATHS[@]}"; do
  new_fm="$new_fm
  - \"$p\""
done
new_fm="$new_fm
---"

if $DRY_RUN; then
  echo "DRY RUN: Would update $RULEFILE"
  echo ""
  if has_frontmatter "$FILE"; then
    echo "CURRENT FRONTMATTER:"
    sed -n '1,/^---$/p' "$FILE" | tail -n +2
    echo ""
  else
    echo "CURRENT FRONTMATTER: (none)"
    echo ""
  fi
  echo "NEW FRONTMATTER:"
  echo "$new_fm"
  exit 0
fi

# Apply
TMPFILE=$(mktemp)
if has_frontmatter "$FILE"; then
  # Replace existing frontmatter
  echo "$new_fm" > "$TMPFILE"
  # Skip old frontmatter (from line 2 to second ---)
  # Get line number of second ---
  second_fence=$(awk 'NR>1 && /^---$/ { print NR; exit }' "$FILE")
  if [[ -n "$second_fence" ]]; then
    tail -n +"$((second_fence + 1))" "$FILE" >> "$TMPFILE"
  fi
else
  # Insert new frontmatter at top
  echo "$new_fm" > "$TMPFILE"
  echo "" >> "$TMPFILE"
  cat "$FILE" >> "$TMPFILE"
fi

mv "$TMPFILE" "$FILE"
echo "Updated $RULEFILE with paths: ${PATHS[*]}"
