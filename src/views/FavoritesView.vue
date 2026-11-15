<script setup lang="ts">
import { ref } from "vue";
import { useAppState } from "../composables/useAppState";
import { desktopApi } from "../services/desktop";
import { moveIds, type Favorite } from "../types";

const { state, busy, message, run } = useAppState();
const editing = ref<string>();
const draft = ref("");

/** A cancelled native folder chooser never becomes a favorite path. */
async function add(): Promise<void> {
  await run(async () => {
    const path = await desktopApi.chooseDirectory();
    if (path) {
      const result = await desktopApi.addFavorite(path);
      if (!result.created) {
        message.value = "这个目录已经在收藏中。";
      }
    }
  });
}
/** Keep the draft available when the backend rejects a rename. */
async function rename(item: Favorite): Promise<void> {
  if (await run(() => desktopApi.renameFavorite(item.id, draft.value))) {
    editing.value = undefined;
  }
}
/** Reorder by stable ids while leaving the rendered snapshot untouched until saved. */
async function move(item: Favorite, delta: number): Promise<void> {
  if (state.value) {
    const ids = moveIds(state.value.favorites, item.id, delta);
    await run(() => desktopApi.reorderFavorites(ids));
  }
}
/** Rebind the existing bookmark only after an explicit directory selection. */
async function relocate(item: Favorite): Promise<void> {
  await run(async () => {
    const path = await desktopApi.chooseDirectory();
    if (path) {
      await desktopApi.relocateFavorite(item.id, path);
    }
  });
}
</script>

<template>
  <div class="section-heading">
    <h2>你的常用目录</h2>
    <button class="primary" :disabled="busy || !state" @click="add">
      ＋ 添加目录
    </button>
  </div>
  <section class="card favorite-list">
    <article
      v-for="(item, index) in state?.favorites"
      :key="item.id"
      class="favorite-row"
    >
      <span class="folder-icon">▰</span>
      <div class="favorite-info">
        <form v-if="editing === item.id" @submit.prevent="rename(item)">
          <input v-model="draft" aria-label="收藏名称" autofocus /><button
            :disabled="busy"
          >
            保存</button
          ><button type="button" @click="editing = undefined">取消</button>
        </form>
        <button
          v-else
          class="favorite-name"
          :disabled="busy"
          @click="run(() => desktopApi.openFavorite(item.id))"
        >
          {{ item.displayName }} ↗</button
        ><small :title="item.path">{{ item.path }}</small>
      </div>
      <div class="row-actions">
        <button
          title="上移"
          :disabled="busy || index === 0"
          @click="move(item, -1)"
        >
          ↑</button
        ><button
          title="下移"
          :disabled="busy || index === (state?.favorites.length ?? 0) - 1"
          @click="move(item, 1)"
        >
          ↓</button
        ><button
          :disabled="busy"
          @click="
            editing = item.id;
            draft = item.displayName;
          "
        >
          改名</button
        ><button :disabled="busy" @click="relocate(item)">定位</button
        ><button
          class="danger"
          :disabled="busy"
          @click="
            run(
              () => desktopApi.removeFavorite(item.id),
              '已移除收藏，文件夹保持原样。',
            )
          "
        >
          移除
        </button>
      </div>
    </article>
    <div v-if="!state?.favorites.length" class="empty">
      <span>☆</span>
      <h3>把常去的地方放在这里</h3>
      <p>添加一个目录，或在 Finder 中右键<br />选择“添加到常用目录”。</p>
    </div>
  </section>
  <p class="footnote">
    可在 Finder 的“常用目录”子菜单直接打开。移除收藏不会删除实际文件夹。
  </p>
</template>
