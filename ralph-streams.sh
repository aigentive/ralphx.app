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

# Track claude PID for cleanup
CLAUDE_PID=""

# Cleanup function to kill child processes for THIS stream only
cleanup() {
  echo ""
  echo -e "  ${YELLOW}Cleaning up...${NC}"

  # Kill the tracked claude process first
  if [ -n "$CLAUDE_PID" ] && kill -0 "$CLAUDE_PID" 2>/dev/null; then
    kill -9 "$CLAUDE_PID" 2>/dev/null || true
  fi

  # Kill all child processes of this script
  pkill -9 -P $$ 2>/dev/null || true

  echo -e "  ${YELLOW}Cleanup complete.${NC}"
}

# Set up trap for cleanup on exit/interrupt
trap cleanup EXIT INT TERM

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
DIM='\033[2m'
DARK_GRAY='\033[90m'
NC='\033[0m' # No Color

# Padding for visual spacing
PAD="  "

# Valid streams
VALID_STREAMS="features|refactor|polish|verify|hygiene|visual-qa"

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

echo ""
if [ "$STREAM_MODE" = true ]; then
  echo -e "${PAD}${CYAN}Stream:${NC} ${GREEN}$STREAM${NC}"
fi
echo -e "${PAD}${CYAN}Model:${NC} $MODEL"
echo -e "${PAD}${CYAN}Max iterations:${NC} $MAX_ITERATIONS"
echo -e "${PAD}${CYAN}Prompt:${NC} ${DIM}$PROMPT_FILE${NC}"
echo ""
echo -e "${PAD}${YELLOW}Starting in 3 seconds...${NC} ${DIM}Press Ctrl+C to abort${NC}"
sleep 3

# Determine stream prefix for output
if [ "$STREAM_MODE" = true ]; then
  STREAM_PREFIX="[$STREAM] "
else
  STREAM_PREFIX=""
fi

# Graceful stop signal file
STOP_SIGNAL_FILE=".ralph-stop"

# Main loop
for ((i=1; i<=MAX_ITERATIONS; i++)); do
  # Check for graceful stop signal before starting iteration
  if [ -f "$STOP_SIGNAL_FILE" ]; then
    echo ""
    echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
    echo -e "${PAD}${YELLOW}■ STOPPED${NC} ${DIM}graceful stop requested${NC}"
    echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
    echo ""
    exit 0
  fi

  echo ""
  echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
  echo -e "${PAD}${CYAN}${STREAM_PREFIX}${NC}${GREEN}Iteration $i${NC} ${DIM}of $MAX_ITERATIONS${NC}"
  echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
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
  # Use a temp file to capture PID since process substitution runs in subshell
  CLAUDE_FIFO=$(mktemp -u)
  mkfifo "$CLAUDE_FIFO"
  claude -p "$(cat "$PROMPT_FILE")" $MODEL_FLAG --output-format stream-json --verbose --dangerously-skip-permissions 2>&1 > "$CLAUDE_FIFO" &
  CLAUDE_PID=$!
  exec 3< "$CLAUDE_FIFO"
  rm -f "$CLAUDE_FIFO"

  # State tracking for clean output formatting
  last_output_type=""  # "text", "tool", "result"

  while IFS= read -r line <&3; do
    # Save raw JSON to file for completion check
    echo "$line" >> $LOG_FILE

    # Parse and display readable content
    type=$(echo "$line" | jq -r '.type // empty' 2>/dev/null)

    if [[ "$type" == "assistant" ]]; then
      # Extract text content from assistant messages
      text=$(echo "$line" | jq -r '.message.content[]? | select(.type=="text") | .text // empty' 2>/dev/null)
      if [[ -n "$text" ]]; then
        # Add spacing when transitioning from tools to text
        if [[ "$last_output_type" == "tool" || "$last_output_type" == "result" ]]; then
          echo ""
        fi
        echo -e "${PAD}${GREEN}$text${NC}"
        echo ""  # Always add newline after text
        last_output_type="text"
      fi

      # Show tool calls (cyan, medium visibility)
      tool_name=$(echo "$line" | jq -r '.message.content[]? | select(.type=="tool_use") | .name // empty' 2>/dev/null)
      if [[ -n "$tool_name" ]]; then
        # Add spacing when transitioning from text to tools
        if [[ "$last_output_type" == "text" ]]; then
          : # text already added newline
        fi
        tool_input=$(echo "$line" | jq -r '.message.content[]? | select(.type=="tool_use") | .input | to_entries | map("\(.key)=\(.value | tostring | .[0:40])") | join(" ") | .[0:70]' 2>/dev/null)
        echo -e "${PAD}  ${CYAN}▸ ${tool_name}${NC} ${DARK_GRAY}${tool_input}${NC}"
        last_output_type="tool"
      fi
    elif [[ "$type" == "user" ]]; then
      # Show tool results briefly (very dim)
      tool_result=$(echo "$line" | jq -r '.message.content[]? | select(.type=="tool_result") | .content // empty' 2>/dev/null | head -c 60 | tr '\n' ' ' | tr -s ' ')
      if [[ -n "$tool_result" ]]; then
        echo -e "${PAD}    ${DIM}${DARK_GRAY}└─ ${tool_result:0:55}…${NC}"
        last_output_type="result"
      fi
    fi
  done || true
  exec 3<&-  # Close file descriptor
  result=$(cat $LOG_FILE 2>/dev/null || echo "")
  echo ""

  # Check for completion signals - only in the LAST assistant text message
  # This prevents false positives when Claude quotes/discusses the signal format
  # Get the last assistant message's text content only
  last_assistant_text=$(jq -s '[.[] | select(.type=="assistant")] | last | .message.content[]? | select(.type=="text") | .text // empty' $LOG_FILE 2>/dev/null)

  if [[ "$last_assistant_text" == *"<promise>COMPLETE</promise>"* ]]; then
    echo ""
    echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
    echo -e "${PAD}${GREEN}✓ COMPLETE${NC} ${DIM}after $i iteration(s)${NC}"
    echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
    echo ""
    exit 0
  fi

  if [[ "$last_assistant_text" == *"<promise>IDLE</promise>"* ]]; then
    echo ""
    echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
    echo -e "${PAD}${YELLOW}◆ IDLE${NC} ${DIM}no work available · watching for changes${NC}"
    echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
    echo ""
    exit 0
  fi

  echo ""
  echo -e "${PAD}${DIM}── end of iteration $i ──${NC}"

  # Small delay between iterations to prevent hammering
  sleep 2
done

echo ""
echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
echo -e "${PAD}${RED}✗ MAX ITERATIONS${NC} ${DIM}reached $MAX_ITERATIONS without completion${NC}"
echo -e "${PAD}${DIM}─────────────────────────────────────────────────────────────────${NC}"
echo ""
exit 1
