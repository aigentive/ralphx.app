#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Build RalphX in release mode for local use.

Usage:
  ./scripts/build-release.sh [--bundle] [--sync-db] [--skip-build]

Options:
  --bundle      Create app bundle artifacts too (slower build)
  --sync-db     Force copy dev DB to app-data DB (overwrites target DB)
  --skip-build  Skip build and only run DB seed/sync logic
  -h, --help    Show this help
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BUILD_BUNDLE="false"
FORCE_DB_SYNC="false"
SKIP_BUILD="false"

for arg in "$@"; do
  case "${arg}" in
    --bundle) BUILD_BUNDLE="true" ;;
    --sync-db) FORCE_DB_SYNC="true" ;;
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

if [[ ! -f "${DEV_DB}" ]]; then
  echo "Dev DB not found: ${DEV_DB}" >&2
  exit 1
fi

if [[ "${SKIP_BUILD}" != "true" ]]; then
  echo "Building RalphX release..."
  cd "${PROJECT_ROOT}"

  if [[ "${BUILD_BUNDLE}" == "true" ]]; then
    CI=false npm run tauri build
  else
    CI=false npm run tauri build -- --no-bundle
  fi
fi

echo "Preparing production DB in app data..."
mkdir -p "${APP_DATA_DIR}"

if [[ "${FORCE_DB_SYNC}" == "true" ]]; then
  cp -f "${DEV_DB}" "${PROD_DB}"
  echo "Forced DB sync complete: ${PROD_DB}"
elif [[ ! -f "${PROD_DB}" ]]; then
  cp "${DEV_DB}" "${PROD_DB}"
  echo "Seeded production DB from dev DB: ${PROD_DB}"
else
  echo "Production DB already exists, leaving untouched: ${PROD_DB}"
fi

echo ""
echo "Done."
echo "Dev DB (source, untouched): ${DEV_DB}"
echo "Prod DB (used by release app): ${PROD_DB}"

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
