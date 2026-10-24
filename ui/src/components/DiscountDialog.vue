<script setup lang="ts">
// A manager's discount at the till: a percent, an amount or "Offert"
// for the whole order, or "Offert" for one line. The reason is one of
// four choices plus an optional text typed on the on-screen keypad. The
// dialog only collects the input; the caller confirms it with the PIN
// and the engine applies every rule.
import { computed, ref } from 'vue'
import type { DiscountInput, DiscountKind, DiscountReason } from '../types'
import NumPad from './NumPad.vue'
import TextKeypad from './TextKeypad.vue'
import { useI18n } from '../i18n'
import { formatPrice } from '../utils/format'
import { discountPreviewCents } from '../utils/cartAmounts'

const props = defineProps<{
  /** The amount the discount applies to: the order's net total, or the line's. */
  baseCents: number
  /** A line to give for free; absent: a discount on the whole order. */
  lineId?: number
}>()

const emit = defineEmits<{
  confirm: [input: DiscountInput]
  cancel: []
}>()

const { t } = useI18n()

const KINDS: DiscountKind[] = ['percent', 'amount', 'free']
const QUICK_PERCENTS = [5, 10, 15, 20, 50]
const REASONS: DiscountReason[] = ['commercial_gesture', 'staff_meal', 'service_issue', 'other']
const TEXT_SYMBOLS = ['.', ',', '-', '/', '(', ')']
const TEXT_MAX = 120

const isLine = computed(() => props.lineId !== undefined)
const kind = ref<DiscountKind>(props.lineId !== undefined ? 'free' : 'percent')
// Digits typed on the pad: a percent, or the amount in cents.
const typed = ref('')
const reason = ref<DiscountReason | null>(null)
const reasonText = ref('')
const typing = ref(false)

function selectKind(k: DiscountKind) {
  kind.value = k
  typed.value = ''
}

function onKey(key: string) {
  const max = kind.value === 'percent' ? 2 : 7
  if (key === 'BACKSPACE') typed.value = typed.value.slice(0, -1)
  else if (key === 'CLEAR') typed.value = ''
  else if (typed.value.length < max) typed.value = (typed.value + key).replace(/^0+/, '')
}

function quickPercent(p: number) {
  typed.value = String(p)
}

const value = computed(() => (typed.value ? Number(typed.value) : 0))

const valueOk = computed(() => {
  if (kind.value === 'free') return true
  if (kind.value === 'percent') return value.value >= 1 && value.value <= 99
  return value.value >= 1 && value.value <= props.baseCents
})

const canConfirm = computed(() => valueOk.value && reason.value !== null && props.baseCents > 0)

const newTotal = computed(() => {
  if (!valueOk.value) return props.baseCents
  return props.baseCents - discountPreviewCents(kind.value, value.value, props.baseCents)
})

function confirm() {
  if (!canConfirm.value || reason.value === null) return
  const text = reasonText.value.trim()
  emit('confirm', {
    target:
      props.lineId !== undefined ? { type: 'line', line_id: props.lineId } : { type: 'order' },
    kind: kind.value,
    ...(kind.value === 'free' ? {} : { value: value.value }),
    reason_code: reason.value,
    reason_text: text ? text : null,
  })
}

function textTyped(text: string) {
  reasonText.value = text.trim()
  typing.value = false
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div
      class="dialog"
      role="dialog"
      :aria-label="isLine ? t('discount_offer_title') : t('discount_title')"
      data-testid="discount-dialog"
    >
      <h2 class="title">{{ isLine ? t('discount_offer_title') : t('discount_title') }}</h2>

      <div v-if="!isLine" class="kinds" role="tablist">
        <button
          v-for="k in KINDS"
          :key="k"
          class="kind"
          :class="{ active: kind === k }"
          role="tab"
          :aria-selected="kind === k"
          :data-testid="`discount-kind-${k}`"
          @click="selectKind(k)"
        >
          {{ t(`discount_tab_${k}`) }}
        </button>
      </div>

      <div class="body">
        <div v-if="!isLine && kind !== 'free'" class="entry">
          <div v-if="kind === 'percent'" class="quick">
            <button
              v-for="p in QUICK_PERCENTS"
              :key="p"
              class="chip"
              :class="{ active: typed === String(p) }"
              :data-testid="`discount-quick-${p}`"
              @click="quickPercent(p)"
            >
              {{ p }} %
            </button>
          </div>
          <div class="typed" data-testid="discount-typed">
            <span class="typed-label">{{
              kind === 'percent' ? t('discount_custom_percent') : t('discount_custom_amount')
            }}</span>
            <span class="typed-value">{{
              kind === 'percent' ? `${typed || '0'} %` : `${formatPrice(value)} €`
            }}</span>
          </div>
          <NumPad :model-value="typed" @update:model-value="onKey" />
        </div>
        <p v-else-if="!isLine" class="note">{{ t('discount_free_note') }}</p>

        <div class="reasons">
          <h3>{{ t('discount_reason') }}</h3>
          <div class="reason-grid">
            <button
              v-for="r in REASONS"
              :key="r"
              class="reason"
              :class="{ active: reason === r }"
              :data-testid="`discount-reason-${r}`"
              @click="reason = r"
            >
              {{ t(`discount_reason_${r}`) }}
            </button>
          </div>
          <button class="text-btn" data-testid="discount-reason-text" @click="typing = true">
            <span v-if="reasonText">{{ reasonText }}</span>
            <span v-else>{{ t('discount_reason_text_add') }}</span>
          </button>
        </div>
      </div>

      <p class="preview" data-testid="discount-preview">
        {{ t('discount_preview', { from: formatPrice(baseCents), to: formatPrice(newTotal) }) }}
      </p>

      <div class="actions">
        <button class="btn btn-cancel" @click="emit('cancel')">{{ t('back') }}</button>
        <button
          class="btn btn-confirm"
          :disabled="!canConfirm"
          data-testid="discount-confirm"
          @click="confirm"
        >
          {{ t('discount_apply') }}
        </button>
      </div>
    </div>

    <TextKeypad
      v-if="typing"
      :title="t('discount_reason_text_title')"
      :placeholder="t('discount_reason_text_placeholder')"
      :initial="reasonText"
      :max-len="TEXT_MAX"
      :symbols="TEXT_SYMBOLS"
      space
      @confirm="textTyped"
      @cancel="typing = false"
    />
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
  width: 880px;
  max-width: 96vw;
  max-height: 96vh;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.kinds {
  display: flex;
  gap: var(--space-sm);
}
.kind {
  flex: 1;
  height: 56px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.kind.active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.body {
  display: flex;
  gap: var(--space-lg);
  align-items: flex-start;
}
.entry {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  align-items: center;
}
.quick {
  display: flex;
  gap: var(--space-xs);
}
.chip {
  min-width: 64px;
  height: 56px;
  padding: 0 12px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.chip.active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.typed {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  padding: 8px 12px;
  border-radius: var(--radius-md);
  background: var(--bg-primary);
}
.typed-label {
  font-size: 14px;
  color: var(--text-secondary);
}
.typed-value {
  font-family: var(--font-mono);
  font-size: 28px;
  font-weight: var(--font-weight-bold);
}
.note {
  flex: 1;
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}
.reasons {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.reasons h3 {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}
.reason-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-sm);
}
.reason {
  min-height: 64px;
  padding: 0 12px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
}
.reason.active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.text-btn {
  min-height: 56px;
  padding: 0 12px;
  border-radius: var(--radius-md);
  border: 1px dashed var(--border);
  color: var(--text-secondary);
  font-size: 15px;
  text-align: left;
  overflow-wrap: anywhere;
}
.preview {
  font-family: var(--font-mono);
  font-size: 24px;
  font-weight: var(--font-weight-bold);
  text-align: center;
}
.actions {
  display: flex;
  gap: var(--space-sm);
}
.btn {
  flex: 1;
  height: 64px;
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
  pointer-events: none;
}
</style>
