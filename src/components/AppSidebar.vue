<script setup lang="ts">
import type { SettingsTab } from "../types";
defineProps<{
  activeTab: SettingsTab;
  version?: string;
  favoriteCount?: number;
}>();
const emit = defineEmits<{ navigate: [tab: SettingsTab] }>();
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-mark">↗</span>
      <div><strong>轻右键</strong><small>QING YOU JIAN</small></div>
    </div>
    <nav aria-label="设置分类">
      <button
        :class="{ active: activeTab === 'general' }"
        @click="emit('navigate', 'general')"
      >
        <span>⌘</span>通用设置
      </button>
      <button
        :class="{ active: activeTab === 'favorites' }"
        @click="emit('navigate', 'favorites')"
      >
        <span>☆</span>常用目录
        <em v-if="favoriteCount !== undefined">{{ favoriteCount }}</em>
      </button>
      <button
        :class="{ active: activeTab === 'about' }"
        @click="emit('navigate', 'about')"
      >
        <span>ⓘ</span>关于轻右键
      </button>
    </nav>
    <div class="sidebar-bottom">
      <i></i>只在本机，好用就行。<small>VERSION {{ version ?? "0.1.0" }}</small>
    </div>
  </aside>
</template>
