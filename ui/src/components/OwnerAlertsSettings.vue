<script setup lang="ts">
// Owner alerts for Settings (manager only: the engine refuses the
// commands to anyone else, and SettingsView renders the panel for a
// manager session only). A draft the manager edits and saves; the bot
// token is not editable here (it is a file named by pos.json) and the
// panel only learns whether it is there.
import { computed, onMounted, ref, watch } from 'vue'
import * as ipc from '../api/ipc'
import NumberEntryDialog from './NumberEntryDialog.vue'
import { useI18n } from '../i18n'
import { useOwnerAlertsStore } from '../stores/ownerAlerts'
import { formatDayTime, formatTimestamp } from '../utils/datetime'

type KindKey = keyof ipc.AlertKinds
type Field = 'chat_id' | 'z_deadline_hour' | 'online_wait_minutes' | 'reminder_minutes'

const KINDS: KindKey[] = ['fdm', 'printer', 'terminal', 'z_missing', 'online_waiting']
const LANGUAGES = ['fr', 'zh', 'en'] as const

const { t } = useI18n()
const store = useOwnerAlertsStore()

const draft = ref<ipc.OwnerAlertsInput | null>(null)
const editing = ref<Field | null>(null)
const saved = ref(false)

function toDraft(s: ipc.OwnerAlerts): ipc.OwnerAlertsInput {
  return {
    enabled: s.enabled,
    chat_id: s.chat_id,
    label: s.label,
    language: (LANGUAGES as readonly string[]).includes(s.language)
      ? (s.language as ipc.OwnerAlertsInput['language'])
      : 'fr',
    kinds: { ...s.kinds },
    z_deadline_hour: s.z_deadline_hour,
    online_wait_minutes: s.online_wait_minutes,
    reminder_minutes: s.reminder_minutes,
  }
}

watch(
  () => store.settings,
  (s) => {
    if (s) draft.value = toDraft(s)
  },
)

onMounted(() => void store.load())

const dirty = computed(() => {
  const s = store.settings
  const d = draft.value
  return s !== null && d !== null && JSON.stringify(toDraft(s)) !== JSON.stringify(d)
})

async function save(): Promise<boolean> {
  if (!draft.value) return false
  saved.value = false
  const ok = await store.save(draft.value)
  saved.value = ok
  return ok
}

// The test goes out with the stored settings, so a draft is saved first.
async function test() {
  if (dirty.value && !(await save())) return
  await store.sendTest()
}

function toggleEnabled() {
  if (draft.value) draft.value.enabled = !draft.value.enabled
  saved.value = false
}

function toggleKind(k: KindKey) {
  if (draft.value) draft.value.kinds[k] = !draft.value.kinds[k]
  saved.value = false
}

function setLanguage(l: ipc.OwnerAlertsInput['language']) {
  if (draft.value) draft.value.language = l
  saved.value = false
}

const dialog = computed(() => {
  const d = draft.value
  switch (editing.value) {
    case 'chat_id':
      return (
        d && {
          title: t('alerts_chat_id'),
          initial: d.chat_id,
          maxDigits: 20,
          allowMinus: true,
          min: 0,
          max: 0,
        }
      )
    case 'z_deadline_hour':
      return (
        d && {
          title: t('alerts_z_deadline'),
          initial: String(d.z_deadline_hour),
          maxDigits: 2,
          allowMinus: false,
          min: 0,
          max: 12,
        }
      )
    case 'online_wait_minutes':
      return (
        d && {
          title: t('alerts_online_wait'),
          initial: String(d.online_wait_minutes),
          maxDigits: 3,
          allowMinus: false,
          min: 1,
          max: 120,
        }
      )
    case 'reminder_minutes':
      return (
        d && {
          title: t('alerts_reminder'),
          initial: String(d.reminder_minutes),
          maxDigits: 4,
          allowMinus: false,
          min: 0,
          max: 1440,
        }
      )
    default:
      return null
  }
})

function onEntered(value: string) {
  const d = draft.value
  const field = editing.value
  editing.value = null
  if (!d || !field) return
  saved.value = false
  if (field === 'chat_id') d.chat_id = value
  else d[field] = Number(value)
}

const reminderText = computed(() =>
  draft.value && draft.value.reminder_minutes === 0
    ? t('alerts_value_never')
    : t('alerts_value_minutes', { n: draft.value?.reminder_minutes ?? 0 }),
)

const time = (iso: string) => formatTimestamp(iso, formatDayTime)
</script>

<template>
  <section class="alerts-panel" data-testid="alerts-panel">
    <div class="panel-head">
      <h2>{{ t('alerts_title') }}</h2>
      <button
        v-if="draft"
        class="toggle"
        :class="{ on: draft.enabled }"
        data-testid="alerts-enabled"
        role="switch"
        :aria-checked="draft.enabled"
        @click="toggleEnabled"
      >
        <span>{{ draft.enabled ? t('alerts_state_on') : t('alerts_state_off') }}</span>
        <span class="track"><span class="thumb"></span></span>
      </button>
    </div>

    <p v-if="store.error" class="call-error" role="alert" data-testid="alerts-error">
      {{ store.error }}
    </p>

    <template v-if="draft && store.settings">
      <div class="pills">
        <span
          class="pill"
          :class="store.settings.token_configured ? 'state-ok' : 'state-warn'"
          data-testid="alerts-token"
        >
          <span class="dot"></span
          >{{
            store.settings.token_configured ? t('alerts_token_ok') : t('alerts_token_missing_pill')
          }}
        </span>
      </div>
      <p v-if="!store.settings.token_configured" class="panel-note">
        {{ t('alerts_token_missing_note') }}
      </p>

      <div class="row">
        <span>{{ t('alerts_chat_id') }}</span>
        <button class="value-btn mono" data-testid="alerts-chat-id" @click="editing = 'chat_id'">
          {{ draft.chat_id || t('alerts_chat_id_unset') }}
        </button>
      </div>

      <label class="row">
        <span>{{ t('alerts_label') }}</span>
        <input
          v-model="draft.label"
          class="field"
          maxlength="40"
          :placeholder="t('alerts_label_placeholder')"
          data-testid="alerts-label"
          @input="saved = false"
        />
      </label>

      <span class="section-label">{{ t('alerts_language') }}</span>
      <div class="segmented">
        <button
          v-for="l in LANGUAGES"
          :key="l"
          class="seg-btn"
          :class="{ active: draft.language === l }"
          :data-testid="`alerts-lang-${l}`"
          @click="setLanguage(l)"
        >
          {{ t(`settings_lang_name_${l}`) }}
        </button>
      </div>

      <span class="section-label">{{ t('alerts_kinds') }}</span>
      <button
        v-for="k in KINDS"
        :key="k"
        class="kind"
        :class="{ on: draft.kinds[k] }"
        role="switch"
        :aria-checked="draft.kinds[k]"
        :data-testid="`alerts-kind-${k}`"
        @click="toggleKind(k)"
      >
        <span>{{ t(`alerts_kind_${k}`) }}</span>
        <span class="track"><span class="thumb"></span></span>
      </button>

      <div class="row">
        <span>{{ t('alerts_z_deadline') }}</span>
        <button
          class="value-btn mono"
          data-testid="alerts-z-hour"
          @click="editing = 'z_deadline_hour'"
        >
          {{ t('alerts_value_hour', { n: draft.z_deadline_hour }) }}
        </button>
      </div>
      <div class="row">
        <span>{{ t('alerts_online_wait') }}</span>
        <button
          class="value-btn mono"
          data-testid="alerts-online-wait"
          @click="editing = 'online_wait_minutes'"
        >
          {{ t('alerts_value_minutes', { n: draft.online_wait_minutes }) }}
        </button>
      </div>
      <div class="row">
        <span>{{ t('alerts_reminder') }}</span>
        <button
          class="value-btn mono"
          data-testid="alerts-reminder"
          @click="editing = 'reminder_minutes'"
        >
          {{ reminderText }}
        </button>
      </div>

      <div class="actions">
        <button
          class="act save"
          :disabled="store.busy || !dirty"
          data-testid="alerts-save"
          @click="save"
        >
          {{ t('alerts_save') }}
        </button>
        <button
          class="act"
          :disabled="store.busy || !store.settings.token_configured || !draft.chat_id"
          data-testid="alerts-test"
          @click="test"
        >
          {{ t('alerts_test') }}
        </button>
      </div>
      <p v-if="saved && !dirty" class="state-ok" data-testid="alerts-saved">
        {{ t('alerts_saved') }}
      </p>
      <p v-if="store.testResult?.sent" class="state-ok" data-testid="alerts-test-sent">
        {{ t('alerts_test_sent') }}
      </p>

      <p class="panel-note">{{ t('alerts_note') }}</p>

      <span class="section-label">{{ t('alerts_log_title') }}</span>
      <p v-if="store.log.length === 0" class="panel-note" data-testid="alerts-log-empty">
        {{ t('alerts_log_empty') }}
      </p>
      <ul v-else class="log" data-testid="alerts-log">
        <li v-for="r in store.log" :key="r.id" class="log-row">
          <span class="log-main">
            {{ t(`alerts_transition_${r.transition}`) }}
            <template v-if="r.transition !== 'test'"> · {{ t(`alerts_kind_${r.kind}`) }}</template>
          </span>
          <span class="log-meta" :class="r.outcome === 'sent' ? 'state-ok' : 'state-warn'">
            {{ t(`alerts_outcome_${r.outcome}`) }} · {{ time(r.created_at) }}
          </span>
        </li>
      </ul>
    </template>

    <NumberEntryDialog
      v-if="dialog"
      v-bind="dialog"
      @confirm="onEntered"
      @cancel="editing = null"
    />
  </section>
</template>

<style scoped>
.alerts-panel {
  min-height: 320px;
}
.alerts-panel h2 {
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
.section-label {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.pills {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.pill {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 14px;
  border-radius: 18px;
  border: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.pill .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}
.state-ok {
  color: var(--accent-success);
}
.state-warn {
  color: var(--accent-warning);
}

.toggle,
.kind {
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
.toggle.on,
.kind.on {
  color: var(--text-primary);
}
.toggle:active,
.kind:active {
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
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  min-height: 56px;
  border-bottom: 1px solid var(--border);
  font-size: 16px;
}
.row > span:first-child {
  color: var(--text-secondary);
}
.value-btn {
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  overflow-wrap: anywhere;
  transition: transform var(--tap-duration) ease;
}
.value-btn:active {
  transform: scale(var(--tap-scale));
}
.mono {
  font-family: var(--font-mono);
}
.field {
  flex: 1;
  min-width: 0;
  max-width: 60%;
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-primary);
  font-size: 16px;
}
.field::placeholder {
  color: var(--text-secondary);
}

.segmented {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.seg-btn {
  height: 48px;
  font-size: 16px;
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

.log {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}
.log-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  min-height: 44px;
  border-bottom: 1px solid var(--border);
  font-size: 15px;
}
.log-meta {
  font-family: var(--font-mono);
  font-size: 14px;
  text-align: right;
}
</style>
