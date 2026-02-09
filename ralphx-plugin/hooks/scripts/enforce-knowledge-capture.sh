#!/bin/bash
# Stop hook: suggest running /knowledge-capture if the session did meaningful work
# and the skill hasn't been invoked yet. Only blocks ONCE (via stop_hook_active).

INPUT=$(cat)
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')
TRANSCRIPT=$(echo "$INPUT" | jq -r '.transcript_path // empty')

# If we already blocked and the agent continued, allow stop
# (prevents infinite loop — we only suggest once)
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# No transcript? Allow stop
if [ -z "$TRANSCRIPT" ] || [ ! -f "$TRANSCRIPT" ]; then
  exit 0
fi

# Check if knowledge-capture was already invoked this session
KC_INVOKED=$(grep -c 'knowledge-capture\|knowledge_capture' "$TRANSCRIPT" 2>/dev/null || echo "0")
if [ "$KC_INVOKED" -gt 0 ]; then
  exit 0
fi

# Check if meaningful work was done (more than just a greeting)
# Look for tool uses beyond the initial SessionStart hook
TOOL_USES=$(grep -c '"type":"tool_use"\|"type": "tool_use"' "$TRANSCRIPT" 2>/dev/null || echo "0")
if [ "$TOOL_USES" -lt 3 ]; then
  # Trivial session, skip knowledge capture
  exit 0
fi

# Meaningful work done, knowledge-capture not yet run — block once
echo '{"decision":"block","reason":"This session involved meaningful work. Consider running /knowledge-capture to evaluate if any specialized project knowledge should be captured as scoped .claude/rules/ files. If nothing is worth capturing, just say so and stop."}'
exit 0
