<script setup lang="ts">
// Refusing an online order (UX audit O1): the refusal goes to the platform
// and cannot be undone, so the staff member picks a reason first. The red
// button stays disabled until a reason is chosen. The reason is kept on
// the till only: HubRise has no reject reason field.
import { ref } from 'vue'
import { useI18n } from '../i18n'
import { REJECT_REASONS, rejectReasonKey, type RejectReason } from '../utils/webshopRights'

const emit = defineEmits<{
  confirm: [reason: RejectReason]
  cancel: []
}>()

const { t } = useI18n()
const reason = ref<RejectReason | null>(null)

function confirm() {
  if (reason.value) emit('confirm', reason.value)
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div
      class="dialog"
      role="dialog"
      :aria-label="t('webshop_reject_title')"
      data-testid="reject-dialog"
    >
      <h2 class="title">{{ t('webshop_reject_title') }}</h2>
      <p class="message">{{ t('webshop_reject_message') }}</p>
      <p class="label">{{ t('webshop_reject_reason_label') }}</p>
      <div class="chips">
        <button
          v-for="r in REJECT_REASONS"
          :key="r"
          class="chip"
          :class="{ active: reason === r }"
          :aria-pressed="reason === r"
          :data-testid="`reject-reason-${r}`"
          @click="reason = r"
        >
          {{ t(rejectReasonKey(r)) }}
        </button>
      </div>
      <div class="actions">
        <button class="btn btn-cancel" data-testid="reject-cancel" @click="emit('cancel')">
          {{ t('back') }}
        </button>
        <button
          class="btn btn-danger"
          :disabled="!reason"
          data-testid="reject-confirm"
          @click="confirm"
        >
          {{ t('webshop_reject_confirm') }}
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
  gap: var(--space-md);
  width: min(640px, 92vw);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.message {
  color: var(--text-secondary);
}
.label {
  font-weight: var(--font-weight-bold);
}
.chips {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-sm);
}
.chip {
  height: 64px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.chip.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.actions {
  display: flex;
  gap: var(--space-sm);
}
.btn {
  flex: 1;
  height: 64px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.btn-cancel {
  background: var(--bg-surface);
  color: var(--text-primary);
}
.btn-danger {
  background: var(--accent-danger);
  color: var(--text-on-accent);
}
.btn-danger:disabled {
  opacity: 0.4;
}
.chip:active,
.btn:active:not(:disabled) {
  transform: scale(var(--tap-scale));
}
</style>
