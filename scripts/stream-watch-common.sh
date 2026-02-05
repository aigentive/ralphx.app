#!/bin/bash
# stream-watch-common.sh - Shared functions for stream watchers
#
# Source this file from stream-watch-*.sh scripts:
#   source "$(dirname "$0")/stream-watch-common.sh"
#
# Required variables (set before sourcing):
#   STREAM        - Stream name (e.g., "features", "refactor")
#   MODEL         - Model to use (e.g., "opus", "sonnet")
#   WATCH_FILES   - Array of files to watch
#
# Optional variables:
#   COOLDOWN_SECONDS - Seconds to ignore triggers after cycle (default: 5)
#   FSWATCH_LATENCY  - Seconds to debounce file changes (default: 3)

# Ensure required variables are set
: "${STREAM:?STREAM must be set before sourcing stream-watch-common.sh}"
: "${MODEL:?MODEL must be set before sourcing stream-watch-common.sh}"
: "${WATCH_FILES:?WATCH_FILES must be set before sourcing stream-watch-common.sh}"

# Defaults
COOLDOWN_SECONDS="${COOLDOWN_SECONDS:-5}"
FSWATCH_LATENCY="${FSWATCH_LATENCY:-3}"
CLAUDE_CLEANUP_MODE="${CLAUDE_CLEANUP_MODE:-stale}" # off | stale | all
CLAUDE_CLEANUP_MIN_AGE="${CLAUDE_CLEANUP_MIN_AGE:-300}" # seconds

# Project root (scripts/..)
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Derived paths (absolute to avoid cwd mismatches)
LOCK_FILE="$ROOT_DIR/.stream-${STREAM}-lock"
WATCHER_LOCK_FILE="$ROOT_DIR/.stream-${STREAM}-watcher-lock"
MTIME_FILE="$ROOT_DIR/.stream-${STREAM}-mtimes"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
DIM='\033[2m'
NC='\033[0m' # No Color

# Padding for visual spacing
PAD="  "

#------------------------------------------------------------------------------
# Utility Functions
#------------------------------------------------------------------------------

# Check if a PID is still running
is_pid_alive() {
    kill -0 "$1" 2>/dev/null
}

# Find Claude-run shell PIDs for this repo
list_claude_shell_pids() {
    local etimes_test
    etimes_test="$(ps -o etimes= -p $$ 2>/dev/null | awk 'NF{print; exit}')"
    if [[ -n "$etimes_test" ]]; then
        ps -A -o pid= -o etimes= -o command= | awk \
            -v root="$ROOT_DIR" \
            -v mode="$CLAUDE_CLEANUP_MODE" \
            -v min_age="$CLAUDE_CLEANUP_MIN_AGE" \
            '
            $0 ~ /(\/\.claude\/shell-snapshots\/|\/var\/folders\/.*\/claude-.*-cwd)/ && $0 ~ root {
                pid=$1; age=$2;
                if (mode == "all" || (mode == "stale" && age >= min_age)) {
                    print pid
                }
            }'
    else
        ps -A -o pid= -o etime= -o command= | awk \
            -v root="$ROOT_DIR" \
            -v mode="$CLAUDE_CLEANUP_MODE" \
            -v min_age="$CLAUDE_CLEANUP_MIN_AGE" \
            '
            function etime_to_seconds(et,    a,b,n,days,hours,mins,secs,total) {
                n=split(et,a,":");
                days=0; hours=0; mins=0; secs=0;
                if (n==3) { hours=a[1]; mins=a[2]; secs=a[3]; }
                else if (n==2) { mins=a[1]; secs=a[2]; }
                else { secs=a[1]; }
                if (hours ~ /-/) { split(hours,b,"-"); days=b[1]; hours=b[2]; }
                total=(((days*24)+hours)*60+mins)*60+secs;
                return total;
            }
            $0 ~ /(\/\.claude\/shell-snapshots\/|\/var\/folders\/.*\/claude-.*-cwd)/ && $0 ~ root {
                pid=$1; age=etime_to_seconds($2);
                if (mode == "all" || (mode == "stale" && age >= min_age)) {
                    print pid
                }
            }'
    fi
}

# Kill stale Claude-run shell processes to prevent runaway load
cleanup_claude_shells() {
    if [ "$CLAUDE_CLEANUP_MODE" = "off" ]; then
        return 0
    fi
    local pids=()
    while IFS= read -r pid; do
        [[ -n "$pid" ]] && pids+=("$pid")
    done < <(list_claude_shell_pids)

    if (( ${#pids[@]} == 0 )); then
        return 0
    fi

    echo -e "${PAD}${YELLOW}[$STREAM] Cleaning stale Claude shell processes: ${#pids[@]}${NC}"
    kill -TERM "${pids[@]}" 2>/dev/null || true
    sleep 0.5
    for pid in "${pids[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill -KILL "$pid" 2>/dev/null || true
        fi
    done
}

# Get file/directory mtime as epoch seconds (macOS compatible)
# For directories, returns the most recent mtime of any file within
get_mtime() {
    local path="$1"
    if [ -f "$path" ]; then
        stat -f %m "$path" 2>/dev/null || echo "0"
    elif [ -d "$path" ]; then
        # For directories, get the max mtime of files within (1 level deep)
        find "$path" -maxdepth 1 -type f -exec stat -f %m {} \; 2>/dev/null | sort -rn | head -1 || echo "0"
    else
        echo "0"
    fi
}

# Snapshot current mtimes of watched files
snapshot_mtimes() {
    for file in "${WATCH_FILES[@]}"; do
        echo "$file:$(get_mtime "$file")"
    done > "$MTIME_FILE"
}

# Check which files changed since last snapshot
# Returns: comma-separated list of changed files, or "none-detected"
get_changed_files() {
    local changed=""
    if [ ! -f "$MTIME_FILE" ]; then
        echo "no-previous-snapshot"
        return
    fi

    for file in "${WATCH_FILES[@]}"; do
        local current_mtime=$(get_mtime "$file")
        local saved_mtime=$(grep "^$file:" "$MTIME_FILE" 2>/dev/null | cut -d: -f2)
        saved_mtime=${saved_mtime:-0}

        if [ "$current_mtime" != "$saved_mtime" ]; then
            if [ -n "$changed" ]; then
                changed="$changed, "
            fi
            changed="${changed}$(basename "$file")"
        fi
    done

    if [ -z "$changed" ]; then
        echo "none-detected"
    else
        echo "$changed"
    fi
}

#------------------------------------------------------------------------------
# Cleanup
#------------------------------------------------------------------------------

cleanup() {
    echo ""
    echo -e "${PAD}${YELLOW}[$STREAM] Shutting down...${NC}"
    # Remove lock files and mtime file
    rm -f "$LOCK_FILE" "$WATCHER_LOCK_FILE" "$MTIME_FILE"
    # Kill ralph-streams processes for this stream first (they have their own cleanup)
    pkill -INT -f "ralph-streams.sh $STREAM" 2>/dev/null || true
    sleep 0.5
    # Force kill if still running
    pkill -9 -f "ralph-streams.sh $STREAM" 2>/dev/null || true
    # Kill any remaining descendants of this watcher
    pkill -9 -P $$ 2>/dev/null || true
    # Safety net: kill claude processes tied to this stream's prompt
    pkill -9 -f "claude.*streams/${STREAM}/PROMPT.md" 2>/dev/null || true
    exit 0
}

#------------------------------------------------------------------------------
# Cycle Management
#------------------------------------------------------------------------------

# Run a cycle with lock protection
# Usage: run_cycle <trigger> [trigger_file]
run_cycle() {
    local trigger="$1"
    local trigger_file="${2:-}"

    # Atomic lock acquisition using noclobber
    # This prevents TOCTOU race between check and acquire
    if ! ( set -o noclobber; echo $$ > "$LOCK_FILE" ) 2>/dev/null; then
        echo -e "${PAD}${BLUE}[$STREAM] Already running, skipping trigger from $trigger${NC}"
        return 0
    fi

    # For file change triggers, verify something actually changed
    if [ "$trigger" = "file change" ]; then
        local changed=$(get_changed_files)
        if [ "$changed" = "none-detected" ]; then
            echo -e "${PAD}${DIM}[$STREAM] fswatch fired but no mtime changes detected - ignoring${NC}"
            rm -f "$LOCK_FILE"
            return 0
        fi
        trigger_file="$changed"
    fi

    # Optional guard to cap concurrent Claude processes
    cleanup_claude_shells
    if [ -n "${CLAUDE_MAX_PROCS:-}" ] && [ -x "$ROOT_DIR/scripts/claude-process-guard.sh" ]; then
        if ! "$ROOT_DIR/scripts/claude-process-guard.sh" --max "$CLAUDE_MAX_PROCS" --mode "${CLAUDE_GUARD_MODE:-block}" --ancestor-match "ralph-streams.sh $STREAM"; then
            echo -e "${PAD}${YELLOW}[$STREAM] Claude guard blocked cycle (max=${CLAUDE_MAX_PROCS})${NC}"
            rm -f "$LOCK_FILE"
            return 0
        fi
    fi

    # Print banner
    echo ""
    echo -e "${PAD}${CYAN}┌─────────────────────────────────────────────────────────────────${NC}"
    echo -e "${PAD}${CYAN}│${NC} ${YELLOW}[$STREAM] Starting cycle${NC}"
    echo -e "${PAD}${CYAN}│${NC} ${DIM}Trigger:${NC} $trigger"
    if [ -n "$trigger_file" ]; then
        echo -e "${PAD}${CYAN}│${NC} ${DIM}Changed:${NC} $trigger_file"
    fi
    echo -e "${PAD}${CYAN}│${NC} ${DIM}Time:${NC}    $(date '+%Y-%m-%d %H:%M:%S')"
    echo -e "${PAD}${CYAN}└─────────────────────────────────────────────────────────────────${NC}"
    echo ""

    # Snapshot mtimes BEFORE running (to detect changes made during cycle)
    snapshot_mtimes

    # Run the stream
    ANTHROPIC_MODEL=$MODEL ./ralph-streams.sh $STREAM 50 </dev/null
    local exit_code=$?

    # Snapshot mtimes AFTER running (reset baseline)
    snapshot_mtimes

    # Release lock
    rm -f "$LOCK_FILE"

    echo ""
    echo -e "${PAD}${GREEN}[$STREAM] IDLE - watching for file changes...${NC}"
    echo -e "${PAD}${DIM}Watched: ${WATCH_FILES[*]}${NC}"
}

#------------------------------------------------------------------------------
# Main Loop Setup
#------------------------------------------------------------------------------

# Acquire a watcher-wide lock so only one watcher runs per stream
acquire_watcher_lock() {
    if ! ( set -o noclobber; echo $$ > "$WATCHER_LOCK_FILE" ) 2>/dev/null; then
        local old_pid=$(cat "$WATCHER_LOCK_FILE" 2>/dev/null)
        if [ -n "$old_pid" ] && is_pid_alive "$old_pid"; then
            echo -e "${PAD}${RED}[$STREAM] Watcher already running (PID $old_pid)${NC}"
            echo -e "${PAD}${RED}[$STREAM] Stop it first to avoid duplicate cycles${NC}"
            exit 1
        else
            echo -e "${PAD}${YELLOW}[$STREAM] Removing stale watcher lock (PID $old_pid is dead)${NC}"
            rm -f "$WATCHER_LOCK_FILE"
            if ! ( set -o noclobber; echo $$ > "$WATCHER_LOCK_FILE" ) 2>/dev/null; then
                echo -e "${PAD}${RED}[$STREAM] Failed to acquire watcher lock${NC}"
                exit 1
            fi
        fi
    fi
}

# Check for stale cycle lock and clean up if needed
check_stale_lock() {
    if [ -f "$LOCK_FILE" ]; then
        local old_pid=$(cat "$LOCK_FILE" 2>/dev/null)
        if [ -n "$old_pid" ] && is_pid_alive "$old_pid"; then
            echo -e "${PAD}${RED}[$STREAM] Another instance is already running (PID $old_pid)${NC}"
            echo -e "${PAD}${RED}[$STREAM] Kill it first or wait for it to finish${NC}"
            exit 1
        else
            echo -e "${PAD}${YELLOW}[$STREAM] Removing stale lock (PID $old_pid is dead)${NC}"
            rm -f "$LOCK_FILE"
        fi
    fi
}

# Print startup banner
print_startup_banner() {
    echo ""
    echo -e "${PAD}${GREEN}[$STREAM] Starting with fswatch...${NC}"
    echo -e "${PAD}${BLUE}[$STREAM] Model: $MODEL${NC}"
    echo -e "${PAD}${BLUE}[$STREAM] Watching: ${WATCH_FILES[*]}${NC}"
    if [ "$FSWATCH_LATENCY" -gt 10 ]; then
        echo -e "${PAD}${BLUE}[$STREAM] Debounce: ${FSWATCH_LATENCY}s ($(($FSWATCH_LATENCY / 60))m)${NC}"
    else
        echo -e "${PAD}${BLUE}[$STREAM] Debounce: ${FSWATCH_LATENCY}s${NC}"
    fi
    echo ""
}

# Start fswatch and main loop
# This function does not return - it runs until killed
start_watch_loop() {
    # Set up cleanup trap
    trap cleanup SIGINT SIGTERM EXIT

    # Ensure only one watcher instance per stream
    acquire_watcher_lock

    # Check for stale cycle locks
    check_stale_lock

    # Print startup info
    print_startup_banner

    # Ensure consistent working directory for relative watch paths
    cd "$ROOT_DIR"

    # Take initial mtime snapshot
    snapshot_mtimes

    # Initial run FIRST (no background processes)
    run_cycle "initial"

    # Simple synchronous loop: wait → run → repeat
    # Using fswatch -1 (one-shot mode) eliminates ghost/orphan processes
    while true; do
        echo -e "${PAD}${GREEN}[$STREAM] IDLE - waiting for file changes...${NC}"
        echo -e "${PAD}${DIM}Watched: ${WATCH_FILES[*]}${NC}"

        # Check for stop signal before waiting
        if [[ -f ".ralph-stop" ]]; then
            echo -e "${PAD}${YELLOW}[$STREAM] Stop signal detected${NC}"
            break
        fi

        # fswatch -1 = one-shot mode, exits after first event
        # No background process, no orphan risk
        CHANGED_FILE=$(fswatch -1 -l "$FSWATCH_LATENCY" "${WATCH_FILES[@]}" 2>/dev/null)

        # Check for stop signal after waking
        if [[ -f ".ralph-stop" ]]; then
            echo -e "${PAD}${YELLOW}[$STREAM] Stop signal detected${NC}"
            break
        fi

        # Run cycle synchronously
        run_cycle "file change" "$CHANGED_FILE"
    done
}
