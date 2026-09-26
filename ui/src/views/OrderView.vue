<script setup lang="ts">
import { describeError } from "../utils/errorMessages";
import { formatTime } from "../utils/datetime";
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useMenuStore } from "../stores/menu";
import { useOrderStore } from "../stores/order";
import { useAuthStore } from "../stores/auth";
import { usePosStore } from "../stores/pos";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import * as ipc from "../api/ipc";
import CategoryTabs from "../components/CategoryTabs.vue";
import MenuGrid from "../components/MenuGrid.vue";
import CartSidebar from "../components/CartSidebar.vue";
import PriceDialog from "../components/PriceDialog.vue";
import PinDialog from "../components/PinDialog.vue";
import FdmStatus from "../components/FdmStatus.vue";
import PrinterStatus from "../components/PrinterStatus.vue";
import { HUBRISE_ENABLED } from "../features";
import type { MenuItem } from "../types";

// Webshop components are chosen here rather than with a template v-if:
// the template compiler wraps imported bindings in unref(), which hides
// the constant from Rollup. A folded ternary lets the cert build
// (VITE_FEATURE_HUBRISE=false) drop the components and their stores.
const HubriseStatus = HUBRISE_ENABLED
  ? defineAsyncComponent(() => import("../components/HubriseStatus.vue"))
  : null;
const OnlineOrdersNavButton = HUBRISE_ENABLED
  ? defineAsyncComponent(() => import("../components/OnlineOrdersNavButton.vue"))
  : null;

const router = useRouter();
const menu = useMenuStore();
const order = useOrderStore();
const auth = useAuthStore();
const pos = usePosStore();
const toast = useToastStore();
const { t } = useI18n();

const clock = ref("");
let clockInterval: ReturnType<typeof setInterval>;

function updateClock() {
  clock.value = formatTime(new Date());
}

onMounted(async () => {
  if (!auth.currentUser) {
    router.push({ name: "login" });
    return;
  }
  updateClock();
  clockInterval = setInterval(updateClock, 10_000);

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

onUnmounted(() => {
  clearInterval(clockInterval);
  clearInterval(zCheckInterval);
});

const selectedCategoryColor = computed(() => {
  const cat = menu.categories.find((c) => c.id === menu.selectedCategoryId);
  return cat?.color ?? null;
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

/** Active-nav helpers - each workspace button is highlighted when
 * the current order belongs to that mode, so the nav mirrors the
 * "you are here" state at all times (UX Guide §2.3). */
const isDineInMode = computed(
  () => order.currentOrder?.order_type === 'dine_in',
);
const isTakeawayMode = computed(
  () => order.currentOrder?.order_type === 'takeaway',
);

// The operator changing their own PIN (tap on their name).
const pinOpen = ref(false);

// Free-price product waiting for its price (PriceDialog open).
const pricing = ref<MenuItem | null>(null);

async function handleAddItem(item: MenuItem) {
  if (item.open_price) {
    pricing.value = item;
    return;
  }
  await order.addItem(item);
}

async function handlePriceConfirmed(priceCents: number) {
  const item = pricing.value;
  pricing.value = null;
  if (item) await order.addItem(item, 1, priceCents);
}

async function handleLogout() {
  // Navigate first: the router guard registers the open order (P)
  // before the order is cleared.
  await router.push({ name: "login" });
  order.clearOrder();
  void auth.logout();
}

/**
 * Start a fresh takeaway order. Guarded: if the staff is already
 * on an active takeaway, tapping the button again is a no-op
 * instead of silently replacing the in-progress cart.
 */
async function handleTakeaway() {
  if (isTakeawayMode.value) return;
  try {
    await order.registerPending();
  } catch (e) {
    toast.show(`${t("order_register_failed")} : ${describeError(e)}`, "error", 6000);
    return;
  }
  try {
    await order.startOrder(null);
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  }
}
</script>

<template>
  <!--
    UX Guide §3: layout principal
    Header fixe 48px | FdmStatus | User | Table | Heure
    65% menu grid | 35% cart sidebar
  -->
  <div class="order-view" :class="{ training: pos.trainingMode }">
    <header class="header">
      <FdmStatus />
      <PrinterStatus />
      <component :is="HubriseStatus" v-if="HubriseStatus" />
      <button class="header-user" :aria-label="t('pin_change')" @click="pinOpen = true">{{ auth.currentUser?.name }}</button>
      <span v-if="order.currentOrder?.table_number" class="header-table">
        {{ t('table') }} {{ order.currentOrder.table_number }}
      </span>
      <span v-else-if="order.currentOrder?.order_type === 'takeaway'" class="header-takeaway">
        {{ t('takeaway_badge') }}
      </span>
      <span class="header-clock">{{ clock }}</span>
      <nav class="header-nav">
        <button
          class="nav-btn"
          :class="{ 'nav-btn-active': isDineInMode }"
          @click="router.push({ name: 'tables' })"
        >{{ t('tables') }}</button>
        <button
          class="nav-btn"
          :class="{ 'nav-btn-active': isTakeawayMode }"
          @click="handleTakeaway"
        >{{ t('takeaway_new') }}</button>
        <component :is="OnlineOrdersNavButton" v-if="OnlineOrdersNavButton" />
        <button v-if="auth.isManager" class="nav-btn" @click="router.push({ name: 'reports' })">{{ t('reports') }}</button>
        <button v-if="auth.isManager" class="nav-btn" @click="router.push({ name: 'settings' })">{{ t('config') }}</button>
        <button class="nav-btn nav-logout" @click="handleLogout">{{ t('logout') }}</button>
      </nav>
    </header>

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
        <MenuGrid :items="menu.items" :category-color="selectedCategoryColor" @add-item="handleAddItem" />
      </div>
      <CartSidebar />
    </div>

    <PinDialog
      v-if="pinOpen && auth.currentUser"
      :insz="auth.currentUser.insz"
      :user-name="auth.currentUser.name"
      @done="pinOpen = false"
      @cancel="pinOpen = false"
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
  height: 100vh;
  background: var(--bg-primary);
}

/* UX Guide §9: training = bg violet */
.order-view.training {
  --bg-primary: #2d1b4e;
}

/* UX Guide §3: header fixe 48px */
.header {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding: 0 var(--space-md);
  background: var(--bg-secondary);
  border-bottom: 1px solid rgba(160, 160, 176, 0.1);
  flex-shrink: 0;
  height: 48px;
}

.header-user {
  min-height: 48px;
  padding: 0 var(--space-sm);
  border-radius: var(--radius-sm);
  background: transparent;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}

.header-table {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
}

/* Takeaway badge - same visual weight as the table badge but in
   the warning color so staff can tell at a glance they're on a
   non-seated order. */
.header-takeaway {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--accent-warning);
  padding: 2px var(--space-sm);
  border: 1px solid var(--accent-warning);
  border-radius: var(--radius-sm);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.header-clock {
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.header-nav {
  margin-left: auto;
  display: flex;
  gap: var(--space-xs);
}

.nav-btn {
  position: relative;
  padding: var(--space-xs) var(--space-md);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-secondary);
  min-height: 36px;
  transition: transform var(--tap-duration) ease;
}
.nav-btn:active { transform: scale(var(--tap-scale)); }
.nav-btn-active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.nav-logout { color: var(--accent-danger); }

/* §8.2 non-blocking warning - amber, full-width, auto-dismiss
   when the Z is done. Fires only after 22:00. */
.z-warning {
  padding: var(--space-sm) var(--space-md);
  background: var(--accent-warning);
  color: var(--bg-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  text-align: center;
  flex-shrink: 0;
}

/* UX Guide §3: 65% menu / 35% cart */
.main {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.menu-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
