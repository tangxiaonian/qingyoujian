<script setup lang="ts">
import { ref } from "vue";
import AppSidebar from "./components/AppSidebar.vue";
import { provideAppState } from "./composables/useAppState";
import { desktopApi } from "./services/desktop";
import GeneralView from "./views/GeneralView.vue";
import FavoritesView from "./views/FavoritesView.vue";
import AboutView from "./views/AboutView.vue";
import type { SettingsTab } from "./types";

const tab = ref<SettingsTab>("general");
const { state, message, busy, run } = provideAppState();
const titles = {
  general: ["顺手，就够了。", "把常用操作，留在右键里。"],
  favorites: ["常去的地方，一步到。", "收藏目录，不再层层查找。"],
  about: ["小工具，少一点负担。", "本机运行 · 无账号 · 无订阅"],
};
</script>

<template>
  <div class="shell">
    <AppSidebar
      :active-tab="tab"
      :version="state?.version"
      :favorite-count="state?.favorites.length"
      @navigate="tab = $event"
    />
    <main>
      <header>
        <div class="eyebrow">FINDER, A LITTLE BETTER.</div>
        <h1>{{ titles[tab][0] }}</h1>
        <p>{{ titles[tab][1] }}</p>
      </header>
      <div v-if="message" role="status" class="notice">
        {{ message
        }}<button aria-label="关闭提示" @click="message = ''">×</button>
      </div>
      <GeneralView v-if="tab === 'general'" />
      <FavoritesView v-else-if="tab === 'favorites'" />
      <AboutView v-else />
      <footer>
        <span>更改在下次右键时生效</span>
        <button
          :disabled="busy || !state"
          @click="run(desktopApi.refreshMenu, '菜单已刷新。')"
        >
          ↻ 刷新菜单
        </button>
      </footer>
    </main>
  </div>
</template>
