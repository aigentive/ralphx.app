#!/usr/bin/env bash
# Remove @ references from a rule file's Required Context line
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
RULEFILE="${1:?Usage: rule-strip-refs.sh <rulefile> [--dry-run]}"
DRY_RUN="${2:-}"
FILE="$RULES_DIR/$RULEFILE"

if [[ ! -f "$FILE" ]]; then
  echo "Error: $FILE not found" >&2
  exit 1
fi

# Find lines with @.claude/rules/ references
ref_count=$(count_at_refs "$FILE")
if [[ "$ref_count" -eq 0 ]]; then
  echo "No @ references found in $RULEFILE"
  exit 0
fi

# Get the original line(s)
original=$(grep '@\.claude/rules/' "$FILE")

if [[ "$DRY_RUN" == "--dry-run" ]]; then
  echo "DRY RUN: Would update $RULEFILE"
  echo ""
  echo "ORIGINAL:"
  echo "$original"
  echo ""
  echo "REPLACEMENT:"
  echo "$original" | sed -E 's|@\.claude/rules/||g'
  echo ""
  echo "CHANGES: $ref_count @ references removed"
  exit 0
fi

# Apply: use sed to replace in-place (BSD-compatible)
if [[ "$(uname)" == "Darwin" ]]; then
  sed -i '' 's|@\.claude/rules/||g' "$FILE"
else
  sed -i 's|@\.claude/rules/||g' "$FILE"
fi

echo "Updated $RULEFILE: removed $ref_count @ references"
