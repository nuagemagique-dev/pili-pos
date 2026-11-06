<script setup lang="ts">
// Favourites for Settings ("Favoris", manager only: the engine refuses the write to anyone else, and
// SettingsView renders the panel for a manager session only). Where the first tab of the order screen
// takes its products from: the 30-day best sellers (default) or the manager's own selection, which is
// starred in the menu editor. A tap saves at once. Local data, never fiscal.
import { computed, onMounted, ref } from 'vue'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import { usePosStore } from '../stores/pos'
import { describeError } from '../utils/errorMessages'
import { MAX_SELECTION } from '../utils/favourites'

const { t } = useI18n()
const pos = usePosStore()

const settings = ref<ipc.FavouritesSettings | null>(null)
const names = ref<Map<number, string>>(new Map())
const busy = ref(false)
const error = ref<string | null>(null)

const MODES: ipc.FavouritesMode[] = ['top_sold', 'selection']

onMounted(async () => {
  try {
    settings.value = await ipc.getFavouritesSettings()
    const items = await ipc.getMenuItems(undefined, pos.language)
    names.value = new Map(items.map((i) => [i.id, i.name]))
  } catch (e) {
    error.value = describeError(e)
  }
})

const selection = computed(() => settings.value?.selection ?? [])

async function save(next: ipc.FavouritesSettings) {
  error.value = null
  busy.value = true
  try {
    settings.value = await ipc.setFavouritesSettings(next)
  } catch (e) {
    error.value = describeError(e)
  } finally {
    busy.value = false
  }
}

function setMode(mode: ipc.FavouritesMode) {
  if (!settings.value || settings.value.mode === mode) return
  void save({ mode, selection: [...settings.value.selection] })
}

function remove(id: number) {
  if (!settings.value) return
  void save({ mode: settings.value.mode, selection: selection.value.filter((x) => x !== id) })
}
</script>

<template>
  <section class="favourites-panel" data-testid="favourites-panel">
    <h2>{{ t('fav_title') }}</h2>
    <p v-if="error" class="call-error" role="alert" data-testid="favourites-error">{{ error }}</p>
    <p class="panel-note">{{ t('fav_note') }}</p>

    <template v-if="settings">
      <div class="segmented" role="radiogroup" :aria-label="t('fav_title')">
        <button
          v-for="mode in MODES"
          :key="mode"
          class="mode-btn"
          :class="{ active: settings.mode === mode }"
          role="radio"
          :aria-checked="settings.mode === mode"
          :disabled="busy"
          :data-testid="`favourites-mode-${mode}`"
          @click="setMode(mode)"
        >
          {{ t(`fav_mode_${mode}`) }}
        </button>
      </div>

      <p v-if="settings.mode === 'top_sold'" class="panel-note" data-testid="favourites-top-note">
        {{ t('fav_top_sold_note') }}
      </p>
      <template v-else>
        <p class="panel-note">{{ t('fav_selection_note') }}</p>
        <p v-if="selection.length === 0" class="panel-note" data-testid="favourites-empty">
          {{ t('fav_empty') }}
        </p>
        <div
          v-for="id in selection"
          :key="id"
          class="row"
          data-testid="favourites-row"
          :data-id="id"
        >
          <span class="row-text">{{ names.get(id) ?? `#${id}` }}</span>
          <button
            class="icon-btn remove"
            :disabled="busy"
            :aria-label="t('fav_remove')"
            data-testid="favourites-remove"
            @click="remove(id)"
          >
            ✕
          </button>
        </div>
        <p class="panel-note" data-testid="favourites-count">
          {{ t('fav_count', { n: selection.length, max: MAX_SELECTION }) }}
        </p>
      </template>
    </template>
  </section>
</template>

<style scoped>
.favourites-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.favourites-panel h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
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
.segmented {
  display: flex;
  gap: 8px;
}
.mode-btn {
  flex: 1;
  min-height: 56px;
  padding: 0 12px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.mode-btn.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.mode-btn:active {
  transform: scale(var(--tap-scale));
}
.mode-btn:disabled {
  opacity: 0.6;
}
.row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 56px;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
}
.row-text {
  flex: 1;
  min-width: 0;
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  overflow-wrap: anywhere;
}
.icon-btn {
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  font-size: 16px;
  transition: transform var(--tap-duration) ease;
}
.icon-btn:active {
  transform: scale(var(--tap-scale));
}
.icon-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}
.icon-btn.remove {
  color: var(--accent-danger);
}
</style>
