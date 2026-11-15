<script setup lang="ts">
import { computed } from "vue";
import SettingSwitch from "./SettingSwitch.vue";
import type { AppState, LaunchPreference, Settings } from "../types";
const props = defineProps<{
  settings?: Settings;
  loginStatus?: AppState["loginStatus"];
  busy: boolean;
}>();
const emit = defineEmits<{
  change: [key: LaunchPreference, checked: boolean];
  openLoginSettings: [];
}>();
const loginDescription = computed(() => {
  switch (props.loginStatus) {
    case "requiresApproval":
      return "需要在系统登录项设置中确认。";
    case "enabled":
      return "已启用；登录后静默启动，不弹设置窗口。";
    case "unavailable":
      return "尚未注册或暂时无法读取；开启时由系统检查授权。";
    default:
      return "登录后静默启动，默认关闭。";
  }
});
</script>

<template>
  <div class="section-heading">
    <h2>启动与后台</h2>
    <span>按需出现，平时安静</span>
  </div>
  <section class="card menu-list">
    <SettingSwitch
      label="显示菜单栏图标"
      glyph="◈"
      description="开启后，可从屏幕顶部打开设置或退出。"
      :checked="settings?.showMenuBarIcon ?? false"
      :disabled="busy || !settings"
      @change="emit('change', 'showMenuBarIcon', $event)"
    />
    <SettingSwitch
      label="随系统启动"
      glyph="↗"
      :description="loginDescription"
      :checked="loginStatus === 'enabled' || loginStatus === 'requiresApproval'"
      :disabled="busy || !settings"
      @change="emit('change', 'startAtLogin', $event)"
    />
    <div v-if="loginStatus === 'requiresApproval'" class="login-help">
      <button :disabled="busy" @click="emit('openLoginSettings')">
        打开登录项设置 ↗
      </button>
    </div>
    <SettingSwitch
      label="关闭设置后在后台静默运行"
      glyph="◌"
      description="默认开启，不占 Dock。关闭此项后退出，右键使用时按需启动。"
      :checked="settings?.runInBackground ?? true"
      :disabled="busy || !settings"
      @change="emit('change', 'runInBackground', $event)"
    />
  </section>
</template>
