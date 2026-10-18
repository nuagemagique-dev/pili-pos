<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useMenuStore } from "../stores/menu";
import { useOrderStore } from "../stores/order";
import { useAuthStore } from "../stores/auth";
import { useI18n } from "../i18n";
import * as ipc from "../api/ipc";
import AppHeader from "../components/AppHeader.vue";
import CategoryTabs from "../components/CategoryTabs.vue";
import MenuGrid from "../components/MenuGrid.vue";
import CartSidebar from "../components/CartSidebar.vue";
import PriceDialog from "../components/PriceDialog.vue";
import SoldOutDialog from "../components/SoldOutDialog.vue";
import SoldOutList from "../components/SoldOutList.vue";
import { useToastStore } from "../stores/toast";
import type { MenuItem } from "../types";

const router = useRouter();
const menu = useMenuStore();
const order = useOrderStore();
const auth = useAuthStore();
const { t } = useI18n();

onMounted(async () => {
  if (!auth.currentUser) {
    router.push({ name: "login" });
    return;
  }
  await menu.loadCategories();
  if (menu.selectedCategoryId) {
    await menu.loadItems(menu.selectedCategoryId);
  }
  if (auth.canTakePayment) await menu.loadSoldOut();
  // Z-report reminder - check on mount + every 5 min.
  checkZReport();
  zCheckInterval = setInterval(checkZReport, 5 * 60 * 1000);

  // No active order → bounce to Tables so the operator consciously
  // picks a table OR taps "À emporter". Avoids silently creating a
  // phantom Table 1 when the user just navigated here.
  if (!order.currentOrder) {
    router.push({ name: "tables" });
  }
});

onUnmounted(() => clearInterval(zCheckInterval));

const selectedCategoryColor = computed(() => {
  const cat = menu.categories.find((c) => c.id === menu.selectedCategoryId);
  return cat?.color ?? null;
});

/** Quantity of each product already in the cart, for the badges on
 * the menu buttons. */
const cartQuantities = computed(() => {
  const map = new Map<number, number>();
  for (const line of order.lines) {
    map.set(line.menu_item_id, (map.get(line.menu_item_id) ?? 0) + line.quantity);
  }
  return map;
});

// Z-report daily reminder - polls every 5 min, only shows after 22:00.
const showZWarning = ref(false);
let zCheckInterval: ReturnType<typeof setInterval>;

async function checkZReport() {
  try {
    showZWarning.value = await ipc.needsZReport();
  } catch {
    // Non-critical - degrade silently.
  }
}

// Free-price product waiting for its price (PriceDialog open).
const pricing = ref<MenuItem | null>(null);

// Option picker for a product with option groups (from a mirrored catalog or made in the menu
// editor); its own chunk.
const ModifierDialog = defineAsyncComponent(() => import("../components/ModifierDialog.vue"));
// Product whose options are being picked; the dialog adds it plainly
// when it has none.
const choosing = ref<MenuItem | null>(null);

// "Épuisé" mode (cashier and manager): a tap on a product opens its availability dialog
// instead of adding it. Out of the mode a sold-out product is greyed and a tap shows why
// it cannot be added.
const soldOutMode = ref(false);
const soldOutItem = ref<MenuItem | null>(null);
const showSoldOutList = ref(false);

function toggleSoldOutMode() {
  soldOutMode.value = !soldOutMode.value;
}

async function handleAddItem(item: MenuItem) {
  if (soldOutMode.value) {
    soldOutItem.value = item;
    return;
  }
  if (item.sold_out) {
    useToastStore().show(t("err_product_sold_out"), "error", 4000);
    // The end of the day may have passed since the grid was read.
    if (menu.selectedCategoryId) await menu.loadItems(menu.selectedCategoryId);
    return;
  }
  if (item.open_price) {
    pricing.value = item;
    return;
  }
  // The option picker opens for a product that has option groups (V11-D18), whoever manages the
  // menu: the local editor creates them too.
  if (item.has_modifiers) {
    choosing.value = item;
    return;
  }
  await order.addItem(item);
}

async function handlePriceConfirmed(priceCents: number) {
  const item = pricing.value;
  pricing.value = null;
  if (item) await order.addItem(item, 1, priceCents);
}
</script>

<template>
  <!--
    UX guide §3: header | menu (categories above the grid) | cart 670 px
    on the right, main action at the bottom right.
  -->
  <div class="order-view">
    <AppHeader />

    <!-- Z-report daily reminder - §8.2 non-blocking warning banner -->
    <div v-if="showZWarning" class="z-warning">
      {{ t('z_report_warning') }}
    </div>

    <div class="main">
      <div class="menu-section">
        <CategoryTabs
          :categories="menu.categories"
          :selected-id="menu.selectedCategoryId"
          @select="menu.selectCategory($event)"
        />
        <div v-if="auth.canTakePayment" class="soldout-bar">
          <button
            class="soldout-btn"
            :class="{ active: soldOutMode }"
            :aria-pressed="soldOutMode"
            data-testid="soldout-mode"
            @click="toggleSoldOutMode"
          >
            {{ t('soldout_mode') }}
          </button>
          <span v-if="soldOutMode" class="soldout-hint" data-testid="soldout-hint">
            {{ t('soldout_mode_hint') }}
          </span>
          <button
            class="soldout-btn list"
            data-testid="soldout-list-button"
            @click="showSoldOutList = true"
          >
            {{ t('soldout_list_button', { n: menu.soldOut.length }) }}
          </button>
        </div>
        <MenuGrid
          :items="menu.items"
          :category-color="selectedCategoryColor"
          :cart-quantities="cartQuantities"
          @add-item="handleAddItem"
        />
      </div>
      <CartSidebar />
    </div>

    <component
      :is="ModifierDialog"
      v-if="choosing"
      :key="choosing.id"
      :item="choosing"
      @done="choosing = null"
    />

    <SoldOutDialog v-if="soldOutItem" :item="soldOutItem" @done="soldOutItem = null" />
    <SoldOutList v-if="showSoldOutList" @done="showSoldOutList = false" />

    <PriceDialog
      v-if="pricing"
      :item-name="pricing.name"
      @confirm="handlePriceConfirmed"
      @cancel="pricing = null"
    />
  </div>
</template>

<style scoped>
.order-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: var(--bg-primary);
}

/* §8.2 non-blocking warning - amber, full-width, gone once the Z is
   done. Fires only after 22:00. */
.z-warning {
  padding: 14px 20px;
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  flex-shrink: 0;
}

.soldout-bar {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: 8px 16px 0;
  flex-shrink: 0;
}
.soldout-btn {
  min-height: 56px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-primary);
  border: 2px solid transparent;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.soldout-btn:active { transform: scale(var(--tap-scale)); }
.soldout-btn.active {
  background: var(--accent-warning);
  color: var(--text-on-accent);
}
.soldout-btn.list { margin-left: auto; }
.soldout-hint { color: var(--text-secondary); font-size: var(--font-size-md); }

.main {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.menu-section {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
