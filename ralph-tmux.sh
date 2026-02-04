#!/bin/bash
# ralph-tmux.sh - Tmux-based multi-stream orchestrator for RalphX
#
# Usage:
#   ./ralph-tmux.sh [command]
#
# Commands:
#   start [streams...]  - Create tmux session (all streams, or specified streams)
#   attach              - Attach to existing session
#   stop                - Stop all streams and kill session
#   restart [stream]    - Restart all streams (or single stream if specified)
#   status              - Show session status without attaching

set -e

SESSION_NAME="ralph"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

#------------------------------------------------------------------------------
# Utility Functions
#------------------------------------------------------------------------------

check_tmux() {
    if ! command -v tmux &> /dev/null; then
        echo -e "${RED}Error: tmux is not installed${NC}"
        echo "Install with: brew install tmux"
        exit 1
    fi
}

check_fswatch() {
    if ! command -v fswatch &> /dev/null; then
        echo -e "${RED}Error: fswatch is not installed${NC}"
        echo "Install with: brew install fswatch"
        exit 1
    fi
}

session_exists() {
    tmux has-session -t "$SESSION_NAME" 2>/dev/null
}

# Check if a stream should be started (either no filter or stream is in the filter list)
should_start_stream() {
    local stream="$1"
    shift
    local streams=("$@")

    # If no streams specified, start all
    if [ ${#streams[@]} -eq 0 ]; then
        return 0
    fi

    # Check if stream is in the list
    for s in "${streams[@]}"; do
        if [ "$s" = "$stream" ]; then
            return 0
        fi
    done
    return 1
}

#------------------------------------------------------------------------------
# Session Management
#------------------------------------------------------------------------------

create_session() {
    local streams=("$@")

    if session_exists; then
        echo -e "${YELLOW}Session '$SESSION_NAME' already exists${NC}"
        echo "Use: ./ralph-tmux.sh attach"
        exit 0
    fi

    # Validate stream names if provided
    if [ ${#streams[@]} -gt 0 ]; then
        for stream in "${streams[@]}"; do
            case "$stream" in
                features|refactor|polish|verify|hygiene|visual-qa) ;;
                *)
                    echo -e "${RED}Unknown stream: $stream${NC}"
                    echo "Valid streams: features, refactor, polish, verify, hygiene, visual-qa"
                    exit 1
                    ;;
            esac
        done
        echo -e "${GREEN}Creating RALPH session with streams: ${streams[*]}...${NC}"
    else
        echo -e "${GREEN}Creating RALPH multi-stream session...${NC}"
    fi

    cd "$SCRIPT_DIR"

    # Create new session with first pane (will be header - pane 0)
    tmux new-session -d -s "$SESSION_NAME" -x 200 -y 50

    # Configure session-wide settings
    tmux set-option -t "$SESSION_NAME" mouse on
    tmux set-option -t "$SESSION_NAME" history-limit 50000

    # Set base index to 0 for predictable pane numbering
    tmux set-option -t "$SESSION_NAME" pane-base-index 0

    # Bind Ctrl-b + number to switch panes AND zoom (select + toggle zoom)
    tmux bind-key -T prefix 0 select-pane -t 0 \; resize-pane -Z
    tmux bind-key -T prefix 1 select-pane -t 1 \; resize-pane -Z
    tmux bind-key -T prefix 2 select-pane -t 2 \; resize-pane -Z
    tmux bind-key -T prefix 3 select-pane -t 3 \; resize-pane -Z
    tmux bind-key -T prefix 4 select-pane -t 4 \; resize-pane -Z
    tmux bind-key -T prefix 5 select-pane -t 5 \; resize-pane -Z
    tmux bind-key -T prefix 6 select-pane -t 6 \; resize-pane -Z

    # Bind Ctrl-b S to graceful stop (creates stop signal file)
    tmux bind-key -T prefix S run-shell "touch $SCRIPT_DIR/.ralph-stop && tmux display-message 'Graceful stop initiated'"

    # Create the pane layout
    # Layout: STATUS header at top, FEATURES (60%) on left, 5 sonnet streams stacked on right
    #
    # ┌─────────────────────────────────────────────────────────────────┐
    # │ [0] STATUS (5% height)                                          │
    # ├─────────────────────────────────────┬───────────────────────────┤
    # │                                     │ [2] REFACTOR              │
    # │                                     ├───────────────────────────┤
    # │                                     │ [3] POLISH                │
    # │ [1] FEATURES (opus)                 ├───────────────────────────┤
    # │ 60% width                           │ [4] VERIFY                │
    # │                                     ├───────────────────────────┤
    # │                                     │ [5] HYGIENE               │
    # │                                     ├───────────────────────────┤
    # │                                     │ [6] VISUAL-QA             │
    # └─────────────────────────────────────┴───────────────────────────┘

    # Split: header (5%) on top, main area (95%) below
    # Pane 0 = STATUS, Pane 1 = main area
    tmux split-window -t "$SESSION_NAME:0.0" -v -p 95

    # Split main area: FEATURES (60%) left, right column (40%)
    # Pane 1 = FEATURES, Pane 2 = right column
    tmux split-window -t "$SESSION_NAME:0.1" -h -p 40

    # Split right column into 5 equal parts for sonnet streams
    # Pane 2 = REFACTOR (top), Pane 3 = bottom 80%
    tmux split-window -t "$SESSION_NAME:0.2" -v -p 80

    # Pane 3 = POLISH, Pane 4 = bottom 75%
    tmux split-window -t "$SESSION_NAME:0.3" -v -p 75

    # Pane 4 = VERIFY, Pane 5 = bottom 66%
    tmux split-window -t "$SESSION_NAME:0.4" -v -p 66

    # Pane 5 = HYGIENE, Pane 6 = VISUAL-QA (bottom 50%)
    tmux split-window -t "$SESSION_NAME:0.5" -v -p 50

    # Set pane titles
    tmux select-pane -t "$SESSION_NAME:0.0" -T "STATUS"
    tmux select-pane -t "$SESSION_NAME:0.1" -T "FEATURES"
    tmux select-pane -t "$SESSION_NAME:0.2" -T "REFACTOR"
    tmux select-pane -t "$SESSION_NAME:0.3" -T "POLISH"
    tmux select-pane -t "$SESSION_NAME:0.4" -T "VERIFY"
    tmux select-pane -t "$SESSION_NAME:0.5" -T "HYGIENE"
    tmux select-pane -t "$SESSION_NAME:0.6" -T "VISUAL-QA"

    # Enable pane titles in status bar
    tmux set-option -t "$SESSION_NAME" pane-border-status top
    tmux set-option -t "$SESSION_NAME" pane-border-format " #{pane_title} "

    # Start commands in each pane (conditionally based on streams list)
    tmux send-keys -t "$SESSION_NAME:0.0" "./ralph-tmux-status.sh" C-m

    local guard_env="CLAUDE_MAX_PROCS=\${CLAUDE_MAX_PROCS:-6} CLAUDE_GUARD_MODE=\${CLAUDE_GUARD_MODE:-block}"

    if should_start_stream "features" "${streams[@]}"; then
        tmux send-keys -t "$SESSION_NAME:0.1" "$guard_env ./scripts/stream-watch-features.sh" C-m
    fi
    if should_start_stream "refactor" "${streams[@]}"; then
        tmux send-keys -t "$SESSION_NAME:0.2" "$guard_env ./scripts/stream-watch-refactor.sh" C-m
    fi
    if should_start_stream "polish" "${streams[@]}"; then
        tmux send-keys -t "$SESSION_NAME:0.3" "$guard_env ./scripts/stream-watch-polish.sh" C-m
    fi
    if should_start_stream "verify" "${streams[@]}"; then
        tmux send-keys -t "$SESSION_NAME:0.4" "$guard_env ./scripts/stream-watch-verify.sh" C-m
    fi
    if should_start_stream "hygiene" "${streams[@]}"; then
        tmux send-keys -t "$SESSION_NAME:0.5" "$guard_env ./scripts/stream-watch-hygiene.sh" C-m
    fi
    if should_start_stream "visual-qa" "${streams[@]}"; then
        tmux send-keys -t "$SESSION_NAME:0.6" "$guard_env ./scripts/stream-watch-visual-qa.sh" C-m
    fi

    # Select the appropriate pane as default (first specified stream, or features)
    local selected_pane="1"  # Default to features
    if [ ${#streams[@]} -gt 0 ]; then
        case "${streams[0]}" in
            features)  selected_pane="1" ;;
            refactor)  selected_pane="2" ;;
            polish)    selected_pane="3" ;;
            verify)    selected_pane="4" ;;
            hygiene)   selected_pane="5" ;;
            visual-qa) selected_pane="6" ;;
        esac
    fi
    tmux select-pane -t "$SESSION_NAME:0.$selected_pane"

    if [ ${#streams[@]} -gt 0 ]; then
        echo -e "${GREEN}Session created - started streams: ${streams[*]}${NC}"
    else
        echo -e "${GREEN}Session created with all 6 streams${NC}"
    fi
    echo ""
    echo "Pane layout:"
    echo "  [0] STATUS    - Header (keybindings)"
    echo "  [1] FEATURES  - PRD + P0 fixes (opus, 60% width)$(should_start_stream "features" "${streams[@]}" || echo " [not started]")"
    echo "  [2] REFACTOR  - P1 file splits (sonnet)$(should_start_stream "refactor" "${streams[@]}" || echo " [not started]")"
    echo "  [3] POLISH    - P2/P3 cleanup (sonnet)$(should_start_stream "polish" "${streams[@]}" || echo " [not started]")"
    echo "  [4] VERIFY    - Gap detection (sonnet)$(should_start_stream "verify" "${streams[@]}" || echo " [not started]")"
    echo "  [5] HYGIENE   - Backlog maintenance (sonnet)$(should_start_stream "hygiene" "${streams[@]}" || echo " [not started]")"
    echo "  [6] VISUAL-QA - Playwright tests (sonnet)$(should_start_stream "visual-qa" "${streams[@]}" || echo " [not started]")"
    echo ""
    echo "Ctrl+b <0-6> to switch+zoom, Ctrl+b z to unzoom"
    echo ""
    echo "Attaching to session..."

    tmux attach-session -t "$SESSION_NAME"
}

attach_session() {
    if ! session_exists; then
        echo -e "${YELLOW}Session '$SESSION_NAME' does not exist${NC}"
        echo "Use: ./ralph-tmux.sh start"
        exit 1
    fi

    echo -e "${GREEN}Attaching to session '$SESSION_NAME'...${NC}"
    tmux attach-session -t "$SESSION_NAME"
}

stop_all() {
    if ! session_exists; then
        echo -e "${YELLOW}Session '$SESSION_NAME' is not running${NC}"
        # Clean up stop signal if it exists
        rm -f "$SCRIPT_DIR/.ralph-stop"
        exit 0
    fi

    echo -e "${YELLOW}Stopping RALPH multi-stream session...${NC}"

    # Clean up stop signal file
    rm -f "$SCRIPT_DIR/.ralph-stop"

    # Send Ctrl+C to each pane to stop running processes
    for pane in 0 1 2 3 4 5 6; do
        tmux send-keys -t "$SESSION_NAME:0.$pane" C-c 2>/dev/null || true
    done

    # Give processes time to clean up
    sleep 1

    # Kill any remaining fswatch processes from stream watchers
    # Pattern matches fswatch command (not fswatch-something) with streams/ or specs/ paths
    # Uses word boundary: fswatch preceded by start, space, or path separator
    pkill -f "(^|[/ ])fswatch .*(streams/|specs/)" 2>/dev/null || true

    # Kill any leftover Claude tool helpers from this repo (stale /private/tmp tasks)
    pkill -f "/private/tmp/claude-.*/-Users-lazabogdan-Code-ralphx/tasks" 2>/dev/null || true

    # Kill the tmux session
    tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true

    echo -e "${GREEN}Session stopped${NC}"
}

restart_all() {
    local stream="$1"

    if [ -n "$stream" ]; then
        restart_stream "$stream"
    else
        echo -e "${YELLOW}Restarting all streams...${NC}"
        stop_all
        sleep 1
        create_session
    fi
}

restart_stream() {
    local stream="$1"

    if ! session_exists; then
        echo -e "${YELLOW}Session '$SESSION_NAME' is not running${NC}"
        exit 1
    fi

    local pane=""
    local script=""

    case "$stream" in
        features)
            pane="1"
            script="./scripts/stream-watch-features.sh"
            ;;
        refactor)
            pane="2"
            script="./scripts/stream-watch-refactor.sh"
            ;;
        polish)
            pane="3"
            script="./scripts/stream-watch-polish.sh"
            ;;
        verify)
            pane="4"
            script="./scripts/stream-watch-verify.sh"
            ;;
        hygiene)
            pane="5"
            script="./scripts/stream-watch-hygiene.sh"
            ;;
        visual-qa)
            pane="6"
            script="./scripts/stream-watch-visual-qa.sh"
            ;;
        status)
            pane="0"
            script="./ralph-tmux-status.sh"
            ;;
        *)
            echo -e "${RED}Unknown stream: $stream${NC}"
            echo "Valid streams: features, refactor, polish, verify, hygiene, visual-qa, status"
            exit 1
            ;;
    esac

    echo -e "${YELLOW}Restarting $stream stream (pane $pane)...${NC}"

    # Send Ctrl+C to stop current process
    tmux send-keys -t "$SESSION_NAME:0.$pane" C-c
    sleep 1

    # Start the script
    tmux send-keys -t "$SESSION_NAME:0.$pane" "$script" C-m

    echo -e "${GREEN}Stream $stream restarted${NC}"
}

show_status() {
    if ! session_exists; then
        echo -e "${YELLOW}Session '$SESSION_NAME' is NOT RUNNING${NC}"
        echo ""
        echo "Start with: ./ralph-tmux.sh start"
        exit 0
    fi

    echo -e "${GREEN}Session '$SESSION_NAME' is RUNNING${NC}"
    echo ""

    # Show pane info
    echo "Panes:"
    tmux list-panes -t "$SESSION_NAME" -F "  [#{pane_index}] #{pane_title} - #{pane_current_command}"

    echo ""
    echo "Commands:"
    echo "  ./ralph-tmux.sh attach         - Attach to session"
    echo "  ./ralph-tmux.sh stop           - Stop all streams immediately"
    echo "  ./ralph-tmux.sh graceful-stop  - Stop after current tasks complete"
    echo "  ./ralph-tmux.sh restart        - Restart all streams"
    echo "  ./ralph-tmux.sh restart <name> - Restart single stream"
    echo ""
    echo "Keybinding: Ctrl+b S = graceful stop"
}

graceful_stop() {
    if ! session_exists; then
        echo -e "${YELLOW}Session '$SESSION_NAME' is not running${NC}"
        exit 0
    fi

    echo -e "${YELLOW}Initiating graceful stop...${NC}"
    echo -e "${DIM}Streams will stop after completing current tasks${NC}"
    echo ""

    # Create the stop signal file
    touch "$SCRIPT_DIR/.ralph-stop"

    # Wait for streams to finish (check every 5 seconds, timeout after 5 minutes)
    local timeout=300
    local elapsed=0
    local interval=5

    while [ $elapsed -lt $timeout ]; do
        # Check if any stream panes are still running claude
        local running=0
        for pane in 1 2 3 4 5 6; do
            local cmd=$(tmux display-message -t "$SESSION_NAME:0.$pane" -p '#{pane_current_command}' 2>/dev/null || echo "")
            if [[ "$cmd" == *"claude"* ]] || [[ "$cmd" == *"ralph-streams"* ]]; then
                running=$((running + 1))
            fi
        done

        if [ $running -eq 0 ]; then
            echo -e "${GREEN}All streams idle${NC}"
            break
        fi

        echo -e "${DIM}Waiting... $running stream(s) still active${NC}"
        sleep $interval
        elapsed=$((elapsed + interval))
    done

    # Clean up stop signal
    rm -f "$SCRIPT_DIR/.ralph-stop"

    # Now do the full stop
    echo ""
    stop_all
}

#------------------------------------------------------------------------------
# Main
#------------------------------------------------------------------------------

check_tmux
check_fswatch

command="${1:-start}"
shift 2>/dev/null || true

case "$command" in
    start)
        create_session "$@"
        ;;
    attach)
        attach_session
        ;;
    stop)
        stop_all
        ;;
    graceful-stop)
        graceful_stop
        ;;
    restart)
        restart_all "$1"
        ;;
    status)
        show_status
        ;;
    *)
        echo "Usage: ./ralph-tmux.sh [command] [streams...]"
        echo ""
        echo "Commands:"
        echo "  start [streams...]  - Create tmux session (all streams, or specified streams)"
        echo "  attach              - Attach to existing session"
        echo "  stop                - Stop all streams immediately"
        echo "  graceful-stop       - Wait for current tasks to complete, then stop"
        echo "  restart [stream]    - Restart all streams (or single stream)"
        echo "  status              - Show session status without attaching"
        echo ""
        echo "Streams: features, refactor, polish, verify, hygiene, visual-qa"
        echo ""
        echo "Examples:"
        echo "  ./ralph-tmux.sh start                        # Start all streams"
        echo "  ./ralph-tmux.sh start features               # Start only features stream"
        echo "  ./ralph-tmux.sh start features refactor      # Start features and refactor"
        echo "  ./ralph-tmux.sh start polish verify hygiene  # Start multiple streams"
        echo "  ./ralph-tmux.sh restart polish               # Restart only polish stream"
        echo ""
        echo "Keybinding: Ctrl+b S = graceful stop (from within tmux)"
        exit 1
        ;;
esac
