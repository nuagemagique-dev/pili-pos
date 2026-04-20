import { createRouter, createWebHashHistory } from "vue-router";
import LoginView from "../views/LoginView.vue";
import OrderView from "../views/OrderView.vue";
import CheckoutView from "../views/CheckoutView.vue";
import TablesView from "../views/TablesView.vue";
import ReportsView from "../views/ReportsView.vue";
import SettingsView from "../views/SettingsView.vue";
import WebshopOrdersView from "../views/WebshopOrdersView.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "login", component: LoginView },
    { path: "/order", name: "order", component: OrderView },
    { path: "/checkout", name: "checkout", component: CheckoutView },
    { path: "/tables", name: "tables", component: TablesView },
    { path: "/reports", name: "reports", component: ReportsView },
    { path: "/settings", name: "settings", component: SettingsView },
    { path: "/webshop-orders", name: "webshop-orders", component: WebshopOrdersView },
  ],
});

export default router;
