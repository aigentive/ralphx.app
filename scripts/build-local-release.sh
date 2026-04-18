#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Build RalphX in release mode for local internal use.

Usage:
  ./scripts/build-local-release.sh [--bundle] [--sync-db] [--sync-plugin] [--skip-build]

Options:
  --bundle      Create app bundle artifacts too (slower build)
  --sync-db     Force copy dev DB to app-data DB (overwrites target DB)
  --sync-plugin Force refresh plugin runtime in app-data (redundant: plugin runtime is refreshed every run)
  --skip-build  Skip build and only run DB seed/sync logic
  -h, --help    Show this help

This helper is intentionally local-only:
- it may seed app data from the dev DB
- it refreshes plugin runtime into Application Support
- it is not the production release entrypoint
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BUILD_BUNDLE="false"
FORCE_DB_SYNC="false"
SKIP_BUILD="false"
FORCE_PLUGIN_SYNC="false"

for arg in "$@"; do
  case "${arg}" in
    --bundle) BUILD_BUNDLE="true" ;;
    --sync-db) FORCE_DB_SYNC="true" ;;
    --sync-plugin) FORCE_PLUGIN_SYNC="true" ;;
    --skip-build) SKIP_BUILD="true" ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: ${arg}" >&2
      usage
      exit 1
      ;;
  esac
done

DEV_DB="${PROJECT_ROOT}/src-tauri/ralphx.db"
APP_IDENTIFIER="$(
  node -e 'const fs=require("fs");const c=JSON.parse(fs.readFileSync(process.argv[1],"utf8"));process.stdout.write(c.identifier);' \
  "${PROJECT_ROOT}/src-tauri/tauri.conf.json"
)"
APP_DATA_DIR="${HOME}/Library/Application Support/${APP_IDENTIFIER}"
PROD_DB="${APP_DATA_DIR}/ralphx.db"
SOURCE_PLUGIN_DIR="${PROJECT_ROOT}/plugins/app"
PROD_PLUGIN_DIR="${APP_DATA_DIR}/plugins/app"
PROD_MCP_DIR="${PROD_PLUGIN_DIR}/ralphx-mcp-server"
PROD_MCP_MAIN="${PROD_MCP_DIR}/build/index.js"
PROD_MCP_NODE_MODULES="${PROD_MCP_DIR}/node_modules"
PROD_EXTERNAL_MCP_DIR="${PROD_PLUGIN_DIR}/ralphx-external-mcp"
PROD_EXTERNAL_MCP_MAIN="${PROD_EXTERNAL_MCP_DIR}/build/index.js"
PROD_EXTERNAL_MCP_NODE_MODULES="${PROD_EXTERNAL_MCP_DIR}/node_modules"

create_fresh_prod_db() {
  local db_path="$1"

  python3 - "$db_path" <<'PY'
import sqlite3
import sys

sqlite3.connect(sys.argv[1]).close()
PY
}

ensure_runtime_package() {
  local package_dir="$1"
  local build_output="$2"
  local node_modules_dir="$3"
  local label="$4"

  if [[ ! -d "${package_dir}" ]]; then
    echo "${label} runtime dir not found: ${package_dir}" >&2
    exit 1
  fi

  cd "${package_dir}"

  if [[ ! -d "${node_modules_dir}" ]]; then
    echo "Installing ${label} dependencies in runtime copy..."
    npm install
  fi

  echo "Rebuilding ${label} in runtime copy..."
  npm run build
}

if [[ ! -d "${SOURCE_PLUGIN_DIR}" ]]; then
  echo "Plugin source dir not found: ${SOURCE_PLUGIN_DIR}" >&2
  exit 1
fi

if [[ "${SKIP_BUILD}" != "true" ]]; then
  echo "Building RalphX local release artifacts..."
  cd "${PROJECT_ROOT}/frontend"

  if [[ "${BUILD_BUNDLE}" == "true" ]]; then
    CI=false npm run tauri build
  else
    CI=false npm run tauri build -- --no-bundle
  fi
fi

echo "Preparing local app-data DB..."
mkdir -p "${APP_DATA_DIR}"

if [[ "${FORCE_DB_SYNC}" == "true" ]]; then
  if [[ ! -f "${DEV_DB}" ]]; then
    echo "Dev DB not found for --sync-db: ${DEV_DB}" >&2
    exit 1
  fi
  cp -f "${DEV_DB}" "${PROD_DB}"
  echo "Forced DB sync complete: ${PROD_DB}"
elif [[ ! -f "${PROD_DB}" ]]; then
  if [[ -f "${DEV_DB}" ]]; then
    cp "${DEV_DB}" "${PROD_DB}"
    echo "Seeded local app-data DB from dev DB: ${PROD_DB}"
  else
    create_fresh_prod_db "${PROD_DB}"
    echo "Created fresh local app-data DB: ${PROD_DB}"
  fi
else
  echo "Local app-data DB already exists, leaving untouched: ${PROD_DB}"
fi

echo "Preparing local Application Support plugin runtime..."
if [[ -d "${PROD_PLUGIN_DIR}" ]]; then
  rm -rf "${PROD_PLUGIN_DIR}"
fi

cp -R "${SOURCE_PLUGIN_DIR}" "${PROD_PLUGIN_DIR}"
echo "Refreshed local plugin runtime: ${PROD_PLUGIN_DIR}"

ensure_runtime_package \
  "${PROD_MCP_DIR}" \
  "${PROD_MCP_MAIN}" \
  "${PROD_MCP_NODE_MODULES}" \
  "Internal MCP"

ensure_runtime_package \
  "${PROD_EXTERNAL_MCP_DIR}" \
  "${PROD_EXTERNAL_MCP_MAIN}" \
  "${PROD_EXTERNAL_MCP_NODE_MODULES}" \
  "External MCP"

echo ""
echo "Done."
echo "Dev DB (source, untouched): ${DEV_DB}"
echo "Local app-data DB (used by local release-like runs): ${PROD_DB}"
echo "Local plugin dir (used by local release-like runs): ${PROD_PLUGIN_DIR}"

BIN_PATH="${PROJECT_ROOT}/src-tauri/target/release/ralphx"
APP_PATH="${PROJECT_ROOT}/src-tauri/target/release/bundle/macos/RalphX.app"

if [[ -x "${BIN_PATH}" ]]; then
  echo "Binary: ${BIN_PATH}"
  echo "Run: \"${BIN_PATH}\""
fi

if [[ -d "${APP_PATH}" ]]; then
  echo "App bundle: ${APP_PATH}"
  echo "Open: open \"${APP_PATH}\""
fi
