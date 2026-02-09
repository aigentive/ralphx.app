#!/usr/bin/env bash
# Suggest paths: frontmatter for a rule file
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
PROJECT_ROOT="$(get_project_root)"
RULEFILE="${1:?Usage: rule-suggest-paths.sh <rulefile>}"
FILE="$RULES_DIR/$RULEFILE"

if [[ ! -f "$FILE" ]]; then
  echo "Error: $FILE not found" >&2
  exit 1
fi

# Already has paths?
if [[ "$(has_paths_frontmatter "$FILE")" == "true" ]]; then
  echo "File already has paths: frontmatter"
  get_paths "$FILE"
  exit 0
fi

# Extract potential path terms from the file content
terms=$(grep -oE '`[a-zA-Z0-9_./-]+\.(rs|ts|tsx|js|md|json)`' "$FILE" | tr -d '`' | sort -u || true)
dirs=$(grep -oE '(src|src-tauri|ralphx-plugin|ralphx-mcp-server|streams|specs|tests|screenshots)/[a-zA-Z0-9_/-]+' "$FILE" | sort -u || true)

# Combine and deduplicate
all_terms=$(echo -e "$terms\n$dirs" | grep -v '^$' | sort -u || true)

# For each term, check if it exists in the project
echo "{"
echo "  \"file\": \"$RULEFILE\","
echo "  \"terms_found\": ["

first=true
while IFS= read -r term; do
  [[ -z "$term" ]] && continue
  $first || echo ","
  if [[ -e "$PROJECT_ROOT/$term" ]] || ls "$PROJECT_ROOT"/$term 2>/dev/null | head -1 > /dev/null 2>&1; then
    printf '    {"term": "%s", "exists": true}' "$term"
  else
    printf '    {"term": "%s", "exists": false}' "$term"
  fi
  first=false
done <<< "$all_terms"

echo ""
echo "  ],"

# Generate suggested patterns by clustering directories
# Use temp file instead of associative array (Bash 3.2 compat)
COUNTS_FILE=$(mktemp)
trap 'rm -f "$COUNTS_FILE"' EXIT

while IFS= read -r term; do
  [[ -z "$term" ]] && continue
  echo "$term" | cut -d'/' -f1-2
done <<< "$all_terms" | sort | uniq -c | sort -rn > "$COUNTS_FILE"

echo "  \"suggested_paths\": ["

first=true
while read -r count dir; do
  [[ -z "$dir" ]] && continue
  $first || echo ","
  if [[ -d "$PROJECT_ROOT/$dir" ]]; then
    printf '    {"pattern": "%s/**", "references": %d}' "$dir" "$count"
  else
    printf '    {"pattern": "%s", "references": %d}' "$dir" "$count"
  fi
  first=false
done < "$COUNTS_FILE"

echo ""
echo "  ]"
echo "}"
