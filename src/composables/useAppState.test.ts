import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createRenderer, defineComponent } from "vue";
import { createAppState } from "./useAppState";
import { desktopApi } from "../services/desktop";
import type { AppState } from "../types";

vi.mock("../services/desktop", () => ({
  desktopApi: {
    isAvailable: vi.fn(),
    getState: vi.fn(),
    onSettingsChanged: vi.fn(),
  },
}));

const initial: AppState = {
  version: "0.1.0",
  extensionStatus: "enabled",
  loginStatus: "disabled",
  settings: {
    terminalBundleId: "com.apple.Terminal",
    menuItems: [],
    showMenuBarIcon: false,
    startAtLogin: false,
    runInBackground: true,
  },
  fileTypes: [],
  favorites: [],
};
const detach = vi.fn();
let unmount: (() => void) | undefined;

/** Mount Vue lifecycle hooks without a DOM dependency; this host renders only a comment. */
function mountController() {
  const renderer = createRenderer<object, object>({
    createElement: () => ({}),
    createComment: () => ({}),
    createText: () => ({}),
    insert: () => undefined,
    remove: () => undefined,
    patchProp: () => undefined,
    setText: () => undefined,
    setElementText: () => undefined,
    parentNode: () => null,
    nextSibling: () => null,
  });
  let controller!: ReturnType<typeof createAppState>;
  const app = renderer.createApp(
    defineComponent({
      setup() {
        controller = createAppState();
        return () => null;
      },
    }),
  );
  app.mount({});
  unmount = () => app.unmount();
  return controller;
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(desktopApi.isAvailable).mockReturnValue(true);
  vi.mocked(desktopApi.getState).mockResolvedValue(structuredClone(initial));
  vi.mocked(desktopApi.onSettingsChanged).mockResolvedValue(detach);
});
afterEach(() => {
  unmount?.();
  unmount = undefined;
});

describe("settings window state", () => {
  it("refreshes Finder-originated edits, surfaces refresh failures and releases its listener", async () => {
    const controller = mountController();
    await vi.waitFor(() =>
      expect(desktopApi.onSettingsChanged).toHaveBeenCalledOnce(),
    );
    const changed = vi.mocked(desktopApi.onSettingsChanged).mock.calls[0][0];
    vi.mocked(desktopApi.getState).mockResolvedValue({
      ...initial,
      version: "0.1.1",
    });
    changed();
    await vi.waitFor(() =>
      expect(controller.state.value?.version).toBe("0.1.1"),
    );
    vi.mocked(desktopApi.getState).mockRejectedValue({
      message: "配置暂时不可读",
    });
    changed();
    await vi.waitFor(() =>
      expect(controller.message.value).toBe("配置暂时不可读"),
    );
    expect(controller.state.value?.version).toBe("0.1.1");
    unmount?.();
    unmount = undefined;
    expect(detach).toHaveBeenCalledOnce();
  });

  it("keeps saved state after a rejected mutation and allows a later successful retry", async () => {
    const controller = mountController();
    await vi.waitFor(() => expect(controller.state.value).toEqual(initial));
    const saved = controller.state.value;
    expect(
      await controller.run(async () => {
        throw { message: "登录项未获批准" };
      }),
    ).toBe(false);
    expect(controller.state.value).toBe(saved);
    expect(controller.message.value).toBe("登录项未获批准");
    expect(controller.busy.value).toBe(false);
    expect(await controller.run(async () => undefined, "已保存")).toBe(true);
    expect(controller.message.value).toBe("已保存");
  });

  it("runs only one native mutation at a time", async () => {
    const controller = mountController();
    await vi.waitFor(() => expect(controller.state.value).toEqual(initial));
    let finish!: () => void;
    const pending = new Promise<void>((resolve) => {
      finish = resolve;
    });
    const first = controller.run(() => pending);
    const second = vi.fn(async () => undefined);
    expect(controller.busy.value).toBe(true);
    expect(await controller.run(second)).toBe(false);
    expect(second).not.toHaveBeenCalled();
    finish();
    expect(await first).toBe(true);
    expect(controller.busy.value).toBe(false);
  });

  it("keeps browser preview from invoking native commands or subscribing to native events", () => {
    vi.mocked(desktopApi.isAvailable).mockReturnValue(false);
    const controller = mountController();
    expect(controller.message.value).toContain("浏览器预览");
    expect(desktopApi.getState).not.toHaveBeenCalled();
    expect(desktopApi.onSettingsChanged).not.toHaveBeenCalled();
  });
});
