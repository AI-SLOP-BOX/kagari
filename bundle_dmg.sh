#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building Kagari Studio release binary..."
cargo build --release --no-default-features --features ame --bin kagari-studio

TARGET_DIR="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
"${SCRIPT_DIR}/scripts/package_macos.sh" "${TARGET_DIR}/release/kagari-studio"
