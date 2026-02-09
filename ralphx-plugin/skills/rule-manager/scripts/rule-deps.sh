#!/usr/bin/env bash
# Dependency graph from @ references
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
PROJECT_ROOT="$(get_project_root)"
FORMAT="${1:---text}"  # --json or --text

# Build edge list as temp file: "from<TAB>refs_csv" per line
EDGES_FILE=$(mktemp)
trap 'rm -f "$EDGES_FILE"' EXIT

# Scan rule files
for file in "$RULES_DIR"/*.md; do
  [[ -f "$file" ]] || continue
  [[ "$(basename "$file")" == .* ]] && continue
  name="$(basename "$file")"
  refs=$(get_at_refs "$file" | tr '\n' ',' | sed 's/,$//')
  printf '%s\t%s\n' "$name" "$refs" >> "$EDGES_FILE"
done

# Scan CLAUDE.md files
for cmd_file in "$PROJECT_ROOT/CLAUDE.md" "$PROJECT_ROOT/src/CLAUDE.md" "$PROJECT_ROOT/src-tauri/CLAUDE.md"; do
  [[ -f "$cmd_file" ]] || continue
  name="$(basename "$cmd_file")"
  [[ "$cmd_file" != "$PROJECT_ROOT/CLAUDE.md" ]] && name="$(basename "$(dirname "$cmd_file")")/$(basename "$cmd_file")"
  refs=$(get_at_refs "$cmd_file" | tr '\n' ',' | sed 's/,$//')
  [[ -n "$refs" ]] && printf '%s\t%s\n' "$name" "$refs" >> "$EDGES_FILE"
done

case "$FORMAT" in
  --json)
    echo "{"
    echo "  \"edges\": ["
    first=true
    while IFS=$'\t' read -r from refs; do
      [[ -z "$refs" ]] && continue
      IFS=',' read -ra deps <<< "$refs"
      for to in "${deps[@]}"; do
        [[ -z "$to" ]] && continue
        $first || echo ","
        printf '    {"from": "%s", "to": "%s"}' "$from" "$to"
        first=false
      done
    done < "$EDGES_FILE"
    echo ""
    echo "  ],"
    echo "  \"rule_files_with_refs\": ["
    first=true
    while IFS=$'\t' read -r from refs; do
      [[ -z "$refs" ]] && continue
      [[ "$from" == *"CLAUDE.md"* ]] && continue
      $first || echo ","
      printf '    "%s"' "$from"
      first=false
    done < "$EDGES_FILE"
    echo ""
    echo "  ]"
    echo "}"
    ;;
  --text|*)
    echo "DEPENDENCY GRAPH"
    echo "================"
    echo ""
    echo "RULE FILE REFERENCES:"
    sort "$EDGES_FILE" | while IFS=$'\t' read -r from refs; do
      [[ -z "$refs" ]] && continue
      [[ "$from" == *"CLAUDE.md"* ]] && continue
      echo "  $from -> $refs"
    done
    echo ""
    echo "CLAUDE.MD REFERENCES:"
    sort "$EDGES_FILE" | while IFS=$'\t' read -r from refs; do
      [[ "$from" != *"CLAUDE.md"* ]] && continue
      echo "  $from -> $refs"
    done
    ;;
esac
