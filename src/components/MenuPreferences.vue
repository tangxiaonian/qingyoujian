<script setup lang="ts">
import SettingSwitch from "./SettingSwitch.vue";
import { menuLabels, menuGlyphs, type MenuItem } from "../types";
defineProps<{ items?: MenuItem[]; busy: boolean; isDesktop: boolean }>();
const emit = defineEmits<{ change: [id: string, checked: boolean] }>();
</script>

<template>
  <div class="section-heading">
    <h2>右键菜单</h2>
    <span>按你的习惯取舍</span>
  </div>
  <section class="card menu-list">
    <SettingSwitch
      v-for="item in items"
      :key="item.id"
      :label="menuLabels[item.id]!"
      :glyph="menuGlyphs[item.id]!"
      :checked="item.enabled"
      :disabled="busy"
      @change="emit('change', item.id, $event)"
    />
    <p v-if="!items" class="empty-inline">
      {{ isDesktop ? "正在读取本机配置…" : "请在桌面应用中查看菜单设置。" }}
    </p>
  </section>
</template>
