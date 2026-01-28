#!/bin/bash

# Ralph Wiggum Autonomous Development Loop - Multi-Stream Version
# ================================================================
# This script runs Claude Code in a continuous loop, each iteration with a fresh
# context window. It reads from stream-specific PROMPT.md files and feeds to Claude
# until tasks are complete or max iterations is reached.
#
# Usage: ./ralph-streams.sh <stream> [max_iterations]
# Streams: features | refactor | polish | verify | hygiene
# Model: Set ANTHROPIC_MODEL env var (default: opus)
#
# Examples:
#   ./ralph-streams.sh features 20
#   ./ralph-streams.sh refactor 10
#   ANTHROPIC_MODEL=sonnet ./ralph-streams.sh polish 15
#
# Legacy mode (backward compatible):
#   ./ralph-streams.sh 20  # Uses PROMPT.md like original ralph.sh

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

# Valid streams
VALID_STREAMS="features|refactor|polish|verify|hygiene"

# Parse arguments - support both stream mode and legacy mode
# Legacy mode: ./ralph-streams.sh <max_iterations>
# Stream mode: ./ralph-streams.sh <stream> [max_iterations]
if [ -z "$1" ]; then
  echo -e "${RED}Error: Missing required argument${NC}"
  echo ""
  echo "Usage: $0 <stream> [max_iterations]"
  echo "       $0 <max_iterations>  (legacy mode)"
  echo ""
  echo "Streams: $VALID_STREAMS"
  echo ""
  echo "Examples:"
  echo "  $0 features 20"
  echo "  $0 refactor 10"
  echo "  ANTHROPIC_MODEL=sonnet $0 polish 15"
  exit 1
fi

# Check if first arg is a number (legacy mode) or a stream name
if [[ "$1" =~ ^[0-9]+$ ]]; then
  # Legacy mode: first arg is max_iterations
  STREAM=""
  MAX_ITERATIONS=$1
  PROMPT_FILE="PROMPT.md"
  STREAM_MODE=false
else
  # Stream mode: first arg is stream name
  STREAM=$1
  MAX_ITERATIONS=${2:-10}  # Default to 10 iterations if not specified

  # Validate stream name
  if [[ ! "$STREAM" =~ ^($VALID_STREAMS)$ ]]; then
    echo -e "${RED}Error: Invalid stream '$STREAM'${NC}"
    echo ""
    echo "Valid streams: $VALID_STREAMS"
    exit 1
  fi

  PROMPT_FILE="streams/${STREAM}/PROMPT.md"
  STREAM_MODE=true
fi

# Model selection via environment variable (default: opus)
MODEL=${ANTHROPIC_MODEL:-opus}
MODEL_FLAG="--model $MODEL"

# Verify required files exist
if [ ! -f "$PROMPT_FILE" ]; then
  echo -e "${RED}Error: $PROMPT_FILE not found${NC}"
  if [ "$STREAM_MODE" = true ]; then
    echo "Please ensure the stream folder structure exists"
  else
    echo "Please create PROMPT.md or run /create-prd first"
  fi
  exit 1
fi

# Only require specs/prd.md in legacy mode or features stream
if [ "$STREAM_MODE" = false ] || [ "$STREAM" = "features" ]; then
  if [ ! -f "specs/prd.md" ]; then
    echo -e "${RED}Error: specs/prd.md not found${NC}"
    echo "Please create your PRD or run /create-prd first"
    exit 1
  fi
fi

# Determine activity file location
if [ "$STREAM_MODE" = true ]; then
  ACTIVITY_FILE="streams/${STREAM}/activity.md"
else
  ACTIVITY_FILE="logs/activity.md"
fi

if [ ! -f "$ACTIVITY_FILE" ]; then
  echo -e "${YELLOW}Warning: $ACTIVITY_FILE not found, creating it...${NC}"
  mkdir -p "$(dirname "$ACTIVITY_FILE")"
  if [ "$STREAM_MODE" = true ]; then
    cat > "$ACTIVITY_FILE" << EOF
# ${STREAM^} Stream Activity

## Session Log

<!-- Agent will append dated entries here -->
EOF
  else
    cat > "$ACTIVITY_FILE" << 'EOF'
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
fi

# Create screenshots and logs directories if they don't exist
mkdir -p screenshots
mkdir -p logs

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}   Ralph Wiggum Autonomous Loop${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
if [ "$STREAM_MODE" = true ]; then
  echo -e "Stream: ${GREEN}$STREAM${NC}"
fi
echo -e "Model: ${GREEN}$MODEL${NC}"
echo -e "Max iterations: ${GREEN}$MAX_ITERATIONS${NC}"
echo -e "Prompt file: ${GREEN}$PROMPT_FILE${NC}"
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

  # Clear previous iteration output (include stream name if in stream mode)
  if [ "$STREAM_MODE" = true ]; then
    LOG_FILE="logs/iteration_${STREAM}_$i.json"
  else
    LOG_FILE="logs/iteration_$i.json"
  fi
  rm -f "$LOG_FILE"

  # Run Claude with the prompt from the appropriate PROMPT file
  # Stream JSON output with verbose, parse for readable display
  # Use process substitution to capture PID while still piping
  exec 3< <(claude -p "$(cat "$PROMPT_FILE")" $MODEL_FLAG --output-format stream-json --verbose --dangerously-skip-permissions 2>&1)
  CLAUDE_PID=$!

  while IFS= read -r line <&3; do
    # Save raw JSON to file for completion check
    echo "$line" >> $LOG_FILE

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
  result=$(cat $LOG_FILE 2>/dev/null || echo "")
  echo ""

  # Check for completion signal - only in assistant text output (not file contents in tool results)
  # Extract just assistant text messages and check for COMPLETE there
  assistant_text=$(jq -r 'select(.type=="assistant") | .message.content[]? | select(.type=="text") | .text // empty' $LOG_FILE 2>/dev/null | tr '\n' ' ')
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
    if [ "$STREAM_MODE" = true ]; then
      echo "  2. Check streams/${STREAM}/activity.md for the stream log"
    else
      echo "  2. Check logs/activity.md for the full build log"
    fi
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
if [ "$STREAM_MODE" = true ]; then
  echo "  1. Run again with more iterations: ./ralph-streams.sh $STREAM 50"
  echo "  2. Check streams/${STREAM}/activity.md to see current progress"
  echo "  3. Check streams/${STREAM}/backlog.md to see remaining tasks"
else
  echo "  1. Run again with more iterations: ./ralph-streams.sh 50"
  echo "  2. Check logs/activity.md to see current progress"
  echo "  3. Check specs/prd.md to see remaining tasks"
fi
echo "  4. Manually complete remaining tasks"
echo ""
exit 1
