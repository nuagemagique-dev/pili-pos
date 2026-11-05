<script setup lang="ts">
// Quick notes for Settings ("Notes rapides", manager only: the engine refuses the write to anyone
// else, and SettingsView renders the panel for a manager session only). The chips of the kitchen
// note dialog: add, remove, reorder, then save. Local data, never fiscal.
import { computed, onMounted, ref } from 'vue'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'
import {
  addQuickNote,
  moveQuickNote,
  NOTE_MAX,
  QUICK_NOTES_MAX,
  removeQuickNote,
} from '../utils/quickNotes'
import TextKeypad from './TextKeypad.vue'

const { t } = useI18n()

const stored = ref<string[] | null>(null)
const notes = ref<string[]>([])
const adding = ref(false)
const saved = ref(false)
const busy = ref(false)
const error = ref<string | null>(null)

function adopt(list: string[]) {
  stored.value = list
  notes.value = [...list]
}

onMounted(async () => {
  try {
    adopt((await ipc.getQuickNotes()).notes)
  } catch (e) {
    error.value = describeError(e)
  }
})

const dirty = computed(
  () => stored.value !== null && JSON.stringify(notes.value) !== JSON.stringify(stored.value),
)
const full = computed(() => notes.value.length >= QUICK_NOTES_MAX)

function change(next: string[]) {
  notes.value = next
  saved.value = false
}

function onAdd(text: string) {
  adding.value = false
  const next = addQuickNote(notes.value, text)
  if (next) change(next)
}

async function save() {
  error.value = null
  busy.value = true
  try {
    adopt((await ipc.setQuickNotes({ notes: notes.value })).notes)
    saved.value = true
  } catch (e) {
    error.value = describeError(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="quick-notes-panel" data-testid="quick-notes-panel">
    <h2>{{ t('quick_notes_title') }}</h2>

    <p v-if="error" class="call-error" role="alert" data-testid="quick-notes-error">{{ error }}</p>
    <p class="panel-note">{{ t('quick_notes_note') }}</p>

    <template v-if="stored">
      <p v-if="notes.length === 0" class="panel-note" data-testid="quick-notes-empty">
        {{ t('quick_notes_empty') }}
      </p>
      <div v-for="(note, i) in notes" :key="note" class="row" :data-testid="`quick-note-${i}`">
        <span class="note-text" data-testid="quick-note-text">{{ note }}</span>
        <button
          class="icon-btn"
          :disabled="i === 0"
          :aria-label="t('quick_notes_up')"
          data-testid="quick-note-up"
          @click="change(moveQuickNote(notes, i, -1))"
        >
          ▲
        </button>
        <button
          class="icon-btn"
          :disabled="i === notes.length - 1"
          :aria-label="t('quick_notes_down')"
          data-testid="quick-note-down"
          @click="change(moveQuickNote(notes, i, 1))"
        >
          ▼
        </button>
        <button
          class="icon-btn remove"
          :aria-label="t('quick_notes_remove')"
          data-testid="quick-note-remove"
          @click="change(removeQuickNote(notes, i))"
        >
          ✕
        </button>
      </div>

      <p v-if="full" class="panel-note" data-testid="quick-notes-full">
        {{ t('quick_notes_full', { max: QUICK_NOTES_MAX }) }}
      </p>
      <div class="actions">
        <button class="act" :disabled="full" data-testid="quick-notes-add" @click="adding = true">
          {{ t('quick_notes_add') }}
        </button>
        <button
          class="act save"
          :disabled="busy || !dirty"
          data-testid="quick-notes-save"
          @click="save"
        >
          {{ t('quick_notes_save') }}
        </button>
      </div>
      <p v-if="saved && !dirty" class="state-ok" data-testid="quick-notes-saved">
        {{ t('quick_notes_saved') }}
      </p>
    </template>

    <TextKeypad
      v-if="adding"
      :title="t('quick_notes_add_title')"
      :max-len="NOTE_MAX"
      :symbols="['é', 'è', 'à', 'ç', 'ê', ',', '.', '-', '\'', '/', '(', ')', '+', '%']"
      space
      show-counter
      :valid="(v: string) => addQuickNote(notes, v) !== null"
      @confirm="onAdd"
      @cancel="adding = false"
    />
  </section>
</template>

<style scoped>
.quick-notes-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.quick-notes-panel h2 {
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
.state-ok {
  color: var(--accent-success);
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 56px;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
}
.note-text {
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
  color: var(--text-primary);
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
