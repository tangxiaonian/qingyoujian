<script setup lang="ts">
defineProps<{
  label: string;
  glyph: string;
  description?: string;
  checked: boolean;
  disabled: boolean;
}>();
const emit = defineEmits<{ change: [checked: boolean] }>();

/** Keep the checkbox controlled by saved state while the native update is pending or fails. */
function change(event: Event, savedValue: boolean): void {
  const input = event.target as HTMLInputElement;
  emit("change", input.checked);
  input.checked = savedValue;
}
</script>

<template>
  <label class="menu-row">
    <span class="item-icon">{{ glyph }}</span>
    <span :class="{ 'preference-label': description }">
      {{ label }}<small v-if="description">{{ description }}</small>
    </span>
    <input
      type="checkbox"
      role="switch"
      :checked="checked"
      :disabled="disabled"
      @change="change($event, checked)"
    />
  </label>
</template>
