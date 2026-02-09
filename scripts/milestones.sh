#!/bin/bash
# RalphX Development Milestones - Shared Config
# Sourced by loc-trends.sh and count-loc.sh
# Compatible with bash 3.2+
#
# Format: "EPOCH|YYYY-MM-DD HH:MM|LABEL|DESCRIPTION"
# Epoch computation: date -j -f "%Y-%m-%d %H:%M:%S %z" "YYYY-MM-DD HH:MM:SS +0200" "+%s"

# Milestone definitions (pipe-delimited)
MILESTONES=(
    "1770530400|2026-02-08 08:00|RALF Loop -> Self-Improvement|Transitioned from Tmux-based RALF loop to RalphX self-improvement"
)

# Parse a milestone entry into component variables.
# Usage: parse_milestone "$entry"
# Sets: MS_EPOCH, MS_DATE, MS_TIME, MS_LABEL, MS_DESC
parse_milestone() {
    local entry="$1"
    local IFS='|'
    local parts
    read -ra parts <<< "$entry"
    MS_EPOCH="${parts[0]}"
    # Split "YYYY-MM-DD HH:MM" into date and time
    local datetime="${parts[1]}"
    MS_DATE="${datetime%% *}"
    MS_TIME="${datetime#* }"
    MS_LABEL="${parts[2]}"
    MS_DESC="${parts[3]}"
}

# Check if a YYYY-MM-DD date matches any milestone.
# Usage: date_has_milestone "2026-02-08"
# Returns: 0 (true) if match, 1 (false) if no match
date_has_milestone() {
    local check_date="$1"
    local entry
    for entry in "${MILESTONES[@]}"; do
        parse_milestone "$entry"
        if [ "$MS_DATE" = "$check_date" ]; then
            return 0
        fi
    done
    return 1
}

# Get the label for a milestone matching a given date.
# Usage: label=$(milestone_label_for_date "2026-02-08")
# Returns: label string via stdout, or empty if no match
milestone_label_for_date() {
    local check_date="$1"
    local entry
    for entry in "${MILESTONES[@]}"; do
        parse_milestone "$entry"
        if [ "$MS_DATE" = "$check_date" ]; then
            echo "$MS_LABEL"
            return 0
        fi
    done
    return 1
}
