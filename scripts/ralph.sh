#!/bin/bash

# Ralph Wiggum Autonomous Development Loop
# =========================================
# This script runs Claude Code in a continuous loop, each iteration with a fresh
# context window. It reads PROMPT.md and feeds it to Claude until all tasks are
# complete or max iterations is reached.
#
# Usage: ./ralph.sh <max_iterations>
# Example: ./ralph.sh 20

set -e

# Track child processes for cleanup
CLAUDE_PID=""

# Cleanup function to kill all child processes
cleanup() {
  echo ""
  echo -e "${YELLOW}Cleaning up...${NC}"

  # Kill the Claude process if running
  if [[ -n "$CLAUDE_PID" ]] && kill -0 "$CLAUDE_PID" 2>/dev/null; then
    echo -e "${YELLOW}Killing Claude process (PID: $CLAUDE_PID)...${NC}"
    kill -TERM "$CLAUDE_PID" 2>/dev/null || true
    sleep 1
    kill -KILL "$CLAUDE_PID" 2>/dev/null || true
  fi

  # Kill any remaining child processes in our process group
  # This catches any bash commands Claude spawned
  pkill -P $$ 2>/dev/null || true

  # Also kill any 'claude' processes started by this script
  pkill -f "claude -p" 2>/dev/null || true

  echo -e "${YELLOW}Cleanup complete.${NC}"
}

# Set up trap for cleanup on exit/interrupt
trap cleanup EXIT INT TERM

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check for required argument
if [ -z "$1" ]; then
  echo -e "${RED}Error: Missing required argument${NC}"
  echo ""
  echo "Usage: $0 <max_iterations>"
  echo "Example: $0 20"
  exit 1
fi

MAX_ITERATIONS=$1

# Verify required files exist
if [ ! -f "PROMPT.md" ]; then
  echo -e "${RED}Error: PROMPT.md not found${NC}"
  echo "Please create PROMPT.md or run /create-prd first"
  exit 1
fi

if [ ! -f "specs/prd.md" ]; then
  echo -e "${RED}Error: specs/prd.md not found${NC}"
  echo "Please create your PRD or run /create-prd first"
  exit 1
fi

if [ ! -f "logs/activity.md" ]; then
  echo -e "${YELLOW}Warning: logs/activity.md not found, creating it...${NC}"
  cat > logs/activity.md << 'EOF'
# Project Build - Activity Log

## Current Status
**Last Updated:** Not started
**Tasks Completed:** 0
**Current Task:** None

---

## Session Log

<!-- Agent will append dated entries here -->
EOF
fi

# Create screenshots and logs directories if they don't exist
mkdir -p screenshots
mkdir -p logs

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}   Ralph Wiggum Autonomous Loop${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
echo -e "Max iterations: ${GREEN}$MAX_ITERATIONS${NC}"
echo -e "Completion signal: ${GREEN}<promise>COMPLETE</promise>${NC}"
echo ""
echo -e "${YELLOW}Starting in 3 seconds... Press Ctrl+C to abort${NC}"
sleep 3
echo ""

# Main loop
for ((i=1; i<=MAX_ITERATIONS; i++)); do
  echo -e "${BLUE}======================================${NC}"
  echo -e "${BLUE}   Iteration $i of $MAX_ITERATIONS${NC}"
  echo -e "${BLUE}======================================${NC}"
  echo ""

  # Clear previous iteration output
  rm -f logs/iteration_$i.json

  # Run Claude with the prompt from PROMPT.md
  # Stream JSON output with verbose, parse for readable display
  # Use process substitution to capture PID while still piping
  exec 3< <(claude -p "$(cat PROMPT.md)" --output-format stream-json --verbose --dangerously-skip-permissions 2>&1)
  CLAUDE_PID=$!

  while IFS= read -r line <&3; do
    # Save raw JSON to file for completion check
    echo "$line" >> logs/iteration_$i.json

    # Parse and display readable content
    type=$(echo "$line" | jq -r '.type // empty' 2>/dev/null)

    if [[ "$type" == "assistant" ]]; then
      # Extract text content from assistant messages
      text=$(echo "$line" | jq -r '.message.content[]? | select(.type=="text") | .text // empty' 2>/dev/null)
      if [[ -n "$text" ]]; then
        echo -e "${GREEN}$text${NC}"
      fi

      # Show tool calls
      tool=$(echo "$line" | jq -r '.message.content[]? | select(.type=="tool_use") | "\(.name): \(.input | tostring | .[0:100])"' 2>/dev/null)
      if [[ -n "$tool" ]]; then
        echo -e "${YELLOW}→ $tool${NC}"
      fi
    elif [[ "$type" == "user" ]]; then
      # Show tool results briefly
      tool_result=$(echo "$line" | jq -r '.message.content[]? | select(.type=="tool_result") | .content // empty' 2>/dev/null | head -c 200)
      if [[ -n "$tool_result" ]]; then
        echo -e "${BLUE}← ${tool_result:0:150}...${NC}"
      fi
    fi
  done || true
  exec 3<&-  # Close file descriptor
  CLAUDE_PID=""  # Clear PID after completion
  result=$(cat logs/iteration_$i.json 2>/dev/null || echo "")
  echo ""

  # Check for completion signal - only in assistant text output (not file contents in tool results)
  # Extract just assistant text messages and check for COMPLETE there
  assistant_text=$(jq -r 'select(.type=="assistant") | .message.content[]? | select(.type=="text") | .text // empty' logs/iteration_$i.json 2>/dev/null | tr '\n' ' ')
  if [[ "$assistant_text" == *"<promise>COMPLETE</promise>"* ]]; then
    echo ""
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}   ALL TASKS COMPLETE!${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""
    echo -e "Finished after ${GREEN}$i${NC} iteration(s)"
    echo ""
    echo "Next steps:"
    echo "  1. Review the completed work in your project"
    echo "  2. Check logs/activity.md for the full build log"
    echo "  3. Review screenshots/ for visual verification"
    echo "  4. Run your tests to verify everything works"
    echo ""
    exit 0
  fi

  echo ""
  echo -e "${YELLOW}--- End of iteration $i ---${NC}"
  echo ""

  # Small delay between iterations to prevent hammering
  sleep 2
done

echo ""
echo -e "${RED}======================================${NC}"
echo -e "${RED}   MAX ITERATIONS REACHED${NC}"
echo -e "${RED}======================================${NC}"
echo ""
echo -e "Reached max iterations (${RED}$MAX_ITERATIONS${NC}) without completion."
echo ""
echo "Options:"
echo "  1. Run again with more iterations: ./ralph.sh 50"
echo "  2. Check logs/activity.md to see current progress"
echo "  3. Check specs/prd.md to see remaining tasks"
echo "  4. Manually complete remaining tasks"
echo ""
exit 1
