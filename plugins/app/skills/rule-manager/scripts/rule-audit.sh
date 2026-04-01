#!/usr/bin/env bash
# Full audit of .claude/rules/ directory
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

RULES_DIR="$(get_rules_dir)"
FORMAT="${1:---text}"  # --json or --text or --brief

if [[ ! -d "$RULES_DIR" ]]; then
  echo "No .claude/rules/ directory found" >&2
  exit 1
fi

# Collect data
total_files=0
total_lines=0
total_tokens=0
total_refs=0
files_with_paths=0
oversized_files=""
unscoped_files=""
high_ref_files=""
json_files="["

for file in "$RULES_DIR"/*.md; do
  [[ -f "$file" ]] || continue
  [[ "$(basename "$file")" == .* ]] && continue  # skip hidden files

  name="$(basename "$file")"
  lines=$(count_lines "$file")
  tokens=$(estimate_tokens "$file")
  has_paths=$(has_paths_frontmatter "$file")
  ref_count=$(count_at_refs "$file")
  refs=$(get_at_refs "$file" | tr '\n' ',' | sed 's/,$//')

  total_files=$((total_files + 1))
  total_lines=$((total_lines + lines))
  total_tokens=$((total_tokens + tokens))
  total_refs=$((total_refs + ref_count))
  [[ "$has_paths" == "true" ]] && files_with_paths=$((files_with_paths + 1))
  [[ $lines -gt 400 ]] && oversized_files="$oversized_files $name($lines)"
  [[ "$has_paths" == "false" ]] && unscoped_files="$unscoped_files $name"
  [[ $ref_count -gt 3 ]] && high_ref_files="$high_ref_files $name($ref_count)"

  # Build JSON entry
  [[ $total_files -gt 1 ]] && json_files="$json_files,"
  json_files="$json_files{\"name\":\"$name\",\"lines\":$lines,\"tokens\":$tokens,\"has_paths\":$has_paths,\"ref_count\":$ref_count,\"refs\":\"$refs\"}"
done

json_files="$json_files]"

# Also check CLAUDE.md files for @ refs
claude_md_refs=""
for cmd_file in "$(get_project_root)/CLAUDE.md" "$(get_project_root)/frontend/src/CLAUDE.md" "$(get_project_root)/src-tauri/CLAUDE.md"; do
  if [[ -f "$cmd_file" ]]; then
    refs=$(get_at_refs "$cmd_file" | tr '\n' ',' | sed 's/,$//')
    [[ -n "$refs" ]] && claude_md_refs="$claude_md_refs $(basename "$(dirname "$cmd_file")")/$(basename "$cmd_file"):$refs"
  fi
done

# Output
case "$FORMAT" in
  --json)
    cat <<EOF
{
  "summary": {
    "total_files": $total_files,
    "total_lines": $total_lines,
    "estimated_tokens": $total_tokens,
    "files_with_paths": $files_with_paths,
    "files_without_paths": $((total_files - files_with_paths)),
    "total_at_refs": $total_refs
  },
  "files": $json_files,
  "health": {
    "oversized": "$(echo $oversized_files | xargs)",
    "unscoped": "$(echo $unscoped_files | xargs)",
    "high_refs": "$(echo $high_ref_files | xargs)",
    "claude_md_refs": "$(echo $claude_md_refs | xargs)"
  }
}
EOF
    ;;
  --brief)
    issues=""
    issue_count=0
    [[ $total_refs -gt 0 ]] && issues="$issues $total_refs redundant @ refs," && issue_count=$((issue_count + 1))
    [[ -n "$(echo $oversized_files | xargs)" ]] && issues="$issues oversized files:$oversized_files," && issue_count=$((issue_count + 1))
    unscoped_count=$((total_files - files_with_paths))
    [[ $unscoped_count -gt 6 ]] && issues="$issues $unscoped_count unscoped rules," && issue_count=$((issue_count + 1))

    if [[ $issue_count -gt 0 ]]; then
      echo "Rule health: $total_files files, ~${total_tokens} tokens | ${issue_count} issue(s):${issues%,}"
      echo "Run /rule-manager to apply next optimization."
    else
      echo "Rules healthy. $total_files files, ~${total_tokens} tokens, $files_with_paths scoped."
    fi
    ;;
  --text|*)
    echo "RULE AUDIT REPORT"
    echo "================="
    echo "Total Files: $total_files"
    echo "Total Lines: $total_lines"
    echo "Estimated Tokens: ~$total_tokens"
    echo ""
    echo "Files with paths: frontmatter: $files_with_paths"
    echo "Files with @ references: $(echo $total_refs)"
    echo ""
    if [[ -n "$(echo $oversized_files | xargs)" ]]; then
      echo "OVERSIZED (>400 LOC):$oversized_files"
    fi
    if [[ $((total_files - files_with_paths)) -gt 6 ]]; then
      echo "UNSCOPED RULES: $((total_files - files_with_paths)) files without paths:"
      echo " $unscoped_files"
    fi
    if [[ -n "$(echo $high_ref_files | xargs)" ]]; then
      echo "HIGH REF COUNT (>3):$high_ref_files"
    fi
    if [[ -n "$(echo $claude_md_refs | xargs)" ]]; then
      echo "CLAUDE.MD REFS:$claude_md_refs"
    fi
    ;;
esac
