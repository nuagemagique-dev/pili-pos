import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { Order, OrderLine, CartLine, MenuItem } from "../types";
import * as ipc from "../api/ipc";
import { useAuthStore } from "./auth";
import { useToastStore } from "./toast";
import { describeError } from "../utils/errorMessages";
import { useI18n } from "../i18n";
import type { SlipOutcome } from "../types";

/** The order is signed either way; a slip that did not print must be
 * told to the bar / kitchen by hand. */
export function warnIfSlipFailed(outcome: SlipOutcome | null | undefined) {
  if (outcome === "failed") {
    useToastStore().show(useI18n().t("kitchen_slip_failed"), "warning", 8000);
  }
}

/** Run a cart action; a refusal (order paid meanwhile, product no
 * longer available, session closed) is shown instead of being lost. */
async function reported(action: () => Promise<void>): Promise<boolean> {
  try {
    await action();
    return true;
  } catch (e) {
    useToastStore().show(describeError(e), "error", 5000);
    return false;
  }
}

export const useOrderStore = defineStore("order", () => {
  const currentOrder = ref<Order | null>(null);
  const lines = ref<CartLine[]>([]);
  const openOrders = ref<Order[]>([]);
  // Client-side lookup: menu_item_id → category_id
  const itemCategoryMap = ref<Map<number, number>>(new Map());

  const totalCents = computed(() =>
    lines.value.reduce((sum, line) => sum + line.unit_price_cents * line.quantity, 0)
  );

  /** Units ordered but not yet sent to the bar / kitchen (dine-in: they
   * go out with the next P; takeaway: with the sale). */
  const unsentCount = computed(() =>
    lines.value.reduce((sum, line) => sum + Math.max(0, line.unsent_quantity ?? 0), 0)
  );

  async function startOrder(tableNumber: number | null) {
    const auth = useAuthStore();
    if (!auth.currentUser) throw new Error("Not logged in");

    currentOrder.value = await ipc.createOrder({
      order_type: tableNumber ? "dine_in" : "takeaway",
      table_number: tableNumber,
      user_insz: auth.currentUser.insz,
    });
    // A busy table returns its open order: show its lines.
    lines.value = tableNumber ? await ipc.listCartLines(currentOrder.value.id) : [];
  }

  /**
   * Rehydrate an existing order + its lines from SQLite. Used when
   * the operator re-opens a table (or any previously-started order)
   * after having switched away - without this, `currentOrder` would
   * point at the right row but `lines` would stay empty and the
   * cart would appear blank.
   */
  async function loadOrder(orderId: number) {
    const [order, orderLines] = await Promise.all([
      ipc.getOrder(orderId),
      ipc.listCartLines(orderId),
    ]);
    currentOrder.value = order;
    lines.value = orderLines;
  }

  /** `unitPriceCents` is the operator-entered price of a free-price
   * product; fixed-price products take the catalogue price. */
  function addItem(item: MenuItem, quantity = 1, unitPriceCents?: number) {
    return reported(() => addItemUnreported(item, quantity, unitPriceCents));
  }

  async function addItemUnreported(item: MenuItem, quantity: number, unitPriceCents?: number) {
    if (!currentOrder.value) throw new Error("No active order");

    // If item already in cart, increment quantity instead of adding a
    // duplicate line - except free-price products, whose lines can
    // carry different prices.
    // A line with options is never merged into: its options differ.
    const existing = item.open_price
      ? undefined
      : lines.value.find((l) => l.menu_item_id === item.id && !l.modifiers_json);
    if (existing) {
      await updateQuantityUnreported(existing.id, existing.quantity + quantity);
      return;
    }

    const line = await ipc.addOrderLine({
      order_id: currentOrder.value.id,
      menu_item_id: item.id,
      quantity,
      modifiers_json: null,
      ...(item.open_price ? { unit_price_cents: unitPriceCents } : {}),
    });
    await lineAdded(item, line, quantity);
  }

  /** A line another command added (a product with options): show it. */
  function addLineWith(item: MenuItem, quantity: number, create: (orderId: number) => Promise<OrderLine>) {
    return reported(async () => {
      if (!currentOrder.value) throw new Error("No active order");
      await lineAdded(item, await create(currentOrder.value.id), quantity);
    });
  }

  async function lineAdded(item: MenuItem, line: OrderLine, quantity: number) {
    if (!currentOrder.value) return;
    lines.value.push({
      ...line,
      name_fr: item.name_fr ?? null,
      name_zh: item.name_zh ?? null,
      unsent_quantity: quantity,
    });
    itemCategoryMap.value.set(item.id, item.category_id);

    currentOrder.value = await ipc.getOrder(currentOrder.value.id);
  }

  function removeLine(lineId: number) {
    return reported(() => removeLineUnreported(lineId));
  }

  async function removeLineUnreported(lineId: number) {
    await ipc.removeOrderLine(lineId);
    lines.value = lines.value.filter((l) => l.id !== lineId);
    if (currentOrder.value) {
      currentOrder.value = await ipc.getOrder(currentOrder.value.id);
    }
  }

  function updateQuantity(lineId: number, quantity: number) {
    return reported(() => updateQuantityUnreported(lineId, quantity));
  }

  /** Quantities never go below 1 here (UX guide §4.3): removing a line
   * is its own action (`removeLine`). */
  async function updateQuantityUnreported(lineId: number, quantity: number) {
    if (quantity < 1) return;
    await ipc.updateOrderLineQuantity(lineId, quantity);
    const line = lines.value.find((l) => l.id === lineId);
    if (line) {
      line.unsent_quantity = (line.unsent_quantity ?? 0) + (quantity - line.quantity);
      line.quantity = quantity;
    }
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

  /**
   * Register with the FDM what was ordered since the last registration
   * (event P, interrupted sale - POS spec §1.3). Called whenever the
   * operator leaves an open table order unpaid. The backend is a no-op when
   * nothing changed; on failure the entries stay pending and are sent
   * at the next registration, pre-bill or sale.
   */
  async function registerPending() {
    const current = currentOrder.value;
    // Not skipped when the cart is empty: lines added then removed are
    // corrections the FDM must still receive.
    if (!current || current.status !== "open") return;
    // A takeaway is a direct sale: nothing is registered when it is
    // left, it stays a draft listed on the Tables screen until it is
    // paid or closed unpaid, and ends in an N (FAQ Q205).
    if (current.order_type === "takeaway") return;
    warnIfSlipFailed(await ipc.signTableOrder(current.id));
    markAllSent();
  }

  /** Everything ordered so far went out with a signed P (or the sale). */
  function markAllSent() {
    for (const line of lines.value) line.unsent_quantity = 0;
  }

  /** Provisional bill ("addition provisoire", §1.4), printed PRO FORMA. */
  async function preBill() {
    if (!currentOrder.value) throw new Error("No active order");
    // Entries not yet registered are sent as an order first.
    warnIfSlipFailed(await ipc.signPreBill(currentOrder.value.id));
    markAllSent();
  }

  /** Close the current order unpaid, confirmed by the manager's PIN.
   * A refusal is thrown (the PIN dialog shows it); on success the
   * order is signed and closed, and the cart is cleared. */
  async function closeUnpaid(pin: string) {
    const current = currentOrder.value;
    const insz = useAuthStore().currentUser?.insz;
    if (!current || !insz) throw new Error("No active order");
    const result = await ipc.closeUnpaid(current.id, insz, pin);
    // The error code as the backend sent it, for describeError.
    if (!result.success) throw result.error ?? "";
    warnIfSlipFailed(result.kitchen_slip);
    clearOrder();
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
    unsentCount,
    startOrder,
    loadOrder,
    addItem,
    addLineWith,
    removeLine,
    updateQuantity,
    loadOpenOrders,
    registerPending,
    preBill,
    closeUnpaid,
    clearOrder,
    getCategoryId,
  };
});
