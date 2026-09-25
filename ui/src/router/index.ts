import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import LoginView from "../views/LoginView.vue";
import OrderView from "../views/OrderView.vue";
import CheckoutView from "../views/CheckoutView.vue";
import TablesView from "../views/TablesView.vue";
import ReportsView from "../views/ReportsView.vue";
import SettingsView from "../views/SettingsView.vue";
import { HUBRISE_ENABLED } from "../features";
import { useOrderStore } from "../stores/order";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import { describeError } from "../utils/fdmMessages";

const routes: RouteRecordRaw[] = [
  { path: "/", name: "login", component: LoginView },
  { path: "/order", name: "order", component: OrderView },
  { path: "/checkout", name: "checkout", component: CheckoutView },
  { path: "/tables", name: "tables", component: TablesView },
  { path: "/reports", name: "reports", component: ReportsView },
  { path: "/settings", name: "settings", component: SettingsView },
];

if (HUBRISE_ENABLED) {
  // Lazy import keeps WebshopOrdersView out of the main chunk and
  // (with the static `if (HUBRISE_ENABLED)` constant) lets Vite
  // eliminate the file from the cert build entirely.
  routes.push({
    path: "/webshop-orders",
    name: "webshop-orders",
    component: () => import("../views/WebshopOrdersView.vue"),
  });
}

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

// Screens where the current order is being built or paid. Leaving them
// with the order unpaid interrupts the sale, so what was ordered is
// registered with the FDM first (event P, POS spec §1.3).
const ORDER_ROUTES = new Set(["order", "checkout"]);

router.beforeEach(async (to, from) => {
  if (!ORDER_ROUTES.has(String(from.name)) || ORDER_ROUTES.has(String(to.name))) return true;
  try {
    await useOrderStore().registerPending();
    return true;
  } catch (e) {
    const { t } = useI18n();
    useToastStore().show(`${t("order_register_failed")} : ${describeError(e)}`, "error", 6000);
    // The operator stays on the order and retries once the FDM is back.
    // Locking the screen (idle timer, logout) always goes through: the
    // entries stay pending and are sent at the next registration.
    return to.name === "login";
  }
});

export default router;
