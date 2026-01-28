#!/bin/bash
# ralph-tmux.sh - Tmux-based multi-stream orchestrator for RalphX
#
# Usage:
#   ./ralph-tmux.sh [command]
#
# Commands:
#   start   - Create tmux session with all streams (default)
#   attach  - Attach to existing session
#   stop    - Stop all streams and kill session
#   restart - Restart all streams (or single stream with: restart <stream>)
#   status  - Show session status without attaching

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

#------------------------------------------------------------------------------
# Session Management
#------------------------------------------------------------------------------

create_session() {
    if session_exists; then
        echo -e "${YELLOW}Session '$SESSION_NAME' already exists${NC}"
        echo "Use: ./ralph-tmux.sh attach"
        exit 0
    fi

    echo -e "${GREEN}Creating RALPH multi-stream session...${NC}"

    cd "$SCRIPT_DIR"

    # Create new session with first pane (will be header - pane 0)
    tmux new-session -d -s "$SESSION_NAME" -x 200 -y 50

    # Configure session-wide settings
    tmux set-option -t "$SESSION_NAME" mouse on
    tmux set-option -t "$SESSION_NAME" history-limit 50000

    # Set base index to 0 for predictable pane numbering
    tmux set-option -t "$SESSION_NAME" pane-base-index 0

    # Bind Ctrl-b + number to switch panes directly (override default window switching)
    tmux bind-key 0 select-pane -t "$SESSION_NAME:0.0"
    tmux bind-key 1 select-pane -t "$SESSION_NAME:0.1"
    tmux bind-key 2 select-pane -t "$SESSION_NAME:0.2"
    tmux bind-key 3 select-pane -t "$SESSION_NAME:0.3"
    tmux bind-key 4 select-pane -t "$SESSION_NAME:0.4"
    tmux bind-key 5 select-pane -t "$SESSION_NAME:0.5"

    # Create the pane layout
    # Start with pane 0 (header) at top

    # Split horizontally to create header (top) and main area (bottom)
    # Pane 0 = header (small), Pane 1 = main
    tmux split-window -t "$SESSION_NAME:0.0" -v -p 90

    # Now in main area (pane 1), split vertically for features/refactor
    # Pane 1 = features (left), Pane 2 = refactor (right)
    tmux split-window -t "$SESSION_NAME:0.1" -h -p 50

    # Split features pane (1) horizontally to create bottom row
    # Pane 1 = features, Pane 3 = bottom-left area
    tmux split-window -t "$SESSION_NAME:0.1" -v -p 40

    # Split refactor pane (2) horizontally for bottom-right
    # Pane 2 = refactor, Pane 4 = bottom-right area
    tmux split-window -t "$SESSION_NAME:0.2" -v -p 40

    # Split bottom-left (3) into polish and verify
    # Pane 3 = polish, Pane 5 = verify
    tmux split-window -t "$SESSION_NAME:0.3" -h -p 50

    # Split bottom-right (4) - verify is already 5, need hygiene
    # Wait, let me recalculate after splits...
    # After all splits we should have panes: 0(header), 1(features), 2(refactor), 3(polish), 4(?), 5(?)
    # Let's verify by setting titles

    # Set pane titles
    tmux select-pane -t "$SESSION_NAME:0.0" -T "STATUS"
    tmux select-pane -t "$SESSION_NAME:0.1" -T "FEATURES"
    tmux select-pane -t "$SESSION_NAME:0.2" -T "REFACTOR"
    tmux select-pane -t "$SESSION_NAME:0.3" -T "POLISH"
    tmux select-pane -t "$SESSION_NAME:0.4" -T "VERIFY"
    tmux select-pane -t "$SESSION_NAME:0.5" -T "HYGIENE"

    # Enable pane titles in status bar
    tmux set-option -t "$SESSION_NAME" pane-border-status top
    tmux set-option -t "$SESSION_NAME" pane-border-format " #{pane_title} "

    # Start commands in each pane
    tmux send-keys -t "$SESSION_NAME:0.0" "./ralph-tmux-status.sh" C-m
    tmux send-keys -t "$SESSION_NAME:0.1" "./scripts/stream-watch-features.sh" C-m
    tmux send-keys -t "$SESSION_NAME:0.2" "./scripts/stream-watch-refactor.sh" C-m
    tmux send-keys -t "$SESSION_NAME:0.3" "./scripts/stream-watch-polish.sh" C-m
    tmux send-keys -t "$SESSION_NAME:0.4" "./scripts/stream-watch-verify.sh" C-m
    tmux send-keys -t "$SESSION_NAME:0.5" "./scripts/stream-watch-hygiene.sh" C-m

    # Select features pane as default
    tmux select-pane -t "$SESSION_NAME:0.1"

    echo -e "${GREEN}Session created with 6 panes${NC}"
    echo ""
    echo "Pane layout:"
    echo "  [0] STATUS  - Header with uptime and backlog counts"
    echo "  [1] FEATURES - PRD tasks + P0 fixes (opus)"
    echo "  [2] REFACTOR - P1 large file splits (sonnet)"
    echo "  [3] POLISH   - P2/P3 cleanup (sonnet)"
    echo "  [4] VERIFY   - Gap detection (sonnet)"
    echo "  [5] HYGIENE  - Backlog maintenance (sonnet)"
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
        exit 0
    fi

    echo -e "${YELLOW}Stopping RALPH multi-stream session...${NC}"

    # Send Ctrl+C to each pane to stop running processes
    for pane in 0 1 2 3 4 5; do
        tmux send-keys -t "$SESSION_NAME:0.$pane" C-c 2>/dev/null || true
    done

    # Give processes time to clean up
    sleep 1

    # Kill any remaining fswatch processes from stream watchers
    # Pattern matches both streams/ paths and specs/ paths (verify stream)
    pkill -f "fswatch.*(streams/|specs/)" 2>/dev/null || true

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
        status)
            pane="0"
            script="./ralph-tmux-status.sh"
            ;;
        *)
            echo -e "${RED}Unknown stream: $stream${NC}"
            echo "Valid streams: features, refactor, polish, verify, hygiene, status"
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
    echo "  ./ralph-tmux.sh stop           - Stop all streams"
    echo "  ./ralph-tmux.sh restart        - Restart all streams"
    echo "  ./ralph-tmux.sh restart <name> - Restart single stream"
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
        create_session
        ;;
    attach)
        attach_session
        ;;
    stop)
        stop_all
        ;;
    restart)
        restart_all "$1"
        ;;
    status)
        show_status
        ;;
    *)
        echo "Usage: ./ralph-tmux.sh [start|attach|stop|restart|status]"
        echo ""
        echo "Commands:"
        echo "  start   - Create tmux session with all streams (default)"
        echo "  attach  - Attach to existing session"
        echo "  stop    - Stop all streams and kill session"
        echo "  restart - Restart all streams (or single stream with: restart <stream>)"
        echo "  status  - Show session status without attaching"
        exit 1
        ;;
esac
