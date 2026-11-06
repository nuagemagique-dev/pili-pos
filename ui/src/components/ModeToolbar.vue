<script setup lang="ts">
import { useI18n } from '../i18n'
import { useAllergensStore } from '../stores/allergens'
import UiIcon from './UiIcon.vue'

/**
 * The one row of mode buttons between the category tabs and the product grid (UX audit C1): "Épuisé", "Saisie
 * code", "Allergènes", "Bon cadeau" and, on the right, the list of sold-out products, each with an icon and a short
 * label. A mode that takes over a tap on a product shows its hint at the end of the row; the allergen legend (14
 * numbers) opens in a strip under the row while that mode is on. The modes themselves belong to the order view.
 */
const { canSoldOut, soldOutMode, soldOutCount, codePadOpen, showVoucher } = defineProps<{
  /** Cashier and manager: the "Épuisé" mode and the list of sold-out products. */
  canSoldOut: boolean
  soldOutMode: boolean
  soldOutCount: number
  codePadOpen: boolean
  /** Gift vouchers on, an open order and the right to take a payment. */
  showVoucher: boolean
}>()

const emit = defineEmits<{
  toggleSoldOut: []
  openSoldOutList: []
  toggleCodePad: []
  toggleAllergens: []
  sellVoucher: []
}>()

const { t } = useI18n()
const allergens = useAllergensStore()
</script>

<template>
  <div class="mode-toolbar" data-testid="mode-toolbar">
    <div class="row">
      <button
        v-if="canSoldOut"
        class="mode-btn warn"
        :class="{ on: soldOutMode }"
        :aria-pressed="soldOutMode"
        data-testid="soldout-mode"
        @click="emit('toggleSoldOut')"
      >
        <UiIcon name="soldOut" :size="22" />
        <span class="label">{{ t('soldout_mode') }}</span>
      </button>
      <button
        class="mode-btn"
        :class="{ on: codePadOpen }"
        :aria-pressed="codePadOpen"
        data-testid="code-pad-toggle"
        @click="emit('toggleCodePad')"
      >
        <UiIcon name="codePad" :size="22" />
        <span class="label">{{ t('code_pad_toggle') }}</span>
      </button>
      <button
        class="mode-btn"
        :class="{ on: allergens.mode }"
        :aria-pressed="allergens.mode"
        data-testid="allergen-mode"
        @click="emit('toggleAllergens')"
      >
        <UiIcon name="allergens" :size="22" />
        <span class="label">{{ t('allergens_mode') }}</span>
      </button>
      <button
        v-if="showVoucher"
        class="mode-btn"
        data-testid="gift-voucher-sell"
        @click="emit('sellVoucher')"
      >
        <UiIcon name="giftVoucher" :size="22" />
        <span class="label">{{ t('gv_sell_button') }}</span>
      </button>
      <span v-if="soldOutMode" class="hint" data-testid="soldout-hint">
        {{ t('soldout_mode_hint') }}
      </span>
      <span v-else-if="allergens.mode" class="hint info" data-testid="allergen-hint">
        {{ t('allergens_hint') }}
      </span>
      <span v-else class="spacer" />
      <button
        v-if="canSoldOut"
        class="mode-btn"
        data-testid="soldout-list-button"
        @click="emit('openSoldOutList')"
      >
        <UiIcon name="soldOutList" :size="22" />
        <span class="label">{{ t('soldout_list_button', { n: soldOutCount }) }}</span>
      </button>
    </div>
    <div v-if="allergens.mode" class="legend-strip">
      <span
        v-if="allergens.error && !allergens.sheetItem"
        class="legend-error"
        role="alert"
        data-testid="allergen-legend-error"
        >{{ allergens.error }}</span
      >
      <ul class="allergen-legend" data-testid="allergen-legend" :aria-label="t('allergens_legend')">
        <li v-for="a in allergens.legend" :key="a.code" class="legend-item">
          <span class="legend-num">{{ a.number }}</span>
          <span>{{ t(`allergen_${a.code}`) }}</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.mode-toolbar {
  flex-shrink: 0;
  padding: 0 16px;
}

/* 56 px in all: 8 px of air, then the 48 px buttons. */
.row {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  height: 56px;
  padding-top: 8px;
}

.mode-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 48px;
  padding: 0 14px;
  flex-shrink: 0;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.mode-btn:active {
  transform: scale(var(--tap-scale));
}
.mode-btn.on {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
/* "Épuisé" is the amber mode: it changes what a tap on a product does. */
.mode-btn.warn.on {
  background: var(--accent-warning);
  color: var(--text-on-accent);
}

.spacer {
  flex: 1;
}
.hint {
  flex: 1;
  min-width: 0;
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  overflow: hidden;
  padding: 0 var(--space-xs);
  font-size: var(--font-size-sm);
  line-height: 1.2;
  color: var(--text-secondary);
}
.hint.info {
  color: var(--accent-info);
  font-weight: var(--font-weight-bold);
}

.legend-strip {
  padding-top: 8px;
}
.legend-error {
  display: block;
  margin-bottom: 4px;
  font-size: var(--font-size-sm);
  color: var(--accent-danger);
  font-weight: var(--font-weight-bold);
}
.allergen-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
}
.legend-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.legend-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent-info);
  color: var(--text-on-accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: var(--font-weight-bold);
}
</style>
