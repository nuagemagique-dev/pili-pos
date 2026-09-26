<script setup lang="ts">
import { describeError } from "../utils/errorMessages";
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { useOrderStore } from "../stores/order";
import { useAuthStore } from "../stores/auth";
import { useMenuStore } from "../stores/menu";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import { formatPrice } from "../utils/format";
import OrderLineItem from "./OrderLineItem.vue";
import VatSummary from "./VatSummary.vue";

const router = useRouter();
const order = useOrderStore();
const auth = useAuthStore();
const menu = useMenuStore();
const toast = useToastStore();
const { t } = useI18n();

function goToCheckout() {
  if (order.lines.length > 0) {
    router.push({ name: "checkout" });
  }
}

// Provisional bill for a table (§1.4). Takeaway sales are paid at once.
const canPreBill = computed(
  () => order.currentOrder?.order_type === "dine_in" && order.lines.length > 0,
);
const preBilling = ref(false);

async function printPreBill() {
  if (!canPreBill.value || preBilling.value) return;
  preBilling.value = true;
  try {
    await order.preBill();
    toast.show(t("pre_bill_printed"), "success");
  } catch (e) {
    toast.show(`${t("pre_bill_failed")} : ${describeError(e)}`, "error", 6000);
  } finally {
    preBilling.value = false;
  }
}
</script>

<template>
  <!-- UX Guide §3: cart sidebar TOUJOURS à droite, ~35% largeur -->
  <aside class="cart">
    <div class="cart-header">
      <h2>{{ t('order') }}</h2>
      <span v-if="order.currentOrder?.table_number" class="table-badge">
        T{{ order.currentOrder.table_number }}
      </span>
    </div>

    <div class="cart-lines scroll-touch">
      <OrderLineItem
        v-for="line in order.lines"
        :key="line.id"
        :line="line"
        :category-name="menu.getCategoryName(order.getCategoryId(line.menu_item_id) ?? 0)"
        @increment="order.updateQuantity($event, (order.lines.find(l => l.id === $event)?.quantity ?? 0) + 1)"
        @decrement="order.updateQuantity($event, (order.lines.find(l => l.id === $event)?.quantity ?? 0) - 1)"
        @remove="order.removeLine($event)"
      />
      <div v-if="order.lines.length === 0" class="cart-empty">
        {{ t('no_items') }}
      </div>
    </div>

    <VatSummary :lines="order.lines" />

    <div class="cart-footer">
      <div class="cart-total">
        <span>{{ t('total') }}</span>
        <span class="total-amount">{{ formatPrice(order.totalCents) }}</span>
      </div>

      <div class="cart-actions">
        <button
          v-if="auth.canTakePayment && order.currentOrder?.order_type === 'dine_in'"
          class="pre-bill-btn"
          :disabled="!canPreBill || preBilling"
          @click="printPreBill"
        >
          {{ t('pre_bill') }}
        </button>
        <!-- UX Guide §2.3: bouton principal TOUJOURS en bas à droite -->
        <button
          v-if="auth.canTakePayment"
          class="checkout-btn"
          :disabled="order.lines.length === 0"
          @click="goToCheckout"
        >
          {{ t('pay') }}
        </button>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.cart {
  width: 670px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid rgba(160, 160, 176, 0.1);
}

.cart-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-sm) var(--space-md);
  border-bottom: 1px solid rgba(160, 160, 176, 0.1);
}

.cart-header h2 {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}

.table-badge {
  padding: var(--space-xs) var(--space-md);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
}

.cart-lines {
  flex: 1;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior-y: contain;
  padding: 0 var(--space-md);
}

.cart-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 120px;
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}

.cart-footer {
  border-top: 1px solid rgba(160, 160, 176, 0.1);
  padding: var(--space-md);
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.cart-total {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}

.total-amount {
  color: var(--accent-primary);
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  font-variant-numeric: tabular-nums;
}

.cart-actions {
  display: flex;
  gap: var(--space-sm);
}

/* Secondary action, left of the primary PAY button (UX Guide §2.3). */
.pre-bill-btn {
  flex: 0 0 35%;
  height: 72px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.pre-bill-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}
.pre-bill-btn:active { transform: scale(var(--tap-scale)); }

/* UX Guide §4.1: pleine largeur × 72px */
.checkout-btn {
  flex: 1;
  width: 100%;
  height: 72px;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
  letter-spacing: 0.05em;
}

.checkout-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}

/* UX Guide §4.2: darker on active */
.checkout-btn:active {
  transform: scale(var(--tap-scale));
  filter: brightness(0.85);
}
</style>
