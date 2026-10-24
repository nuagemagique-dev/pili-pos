<script setup lang="ts">
// Pay with a gift voucher: the 12-digit code is typed on the pad, the
// engine finds the voucher (5 failed tries a minute, then it makes the
// operator wait), and the screen shows its balance and expiry. The
// amount used is the smaller of the balance and what is left to pay.
import { computed, ref } from 'vue'
import NumPad from './NumPad.vue'
import * as ipc from '../api/ipc'
import { formatPrice } from '../utils/format'
import { describeError } from '../utils/errorMessages'
import { useI18n } from '../i18n'

const props = defineProps<{
  /** What is left to pay on the sale, in cents. */
  restCents: number
}>()

const emit = defineEmits<{
  confirm: [voucher: ipc.GiftVoucherView, amountCents: number]
  cancel: []
}>()

const { t } = useI18n()

const CODE_LENGTH = 12
const digits = ref('')
const voucher = ref<ipc.GiftVoucherView | null>(null)
const error = ref<string | null>(null)
const busy = ref(false)

/** `4812 0000 9095`, as the voucher prints it. */
const shown = computed(() => (digits.value.match(/.{1,4}/g) ?? []).join(' '))
const amount = computed(() => Math.min(voucher.value?.balance_cents ?? 0, props.restCents))

function onKey(key: string) {
  error.value = null
  voucher.value = null
  if (key === 'BACKSPACE') digits.value = digits.value.slice(0, -1)
  else if (key === 'CLEAR') digits.value = ''
  else if (digits.value.length < CODE_LENGTH) digits.value += key
}

/** `2027-10-17` as `17.10.2027`. */
function displayDate(iso: string): string {
  const [y, m, d] = iso.split('-')
  return d && m && y ? `${d}.${m}.${y}` : iso
}

async function lookup() {
  if (digits.value.length !== CODE_LENGTH || busy.value) return
  busy.value = true
  error.value = null
  try {
    voucher.value = await ipc.lookupGiftVoucher(digits.value)
  } catch (e) {
    voucher.value = null
    error.value = describeError(e)
  } finally {
    busy.value = false
  }
}

function confirm() {
  if (voucher.value && voucher.value.status === 'active' && amount.value > 0) {
    emit('confirm', voucher.value, amount.value)
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="t('gv_dialog_title')" data-testid="gv-dialog">
      <h2 class="title">{{ t('gv_dialog_title') }}</h2>
      <p class="item">{{ t('gv_code_label') }}</p>
      <p class="code" data-testid="gv-code">{{ shown || ' ' }}</p>
      <NumPad :model-value="digits" @update:model-value="onKey" />

      <p v-if="error" class="call-error" role="alert" data-testid="gv-error">{{ error }}</p>

      <div v-if="voucher" class="found" data-testid="gv-found">
        <div class="found-row">
          <span>{{ voucher.code_masked }}</span>
          <span class="status" :class="voucher.status" data-testid="gv-status">{{
            t(`gv_status_${voucher.status}`)
          }}</span>
        </div>
        <div class="found-row">
          <span>{{ t('gv_balance') }}</span>
          <strong data-testid="gv-balance">{{ formatPrice(voucher.balance_cents) }} €</strong>
        </div>
        <div class="found-row small">
          <span>{{ t('gv_expires', { date: displayDate(voucher.expires_on) }) }}</span>
          <span v-if="voucher.training">{{ t('gv_training_tag') }}</span>
        </div>
        <div v-if="voucher.status === 'active'" class="found-row">
          <span>{{ t('gv_use_amount') }}</span>
          <strong data-testid="gv-amount">{{ formatPrice(amount) }} €</strong>
        </div>
      </div>

      <div class="actions">
        <button class="btn btn-cancel" data-testid="gv-cancel" @click="emit('cancel')">
          {{ t('back') }}
        </button>
        <button
          v-if="!voucher || voucher.status !== 'active'"
          class="btn btn-confirm"
          :disabled="digits.length !== CODE_LENGTH || busy"
          data-testid="gv-lookup"
          @click="lookup"
        >
          {{ t('gv_lookup') }}
        </button>
        <button
          v-else
          class="btn btn-confirm"
          :disabled="amount <= 0"
          data-testid="gv-apply"
          @click="confirm"
        >
          {{ t('gv_apply') }}
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
  max-height: 100%;
  overflow-y: auto;
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.item {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}
.code {
  min-width: 320px;
  text-align: center;
  font-family: var(--font-mono);
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
  font-variant-numeric: tabular-nums;
}
.call-error {
  font-size: 15px;
  color: var(--accent-danger);
  overflow-wrap: anywhere;
  max-width: 320px;
  text-align: center;
}
.found {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: var(--font-size-md);
}
.found-row {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: baseline;
}
.found-row.small {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.status {
  font-weight: var(--font-weight-bold);
}
.status.active {
  color: var(--accent-success);
}
.status.used,
.status.expired,
.status.cancelled,
.status.not_usable {
  color: var(--accent-danger);
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
.btn-cancel {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
.btn-confirm {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.btn-confirm:disabled {
  opacity: 0.4;
}
</style>
