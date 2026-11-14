#!/bin/bash
# Explicit local installation; no Finder restart or system preference change.
set -euo pipefail
cd "$(dirname "$0")/.."
SOURCE="${1:-$PWD/src-tauri/target/release/bundle/macos/轻右键.app}"
DEST="$HOME/Applications/轻右键.app"
if [ ! -d "$SOURCE" ]; then
  echo '请先运行 npm run build:macos' >&2
  exit 1
fi
if [ -d "$DEST" ]; then
  echo '目标应用已存在。请退出旧版并将旧应用移走，再运行安装。配置不会删除。' >&2
  exit 1
fi
mkdir -p "$HOME/Applications"
cp -R "$SOURCE" "$DEST"
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DEST"
pluginkit -a "$DEST/Contents/PlugIns/QingRightExtension.appex"
pluginkit -e use -i local.qingyoujian.extension
open "$DEST"
echo '已安装。若右键菜单未出现，请在系统设置 → 通用 → 登录项与扩展 → Finder 扩展中启用轻右键。'
