<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { CartLine } from '../types'
import { formatPrice, formatWeight } from '../utils/format'
import { lineCategory, useBilingualName } from '../utils/names'
import { usePosStore } from '../stores/pos'
import { useI18n } from '../i18n'
import { lineNetCents } from '../utils/cartAmounts'
import { GIFT_VOUCHER_DEPARTMENT } from '../stores/giftVouchers'

const {
  line,
  pending,
  canOffer,
  canRemoveDiscount,
  canNote,
  showCourse,
  showState,
  selected,
  flash = 0,
} = defineProps<{
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
  /** Dine-in order: the line carries its kitchen mark (sent check, amber dot). */
  showState?: boolean
  /** The line is the selected one: its actions show under it (UX audit C2). */
  selected?: boolean
  /** Grows with each add that went to this line (0 when none): the line flashes on every change. */
  flash?: number
}>()

const emit = defineEmits<{
  select: [id: number]
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
const pos = usePosStore()
/** The product's category (one name can be in several categories), in the detail of the selected line. */
const sub = computed(() =>
  line.department_id === GIFT_VOUCHER_DEPARTMENT ? '' : (lineCategory(line, pos.language) ?? ''),
)
/** The menu code, small and secondary; it leads (bold) when products are shown by code. */
const codeText = computed(() => line.code?.trim() ?? '')
/** The product name in the UI language: the code leads in code mode, so the name is then the secondary one. */
const nameText = computed(() =>
  names.value.isCode ? (names.value.secondary ?? '') : names.value.primary,
)
/** A gift voucher sold: the quantity is fixed (one line per voucher). */
const isVoucher = computed(() => line.department_id === GIFT_VOUCHER_DEPARTMENT)
/** A weighing: the scale gave the quantity, so it cannot be edited. To change
 * it the line is removed and the product weighed again. */
const weight = computed(() => line.weight_grams ?? null)
/** A paid option of a weighed bowl (its broth): it follows the bowl, no control of its own. */
const isBowlOption = computed(() => line.bowl_line_id !== null && line.bowl_line_id !== undefined)
/** The options picked with a weighed bowl ("Bouillon tomate · Piquant moyen"). */
const bowlOptions = computed(() => (line.bowl_options ?? []).join(' · '))

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
const canOfferHere = computed(() => Boolean(canOffer) && !isVoucher.value && !isBowlOption.value)
const canNoteHere = computed(() => Boolean(canNote) && !isVoucher.value && !isBowlOption.value)
/** A bowl's option is not selectable: it follows its bowl. */
const selectable = computed(() => !isBowlOption.value)
/** The first change of the line as the chip of the row ("Offert", "Remise 50 %"). */
const changeChip = computed(() => changes.value[0]?.name ?? '')
/** The course tag of the row: the course name, locked once a slip carried the line. */
const courseTag = computed(() => showCourse && !isVoucher.value && !isBowlOption.value)

const lineRef = ref<HTMLElement | null>(null)

/** Reactive, so that a class change of the row (selected, pending) does not wipe the flash. */
const flashing = ref(false)

async function flashAdded() {
  // Off, one frame, then on: the animation restarts on every add.
  flashing.value = false
  await nextTick()
  void lineRef.value?.offsetWidth
  flashing.value = true
}

// The 200 ms green flash of an add (C9) on the line it went to.
watch(
  () => flash,
  (seq) => {
    if (seq) void flashAdded()
  },
)

// The selected line opens under the row: keep it in view in the scrolling cart.
watch(
  () => selected,
  (on) => {
    if (on) void nextTick().then(() => lineRef.value?.scrollIntoView?.({ block: 'nearest' }))
  },
  { flush: 'post' },
)

// Called externally to trigger the green flash on a new line.
defineExpose({ flashAdded })
</script>

<template>
  <!-- One 56 px row (UX audit C2): kitchen mark, quantity, code, name, chips, price. The actions
       open under the selected line only. -->
  <div
    ref="lineRef"
    class="order-line"
    :class="{ pending, selected, 'bowl-option': isBowlOption, 'flash-added': flashing }"
    :data-line-id="line.id"
  >
    <component
      :is="selectable ? 'button' : 'div'"
      class="line-row"
      :type="selectable ? 'button' : undefined"
      :aria-expanded="selectable ? Boolean(selected) : undefined"
      :aria-label="selectable ? t('line_select') : undefined"
      :data-testid="selectable ? `line-row-${line.id}` : undefined"
      @click="selectable && emit('select', line.id)"
    >
      <span
        class="state-mark"
        :class="{ sent: showState && !pending, wait: pending }"
        :data-testid="pending ? 'line-state-pending' : showState ? 'line-state-sent' : undefined"
        :role="showState || pending ? 'img' : undefined"
        :aria-label="
          pending ? t('line_state_pending') : showState ? t('line_state_sent') : undefined
        "
        >{{ pending ? '' : showState ? '✓' : '' }}</span
      >
      <span v-if="isVoucher" class="line-lead voucher-badge" data-testid="voucher-badge">{{
        t('gv_line_badge')
      }}</span>
      <span v-else-if="isBowlOption" class="line-lead bowl-link" data-testid="bowl-option">+</span>
      <span v-else-if="weight !== null" class="line-lead weight-badge" data-testid="line-weight">{{
        formatWeight(weight)
      }}</span>
      <span v-else class="line-lead line-qty" data-testid="line-qty">{{ line.quantity }}×</span>
      <span class="line-main">
        <span v-if="codeText" class="line-code" :class="{ lead: names.isCode }">{{
          codeText
        }}</span>
        <span class="line-name">{{ nameText }}</span>
        <span v-if="bowlOptions" class="chip chip-note-opts" data-testid="line-bowl-options-chip">{{
          bowlOptions
        }}</span>
        <span v-if="notes" class="chip chip-opts">{{ notes }}</span>
        <span v-if="line.kitchen_note" class="chip chip-kitchen" data-testid="line-kitchen-note">{{
          line.kitchen_note
        }}</span>
        <span
          v-if="courseTag"
          class="chip chip-course"
          :class="[line.kitchen_state]"
          :data-testid="`course-tag-${line.id}`"
          >{{ t(`course_${line.course}`)
          }}<template v-if="line.kitchen_state === 'held'">
            · {{ t('course_held_tag') }}</template
          ></span
        >
        <span v-if="changeChip" class="chip chip-change" data-testid="line-change-chip">{{
          changeChip
        }}</span>
      </span>
      <span class="line-price">{{ formatPrice(lineNetCents(line)) }}</span>
    </component>

    <!-- The selected line: what the row cannot say, then its actions. -->
    <div v-if="selected && selectable" class="line-detail" data-testid="line-detail">
      <div
        v-if="sub || weight !== null || bowlOptions || notes || line.kitchen_note || changes.length"
        class="detail-info"
      >
        <span v-if="sub" class="line-sub" data-testid="line-sub">{{ sub }}</span>
        <span v-if="weight !== null" class="line-per-kilo" data-testid="line-per-kilo"
          >{{ formatPrice(line.unit_price_cents) }} €/kg</span
        >
        <span v-if="bowlOptions" class="line-notes" data-testid="line-bowl-options">{{
          bowlOptions
        }}</span>
        <span v-if="notes" class="line-notes" data-testid="line-notes">{{ notes }}</span>
        <span
          v-if="line.kitchen_note"
          class="line-kitchen-note"
          data-testid="line-kitchen-note-full"
          >{{ line.kitchen_note }}</span
        >
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
      <div class="detail-actions">
        <div v-if="!isVoucher && weight === null" class="qty-controls">
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
        <button
          v-if="canNoteHere"
          class="act-btn note-btn"
          :class="{ filled: !!line.kitchen_note }"
          data-testid="line-note"
          :aria-label="t('note_button')"
          @click="emit('note', line.id)"
        >
          {{ t('note_button') }}
        </button>
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
        <!-- Course picker: locked once a slip carried the line. -->
        <button
          v-if="courseTag"
          class="act-btn course-btn"
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
        <!-- Destructive: last, set apart by a rule (§4.3). -->
        <button
          class="act-btn remove-btn"
          data-testid="line-remove"
          :aria-label="t('line_remove')"
          @click="emit('remove', line.id)"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
            <path
              d="M2 2l10 10M12 2L2 12"
              stroke="currentColor"
              stroke-width="2.2"
              stroke-linecap="round"
            />
          </svg>
          {{ t('line_delete') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.order-line {
  display: flex;
  flex-direction: column;
  border-bottom: 1px solid var(--border);
  transition: box-shadow var(--flash-duration) ease;
}
.order-line.pending {
  border-left: 3px solid var(--accent-warning);
}
.order-line.selected {
  background: var(--bg-surface);
  border-radius: var(--radius-md);
}

/* The row: 56 px, the whole line is the tap target that selects it. */
.line-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 56px;
  padding: 0 4px;
  text-align: left;
  color: var(--text-primary);
}
.line-row:active {
  background: var(--bg-pressed);
}
div.line-row:active {
  background: none;
}

/* Kitchen mark: a check when the bar and kitchen have the line, an amber dot while they do not. */
.state-mark {
  flex: 0 0 18px;
  width: 18px;
  text-align: center;
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.state-mark.wait {
  height: 10px;
  width: 10px;
  flex-basis: 10px;
  margin: 0 4px;
  border-radius: 50%;
  background: var(--accent-warning);
}

.line-lead {
  flex: 0 0 auto;
  min-width: 52px;
  text-align: left;
}
.line-qty {
  font-family: var(--font-mono);
  font-size: 20px;
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
}
.voucher-badge,
.weight-badge {
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  text-align: center;
  white-space: nowrap;
}
.voucher-badge {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.weight-badge {
  background: var(--bg-surface);
  font-family: var(--font-mono);
}
.bowl-link {
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  text-align: center;
}

.line-main {
  flex: 1 1 0;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
}
.line-code {
  flex: 0 0 auto;
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-secondary);
}
.line-code.lead {
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
}
.line-name {
  flex: 0 1 auto;
  min-width: 3ch;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 17px;
  font-weight: var(--font-weight-bold);
}

/* Chips of the row: inline, truncated, never wrapping. */
.chip {
  flex: 0 1 auto;
  min-width: 0;
  max-width: 34%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 2px 10px;
  border-radius: 12px;
  background: var(--bg-surface);
  font-size: 13px;
  line-height: 1.4;
  color: var(--text-secondary);
}
.order-line.selected .chip {
  background: var(--bg-primary);
}
.chip-kitchen {
  font-style: italic;
}
.chip-opts,
.chip-note-opts {
  color: var(--accent-warning);
}
.chip-course {
  font-family: var(--font-mono);
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
}
.chip-course.held {
  color: var(--accent-warning);
}
.chip-course.printed {
  opacity: 0.6;
}
.chip-change {
  color: var(--accent-warning);
  font-weight: var(--font-weight-bold);
}

.line-price {
  flex: 0 0 auto;
  min-width: 78px;
  text-align: right;
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
}

/* A bowl's paid option sits under its bowl, without controls. */
.order-line.bowl-option .line-row {
  min-height: 44px;
}

/* The selected line: its detail and actions. */
.line-detail {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 0 4px 10px;
}
.detail-info {
  display: flex;
  flex-direction: column;
  padding-left: 32px;
  gap: 2px;
}
.line-sub,
.line-per-kilo {
  font-size: 13px;
  color: var(--text-secondary);
}
.line-per-kilo {
  font-family: var(--font-mono);
}
.line-notes {
  font-size: 13px;
  color: var(--accent-warning);
}
.line-kitchen-note {
  font-size: 14px;
  font-style: italic;
  color: var(--text-secondary);
  overflow-wrap: anywhere;
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

.detail-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
.qty-controls {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  border-radius: var(--radius-md);
  background: var(--bg-primary);
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

.act-btn {
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.act-btn:active {
  transform: scale(var(--tap-scale));
}
.act-btn:disabled {
  opacity: 0.45;
  pointer-events: none;
}
.note-btn.filled {
  background: var(--bg-pressed);
}
.course-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono);
}
.course-btn.held {
  color: var(--accent-warning);
}
.chip-held {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
/* Destructive: pushed to the right, set apart by a rule, never where "−" is (§4.3). */
.remove-btn {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  border-left: 1px solid var(--border);
  color: var(--accent-danger);
}
</style>
