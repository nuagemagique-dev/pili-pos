<script setup lang="ts">
// The four courses of a line (0 "Direct" to 3 "Desserts") as big buttons.
// The current one is marked; the engine refuses a change once a kitchen
// slip carried the line (the caller shows the refusal).
import { useI18n } from '../i18n'

defineProps<{
  /** The line's current course. */
  current: number
}>()

const emit = defineEmits<{
  pick: [course: number]
  cancel: []
}>()

const { t } = useI18n()
const COURSES = [0, 1, 2, 3]
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div
      class="dialog"
      role="dialog"
      :aria-label="t('course_picker_title')"
      data-testid="course-picker"
    >
      <h2 class="title">{{ t('course_picker_title') }}</h2>
      <div class="choices">
        <button
          v-for="c in COURSES"
          :key="c"
          class="choice"
          :class="{ active: c === current }"
          :aria-pressed="c === current"
          :data-testid="`course-pick-${c}`"
          @click="emit('pick', c)"
        >
          {{ t(`course_${c}`) }}
        </button>
      </div>
      <button class="cancel" data-testid="course-picker-cancel" @click="emit('cancel')">
        {{ t('course_cancel') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  z-index: 50;
}
.dialog {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  width: min(560px, 92vw);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.choices {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-sm);
}
.choice,
.cancel {
  height: 72px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.choice.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.choice:active,
.cancel:active {
  transform: scale(var(--tap-scale));
}
.cancel {
  height: 56px;
}
</style>
