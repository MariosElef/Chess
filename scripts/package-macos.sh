#!/usr/bin/env bash
# Builds Chess.app for this Mac's architecture, with the server's address built in and Stockfish
# inside, and wraps it in a .dmg in dist/.
#   CHESS_SERVER_URL=https://your-mac.your-tailnet.ts.net scripts/package-macos.sh
set -euo pipefail
cd "$(dirname "$0")/.."

: "${CHESS_SERVER_URL:?Set CHESS_SERVER_URL to the https:// address of the server}"
export CHESS_SERVER_URL
stockfish_release=sf_19
# The desktop crate's version, between the quotes of its `version = "…"` line.
version_line=$(grep -m1 '^version' desktop/Cargo.toml)
version=${version_line#*\"}
version=${version%\"*}
arch=$(uname -m)

app=dist/Chess.app
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

echo "Building the app for ${CHESS_SERVER_URL}…"
cargo build --release -p chess-desktop
cp target/release/chess-desktop "$app/Contents/MacOS/Chess"

echo "Adding Stockfish ($stockfish_release)…"
curl -fsSL -o "$work/stockfish.tar.gz" \
    "https://github.com/official-stockfish/Stockfish/releases/download/$stockfish_release/stockfish-macos-universal.tar.gz"
tar -xzf "$work/stockfish.tar.gz" -C "$work"
cp "$work/stockfish/stockfish-macos-universal" "$app/Contents/Resources/stockfish"
cp "$work/stockfish/Copying.txt" "$app/Contents/Resources/Stockfish-COPYING.txt"
chmod +x "$app/Contents/Resources/stockfish"

echo "Making the icon…"
iconset="$work/AppIcon.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z $size $size desktop/assets/shell/app_icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z $double $double desktop/assets/shell/app_icon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Chess</string>
    <key>CFBundleDisplayName</key><string>Chess</string>
    <key>CFBundleIdentifier</key><string>com.marioselef.chess</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleExecutable</key><string>Chess</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Signed ad hoc, so Apple silicon runs it; others still right-click → Open the first time.
codesign --force --deep --sign - "$app"

dmg="dist/Chess-$version-macos-$arch.dmg"
echo "Making ${dmg}…"
rm -f "$dmg"
hdiutil create -volname Chess -srcfolder "$app" -ov -format UDZO "$dmg" >/dev/null
echo "Done: $dmg"
