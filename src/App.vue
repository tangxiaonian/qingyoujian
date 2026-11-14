<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  errorText,
  menuLabels,
  menuGlyphs,
  moveIds,
  type AppState,
  type Favorite,
  type Settings,
} from "./types";

const tab = ref<"general" | "favorites" | "about">("general");
const state = ref<AppState>();
const message = ref("");
const busy = ref(false);
const editing = ref<string>();
const draft = ref("");
let unlisten: UnlistenFn | undefined;
const titles = {
  general: ["顺手，就够了。", "把常用操作，留在右键里。"],
  favorites: ["常去的地方，一步到。", "收藏目录，不再层层查找。"],
  about: ["小工具，少一点负担。", "本机运行 · 无账号 · 无订阅"],
};
const status = computed(() =>
  state.value?.extensionStatus === "enabled"
    ? "Finder 扩展已启用"
    : state.value?.extensionStatus === "disabled"
      ? "Finder 扩展未启用"
      : "扩展状态待检查",
);

/** Always reload from Rust; the UI is not a second persistence store. */
async function reload(): Promise<void> {
  state.value = await invoke<AppState>("get_app_state");
}
/** Serialize visible UI mutations and preserve useful native errors. */
async function run(work: () => Promise<unknown>, success = ""): Promise<void> {
  if (busy.value) {
    return;
  }
  busy.value = true;
  message.value = "";
  try {
    await work();
    await reload();
    message.value = success;
  } catch (error) {
    message.value = errorText(error);
  } finally {
    busy.value = false;
  }
}
/** A selected directory is an explicit user choice, not a path typed into a shell. */
async function add(): Promise<void> {
  await run(async () => {
    const path = await invoke<string | null>("choose_directory");
    if (path) {
      const result = await invoke<{ created: boolean }>("add_favorite", {
        path,
      });
      if (!result.created) {
        message.value = "这个目录已经在收藏中。";
      }
    }
  });
}
/** Update a bookmark label without touching the actual directory. */
async function rename(item: Favorite): Promise<void> {
  await run(() =>
    invoke("rename_favorite", { id: item.id, displayName: draft.value }),
  );
  editing.value = undefined;
}
/** Reordering sends the complete current id list for atomic backend validation. */
async function move(item: Favorite, delta: number): Promise<void> {
  if (state.value) {
    await run(() =>
      invoke("reorder_favorites", {
        ids: moveIds(state.value!.favorites, item.id, delta),
      }),
    );
  }
}
/** Relocation retains the favorite id so already-open menus stay valid. */
async function relocate(item: Favorite): Promise<void> {
  await run(async () => {
    const path = await invoke<string | null>("choose_directory");
    if (path) {
      await invoke("relocate_favorite", { id: item.id, path });
    }
  });
}
/** Submit a cloned settings value so failed changes do not falsely appear saved. */
async function toggle(id: string, event: Event): Promise<void> {
  if (!state.value) {
    return;
  }
  const checked = (event.target as HTMLInputElement).checked;
  const patch = settingsPatch();
  patch.menuItems = patch.menuItems.map((item) =>
    item.id === id ? { ...item, enabled: checked } : item,
  );
  await run(() => invoke("update_settings", { patch }));
  await reload();
}

/** Clone backend state and use the OS login status as the effective checkbox value. */
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
/** Changes to launch behavior are saved immediately; registration failures remain visible. */
async function toggleLaunch(
  key: "showMenuBarIcon" | "startAtLogin" | "runInBackground",
  event: Event,
): Promise<void> {
  const patch = settingsPatch();
  patch[key] = (event.target as HTMLInputElement).checked;
  await run(() => invoke("update_settings", { patch }));
  await reload();
}

onMounted(async () => {
  if (!isTauri()) {
    message.value = "当前是浏览器预览；本机操作仅在轻右键应用中可用。";
    return;
  }
  try {
    await reload();
    unlisten = await listen("settings_changed", () => {
      void reload();
    });
  } catch (error) {
    message.value = errorText(error);
  }
});
onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-mark">↗</span>
        <div><strong>轻右键</strong><small>QING YOU JIAN</small></div>
      </div>
      <nav aria-label="设置分类">
        <button :class="{ active: tab === 'general' }" @click="tab = 'general'">
          <span>⌘</span>通用设置
        </button>
        <button
          :class="{ active: tab === 'favorites' }"
          @click="tab = 'favorites'"
        >
          <span>☆</span>常用目录
          <em v-if="state">{{ state.favorites.length }}</em>
        </button>
        <button :class="{ active: tab === 'about' }" @click="tab = 'about'">
          <span>ⓘ</span>关于轻右键
        </button>
      </nav>
      <div class="sidebar-bottom">
        <i></i>只在本机，好用就行。<small
          >VERSION {{ state?.version ?? "0.1.0" }}</small
        >
      </div>
    </aside>
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
      <template v-if="tab === 'general'">
        <section class="connection">
          <span class="status-symbol">↗</span>
          <div>
            <strong>{{ status }}</strong>
            <p>启用一次，之后在 Finder 中直接右键。</p>
          </div>
          <button
            :disabled="busy || !state"
            @click="run(() => invoke('open_extension_settings'))"
          >
            打开系统设置 ↗
          </button>
        </section>
        <div class="section-heading">
          <h2>启动与后台</h2>
          <span>按需出现，平时安静</span>
        </div>
        <section class="card menu-list">
          <label class="menu-row"
            ><span class="item-icon">◈</span
            ><span class="preference-label"
              >显示菜单栏图标<small
                >开启后，可从屏幕顶部打开设置或退出。</small
              ></span
            ><input
              type="checkbox"
              role="switch"
              :checked="state?.settings.showMenuBarIcon ?? false"
              :disabled="busy || !state"
              @change="toggleLaunch('showMenuBarIcon', $event)"
          /></label>
          <label class="menu-row"
            ><span class="item-icon">↗</span
            ><span class="preference-label"
              >随系统启动<small>{{
                state?.loginStatus === "requiresApproval"
                  ? "需要在系统登录项设置中确认。"
                  : state?.loginStatus === "enabled"
                    ? "已启用；登录后静默启动，不弹设置窗口。"
                    : state?.loginStatus === "unavailable"
                      ? "尚未注册或暂时无法读取；开启时由系统检查授权。"
                      : "登录后静默启动，默认关闭。"
              }}</small></span
            ><input
              type="checkbox"
              role="switch"
              :checked="
                state?.loginStatus === 'enabled' ||
                state?.loginStatus === 'requiresApproval'
              "
              :disabled="busy || !state"
              @change="toggleLaunch('startAtLogin', $event)"
          /></label>
          <div
            v-if="state?.loginStatus === 'requiresApproval'"
            class="login-help"
          >
            <button @click="run(() => invoke('open_login_settings'))">
              打开登录项设置 ↗
            </button>
          </div>
          <label class="menu-row"
            ><span class="item-icon">◌</span
            ><span class="preference-label"
              >关闭设置后在后台静默运行<small
                >默认开启，不占
                Dock。关闭此项后退出，右键使用时按需启动。</small
              ></span
            ><input
              type="checkbox"
              role="switch"
              :checked="state?.settings.runInBackground ?? true"
              :disabled="busy || !state"
              @change="toggleLaunch('runInBackground', $event)"
          /></label>
        </section>
        <div class="section-heading">
          <h2>右键菜单</h2>
          <span>按你的习惯取舍</span>
        </div>
        <section class="card menu-list">
          <label
            v-for="item in state?.settings.menuItems"
            :key="item.id"
            class="menu-row"
            ><span class="item-icon">{{ menuGlyphs[item.id] }}</span
            ><span>{{ menuLabels[item.id] }}</span
            ><input
              type="checkbox"
              role="switch"
              :checked="item.enabled"
              :disabled="busy"
              @change="toggle(item.id, $event)"
          /></label>
          <p v-if="!state" class="empty-inline">
            {{
              isTauri() ? "正在读取本机配置…" : "请在桌面应用中查看菜单设置。"
            }}
          </p>
        </section>
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
              <b>›_</b
              ><span>系统 Terminal<small>直接进入右键目标目录</small></span>
            </div>
            <p>更多终端将在后续版本提供。</p>
          </section>
        </div>
        <p class="footnote">
          隐藏文件切换对所有 Finder 窗口生效；需要重启 Finder
          时，会先征求你的确认。
        </p>
      </template>
      <template v-else-if="tab === 'favorites'">
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
                ><button type="button" @click="editing = undefined">
                  取消
                </button>
              </form>
              <button
                v-else
                class="favorite-name"
                :disabled="busy"
                @click="run(() => invoke('open_favorite', { id: item.id }))"
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
                    () => invoke('remove_favorite', { id: item.id }),
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
      <template v-else>
        <section class="card about">
          <span class="brand-mark large">↗</span>
          <h2>
            轻右键 <small>{{ state?.version ?? "0.1.0" }}</small>
          </h2>
          <p>常用操作，少走几步。</p>
          <p>
            拷贝路径 · 新建文件 · 打开终端<br />显示隐藏文件 · 常用目录<br />移到废纸篓
            · 复制到 · 剪切到
          </p>
          <hr />
          <p>
            离线运行，不需要账号，不收集使用数据。<br />设置窗口关闭后释放网页界面，可选择后台静默驻留。
          </p>
          <p class="muted">
            基于 Tauri、Vue 与 macOS Finder Sync。<br />部分机制和图标参考
            nikgrbn/finder-menu，遵循 MIT 许可。<br />Word / Excel
            空白模板由项目发起者提供。
          </p>
          <button :disabled="!state" @click="invoke('quit_app')">
            退出轻右键
          </button>
        </section>
      </template>
      <footer>
        <span>更改在下次右键时生效</span
        ><button
          :disabled="busy || !state"
          @click="run(() => invoke('refresh_menu_snapshot'), '菜单已刷新。')"
        >
          ↻ 刷新菜单
        </button>
      </footer>
    </main>
  </div>
</template>
