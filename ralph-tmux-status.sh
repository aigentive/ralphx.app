#!/bin/bash
# ralph-tmux-status.sh - Status header display for RALPH multi-stream orchestrator
# Shows uptime, backlog counts, and quick-reference keys

# Track start time
START_TIME=$(date +%s)

# RalphX orange (ANSI 256 color - closest to #ff6b35)
ORANGE='\033[38;5;209m'
RESET='\033[0m'
BOLD='\033[1m'
DIM='\033[2m'

# Count unchecked items in a file
count_items() {
    local file="$1"
    grep -c '^- \[ \]' "$file" 2>/dev/null || echo 0
}

# Main display loop
while true; do
    clear

    # Calculate uptime
    NOW=$(date +%s)
    ELAPSED=$((NOW - START_TIME))
    HOURS=$((ELAPSED / 3600))
    MINUTES=$(((ELAPSED % 3600) / 60))
    UPTIME=$(printf "%02d:%02d" "$HOURS" "$MINUTES")

    # Current time
    CURRENT_TIME=$(date "+%H:%M:%S")

    # Backlog counts
    P0_COUNT=$(count_items "streams/features/backlog.md")
    P1_COUNT=$(count_items "streams/refactor/backlog.md")
    P2P3_COUNT=$(count_items "streams/polish/backlog.md")

    # Display header
    echo -e "${BOLD}${ORANGE}╔════════════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}${ORANGE}║         RALPH MULTI-STREAM ORCHESTRATOR                        ║${RESET}"
    echo -e "${BOLD}${ORANGE}╚════════════════════════════════════════════════════════════════╝${RESET}"
    echo ""
    echo -e "${BOLD}Uptime:${RESET} ${UPTIME}   ${BOLD}Time:${RESET} ${CURRENT_TIME}"
    echo ""
    echo -e "${BOLD}Backlogs:${RESET}"
    echo -e "  P0 (features):  ${P0_COUNT} items"
    echo -e "  P1 (refactor):  ${P1_COUNT} items"
    echo -e "  P2/P3 (polish): ${P2P3_COUNT} items"
    echo ""
    echo -e "${DIM}─────────────────────────────────────────────────────────────────${RESET}"
    echo -e "${BOLD}Quick Keys:${RESET}"
    echo -e "  ${BOLD}Ctrl+b 0${RESET}  Status (this pane)"
    echo -e "  ${BOLD}Ctrl+b 1${RESET}  Features (opus)"
    echo -e "  ${BOLD}Ctrl+b 2${RESET}  Refactor (sonnet)"
    echo -e "  ${BOLD}Ctrl+b 3${RESET}  Polish (sonnet)"
    echo -e "  ${BOLD}Ctrl+b 4${RESET}  Verify (sonnet)"
    echo -e "  ${BOLD}Ctrl+b 5${RESET}  Hygiene (sonnet)"
    echo ""
    echo -e "  ${BOLD}Ctrl+b d${RESET}  Detach (streams keep running)"
    echo -e "  ${BOLD}Ctrl+b [${RESET}  Scroll mode (q to exit)"
    echo -e "  ${BOLD}Ctrl+b z${RESET}  Zoom pane (toggle)"
    echo ""
    echo -e "${DIM}Refreshes every 5s${RESET}"

    sleep 5
done
