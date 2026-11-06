<script setup lang="ts">
// The phone of a takeaway customer (UX audit T2), typed on the shared numeric pad: digits only, shown
// in groups, optional. "Valider" with nothing typed clears the phone; "Passer" leaves it as it was.
import { computed, ref } from 'vue'
import NumPad from './NumPad.vue'
import { useI18n } from '../i18n'
import { phoneDisplay, phoneKey } from '../utils/takeaway'

const { initial = '' } = defineProps<{
  /** The phone being edited, in any form ("0470 12 34 56"). */
  initial?: string | null
}>()

const emit = defineEmits<{
  confirm: [digits: string]
  skip: []
}>()

const { t } = useI18n()

const digits = ref((initial ?? '').replace(/\D/gu, ''))
const shown = computed(() => phoneDisplay(digits.value))
</script>

<template>
  <div class="backdrop" @click.self="emit('skip')">
    <div class="dialog" role="dialog" :aria-label="t('takeaway_phone_title')">
      <h2 class="title">{{ t('takeaway_phone_title') }}</h2>
      <p class="phone" data-testid="phone-entry-value">{{ shown || t('takeaway_phone_none') }}</p>
      <NumPad :model-value="digits" @update:model-value="digits = phoneKey(digits, $event)" />
      <div class="actions">
        <button class="btn btn-skip" data-testid="phone-entry-skip" @click="emit('skip')">
          {{ t('takeaway_phone_skip') }}
        </button>
        <button class="btn btn-ok" data-testid="phone-entry-ok" @click="emit('confirm', digits)">
          {{ t('validate') }}
        </button>
      </div>
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
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.phone {
  min-height: 1.2em;
  font-family: var(--font-mono);
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
  font-variant-numeric: tabular-nums;
}
.actions {
  display: flex;
  gap: var(--space-sm);
  width: 100%;
}
.btn {
  flex: 1;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.btn:active {
  transform: scale(var(--tap-scale));
}
.btn-skip {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
.btn-ok {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
</style>
