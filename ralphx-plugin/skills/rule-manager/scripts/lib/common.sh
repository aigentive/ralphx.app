#!/usr/bin/env bash
# Shared functions for rule-manager scripts
set -euo pipefail

# Resolve project root (git root)
get_project_root() {
  git rev-parse --show-toplevel 2>/dev/null || pwd
}

# Resolve .claude/rules/ directory
get_rules_dir() {
  echo "$(get_project_root)/.claude/rules"
}

# Check if file has YAML frontmatter
has_frontmatter() {
  local file="$1"
  head -1 "$file" 2>/dev/null | grep -q '^---$'
}

# Check if file has paths: in frontmatter
has_paths_frontmatter() {
  local file="$1"
  if ! has_frontmatter "$file"; then
    echo "false"
    return
  fi
  # Extract frontmatter block and check for paths:
  sed -n '2,/^---$/p' "$file" | grep -q '^paths:' && echo "true" || echo "false"
}

# Extract paths: values from frontmatter
get_paths() {
  local file="$1"
  if ! has_frontmatter "$file"; then return; fi
  sed -n '2,/^---$/p' "$file" | sed -n '/^paths:/,/^[^ -]/p' | grep '^ *- ' | sed 's/^ *- *//' | tr -d "\"'"
}

# Estimate tokens (chars / 4)
estimate_tokens() {
  local file="$1"
  local chars
  chars=$(wc -c < "$file" | tr -d ' ')
  echo $(( chars / 4 ))
}

# Count lines
count_lines() {
  local file="$1"
  wc -l < "$file" | tr -d ' '
}

# Count @ references to .claude/rules/ in a file (occurrences, not lines)
count_at_refs() {
  local file="$1"
  local count
  count=$(grep -oE '@\.claude/rules/[a-zA-Z0-9_-]+\.md' "$file" 2>/dev/null | wc -l | tr -d ' ') || count=0
  echo "$count"
}

# Extract @ reference targets from a file
get_at_refs() {
  local file="$1"
  grep -oE '@\.claude/rules/[a-zA-Z0-9_-]+\.md' "$file" 2>/dev/null | sed 's|@\.claude/rules/||' || true
}

# Get optimization log path
get_log_path() {
  echo "$(get_rules_dir)/.optimization-log.md"
}

# JSON escape a string
json_escape() {
  printf '%s' "$1" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()), end="")'
}
