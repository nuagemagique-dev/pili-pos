<script setup lang="ts">
// Gift vouchers for Settings (manager only: the engine refuses the
// commands to anyone else, and SettingsView renders the panel for a
// manager session only). A switch, the validity in months (12 to 36)
// and the largest voucher sold, in euros.
import { computed, onMounted, ref } from 'vue'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'

const { t } = useI18n()

const stored = ref<ipc.GiftVoucherSettings | null>(null)
const enabled = ref(false)
const validity = ref('12')
const maxEuros = ref('500')
const saved = ref(false)
const busy = ref(false)
const error = ref<string | null>(null)

function adopt(s: ipc.GiftVoucherSettings) {
  stored.value = s
  enabled.value = s.enabled
  validity.value = String(s.validity_months)
  maxEuros.value = String(s.max_amount_cents / 100)
}

function draft(): ipc.GiftVoucherSettings {
  return {
    enabled: enabled.value,
    // A number input gives a number once the field holds one.
    validity_months: Math.round(Number(String(validity.value))),
    max_amount_cents: Math.round(Number(String(maxEuros.value).replace(',', '.')) * 100),
  }
}

onMounted(async () => {
  try {
    adopt(await ipc.getGiftVoucherSettings())
  } catch (e) {
    error.value = describeError(e)
  }
})

const dirty = computed(
  () => !!stored.value && JSON.stringify(draft()) !== JSON.stringify(stored.value),
)

function toggle() {
  enabled.value = !enabled.value
  saved.value = false
}

async function save() {
  error.value = null
  busy.value = true
  try {
    adopt(await ipc.setGiftVoucherSettings(draft()))
    saved.value = true
  } catch (e) {
    error.value = describeError(e)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <section class="gv-panel" data-testid="gv-settings">
    <div class="panel-head">
      <h2>{{ t('gv_settings_title') }}</h2>
      <button
        v-if="stored"
        class="toggle"
        :class="{ on: enabled }"
        data-testid="gv-enabled"
        role="switch"
        :aria-checked="enabled"
        @click="toggle"
      >
        <span>{{ enabled ? t('gv_settings_on') : t('gv_settings_off') }}</span>
        <span class="track"><span class="thumb"></span></span>
      </button>
    </div>

    <p v-if="error" class="call-error" role="alert" data-testid="gv-settings-error">{{ error }}</p>
    <p class="panel-note">{{ t('gv_settings_note') }}</p>

    <template v-if="stored">
      <label class="field">
        <span class="section-label">{{ t('gv_settings_validity') }}</span>
        <input
          v-model="validity"
          class="input"
          type="number"
          min="12"
          max="36"
          data-testid="gv-validity"
          @input="saved = false"
        />
      </label>
      <label class="field">
        <span class="section-label">{{ t('gv_settings_max') }}</span>
        <input
          v-model="maxEuros"
          class="input"
          type="number"
          inputmode="decimal"
          min="1"
          data-testid="gv-max"
          @input="saved = false"
        />
      </label>
      <div class="actions">
        <button
          class="act save"
          :disabled="busy || !dirty"
          data-testid="gv-settings-save"
          @click="save"
        >
          {{ t('gv_settings_save') }}
        </button>
      </div>
      <p v-if="saved && !dirty" class="state-ok" data-testid="gv-settings-saved">
        {{ t('gv_settings_saved') }}
      </p>
    </template>
  </section>
</template>

<style scoped>
.gv-panel h2 {
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
.field {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 8px 0;
}
.input {
  height: 56px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 20px;
  font-weight: var(--font-weight-bold);
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
