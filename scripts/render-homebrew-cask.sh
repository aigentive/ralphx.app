#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Render the RalphX Homebrew cask.

Usage:
  ./scripts/render-homebrew-cask.sh <version> <arm_sha256> <intel_sha256>
EOF
}

if [[ $# -ne 3 ]]; then
  usage >&2
  exit 1
fi

version="$1"
arm_sha="$2"
intel_sha="$3"

cat <<EOF
cask "ralphx" do
  arch arm: "aarch64", intel: "x86_64"

  version "${version}"
  sha256 arm:   "${arm_sha}",
         intel: "${intel_sha}"

  url "https://github.com/aigentive/ralphx.app/releases/download/v#{version}/RalphX_#{version}_#{arch}.dmg"
  name "RalphX"
  desc "Native Mac GUI for autonomous AI development"
  homepage "https://github.com/aigentive/ralphx.app"
  auto_updates true

  depends_on formula: "node"
  depends_on macos: ">= :ventura"

  app "RalphX.app"

  caveats do
    <<~EOS
      Install at least one supported AI harness CLI after install.
      RalphX can update itself in-app after install.
      To force a Homebrew-managed refresh, run: brew upgrade --cask ralphx
    EOS
  end
end
EOF
