#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
cd "$REPO_ROOT"

APP_BINARY="${1:-target/release/kagari-studio}"
DMG_PATH="${2:-${REPO_ROOT}/Kagari-Studio-macOS.dmg}"
if [[ "$APP_BINARY" != /* ]]; then
    APP_BINARY="${REPO_ROOT}/${APP_BINARY}"
fi
if [[ "$DMG_PATH" != /* ]]; then
    DMG_PATH="${REPO_ROOT}/${DMG_PATH}"
fi
SIGNING_IDENTITY="${KAGARI_CODESIGN_IDENTITY:-}"
NOTARY_PROFILE="${KAGARI_NOTARY_PROFILE:-}"
if [[ -n "$NOTARY_PROFILE" && -z "$SIGNING_IDENTITY" ]]; then
    echo "Notarization requires KAGARI_CODESIGN_IDENTITY." >&2
    exit 1
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "macOS app packaging requires macOS tools (sips, iconutil, hdiutil)." >&2
    exit 1
fi
if [[ ! -f "$APP_BINARY" || ! -x "$APP_BINARY" ]]; then
    echo "Release binary is missing, not a regular file, or not executable: $APP_BINARY" >&2
    exit 1
fi
APP_BINARY_CANONICAL="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$APP_BINARY")"
DMG_PATH_CANONICAL="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$DMG_PATH")"
if [[ "$APP_BINARY_CANONICAL" == "$DMG_PATH_CANONICAL" ]]; then
    echo "The DMG output path must not replace the release binary: $APP_BINARY" >&2
    exit 1
fi
if [[ -d "$DMG_PATH" ]]; then
    echo "The DMG output path is a directory: $DMG_PATH" >&2
    exit 1
fi

APP_VERSION="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "kagari-vfx"))')"
APP_BUILD_VERSION="${APP_VERSION%%[-+]*}"
mkdir -p "${REPO_ROOT}/target"
ICONSET_PARENT=""
ICONSET_DIR=""
DMG_STAGE=""
APP_BUNDLE=""
DMG_TEMP_DIR=""
DMG_TEMP=""
cleanup() {
    if [[ -n "$ICONSET_PARENT" ]]; then
        rm -rf "$ICONSET_PARENT"
    fi
    if [[ -n "$DMG_STAGE" ]]; then
        rm -rf "$DMG_STAGE"
    fi
    if [[ -n "$DMG_TEMP_DIR" ]]; then
        rm -rf "$DMG_TEMP_DIR"
    fi
}
trap cleanup EXIT

ICONSET_PARENT="$(mktemp -d "${REPO_ROOT}/target/Kagari-icon.XXXXXX")"
ICONSET_DIR="${ICONSET_PARENT}/Kagari.iconset"
mkdir -p "$ICONSET_DIR"
DMG_STAGE="$(mktemp -d "${TMPDIR:-/tmp}/kagari-dmg.XXXXXX")"
APP_BUNDLE="${DMG_STAGE}/Kagari Studio.app"

echo "Packaging Kagari Studio ${APP_VERSION}..."
mkdir -p "${APP_BUNDLE}/Contents/MacOS" "${APP_BUNDLE}/Contents/Resources"
cp "$APP_BINARY" "${APP_BUNDLE}/Contents/MacOS/Kagari Studio"
chmod +x "${APP_BUNDLE}/Contents/MacOS/Kagari Studio"

ICON_SOURCE="${ICONSET_PARENT}/kagari_logo.png"
sips -s format png assets/kagari_logo.webp --out "$ICON_SOURCE" >/dev/null
sips -z 16 16 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_16x16.png" >/dev/null
sips -z 32 32 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_32x32.png" >/dev/null
sips -z 64 64 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_128x128.png" >/dev/null
sips -z 256 256 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_256x256.png" >/dev/null
sips -z 512 512 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_512x512.png" >/dev/null
sips -z 1024 1024 "$ICON_SOURCE" --out "${ICONSET_DIR}/icon_512x512@2x.png" >/dev/null
iconutil -c icns "$ICONSET_DIR" -o "${APP_BUNDLE}/Contents/Resources/Kagari.icns"

cat > "${APP_BUNDLE}/Contents/Info.plist" <<PLIST
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
    <key>CFBundleIconFile</key>
    <string>Kagari</string>
    <key>CFBundleShortVersionString</key>
    <string>${APP_VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${APP_BUILD_VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST
plutil -lint "${APP_BUNDLE}/Contents/Info.plist"

if [[ -n "$SIGNING_IDENTITY" ]]; then
    codesign --force --options runtime --timestamp --sign "$SIGNING_IDENTITY" "$APP_BUNDLE"
    codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"
else
    echo "Warning: creating an unsigned macOS app; Gatekeeper may block downloaded builds." >&2
fi

ln -s /Applications "${DMG_STAGE}/Applications"
mkdir -p "$(dirname -- "$DMG_PATH")"
DMG_DIR="$(dirname -- "$DMG_PATH")"
DMG_NAME="$(basename -- "$DMG_PATH")"
DMG_STEM="${DMG_NAME%.*}"
DMG_TEMP_DIR="$(mktemp -d "${DMG_DIR}/.${DMG_STEM}.kagari-tmp.XXXXXX")"
DMG_TEMP="${DMG_TEMP_DIR}/${DMG_STEM}.dmg"
diskutil image create from --format UDZO --volumeName "Kagari Studio" "$DMG_STAGE" "$DMG_TEMP"
hdiutil verify "$DMG_TEMP"
if [[ -n "$SIGNING_IDENTITY" ]]; then
    codesign --force --timestamp --sign "$SIGNING_IDENTITY" "$DMG_TEMP"
    codesign --verify --verbose=2 "$DMG_TEMP"
fi
if [[ -n "$NOTARY_PROFILE" ]]; then
    xcrun notarytool submit "$DMG_TEMP" --keychain-profile "$NOTARY_PROFILE" --wait
    xcrun stapler staple "$DMG_TEMP"
    xcrun stapler validate "$DMG_TEMP"
elif [[ -n "$SIGNING_IDENTITY" ]]; then
    echo "Warning: app and DMG are signed but not notarized; Gatekeeper may still block downloaded builds." >&2
fi
hdiutil verify "$DMG_TEMP"
python3 -c 'import os,sys; os.replace(sys.argv[1], sys.argv[2])' "$DMG_TEMP" "$DMG_PATH"
DMG_TEMP=""
echo "Created ${DMG_PATH}"
