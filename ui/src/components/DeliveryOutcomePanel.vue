<script setup lang="ts">
import { describeError } from '../utils/errorMessages'
import { computed, ref } from 'vue'
import type { DeliveryOutcome } from '../types'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import { useToastStore } from '../stores/toast'
import { useAuthStore } from '../stores/auth'

// Shown after checkout, refund or copy: what happened to the ticket,
// and - when the customer still needs a paper original - the one
// print action the "done" screen offers (never a second ORIGINAL: see
// the cert note on print_missing_original in delivery.rs).

const props = defineProps<{
  outcome: DeliveryOutcome | null
  fiscalEventId: number | null
}>()

const { t } = useI18n()
const toast = useToastStore()
const auth = useAuthStore()

const printing = ref(false)
const printed = ref(false)

const message = computed(() => {
  const o = props.outcome
  if (!o) return null
  if (o.email === 'sent') return t('delivery_outcome_sent', { hint: o.recipientHint ?? '' })
  if (o.email === 'queued') return t('delivery_outcome_queued')
  if (o.paperPrinted) return t('delivery_outcome_printed')
  return null
})

// print_missing_original is cashier/admin only on the backend (same
// gate as canTakePayment); hide the button for a waiter rather than
// let them hit a "Forbidden" toast.
const showNeedsPaper = computed(
  () => !!props.outcome?.needsPaper && !printed.value && auth.canTakePayment,
)

async function printMissing() {
  if (!props.fiscalEventId || printing.value) return
  printing.value = true
  try {
    await ipc.printMissingOriginal(props.fiscalEventId)
    printed.value = true
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
  } finally {
    printing.value = false
  }
}
</script>

<template>
  <div v-if="outcome" class="delivery-outcome">
    <p v-if="message" class="delivery-msg">{{ message }}</p>
    <button
      v-if="showNeedsPaper"
      class="delivery-print-btn"
      :disabled="printing"
      @click="printMissing"
    >
      {{ t('delivery_print_missing') }}
    </button>
  </div>
</template>

<style scoped>
.delivery-outcome {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-sm);
  width: 100%;
}
.delivery-msg {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
  text-align: center;
}
.delivery-print-btn {
  width: 100%;
  height: 64px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--accent-warning);
  color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
}
.delivery-print-btn:active {
  transform: scale(var(--tap-scale));
}
.delivery-print-btn:disabled {
  opacity: 0.5;
  pointer-events: none;
}
</style>
