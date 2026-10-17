<script setup lang="ts">
// Badge reader panel of the Settings screen (managers only; the engine
// refuses the change to the others). The reader is chosen per
// installation: off, or a USB wedge reader (a stored built-in NFC choice
// is Android-only hardware and only gets a note here). Plan 4.4.
import { computed, onMounted, ref, watch } from "vue";
import NumPad from "./NumPad.vue";
import { useBadgeStore } from "../stores/badge";
import type { BadgeReader, BadgeSettingsInput } from "../api/ipc";
import { useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";
import { formatDayTime, formatTimestamp } from "../utils/datetime";

const badge = useBadgeStore();
const { t } = useI18n();

const error = ref<string | null>(null);
const open = ref(false);
const readerLabel = computed(() => {
  const r = badge.settings?.reader;
  return r === "off" || r === "usb" ? t(`badge_reader_${r}`) : "NFC";
});
const readers: BadgeReader[] = ["off", "usb"];

type TimingKey = "wedgeMaxGapMs" | "wedgeMinLen" | "wedgeMaxLen";
const timing: Array<{ key: TimingKey; label: string }> = [
  { key: "wedgeMaxGapMs", label: "settings_badge_gap" },
  { key: "wedgeMinLen", label: "settings_badge_min_len" },
  { key: "wedgeMaxLen", label: "settings_badge_max_len" },
];

// The numbers being typed (kept as text so the pad can edit them).
const draft = ref<Record<TimingKey, string>>({ wedgeMaxGapMs: "", wedgeMinLen: "", wedgeMaxLen: "" });
const editing = ref<TimingKey | null>(null);

function syncDraft() {
  const s = badge.settings;
  if (!s) return;
  draft.value = {
    wedgeMaxGapMs: String(s.wedgeMaxGapMs),
    wedgeMinLen: String(s.wedgeMinLen),
    wedgeMaxLen: String(s.wedgeMaxLen),
  };
}
watch(() => badge.settings, syncDraft, { immediate: true });

const dirty = computed(() => {
  const s = badge.settings;
  return (
    !!s &&
    (draft.value.wedgeMaxGapMs !== String(s.wedgeMaxGapMs) ||
      draft.value.wedgeMinLen !== String(s.wedgeMinLen) ||
      draft.value.wedgeMaxLen !== String(s.wedgeMaxLen))
  );
});

function onKey(key: string) {
  const k = editing.value;
  if (!k) return;
  const cur = draft.value[k];
  if (key === "BACKSPACE") draft.value[k] = cur.slice(0, -1);
  else if (key === "CLEAR") draft.value[k] = "";
  else if (cur.length < 3) draft.value[k] = cur + key;
}

function current(over: Partial<BadgeSettingsInput> = {}): BadgeSettingsInput {
  const s = badge.settings!;
  return {
    reader: s.reader,
    loginEnabled: s.loginEnabled,
    wedgeMaxGapMs: s.wedgeMaxGapMs,
    wedgeMinLen: s.wedgeMinLen,
    wedgeMaxLen: s.wedgeMaxLen,
    ...over,
  };
}

async function apply(input: BadgeSettingsInput) {
  error.value = null;
  try {
    await badge.save(input);
  } catch (e) {
    error.value = describeError(e);
    syncDraft();
  }
}

const setReader = (reader: BadgeReader) => apply(current({ reader }));
const toggleLogin = () => apply(current({ loginEnabled: !badge.settings!.loginEnabled }));

async function saveTiming() {
  editing.value = null;
  await apply(
    current({
      wedgeMaxGapMs: Number(draft.value.wedgeMaxGapMs),
      wedgeMinLen: Number(draft.value.wedgeMinLen),
      wedgeMaxLen: Number(draft.value.wedgeMaxLen),
    }),
  );
}

function logLabel(action: string): string {
  return t(`badge_log_${action}`);
}

onMounted(() => void badge.loadOverview());
</script>

<template>
  <section class="panel badge-panel" :class="{ open }" data-testid="badge-settings">
    <!-- Folded by default: the panel is a manager's occasional setting and
         must not squeeze the other panels of the column. -->
    <button class="head" :aria-expanded="open" data-testid="badge-toggle" @click="open = !open">
      <h2>{{ t('settings_badges') }}</h2>
      <span v-if="badge.settings" class="head-state">
        {{ readerLabel }}<span class="chev" aria-hidden="true">{{ open ? '\u25B4' : '\u25BE' }}</span>
      </span>
    </button>
    <p v-if="badge.settings?.relinkNeeded" class="relink" data-testid="badge-relink">{{ t('settings_badge_relink') }}</p>

    <template v-if="open && badge.settings">
      <div class="segmented reader-btns" role="radiogroup" :aria-label="t('settings_badges')">
        <button
          v-for="r in readers"
          :key="r"
          class="reader-btn"
          role="radio"
          :aria-checked="badge.settings.reader === r"
          :class="{ active: badge.settings.reader === r }"
          @click="setReader(r)"
        >{{ t(`badge_reader_${r}`) }}</button>
      </div>
      <p v-if="badge.settings.reader === 'nfc'" class="add-error" data-testid="badge-nfc-unavailable">
        {{ t('settings_badge_nfc_unavailable') }}
      </p>

      <template v-if="badge.settings.reader !== 'off'">
        <div class="panel-head">
          <span class="label">{{ t('settings_badge_login') }}</span>
          <button
            class="toggle"
            :class="{ on: badge.settings.loginEnabled }"
            role="switch"
            :aria-checked="badge.settings.loginEnabled"
            data-testid="badge-login-switch"
            @click="toggleLogin"
          >
            <span class="track"><span class="thumb"></span></span>
          </button>
        </div>
        <p class="panel-note">{{ t('settings_badge_login_note') }}</p>
      </template>

      <template v-if="badge.settings.reader === 'usb'">
        <div class="section-label">{{ t('settings_badge_usb_timing') }}</div>
        <div v-for="f in timing" :key="f.key" class="timing-row">
          <span class="label">{{ t(f.label) }}</span>
          <button
            class="num-field"
            :class="{ editing: editing === f.key }"
            :data-testid="`badge-${f.key}`"
            @click="editing = f.key"
          >{{ draft[f.key] || '·' }}</button>
        </div>
        <div v-if="editing" class="pad"><NumPad :model-value="draft[editing]" @update:model-value="onKey" /></div>
        <button v-if="dirty" class="save-btn" data-testid="badge-timing-save" @click="saveTiming">{{ t('settings_tables_save') }}</button>
      </template>

      <p v-if="error" class="add-error" data-testid="badge-error">{{ error }}</p>
      <p class="panel-note">{{ t('settings_badge_privacy') }}</p>

      <div class="section-label">{{ t('settings_badge_log') }}</div>
      <p v-if="!badge.overview || badge.overview.log.length === 0" class="panel-note">{{ t('settings_badge_log_empty') }}</p>
      <ul v-else class="log" data-testid="badge-log">
        <li v-for="(l, i) in badge.overview.log.slice(0, 6)" :key="i" class="log-row">
          <span class="log-what">{{ logLabel(l.action) }}<template v-if="l.name"> · {{ l.name }}</template></span>
          <span class="log-meta">{{ l.byName ?? '' }} {{ formatTimestamp(l.at, formatDayTime) }}</span>
        </li>
      </ul>
    </template>
  </section>
</template>

<style scoped>
.panel {
  display: flex; flex-direction: column; gap: 12px; min-height: 0;
  padding: 24px; background: var(--bg-secondary);
  border: 1px solid var(--border); border-radius: var(--radius-lg);
}
.badge-panel { flex-shrink: 0; }
.badge-panel.open { max-height: 70%; overflow-y: auto; -webkit-overflow-scrolling: touch; }
.head {
  display: flex; justify-content: space-between; align-items: center; gap: 16px;
  width: 100%; text-align: left; color: inherit;
}
.head-state { font-family: var(--font-mono); font-size: 15px; font-weight: var(--font-weight-bold); color: var(--text-secondary); }
.chev { margin-left: 12px; font-size: 18px; }
.panel h2 { font-size: 28px; font-weight: var(--font-weight-heavy); line-height: 1.2; }
.panel-head { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
.panel-note { font-size: 15px; color: var(--text-secondary); }
.label { font-size: 17px; font-weight: var(--font-weight-bold); }
.section-label {
  font-family: var(--font-mono); font-size: 14px; font-weight: var(--font-weight-bold);
  color: var(--text-secondary); letter-spacing: 0.08em; text-transform: uppercase;
}
.segmented {
  display: grid; grid-template-columns: repeat(2, 1fr); gap: 4px; padding: 4px;
  background: var(--bg-surface); border-radius: 12px;
}
.reader-btn {
  height: 56px; font-size: 18px; border-radius: var(--radius-sm);
  font-weight: var(--font-weight-bold); color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.reader-btn.active { background: var(--selected-bg); color: var(--selected-fg); }
.reader-btn:active { transform: scale(var(--tap-scale)); }
.toggle {
  display: flex; align-items: center; flex-shrink: 0; height: 56px; padding: 0 8px;
  border-radius: 28px; background: var(--bg-surface);
  transition: transform var(--tap-duration) ease;
}
.toggle:active { transform: scale(var(--tap-scale)); }
.track { width: 64px; height: 40px; border-radius: 20px; background: var(--border); position: relative; display: block; }
.toggle.on .track { background: var(--accent-primary); }
.thumb {
  position: absolute; left: 4px; top: 4px; width: 32px; height: 32px; border-radius: 50%;
  background: var(--text-secondary); transition: transform var(--tap-duration) ease;
}
.toggle.on .thumb { transform: translateX(24px); background: var(--text-primary); }
.timing-row { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
.num-field {
  min-width: 96px; height: 48px; padding: 0 16px; border-radius: var(--radius-md);
  border: 1px solid var(--border); font-family: var(--font-mono); font-size: 18px;
  font-weight: var(--font-weight-bold); color: var(--text-primary); background: transparent;
}
.num-field.editing { border-color: var(--accent-primary); }
.pad { display: flex; justify-content: center; }
.save-btn {
  height: 56px; border-radius: var(--radius-md); background: var(--bg-surface);
  color: var(--text-primary); font-size: 17px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.save-btn:active { transform: scale(var(--tap-scale)); }
.add-error { color: var(--accent-danger); font-size: 15px; }
.relink { color: var(--accent-warning); font-size: 16px; font-weight: var(--font-weight-bold); }
.log { list-style: none; margin: 0; padding: 0; }
.log-row { display: flex; flex-direction: column; padding: 8px 0; border-bottom: 1px solid var(--border); }
.log-what { font-size: 16px; font-weight: var(--font-weight-bold); }
.log-meta { font-family: var(--font-mono); font-size: 13px; color: var(--text-secondary); }
</style>
