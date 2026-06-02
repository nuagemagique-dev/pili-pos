import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";
import LoginView from "../views/LoginView.vue";
import OrderView from "../views/OrderView.vue";
import CheckoutView from "../views/CheckoutView.vue";
import TablesView from "../views/TablesView.vue";
import ReportsView from "../views/ReportsView.vue";
import SettingsView from "../views/SettingsView.vue";

// Build-time feature flag. Set `VITE_FEATURE_HUBRISE=false` for the
// certification build so the HubRise (webshop) route and its view
// chunk are tree-shaken out of the bundle. Any other value (including
// unset) keeps HubRise available — dev default.
const HUBRISE_ENABLED = import.meta.env.VITE_FEATURE_HUBRISE !== "false";

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
  // — combined with the static `if (HUBRISE_ENABLED)` constant —
  // lets Vite eliminate the file from the cert build entirely.
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

export default router;
