import {
  inject,
  onMounted,
  onUnmounted,
  provide,
  ref,
  type InjectionKey,
} from "vue";
import { desktopApi } from "../services/desktop";
import { errorText, type AppState } from "../types";

/** Own one snapshot, action lock and event subscription for the entire settings window. */
export function createAppState() {
  const state = ref<AppState>();
  const busy = ref(false);
  const message = ref("");
  const isDesktop = desktopApi.isAvailable();
  let unlisten: (() => void) | undefined;

  /** Refresh from Rust after a mutation or a Finder-originated settings event. */
  async function reload(): Promise<void> {
    state.value = await desktopApi.getState();
  }

  /** Serialize mutations and reload authoritative state; failures stay visible to every view. */
  async function run(
    work: () => Promise<unknown>,
    success = "",
  ): Promise<boolean> {
    if (busy.value) {
      return false;
    }
    busy.value = true;
    message.value = "";
    try {
      await work();
      await reload();
      if (success) {
        message.value = success;
      }
      return true;
    } catch (error) {
      message.value = errorText(error);
      return false;
    } finally {
      busy.value = false;
    }
  }

  /** Event-driven refreshes report errors instead of leaving rejected promises unhandled. */
  async function refreshFromEvent(): Promise<void> {
    try {
      await reload();
    } catch (error) {
      message.value = errorText(error);
    }
  }

  onMounted(async () => {
    if (!isDesktop) {
      message.value = "当前是浏览器预览；本机操作仅在轻右键应用中可用。";
      return;
    }
    try {
      await reload();
      unlisten = await desktopApi.onSettingsChanged(() => {
        void refreshFromEvent();
      });
    } catch (error) {
      message.value = errorText(error);
    }
  });
  onUnmounted(() => {
    unlisten?.();
  });

  return { state, busy, message, isDesktop, run };
}

const appStateKey: InjectionKey<ReturnType<typeof createAppState>> =
  Symbol("app-state");

/** Install a window-scoped controller so navigation never duplicates native listeners. */
export function provideAppState() {
  const controller = createAppState();
  provide(appStateKey, controller);
  return controller;
}

/** Views consume the controller created by App.vue rather than opening their own stores. */
export function useAppState() {
  const controller = inject(appStateKey);
  if (!controller) {
    throw new Error("设置页面必须挂载在应用状态提供者内。");
  }
  return controller;
}
