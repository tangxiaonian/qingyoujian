<script setup lang="ts">
import { computed } from "vue";
import ArchiveHelp from "../components/ArchiveHelp.vue";
import LaunchPreferences from "../components/LaunchPreferences.vue";
import MenuPreferences from "../components/MenuPreferences.vue";
import { useAppState } from "../composables/useAppState";
import { desktopApi } from "../services/desktop";
import type { LaunchPreference, Settings } from "../types";

const { state, busy, isDesktop, run } = useAppState();
const status = computed(() =>
  state.value?.extensionStatus === "enabled"
    ? "Finder 扩展已启用"
    : state.value?.extensionStatus === "disabled"
      ? "Finder 扩展未启用"
      : "扩展状态待检查",
);

/** Clone settings before editing and derive login registration from the OS status. */
function settingsPatch(): Settings {
  if (!state.value) {
    throw new Error("配置尚未加载。");
  }
  return {
    ...state.value.settings,
    startAtLogin:
      state.value.loginStatus === "enabled" ||
      state.value.loginStatus === "requiresApproval",
    menuItems: state.value.settings.menuItems.map((item) => ({ ...item })),
  };
}
/** Persist a menu choice without mutating the current backend snapshot. */
async function toggleMenu(id: string, checked: boolean): Promise<void> {
  const patch = settingsPatch();
  patch.menuItems = patch.menuItems.map((item) =>
    item.id === id ? { ...item, enabled: checked } : item,
  );
  await run(() => desktopApi.updateSettings(patch));
}
/** Persist launch behavior through the same native path as all other settings. */
async function toggleLaunch(
  key: LaunchPreference,
  checked: boolean,
): Promise<void> {
  const patch = settingsPatch();
  patch[key] = checked;
  await run(() => desktopApi.updateSettings(patch));
}
</script>

<template>
  <section class="connection">
    <span class="status-symbol">↗</span>
    <div>
      <strong>{{ status }}</strong>
      <p>启用一次，之后在 Finder 中直接右键。</p>
    </div>
    <button
      :disabled="busy || !state"
      @click="run(desktopApi.openExtensionSettings)"
    >
      打开系统设置 ↗
    </button>
  </section>
  <LaunchPreferences
    :settings="state?.settings"
    :login-status="state?.loginStatus"
    :busy="busy"
    @change="toggleLaunch"
    @open-login-settings="run(desktopApi.openLoginSettings)"
  />
  <MenuPreferences
    :items="state?.settings.menuItems"
    :busy="busy"
    :is-desktop="isDesktop"
    @change="toggleMenu"
  />
  <ArchiveHelp />
  <div class="two-col">
    <section class="card compact">
      <h2>新建文件</h2>
      <p>文本与真实 Office 模板，随手创建。</p>
      <div class="chips">
        <span
          v-for="type in state?.fileTypes ?? [
            { id: 'txt', displayName: 'TXT' },
            { id: 'md', displayName: 'Markdown' },
            { id: 'word', displayName: 'Word' },
            { id: 'excel', displayName: 'Excel' },
          ]"
          :key="type.id"
          >{{ type.displayName }}</span
        >
      </div>
    </section>
    <section class="card compact">
      <h2>默认终端</h2>
      <div class="terminal-option">
        <b>›_</b><span>系统 Terminal<small>直接进入右键目标目录</small></span>
      </div>
      <p>更多终端将在后续版本提供。</p>
    </section>
  </div>
  <p class="footnote">
    隐藏文件切换即时生效，无需重启
    Finder。首次使用需授予轻右键辅助功能权限；也可在 Finder 中直接按 ⌘⇧.。
  </p>
</template>
