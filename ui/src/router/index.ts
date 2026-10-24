import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import LoginView from "../views/LoginView.vue";
import OrderView from "../views/OrderView.vue";
import CheckoutView from "../views/CheckoutView.vue";
import TablesView from "../views/TablesView.vue";
import ReportsView from "../views/ReportsView.vue";
import SettingsView from "../views/SettingsView.vue";
import { useOrderStore } from "../stores/order";
import { useAuthStore } from "../stores/auth";
import type { LeaveReason } from "../api/ipc";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";
import { useIntegrationsStore } from "../stores/integrations";

const routes: RouteRecordRaw[] = [
  { path: "/", name: "login", component: LoginView },
  { path: "/order", name: "order", component: OrderView },
  { path: "/checkout", name: "checkout", component: CheckoutView },
  { path: "/tables", name: "tables", component: TablesView },
  { path: "/reports", name: "reports", component: ReportsView },
  { path: "/settings", name: "settings", component: SettingsView },
];

// Online orders, while HubRise runs or orders taken before a
// switch-off are still to settle. Lazy: out of the main chunk.
routes.push({
  path: "/webshop-orders",
  name: "webshop-orders",
  component: () => import("../views/WebshopOrdersView.vue"),
  beforeEnter: () => useIntegrationsStore().hubriseVisible || { name: "tables" },
});

// The menu editor (managers). Lazy: out of the main chunk.
routes.push({
  path: "/menu-editor",
  name: "menu-editor",
  component: () => import("../views/MenuEditorView.vue"),
  beforeEnter: () => useAuthStore().isManager || { name: "tables" },
});

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

// Screens where the current order is being built or paid. Leaving them
// with the order unpaid interrupts the sale, so what was ordered is
// registered with the FDM first (event P, POS spec §1.3).
const ORDER_ROUTES = new Set(["order", "checkout"]);

const leaveOrder = (reason: LeaveReason) => useOrderStore().leave(reason);

router.beforeEach(async (to, from) => {
  if (!ORDER_ROUTES.has(String(from.name)) || ORDER_ROUTES.has(String(to.name))) return true;
  // No operator or no order: a lock already registered it in the engine.
  if (!useAuthStore().currentUser || !useOrderStore().currentOrder) return true;
  try {
    await leaveOrder("navigate");
    return true;
  } catch (e) {
    const { t } = useI18n();
    useToastStore().show(`${t("order_register_failed")} : ${describeError(e)}`, "error", 6000);
    // The operator stays on the order and retries once the FDM is back.
    // Going to the login screen always goes through: the entries stay
    // pending and are sent at the next registration.
    return to.name === "login";
  }
});

export default router;
