<script setup lang="ts">
/**
 * The one header of every staff screen (UX guide §2.3, §3): logo,
 * device lights, operator, table, clock, workspaces and the lock.
 * 64 px high so its buttons can be 48 px (§4.1, §13).
 */
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from "vue";
import { useIntegrationsStore } from "../stores/integrations";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { useOrderStore } from "../stores/order";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";
import { formatTime } from "../utils/datetime";
import PiliLogo from "./PiliLogo.vue";
import StatusLights from "./StatusLights.vue";
import PinDialog from "./PinDialog.vue";
import FdmAlerts from "./FdmAlerts.vue";
import TrainingBanner from "./TrainingBanner.vue";

// While HubRise runs, or online orders are still to settle.
const OnlineOrdersNavButton = defineAsyncComponent(() => import("./OnlineOrdersNavButton.vue"));
const integrations = useIntegrationsStore();

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const order = useOrderStore();
const toast = useToastStore();
const { t } = useI18n();

const clock = ref(formatTime(new Date()));
let clockTimer: ReturnType<typeof setInterval> | undefined;
onMounted(() => {
  clockTimer = setInterval(() => (clock.value = formatTime(new Date())), 10_000);
});
onUnmounted(() => clearInterval(clockTimer));

const onOrderScreen = computed(() => route.name === "order" || route.name === "checkout");
const orderType = computed(() => (onOrderScreen.value ? order.currentOrder?.order_type : undefined));
const tableNumber = computed(() => (orderType.value === "dine_in" ? order.currentOrder?.table_number : null));

// "You are here" (§2.3): a dine-in order belongs to Tables.
const tablesActive = computed(() => route.name === "tables" || orderType.value === "dine_in");
const takeawayActive = computed(() => orderType.value === "takeaway");

const roleLabel = computed(() => (auth.currentUser ? t(`role_${auth.currentUser.role}`) : ""));

// The operator changing their own PIN (tap on their name).
const pinOpen = ref(false);

/** Start a fresh takeaway order. A second tap while already on a
 * takeaway is a no-op instead of silently replacing the cart. */
async function startTakeaway() {
  if (route.name === "order" && takeawayActive.value) return;
  if (route.name === "order") {
    // Staying on the order screen: register the table order (P) here,
    // since the router guard only does it when leaving the screen.
    try {
      await order.registerPending();
    } catch (e) {
      toast.show(`${t("order_register_failed")} : ${describeError(e)}`, "error", 6000);
      return;
    }
  }
  try {
    await order.startOrder(null);
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
    return;
  }
  if (route.name !== "order") await router.push({ name: "order" });
}

/** Lock the screen. The operator stays on shift: clocking out is done
 * from the login screen ("Fin de service"). */
async function lock() {
  // Navigate first: the router guard registers the open order (P)
  // while the operator's session is still open.
  await router.push({ name: "login" });
  order.clearOrder();
  void auth.lock();
}
</script>

<template>
  <header class="app-header">
    <PiliLogo :height="24" color="var(--text-primary)" mark-color="var(--accent-primary)" />
    <span class="divider" />
    <StatusLights />
    <span class="divider" />
    <button class="header-user" :aria-label="t('pin_change')" @click="pinOpen = true">
      <span class="user-name">{{ auth.currentUser?.name }}</span>
      <span class="user-role">{{ roleLabel }} · PIN</span>
    </button>
    <span v-if="tableNumber" class="header-table">{{ t('table') }} {{ tableNumber }}</span>
    <span v-else-if="takeawayActive" class="header-takeaway">{{ t('takeaway_badge') }}</span>
    <span class="header-clock">{{ clock }}</span>

    <nav class="header-nav">
      <div class="segmented">
        <button
          class="nav-btn"
          :class="{ 'nav-btn-active': tablesActive }"
          @click="router.push({ name: 'tables' })"
        >{{ t('tables') }}</button>
        <button
          class="nav-btn"
          :class="{ 'nav-btn-active': takeawayActive }"
          @click="startTakeaway"
        >{{ t('takeaway_new') }}</button>
        <OnlineOrdersNavButton v-if="integrations.hubriseVisible" />
      </div>
      <div v-if="auth.isManager" class="admin-nav">
        <button
          class="nav-btn nav-admin"
          :class="{ 'nav-btn-active': route.name === 'reports' }"
          @click="router.push({ name: 'reports' })"
        >{{ t('reports') }}</button>
        <button
          class="nav-btn nav-admin"
          :class="{ 'nav-btn-active': route.name === 'settings' }"
          @click="router.push({ name: 'settings' })"
        >{{ t('config') }}</button>
      </div>
      <button class="nav-btn nav-logout" @click="lock">{{ t('lock') }}</button>
    </nav>
  </header>
  <TrainingBanner />
  <FdmAlerts />

  <PinDialog
    v-if="pinOpen && auth.currentUser"
    :insz="auth.currentUser.insz"
    :user-name="auth.currentUser.name"
    @done="pinOpen = false"
    @cancel="pinOpen = false"
  />
</template>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 0 20px;
  height: var(--header-height);
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
}

.divider {
  width: 1px;
  height: 32px;
  background: var(--border);
  flex-shrink: 0;
}

.header-user {
  height: 48px;
  padding: 0 10px;
  border-radius: var(--radius-sm);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  color: var(--text-primary);
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.header-user:active { transform: scale(var(--tap-scale)); background: var(--bg-surface); }
.user-name { font-size: 16px; font-weight: var(--font-weight-bold); line-height: 1.15; }
.user-role { font-size: 12px; color: var(--text-secondary); line-height: 1.2; }

/* The current table: outlined in Brume, never an action colour. */
.header-table,
.header-takeaway {
  height: 40px;
  padding: 0 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--text-primary);
  font-size: 17px;
  font-weight: var(--font-weight-heavy);
  display: flex;
  align-items: center;
  white-space: nowrap;
}
/* Takeaway: amber, so staff see at a glance it is not a seated order. */
.header-takeaway {
  border-color: var(--accent-warning);
  color: var(--accent-warning);
}

.header-clock {
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: 500;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.header-nav {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Workspaces: one segmented control, the current one in Brume. */
.segmented {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.segmented .nav-btn {
  height: 48px;
  padding: 0 22px;
  border-radius: var(--radius-sm);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 10px;
  transition: transform var(--tap-duration) ease;
}

.admin-nav {
  display: flex;
  gap: 8px;
  margin-left: 12px;
}
.nav-admin {
  height: 48px;
  padding: 0 20px;
  border-radius: 12px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}

.nav-btn:active { transform: scale(var(--tap-scale)); }
.segmented .nav-btn.nav-btn-active,
.nav-admin.nav-btn-active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}

.nav-logout {
  height: 48px;
  padding: 0 20px;
  margin-left: 20px;
  border-radius: 12px;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
</style>
