#!/bin/bash
# Stop hook: enforce that /rule-manager was run if SessionStart reported issues.
# Reads the transcript to check for both conditions.

INPUT=$(cat)
TRANSCRIPT=$(echo "$INPUT" | jq -r '.transcript_path // empty')
STOP_HOOK_ACTIVE=$(echo "$INPUT" | jq -r '.stop_hook_active // false')

# Prevent infinite loops — if we already forced continuation, allow stop
if [ "$STOP_HOOK_ACTIVE" = "true" ]; then
  exit 0
fi

# No transcript? Can't check, allow stop
if [ -z "$TRANSCRIPT" ] || [ ! -f "$TRANSCRIPT" ]; then
  exit 0
fi

# Check if SessionStart reported issues (issue count > 0)
HAS_ISSUES=$(grep -c '"Rule health:.*[1-9][0-9]* issue' "$TRANSCRIPT" 2>/dev/null || echo "0")

if [ "$HAS_ISSUES" = "0" ]; then
  # No issues reported, allow stop
  exit 0
fi

# Check if /rule-manager or ralphx:rule-manager was invoked
RAN_RULE_MANAGER=$(grep -c 'rule-manager\|rule_manager' "$TRANSCRIPT" 2>/dev/null || echo "0")

# Subtract the SessionStart hook mention itself (which contains "rule-manager" in the command path)
# Look specifically for skill invocation patterns
SKILL_INVOKED=$(grep -cE '(Skill|/rule-manager|ralphx:rule-manager)' "$TRANSCRIPT" 2>/dev/null || echo "0")

# If rule-manager appears more than just in the hook config, it was invoked
if [ "$SKILL_INVOKED" -gt 2 ]; then
  exit 0
fi

# Issues exist but rule-manager wasn't run — block stop
echo '{"decision":"block","reason":"The SessionStart audit reported rule optimization issues. Please run /rule-manager (the ralphx:rule-manager skill) to apply the next optimization before stopping."}' 
exit 0
