import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { Order, OrderLine, MenuItem } from "../types";
import * as ipc from "../api/ipc";
import { useAuthStore } from "./auth";

export const useOrderStore = defineStore("order", () => {
  const currentOrder = ref<Order | null>(null);
  const lines = ref<OrderLine[]>([]);
  const openOrders = ref<Order[]>([]);
  // Client-side lookup: menu_item_id → category_id
  const itemCategoryMap = ref<Map<number, number>>(new Map());

  const totalCents = computed(() =>
    lines.value.reduce((sum, line) => sum + line.unit_price_cents * line.quantity, 0)
  );

  async function startOrder(tableNumber: number | null) {
    const auth = useAuthStore();
    if (!auth.currentUser) throw new Error("Not logged in");

    currentOrder.value = await ipc.createOrder({
      order_type: tableNumber ? "dine_in" : "takeaway",
      table_number: tableNumber,
      user_insz: auth.currentUser.insz,
    });
    lines.value = [];
  }

  /**
   * Rehydrate an existing order + its lines from SQLite. Used when
   * the operator re-opens a table (or any previously-started order)
   * after having switched away — without this, `currentOrder` would
   * point at the right row but `lines` would stay empty and the
   * cart would appear blank.
   */
  async function loadOrder(orderId: number) {
    const [order, orderLines] = await Promise.all([
      ipc.getOrder(orderId),
      ipc.listOrderLines(orderId),
    ]);
    currentOrder.value = order;
    lines.value = orderLines;
  }

  async function addItem(item: MenuItem, quantity = 1) {
    if (!currentOrder.value) throw new Error("No active order");

    // If item already in cart, increment quantity instead of adding duplicate line
    const existing = lines.value.find((l) => l.menu_item_id === item.id);
    if (existing) {
      await updateQuantity(existing.id, existing.quantity + quantity);
      return;
    }

    const line = await ipc.addOrderLine({
      order_id: currentOrder.value.id,
      menu_item_id: item.id,
      quantity,
      modifiers_json: null,
    });
    lines.value.push(line);
    itemCategoryMap.value.set(item.id, item.category_id);

    currentOrder.value = await ipc.getOrder(currentOrder.value.id);
  }

  async function removeLine(lineId: number) {
    await ipc.removeOrderLine(lineId);
    lines.value = lines.value.filter((l) => l.id !== lineId);
    if (currentOrder.value) {
      currentOrder.value = await ipc.getOrder(currentOrder.value.id);
    }
  }

  async function updateQuantity(lineId: number, quantity: number) {
    if (quantity <= 0) {
      await removeLine(lineId);
      return;
    }
    await ipc.updateOrderLineQuantity(lineId, quantity);
    const line = lines.value.find((l) => l.id === lineId);
    if (line) line.quantity = quantity;
    if (currentOrder.value) {
      currentOrder.value = await ipc.getOrder(currentOrder.value.id);
    }
  }

  async function loadOpenOrders() {
    openOrders.value = await ipc.listOpenOrders();
  }

  function getCategoryId(menuItemId: number): number | undefined {
    return itemCategoryMap.value.get(menuItemId);
  }

  function clearOrder() {
    currentOrder.value = null;
    lines.value = [];
    itemCategoryMap.value.clear();
  }

  return {
    currentOrder,
    lines,
    openOrders,
    totalCents,
    startOrder,
    loadOrder,
    addItem,
    removeLine,
    updateQuantity,
    loadOpenOrders,
    clearOrder,
    getCategoryId,
  };
});
