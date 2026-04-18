#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

prepare_runtime_package() {
  local package_dir="$1"
  local label="$2"

  if [[ ! -d "${package_dir}" ]]; then
    echo "${label} runtime dir not found: ${package_dir}" >&2
    exit 1
  fi

  echo "Preparing ${label} runtime..."
  (
    cd "${package_dir}"
    npm ci
    npm run build
  )
}

prepare_runtime_package \
  "${PROJECT_ROOT}/plugins/app/ralphx-mcp-server" \
  "Internal MCP"

prepare_runtime_package \
  "${PROJECT_ROOT}/plugins/app/ralphx-external-mcp" \
  "External MCP"
