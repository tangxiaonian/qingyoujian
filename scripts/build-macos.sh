#!/bin/bash
# Produce a complete local .app; never install, enable, or relaunch Finder during a build.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"
bash scripts/build-finder-extension.sh
npm run tauri build -- --bundles app
APP="$PWD/src-tauri/target/release/bundle/macos/轻右键.app"
mkdir -p "$APP/Contents/PlugIns"
cp -R build/QingRightExtension.appex "$APP/Contents/PlugIns/"
codesign --force --sign - --entitlements native/finder-extension/Extension.entitlements "$APP/Contents/PlugIns/QingRightExtension.appex"
codesign --force --sign - "$APP"
codesign --verify --deep --strict "$APP"
printf 'Built: %s\n' "$APP"
