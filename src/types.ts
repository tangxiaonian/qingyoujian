export interface Favorite {
  id: string;
  displayName: string;
  path: string;
  sortOrder: number;
}
export interface MenuItem {
  id: string;
  enabled: boolean;
  sortOrder: number;
}
export interface Settings {
  terminalBundleId: string;
  menuItems: MenuItem[];
  showMenuBarIcon: boolean;
  startAtLogin: boolean;
  runInBackground: boolean;
}
export interface FileType {
  id: string;
  displayName: string;
  extension: string;
}
export interface AppState {
  version: string;
  extensionStatus: "enabled" | "disabled" | "unknown";
  loginStatus: "enabled" | "requiresApproval" | "disabled" | "unavailable";
  settings: Settings;
  fileTypes: FileType[];
  favorites: Favorite[];
}
export const menuLabels: Record<string, string> = {
  "new-folder": "新建文件夹",
  "copy-to": "复制到",
  "move-to": "剪切到",
  trash: "移到废纸篓",
  "new-file": "新建文件",
  "copy-path": "拷贝路径",
  terminal: "在此打开终端",
  "show-hidden": "显示所有隐藏文件",
  "hide-hidden": "隐藏所有隐藏文件",
  favorites: "常用目录",
  "add-favorite": "添加到常用目录",
};
/** Move a favorite by one step without mutating the reactive source array. */
export function moveIds(
  items: Favorite[],
  id: string,
  delta: number,
): string[] {
  const ids = items.map((item) => item.id);
  const index = ids.indexOf(id);
  const next = index + delta;
  if (index < 0 || next < 0 || next >= ids.length) {
    return ids;
  }
  [ids[index], ids[next]] = [ids[next]!, ids[index]!];
  return ids;
}
/** Native rejections may carry structured errors; show their Chinese message, not [object Object]. */
export function errorText(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  return typeof error === "string" ? error : "操作未完成，请重试。";
}

export const menuGlyphs: Record<string, string> = {
  "new-folder": "▰",
  "new-file": "＋",
  "copy-path": "⧉",
  "copy-to": "▤",
  "move-to": "✂",
  trash: "⌫",
  terminal: "›_",
  "show-hidden": "◉",
  "hide-hidden": "◌",
  favorites: "☆",
  "add-favorite": "✩",
};
