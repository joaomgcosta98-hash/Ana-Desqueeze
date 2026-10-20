#!/bin/bash
set -euo pipefail
binary=target/aarch64-apple-darwin/release/anadesqueeze
app=dist/AnaDesqueeze.app
# Refuse to package an Intel-only executable.
test "$(lipo -archs "$binary")" = arm64
mkdir -p "$app/Contents/MacOS"
cp "$binary" "$app/Contents/MacOS/AnaDesqueeze"
chmod +x "$app/Contents/MacOS/AnaDesqueeze"
cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>AnaDesqueeze</string>
<key>CFBundleIdentifier</key><string>pt.joaocosta.anadesqueeze</string>
<key>CFBundleName</key><string>AnaDesqueeze</string>
<key>CFBundleDisplayName</key><string>AnaDesqueeze</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.1</string>
<key>CFBundleVersion</key><string>2</string>
<key>LSMinimumSystemVersion</key><string>13.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
plutil -lint "$app/Contents/Info.plist"
# Ad-hoc integrity signature; this is NOT Developer ID or Apple notarization.
codesign --force --sign - --timestamp=none "$app"
codesign --verify --deep --strict --verbose=2 "$app"
ditto -c -k --sequesterRsrc --keepParent "$app" dist/AnaDesqueeze-AppleSilicon-app.zip
mkdir -p dist/dmg
ditto "$app" dist/dmg/AnaDesqueeze.app
ln -s /Applications dist/dmg/Applications
hdiutil create -volname AnaDesqueeze -srcfolder dist/dmg -ov -format UDZO dist/AnaDesqueeze-AppleSilicon.dmg
hdiutil verify dist/AnaDesqueeze-AppleSilicon.dmg
(cd dist && shasum -a 256 AnaDesqueeze-AppleSilicon.dmg AnaDesqueeze-AppleSilicon-app.zip > SHA256SUMS.txt)
