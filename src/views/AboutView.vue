<script setup lang="ts">
import { useAppState } from "../composables/useAppState";
import { desktopApi } from "../services/desktop";
const { state } = useAppState();
</script>

<template>
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
    <button :disabled="!state" @click="desktopApi.quit()">退出轻右键</button>
  </section>
</template>
