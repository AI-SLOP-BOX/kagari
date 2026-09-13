#!/usr/bin/env bash
set -euo pipefail

echo "🦀 Building Kagari Studio release binary..."
cargo build --release --features gui --bin kagari-studio

echo "📦 Packaging macOS App Bundle..."
BUNDLE_DIR="target/bundle/Kagari Studio.app/Contents"
mkdir -p "${BUNDLE_DIR}/MacOS"
mkdir -p "${BUNDLE_DIR}/Resources"

cp "target/release/kagari-studio" "${BUNDLE_DIR}/MacOS/Kagari Studio"
chmod +x "${BUNDLE_DIR}/MacOS/Kagari Studio"

cat << 'PLIST' > "${BUNDLE_DIR}/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>Kagari Studio</string>
    <key>CFBundleIdentifier</key>
    <string>org.kagari.studio</string>
    <key>CFBundleName</key>
    <string>Kagari Studio</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "💿 Creating macOS DMG disk image..."
hdiutil create -volname "Kagari Studio" -srcfolder "target/bundle/Kagari Studio.app" -ov -format UDZO "Kagari-Studio-macOS.dmg"

echo "✅ DMG build complete: Kagari-Studio-macOS.dmg"
