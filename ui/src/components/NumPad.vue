<script setup lang="ts">
defineProps<{
  modelValue: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  submit: [];
}>();

function press(digit: string) { emit("update:modelValue", digit); }
function backspace() { emit("update:modelValue", "BACKSPACE"); }
function clear() { emit("update:modelValue", "CLEAR"); }
</script>

<template>
  <!-- UX Guide §4.1: NumPad keys 96×72px recommended -->
  <div class="numpad">
    <button v-for="d in ['1','2','3','4','5','6','7','8','9']" :key="d" class="key" @click="press(d)">{{ d }}</button>
    <button class="key key-clear" @click="clear">C</button>
    <button class="key" @click="press('0')">0</button>
    <button class="key key-back" @click="backspace">&#9003;</button>
  </div>
</template>

<style scoped>
.numpad {
  display: grid;
  grid-template-columns: repeat(3, 96px);
  gap: var(--space-xs);
}

.key {
  height: 72px;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}

.key:active {
  transform: scale(var(--tap-scale));
  background: var(--bg-pressed);
}

.key-clear { color: var(--accent-warning); }
.key-back { color: var(--accent-danger); }
</style>
