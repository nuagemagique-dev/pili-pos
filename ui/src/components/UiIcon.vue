<script setup lang="ts">
import { ICONS, type IconName } from '../utils/icons'

/** A Material icon in the colour of the text around it (`utils/icons.ts`). */
const { name, size = 24 } = defineProps<{
  name: IconName
  size?: number
}>()
</script>

<template>
  <svg
    class="ui-icon"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    aria-hidden="true"
    focusable="false"
  >
    <path :d="ICONS[name]" fill="currentColor" />
  </svg>
</template>

<style scoped>
.ui-icon {
  flex-shrink: 0;
}
</style>
