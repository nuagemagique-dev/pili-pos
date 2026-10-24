<script setup lang="ts">
// Courses for Settings ("Service en plusieurs temps", manager only: the
// engine refuses the commands to anyone else, and SettingsView renders
// the panel for a manager session only). A switch and, per category, the
// course its products take (0 "Direct" to 3 "Desserts"). Kitchen
// printing only: nothing here reaches the signed data.
import { computed, onMounted, ref } from 'vue'
import * as ipc from '../api/ipc'
import type { Category } from '../types'
import { useI18n } from '../i18n'
import { usePosStore } from '../stores/pos'
import { describeError } from '../utils/errorMessages'

const { t } = useI18n()
const pos = usePosStore()

const COURSES = [0, 1, 2, 3]

const categories = ref<Category[]>([])
const stored = ref<ipc.CourseSettings | null>(null)
const enabled = ref(false)
const mapping = ref<Record<number, number>>({})
const saved = ref(false)
const busy = ref(false)
const error = ref<string | null>(null)

function adopt(s: ipc.CourseSettings) {
  stored.value = s
  enabled.value = s.enabled
  mapping.value = Object.fromEntries(s.category_courses.map((c) => [c.category_id, c.course]))
}

function draft(): ipc.CourseSettings {
  return {
    enabled: enabled.value,
    category_courses: categories.value
      .map((c) => ({ category_id: c.id, course: mapping.value[c.id] ?? 0 }))
      .filter((c) => c.course > 0),
  }
}

onMounted(async () => {
  try {
    const [settings, cats] = await Promise.all([
      ipc.getCourseSettings(),
      ipc.getCategories(pos.language),
    ])
    categories.value = cats.filter((c) => c.active)
    adopt(settings)
  } catch (e) {
    error.value = describeError(e)
  }
})

const dirty = computed(
  () => Boolean(stored.value) && JSON.stringify(draft()) !== JSON.stringify(stored.value),
)

function setCourse(categoryId: number, course: number) {
  mapping.value = { ...mapping.value, [categoryId]: course }
  saved.value = false
}

function toggle() {
  enabled.value = !enabled.value
  saved.value = false
}

async function save() {
  error.value = null
  busy.value = true
  try {
    adopt(await ipc.setCourseSettings(draft()))
    saved.value = true
  } catch (e) {
    error.value = describeError(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="courses-panel" data-testid="courses-panel">
    <div class="panel-head">
      <h2>{{ t('courses_title') }}</h2>
      <button
        v-if="stored"
        class="toggle"
        :class="{ on: enabled }"
        data-testid="courses-enabled"
        role="switch"
        :aria-checked="enabled"
        @click="toggle"
      >
        <span>{{ enabled ? t('courses_state_on') : t('courses_state_off') }}</span>
        <span class="track"><span class="thumb"></span></span>
      </button>
    </div>

    <p v-if="error" class="call-error" role="alert" data-testid="courses-error">{{ error }}</p>
    <p class="panel-note">{{ t('courses_note') }}</p>

    <template v-if="stored">
      <span class="section-label">{{ t('courses_categories') }}</span>
      <p v-if="categories.length === 0" class="panel-note">{{ t('courses_no_categories') }}</p>
      <div
        v-for="cat in categories"
        :key="cat.id"
        class="row"
        :data-testid="`courses-cat-${cat.id}`"
      >
        <span class="cat-name">{{ cat.name }}</span>
        <div class="segmented">
          <button
            v-for="c in COURSES"
            :key="c"
            class="seg-btn"
            :class="{ active: (mapping[cat.id] ?? 0) === c }"
            :data-testid="`courses-cat-${cat.id}-${c}`"
            @click="setCourse(cat.id, c)"
          >
            {{ t(`course_${c}`) }}
          </button>
        </div>
      </div>

      <div class="actions">
        <button
          class="act save"
          :disabled="busy || !dirty"
          data-testid="courses-save"
          @click="save"
        >
          {{ t('courses_save') }}
        </button>
      </div>
      <p v-if="saved && !dirty" class="state-ok" data-testid="courses-saved">
        {{ t('courses_saved') }}
      </p>
    </template>
  </section>
</template>

<style scoped>
.courses-panel h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
}
.panel-note {
  font-size: 15px;
  color: var(--text-secondary);
}
.call-error {
  font-size: 15px;
  color: var(--accent-danger);
  overflow-wrap: anywhere;
}
.state-ok {
  color: var(--accent-success);
}
.section-label {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-shrink: 0;
  min-height: 56px;
  padding: 0 8px 0 16px;
  border-radius: 28px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.toggle.on {
  color: var(--text-primary);
}
.toggle:active {
  transform: scale(var(--tap-scale));
}
.track {
  width: 64px;
  height: 40px;
  border-radius: 20px;
  background: var(--border);
  position: relative;
  display: block;
  flex-shrink: 0;
}
.on .track {
  background: var(--accent-success);
}
.thumb {
  position: absolute;
  left: 4px;
  top: 4px;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--text-secondary);
  transition: transform var(--tap-duration) ease;
}
.on .thumb {
  transform: translateX(24px);
  background: var(--text-primary);
}

.row {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
  font-size: 16px;
}
.cat-name {
  color: var(--text-secondary);
}
.segmented {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.seg-btn {
  height: 48px;
  font-size: 15px;
  border-radius: var(--radius-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.seg-btn.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.seg-btn:active {
  transform: scale(var(--tap-scale));
}

.actions {
  display: flex;
  gap: 8px;
}
.act {
  flex: 1;
  height: 56px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.act.save {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.act:active {
  transform: scale(var(--tap-scale));
}
.act:disabled {
  opacity: 0.4;
}
</style>
