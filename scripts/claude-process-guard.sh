#!/bin/bash
# claude-process-guard.sh - Enforce max concurrent Claude processes

set -euo pipefail

MAX_CLAUDE=4
MODE="warn" # warn | block | kill
MATCH="claude"
MATCH_MODE="exact" # exact | full
VERBOSE=false

usage() {
  cat <<'EOF'
Usage: ./scripts/claude-process-guard.sh [options]

Options:
  --max N           Max allowed Claude processes (default: 4)
  --mode MODE       warn | block | kill (default: warn)
  --match PATTERN   Process match string (default: "claude")
  --match-mode MODE exact | full (default: exact)
  --verbose          Print matched PIDs and commands
  -h, --help        Show this help

Exit codes:
  0 = under or equal to limit
  2 = over limit (warn/block)
  3 = invalid args
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --max)
      MAX_CLAUDE="${2:-}"
      shift 2
      ;;
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    --match)
      MATCH="${2:-}"
      shift 2
      ;;
    --match-mode)
      MATCH_MODE="${2:-}"
      shift 2
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown arg: $1"
      usage
      exit 3
      ;;
  esac
done

if ! [[ "$MAX_CLAUDE" =~ ^[0-9]+$ ]]; then
  echo "Invalid --max: $MAX_CLAUDE"
  exit 3
fi

if [[ "$MODE" != "warn" && "$MODE" != "block" && "$MODE" != "kill" ]]; then
  echo "Invalid --mode: $MODE"
  exit 3
fi

if [[ "$MATCH_MODE" != "exact" && "$MATCH_MODE" != "full" ]]; then
  echo "Invalid --match-mode: $MATCH_MODE"
  exit 3
fi

PIDS=()
if [[ "$MATCH_MODE" == "exact" ]]; then
  while IFS= read -r pid; do
    [[ -n "$pid" ]] && PIDS+=("$pid")
  done < <(pgrep -x "$MATCH" 2>/dev/null || true)
else
  while IFS= read -r pid; do
    [[ -n "$pid" ]] && PIDS+=("$pid")
  done < <(pgrep -f "$MATCH" 2>/dev/null || true)
fi

COUNT=${#PIDS[@]}

if (( COUNT <= MAX_CLAUDE )); then
  if [ "$VERBOSE" = true ]; then
    echo "Claude guard: $COUNT processes <= max $MAX_CLAUDE (mode: $MODE)"
    if (( COUNT > 0 )); then
      ps -o pid= -o command= -p "${PIDS[@]}"
    fi
  fi
  exit 0
fi

echo "Claude guard: $COUNT processes > max $MAX_CLAUDE (mode: $MODE)"
if [ "$VERBOSE" = true ] && (( COUNT > 0 )); then
  ps -o pid= -o command= -p "${PIDS[@]}"
fi

if [[ "$MODE" == "warn" || "$MODE" == "block" ]]; then
  exit 2
fi

# mode = kill: terminate newest processes first to preserve long-running work
PID_AGES=()
while IFS= read -r row; do
  [[ -n "$row" ]] && PID_AGES+=("$row")
done < <(ps -o pid= -o etimes= -p "${PIDS[@]}" | awk '{print $1" "$2}' | sort -k2,2n)
EXCESS=$((COUNT - MAX_CLAUDE))

for ((i=0; i<EXCESS && i<${#PID_AGES[@]}; i++)); do
  PID=$(echo "${PID_AGES[$i]}" | awk '{print $1}')
  echo "Killing claude PID $PID (excess)"
  kill -TERM "$PID" 2>/dev/null || true
done

sleep 0.5

for ((i=0; i<EXCESS && i<${#PID_AGES[@]}; i++)); do
  PID=$(echo "${PID_AGES[$i]}" | awk '{print $1}')
  if kill -0 "$PID" 2>/dev/null; then
    kill -KILL "$PID" 2>/dev/null || true
  fi
done

exit 0
