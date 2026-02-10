#!/usr/bin/env bash
# Append timestamped entry to optimization log
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

ACTION="${1:?Usage: rule-log.sh <action> <details>}"
DETAILS="${2:?Usage: rule-log.sh <action> <details>}"

MEMORY_DIR="$(get_memory_dir)"
mkdir -p "$MEMORY_DIR"
LOG_FILE="$(get_log_path)"

# Create daily log if it doesn't exist
if [[ ! -f "$LOG_FILE" ]]; then
  TODAY=$(date -u +"%Y-%m-%d")
  echo "# $TODAY — Optimization Log" > "$LOG_FILE"
  echo "" >> "$LOG_FILE"
fi

# Append entry
TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S")
{
  echo "## $TIMESTAMP — $ACTION"
  echo "- $DETAILS"
  echo ""
} >> "$LOG_FILE"

echo "Logged: $ACTION"
