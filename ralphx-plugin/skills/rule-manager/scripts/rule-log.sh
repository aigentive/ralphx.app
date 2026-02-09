#!/usr/bin/env bash
# Append timestamped entry to optimization log
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

ACTION="${1:?Usage: rule-log.sh <action> <details>}"
DETAILS="${2:?Usage: rule-log.sh <action> <details>}"

LOG_FILE="$(get_log_path)"

# Create log if it doesn't exist
if [[ ! -f "$LOG_FILE" ]]; then
  cat > "$LOG_FILE" <<'EOF'
# Rule Optimization Log

Automated log of rule-manager and knowledge-capture changes.

EOF
fi

# Append entry
TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S")
{
  echo "## $TIMESTAMP — $ACTION"
  echo "- $DETAILS"
  echo ""
} >> "$LOG_FILE"

echo "Logged: $ACTION"
