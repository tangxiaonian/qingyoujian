#!/bin/bash
# Logic-level AppKit tests, with a fake Finder controller and no user file writes.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p build
if [ -n "${QYJ_MENU_PREVIEW:-}" ]; then
  mkdir -p output
fi
xcrun clang -fobjc-arc -mmacosx-version-min=13.0 -framework AppKit -framework FinderSync -framework UniformTypeIdentifiers -framework CoreGraphics native/tests/MenuTests.m -o build/menu-tests
build/menu-tests
xcrun clang -fobjc-arc -mmacosx-version-min=13.0 -framework AppKit -framework Security -framework Carbon -framework ServiceManagement native/bridge.m native/tests/BridgeTests.m -o build/bridge-tests
build/bridge-tests
