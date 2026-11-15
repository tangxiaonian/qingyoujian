import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppState, Settings } from "../types";

/** Keep native command names and payloads at one typed interface. */
export const desktopApi = {
  isAvailable: isTauri,
  /** Read the complete backend snapshot; Vue does not persist its own settings. */
  getState: (): Promise<AppState> => invoke("get_app_state"),
  /** Submit a complete settings value, including the effective OS login status. */
  updateSettings: (patch: Settings): Promise<Settings> =>
    invoke("update_settings", { patch }),
  /** Native cancellation returns null, never an empty path to operate on. */
  chooseDirectory: (): Promise<string | null> => invoke("choose_directory"),
  /** Deduplication is decided by Rust using normalized directory paths. */
  addFavorite: (path: string): Promise<{ created: boolean }> =>
    invoke("add_favorite", { path }),
  /** Opening a saved directory resolves its current path in the backend. */
  openFavorite: (id: string): Promise<void> => invoke("open_favorite", { id }),
  /** Renaming changes the bookmark label, not the directory name. */
  renameFavorite: (id: string, displayName: string): Promise<void> =>
    invoke("rename_favorite", { id, displayName }),
  /** Supply all ids so the backend can validate and save order atomically. */
  reorderFavorites: (ids: string[]): Promise<void> =>
    invoke("reorder_favorites", { ids }),
  /** Relocation preserves the favorite id used by existing Finder menus. */
  relocateFavorite: (id: string, path: string): Promise<void> =>
    invoke("relocate_favorite", { id, path }),
  /** Removing a bookmark never deletes its directory. */
  removeFavorite: (id: string): Promise<void> =>
    invoke("remove_favorite", { id }),
  /** Open the extension authorization page without changing system permissions. */
  openExtensionSettings: (): Promise<void> => invoke("open_extension_settings"),
  /** Login approvals belong to the system, not a stored checkbox value. */
  openLoginSettings: (): Promise<void> => invoke("open_login_settings"),
  /** Re-publish the database projection consumed by the Finder extension. */
  refreshMenu: (): Promise<{ revision: number }> =>
    invoke("refresh_menu_snapshot"),
  /** Explicit quit bypasses the preference to stay resident in the background. */
  quit: (): Promise<void> => invoke("quit_app"),
  /** Return ownership of the event listener so the settings window can release it. */
  onSettingsChanged: (callback: () => void): Promise<UnlistenFn> =>
    listen("settings_changed", callback),
};
