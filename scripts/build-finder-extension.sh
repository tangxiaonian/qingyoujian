#!/bin/bash
# Compile only; installation and Finder lifecycle are separate user actions.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT="$PWD/build/QingRightExtension.appex"
mkdir -p "$OUT/Contents/MacOS"
cp native/finder-extension/Info.plist "$OUT/Contents/Info.plist"
xcrun clang -fobjc-arc -Os -mmacosx-version-min=13.0 -fapplication-extension -framework AppKit -framework FinderSync -framework UniformTypeIdentifiers native/finder-extension/FinderSync.m native/finder-extension/main.m -o "$OUT/Contents/MacOS/QingRightExtension"
codesign --force --sign - --entitlements native/finder-extension/Extension.entitlements "$OUT"
codesign --verify --strict "$OUT"
