#!/bin/zsh
# Empaqueta Moca Editor como .app firmada e instálala en /Applications.
# Uso: scripts/bundle-macos.sh [identidad-de-firma]
#   Por defecto firma con el certificado "Apple Development" del equipo DSXB7C9XQA
#   (Jose David Moreira). Sirve para esta Mac; distribuir a otros requiere
#   "Developer ID Application" + notarización.
set -euo pipefail
cd "$(dirname "$0")/.."

IDENTITY=${1:-$(security find-identity -v -p codesigning | grep 'Apple Development' | grep -v '(S6Z3H4SHT4)' | grep -m1 'Jose David Moreira' | awk '{print $2}')}
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
APP="target/release/bundle/Moca Editor.app"

source ~/.cargo/env 2>/dev/null || true
cargo build --release

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/moca-editor "$APP/Contents/MacOS/moca-editor"

# Ícono
ICONSET=$(mktemp -d)/AppIcon.iconset
mkdir -p "$ICONSET"
swift scripts/icon.swift "$ICONSET/icon_512x512@2x.png"
for s in 16 32 128 256 512; do
  sips -z $s $s "$ICONSET/icon_512x512@2x.png" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s * 2)) $((s * 2)) "$ICONSET/icon_512x512@2x.png" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Moca Editor</string>
  <key>CFBundleDisplayName</key><string>Moca Editor</string>
  <key>CFBundleIdentifier</key><string>gt.moca.editor</string>
  <key>CFBundleExecutable</key><string>moca-editor</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

codesign --force --options runtime --sign "$IDENTITY" "$APP"
codesign --verify --strict --verbose=2 "$APP"

rm -rf "/Applications/Moca Editor.app"
cp -R "$APP" /Applications/
echo "Instalado: /Applications/Moca Editor.app (v${VERSION}, firmado con ${IDENTITY})"
