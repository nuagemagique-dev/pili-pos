<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from "vue";
import { useIntegrationsStore } from "../stores/integrations";
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

// Option picker for a product with option groups (from a mirrored
// catalog); its own chunk. With HubRise off the menu is plain.
const integrations = useIntegrationsStore();
const ModifierDialog = defineAsyncComponent(() => import("../components/ModifierDialog.vue"));
// Product whose options are being picked; the dialog adds it plainly
// when it has none.
const choosing = ref<MenuItem | null>(null);

async function handleAddItem(item: MenuItem) {
  if (item.open_price) {
    pricing.value = item;
    return;
  }
  if (integrations.hubriseEnabled) {
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
      v-if="integrations.hubriseEnabled && choosing"
      :key="choosing.id"
      :item="choosing"
      @done="choosing = null"
    />

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
