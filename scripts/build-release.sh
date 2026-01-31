#!/bin/bash
set -e

echo "Building RalphX release..."

# Build frontend
npm run build

# Build Tauri (release mode)
cd src-tauri
cargo tauri build

echo ""
echo "Build complete!"
echo "App: src-tauri/target/release/bundle/macos/RalphX.app"
echo "DMG: src-tauri/target/release/bundle/dmg/RalphX_*.dmg"
echo ""
echo "To test: open src-tauri/target/release/bundle/macos/RalphX.app"
