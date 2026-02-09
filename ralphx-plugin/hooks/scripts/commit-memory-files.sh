#!/bin/bash
# Stop hook: check if memory-related files were modified but not committed.
# Blocks stopping until the agent commits them.
# No stop_hook_active guard — this hook is self-terminating (once files are
# committed, git status returns clean and the hook passes).

INPUT=$(cat)

PROJECT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$PROJECT_ROOT" ]; then
  exit 0
fi

# Check for uncommitted changes in memory-related paths
MEMORY_CHANGES=$(git -C "$PROJECT_ROOT" status --porcelain -- \
  '.claude/rules/' \
  2>/dev/null | head -20)

if [ -n "$MEMORY_CHANGES" ]; then
  # Escape newlines for JSON
  ESCAPED=$(echo "$MEMORY_CHANGES" | sed 's/$/\\n/' | tr -d '\n')
  echo "{\"decision\":\"block\",\"reason\":\"Memory files were modified but not committed. Commit ONLY these files with: git add .claude/rules/ && git commit -m 'chore(memory): update rules and optimization log':\\n${ESCAPED}\"}"
  exit 0
fi

# No uncommitted memory files
exit 0
