<script setup lang="ts">
// The "Plus" menu of the cart header (UX audit C3): the rare cart-level
// actions, out of the always-visible footer. Each row appears only for the
// roles and orders that have the action today (the caller decides).
import { useI18n } from '../i18n'

defineProps<{
  /** Move the order to another table. */
  canTransfer: boolean
  /** A manager gives or removes the order discount. */
  canDiscount: boolean
  /** An order discount applies: the row removes it. */
  orderDiscounted: boolean
  /** A manager closes the order unpaid (customer left). */
  canCloseUnpaid: boolean
}>()

const emit = defineEmits<{
  transfer: []
  discount: []
  'discount-remove': []
  unpaid: []
  cancel: []
}>()

const { t } = useI18n()
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="t('cart_more_title')" data-testid="cart-more">
      <h2 class="title">{{ t('cart_more_title') }}</h2>
      <div class="rows">
        <button
          v-if="canTransfer"
          class="row"
          data-testid="transfer-table"
          @click="emit('transfer')"
        >
          {{ t('transfer_table') }}
        </button>
        <button
          v-if="canDiscount && !orderDiscounted"
          class="row"
          data-testid="discount-open"
          @click="emit('discount')"
        >
          {{ t('discount_button') }}
        </button>
        <button
          v-if="canDiscount && orderDiscounted"
          class="row"
          data-testid="discount-remove"
          @click="emit('discount-remove')"
        >
          {{ t('discount_remove') }}
        </button>
        <button
          v-if="canCloseUnpaid"
          class="row unpaid-btn"
          data-testid="unpaid-close"
          @click="emit('unpaid')"
        >
          {{ t('unpaid_close') }}
        </button>
      </div>
      <button class="close" data-testid="cart-more-close" @click="emit('cancel')">
        {{ t('cart_more_close') }}
      </button>
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
  width: min(560px, 92vw);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.rows {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.row,
.close {
  height: 64px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.row:active,
.close:active {
  transform: scale(var(--tap-scale));
}
/* Rare, destructive: outlined red, apart from the plain rows. */
.unpaid-btn {
  background: transparent;
  border: 1px solid var(--accent-danger);
  color: var(--accent-danger);
}
.close {
  height: 56px;
}
</style>
