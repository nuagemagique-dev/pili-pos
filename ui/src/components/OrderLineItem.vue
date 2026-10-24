<script setup lang="ts">
import { computed, ref } from 'vue'
import type { CartLine } from '../types'
import { formatPrice } from '../utils/format'
import { useBilingualName } from '../utils/names'
import { useI18n } from '../i18n'
import { lineNetCents } from '../utils/cartAmounts'
import { GIFT_VOUCHER_DEPARTMENT } from '../stores/giftVouchers'

const { line, pending, canOffer, canRemoveDiscount, canNote, showCourse } = defineProps<{
  line: CartLine
  /** Not yet sent to the bar / kitchen: amber mark on the left. */
  pending?: boolean
  /** A manager may give the line for free (no change on it yet). */
  canOffer?: boolean
  /** A manager may remove the line's own change (an offered line). */
  canRemoveDiscount?: boolean
  /** The line's kitchen note may be written or changed. */
  canNote?: boolean
  /** Courses are on: show the course chip. */
  showCourse?: boolean
}>()

const emit = defineEmits<{
  increment: [id: number]
  decrement: [id: number]
  remove: [id: number]
  offer: [id: number]
  removeDiscount: [id: number]
  note: [id: number]
  course: [id: number]
}>()

const { t } = useI18n()
const bilingual = useBilingualName()
const names = computed(() => bilingual(line))
/** A gift voucher sold: the quantity is fixed (one line per voucher). */
const isVoucher = computed(() => line.department_id === GIFT_VOUCHER_DEPARTMENT)

/** Notes on the line ("Sans sésame"), when the line carries any:
 * `modifiers_json` holds a string or a list of strings. */
const notes = computed(() => {
  const raw = line.modifiers_json
  if (!raw) return ''
  try {
    const value: unknown = JSON.parse(raw)
    if (Array.isArray(value)) return value.filter((v) => typeof v === 'string').join(' · ')
    return typeof value === 'string' ? value : ''
  } catch {
    return raw
  }
})

const changes = computed(() => line.price_changes ?? [])
/** A voucher line is never discounted and goes to no kitchen: no offer, no note. */
const canOfferHere = computed(() => Boolean(canOffer) && !isVoucher.value)
const canNoteHere = computed(() => Boolean(canNote) && !isVoucher.value)
const hasActions = computed(() => canOfferHere.value || canRemoveDiscount)

const lineRef = ref<HTMLElement | null>(null)

// Called externally to trigger the green flash on a new line.
defineExpose({
  flashAdded() {
    const el = lineRef.value
    if (el) {
      el.classList.remove('flash-added')
      void el.offsetWidth
      el.classList.add('flash-added')
    }
  },
})
</script>

<template>
  <!-- One 64 px row: quantity, names, price, delete (UX guide §4.1). -->
  <div ref="lineRef" class="order-line" :class="{ pending }">
    <div v-if="isVoucher" class="voucher-badge" data-testid="voucher-badge">
      {{ t('gv_line_badge') }}
    </div>
    <div v-else class="qty-controls">
      <!-- §4.3: "−" stops at 1; removing a line is its own button. -->
      <button
        class="qty-btn"
        :disabled="line.quantity <= 1"
        :aria-label="t('qty_less')"
        @click="emit('decrement', line.id)"
      >
        −
      </button>
      <span class="qty">{{ line.quantity }}</span>
      <button class="qty-btn" :aria-label="t('qty_more')" @click="emit('increment', line.id)">
        +
      </button>
    </div>
    <div class="line-info">
      <span class="line-name" :class="{ code: names.isCode }">{{ names.primary }}</span>
      <span v-if="names.secondary" class="line-sub">{{ names.secondary }}</span>
      <span v-if="notes" class="line-notes">{{ notes }}</span>
      <span v-if="line.kitchen_note" class="line-kitchen-note" data-testid="line-kitchen-note">{{
        line.kitchen_note
      }}</span>
      <!-- Course chip: tap opens the picker; locked once a slip carried the line. -->
      <button
        v-if="showCourse && !isVoucher"
        class="course-chip"
        :class="line.kitchen_state"
        :disabled="line.kitchen_state === 'printed'"
        :aria-label="t('course_chip_label', { course: t(`course_${line.course}`) })"
        :data-testid="`course-chip-${line.id}`"
        @click="emit('course', line.id)"
      >
        {{ t(`course_${line.course}`) }}
        <span v-if="line.kitchen_state === 'held'" class="chip-held">{{
          t('course_held_tag')
        }}</span>
      </button>
    </div>
    <span class="line-price">{{ formatPrice(lineNetCents(line)) }}</span>
    <button
      v-if="canNoteHere"
      class="note-btn"
      :class="{ filled: !!line.kitchen_note }"
      data-testid="line-note"
      :aria-label="t('note_button')"
      @click="emit('note', line.id)"
    >
      {{ t('note_button') }}
    </button>
    <button class="remove-btn" :aria-label="t('line_remove')" @click="emit('remove', line.id)">
      <svg width="14" height="14" viewBox="0 0 14 14">
        <path
          d="M2 2l10 10M12 2L2 12"
          stroke="currentColor"
          stroke-width="2.2"
          stroke-linecap="round"
        />
      </svg>
    </button>
    <!-- Public price changes of the line, then the manager's line actions. -->
    <div v-if="changes.length > 0 || hasActions" class="line-extra">
      <div class="change-rows">
        <div
          v-for="(c, i) in changes"
          :key="`${c.id}-${i}`"
          class="change-row"
          data-testid="price-change-row"
        >
          <span class="change-name">{{ c.name }}</span>
          <span class="change-amount">{{ formatPrice(c.amount_cents) }}</span>
        </div>
      </div>
      <button
        v-if="canOfferHere"
        class="act-btn"
        data-testid="line-offer"
        @click="emit('offer', line.id)"
      >
        {{ t('discount_offer') }}
      </button>
      <button
        v-if="canRemoveDiscount"
        class="act-btn"
        data-testid="line-remove-discount"
        @click="emit('removeDiscount', line.id)"
      >
        {{ t('discount_remove') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.order-line {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto auto auto;
  align-items: center;
  gap: 16px;
  min-height: 64px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
  transition: box-shadow var(--flash-duration) ease;
}
.order-line.pending {
  padding-left: 12px;
  border-left: 3px solid var(--accent-warning);
}

.voucher-badge {
  min-width: 96px;
  padding: 12px 0;
  border-radius: var(--radius-md);
  background: var(--selected-bg);
  color: var(--selected-fg);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  text-align: center;
}

.qty-controls {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}

/* UX guide §4.1: 48×48 so fingers hit them under rush-hour stress. */
.qty-btn {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-primary);
  font-size: 24px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.qty-btn:active {
  transform: scale(var(--tap-scale));
  background: var(--bg-pressed);
}
.qty-btn:disabled {
  color: var(--border);
  pointer-events: none;
}

.qty {
  min-width: 28px;
  text-align: center;
  font-family: var(--font-mono);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
}

.line-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.line-name,
.line-sub {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.3;
}
.line-name {
  font-size: 17px;
  font-weight: var(--font-weight-bold);
}
.line-name.code {
  font-family: var(--font-mono);
}
.line-sub {
  font-size: 13px;
  color: var(--text-secondary);
}
.line-notes {
  font-size: 13px;
  color: var(--accent-warning);
}

.line-kitchen-note {
  font-size: 14px;
  font-style: italic;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.note-btn {
  height: 48px;
  padding: 0 14px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.note-btn.filled {
  background: var(--bg-pressed);
}
.note-btn:active {
  transform: scale(var(--tap-scale));
}

.line-extra {
  grid-column: 1 / -1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-left: 4px;
}
.change-rows {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.change-row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  font-size: 14px;
  color: var(--accent-warning);
  font-variant-numeric: tabular-nums;
}
.change-amount {
  font-family: var(--font-mono);
}
.act-btn {
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.act-btn:active {
  transform: scale(var(--tap-scale));
}
/* A small pill under the name; the tap target is the whole row of the chip. */
.course-chip {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 32px;
  margin-top: 4px;
  padding: 0 12px;
  border-radius: 16px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 13px;
  font-weight: var(--font-weight-bold);
}
.course-chip:disabled {
  opacity: 0.55;
}
.course-chip.held {
  color: var(--accent-warning);
}
.chip-held {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.line-price {
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
}

/* Destructive: set apart by a rule, never where "−" was (§4.3). */
.remove-btn {
  width: 48px;
  height: 48px;
  margin-left: 8px;
  border-left: 1px solid var(--border);
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform var(--tap-duration) ease;
}
.remove-btn:active {
  color: var(--accent-danger);
  transform: scale(var(--tap-scale));
}
</style>
