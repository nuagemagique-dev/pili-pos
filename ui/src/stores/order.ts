import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { Order, OrderLine, CartLine, MenuItem } from "../types";
import * as ipc from "../api/ipc";
import { useAuthStore } from "./auth";
import { useToastStore } from "./toast";
import { describeError } from "../utils/errorMessages";
import { useI18n } from "../i18n";
import type { SlipOutcome, DiscountInput, DiscountResult } from "../types";
import { netTotalCents, hasOrderDiscount } from "../utils/cartAmounts";

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

  // Courses (kitchen printing only): the setting and the courses of the
  // current order that wait for "Envoyer la suite".
  const courseSettings = ref<ipc.CourseSettings>({ enabled: false, category_courses: [] });
  const heldCourses = ref<ipc.HeldCourse[]>([]);
  const coursesEnabled = computed(() => courseSettings.value.enabled);
  /** The next course waiting at the current order, 1 to 3. */
  const nextHeldCourse = computed(() => heldCourses.value[0]?.course ?? null);

  /** The amount due: the lines' net amounts, till discounts taken off. */
  const totalCents = computed(() => netTotalCents(lines.value));

  /** An order discount (not a free line) applies to the order. */
  const orderDiscounted = computed(() => hasOrderDiscount(lines.value));

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
    await ipc.setActiveOrder(currentOrder.value.id);
    await loadCourses();
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
    await ipc.setActiveOrder(orderId);
    await loadCourses();
  }

  /** Read the courses setting, then the held courses of the order. A
   * till that does not answer simply has courses off. */
  async function loadCourses() {
    try {
      courseSettings.value = await ipc.getCourseSettings();
    } catch {
      courseSettings.value = { enabled: false, category_courses: [] };
    }
    await refreshCourses();
  }

  /** With courses on, the engine owns each line's course and state: read
   * the cart and the held courses again after any change. */
  async function refreshCourses() {
    const current = currentOrder.value;
    if (!courseSettings.value.enabled || !current) {
      heldCourses.value = [];
      return;
    }
    try {
      lines.value = await ipc.listCartLines(current.id);
      heldCourses.value = await ipc.listHeldCourses(current.id);
    } catch {
      // The cart stays as it is; the next action reads it again.
    }
  }

  /** Change a line's course (refused once a slip carried it). */
  function setLineCourse(lineId: number, course: number) {
    return reported(async () => {
      await ipc.setLineCourse(lineId, course);
      await refreshCourses();
    });
  }

  /** "Envoyer la suite": sign what is unsent, release the next held
   * course and print it. A refusal (FDM down, nothing held) is thrown for
   * the caller's toast. */
  async function sendNextCourse(): Promise<ipc.NextCourse | null> {
    const current = currentOrder.value;
    if (!current) return null;
    try {
      const next = await ipc.sendNextCourse(current.id);
      warnIfSlipFailed(next.signed);
      warnIfSlipFailed(next.slip);
      return next;
    } finally {
      await refreshCourses();
      if (currentOrder.value) currentOrder.value = await ipc.getOrder(currentOrder.value.id);
    }
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
    // Nor is a line with a price change (a till discount): the engine
    // refuses a quantity change on it, so the product gets a new line.
    // Nor a line with a kitchen note: "1 with, 1 without" are two lines.
    const existing = item.open_price
      ? undefined
      : lines.value.find(
          (l) =>
            l.menu_item_id === item.id &&
            !l.modifiers_json &&
            (l.price_changes?.length ?? 0) === 0 &&
            !l.kitchen_note,
        );
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
      discount_cents: 0,
      price_changes: [],
      kitchen_note: null,
      // The engine gives the course; with courses on the cart is read again.
      course: 0,
      kitchen_state: "unsent",
    });
    itemCategoryMap.value.set(item.id, item.category_id);

    currentOrder.value = await ipc.getOrder(currentOrder.value.id);
    await refreshCourses();
  }

  /** Sell a gift voucher: a fixed-quantity line of the system product. */
  function addGiftVoucher(amountCents: number) {
    return reported(async () => {
      const current = currentOrder.value;
      if (!current) throw new Error("No active order");
      const line = await ipc.addGiftVoucherLine(current.id, amountCents);
      lines.value.push({
        ...line,
        name_fr: null,
        name_zh: null,
        unsent_quantity: 0,
        discount_cents: 0,
        price_changes: [],
        kitchen_note: null,
        course: 0,
        kitchen_state: "printed",
      });
      currentOrder.value = await ipc.getOrder(current.id);
      await refreshCourses();
    });
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
    await refreshCourses();
  }

  /** Write (or clear, with an empty text) the kitchen note of a line. A
   * refusal is shown as a toast; a note slip that did not print is warned
   * like a failed order slip. */
  function setLineNote(lineId: number, note: string) {
    return reported(async () => {
      warnIfSlipFailed(await ipc.setLineNote(lineId, note));
      const line = lines.value.find((l) => l.id === lineId);
      if (line) line.kitchen_note = note.trim() === "" ? null : note.trim();
    });
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
    await refreshCourses();
  }

  async function loadOpenOrders() {
    openOrders.value = await ipc.listOpenOrders();
  }

  function getCategoryId(menuItemId: number): number | undefined {
    return itemCategoryMap.value.get(menuItemId);
  }

  /**
   * The operator leaves the order (another screen, another order, or
   * "send to kitchen"). The engine registers what was ordered (event P)
   * for an open dine-in order and does nothing for a takeaway or an order
   * that is not open; on failure the entries stay pending and are sent at
   * the next registration. A refusal is thrown for the caller's toast.
   */
  async function leave(reason: ipc.LeaveReason) {
    const current = currentOrder.value;
    if (!current) return;
    warnIfSlipFailed(await ipc.leaveOrder(current.id, reason));
    // Only a dine-in order sent anything: the badge shows the unsent units.
    if (current.order_type === "dine_in") markAllSent();
    await refreshCourses();
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
    await refreshCourses();
  }

  /** Move an open dine-in order to a free table (event P, cost center
   * change). A refusal is thrown for the caller's toast; both slips are
   * reported like "send to kitchen". */
  async function transferOrder(orderId: number, toTable: number) {
    const result = await ipc.transferTable(orderId, toTable);
    warnIfSlipFailed(result.kitchenSlip);
    warnIfSlipFailed(result.transferSlip);
    if (currentOrder.value?.id === orderId) {
      // What was unsent went out with the P that came first.
      markAllSent();
      currentOrder.value = await ipc.getOrder(orderId);
    }
    return result;
  }

  /** Move the order on screen to a free table. */
  async function transferTable(toTable: number) {
    if (!currentOrder.value) throw new Error("No active order");
    return transferOrder(currentOrder.value.id, toTable);
  }

  /** Take the engine's view of the order again after a discount: the
   * lines (net amounts, price changes, what the P registered) and the total. */
  async function reloadFromEngine(orderId: number) {
    const [order, orderLines] = await Promise.all([ipc.getOrder(orderId), ipc.listCartLines(orderId)]);
    if (currentOrder.value?.id !== orderId) return;
    currentOrder.value = order;
    lines.value = orderLines;
  }

  async function discountDone(orderId: number, result: DiscountResult) {
    warnIfSlipFailed(result.kitchenSlip);
    if (result.registrationError) {
      useToastStore().show(useI18n().t("discount_registration_failed"), "warning", 8000);
    }
    await reloadFromEngine(orderId);
  }

  /** Give a discount on the order on screen (manager, confirmed by `pin`).
   * A refusal is thrown (the PIN dialog shows it). */
  async function applyDiscount(input: DiscountInput, pin: string) {
    const current = currentOrder.value;
    const insz = useAuthStore().currentUser?.insz;
    if (!current || !insz) throw new Error("No active order");
    await discountDone(current.id, await ipc.applyDiscount(current.id, input, insz, pin));
  }

  /** Remove the order discount (`lineId` null) or the free line `lineId`. */
  async function removeDiscount(lineId: number | null, pin: string) {
    const current = currentOrder.value;
    const insz = useAuthStore().currentUser?.insz;
    if (!current || !insz) throw new Error("No active order");
    await discountDone(current.id, await ipc.removeDiscount(current.id, lineId, insz, pin));
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

  /** Forget the local order state only: after a lock the engine has
   * already registered and cleared its active order. */
  function resetLocal() {
    currentOrder.value = null;
    lines.value = [];
    heldCourses.value = [];
    itemCategoryMap.value.clear();
  }

  function clearOrder() {
    resetLocal();
    // Best effort: the engine drops it on the next set_active_order anyway.
    void ipc.setActiveOrder(null).catch(() => {});
  }

  return {
    currentOrder,
    lines,
    openOrders,
    totalCents,
    orderDiscounted,
    unsentCount,
    courseSettings,
    heldCourses,
    coursesEnabled,
    nextHeldCourse,
    loadCourses,
    setLineCourse,
    sendNextCourse,
    startOrder,
    loadOrder,
    addItem,
    addLineWith,
    addGiftVoucher,
    removeLine,
    updateQuantity,
    setLineNote,
    loadOpenOrders,
    leave,
    preBill,
    transferOrder,
    transferTable,
    applyDiscount,
    removeDiscount,
    closeUnpaid,
    clearOrder,
    resetLocal,
    getCategoryId,
  };
});
