import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  Order,
  OrderLine,
  CartLine,
  MenuItem,
  SlipOutcome,
  DiscountInput,
  DiscountResult,
} from '../types'
import * as ipc from '../api/ipc'
import { useAuthStore } from './auth'
import { useToastStore } from './toast'
import { useTablePlanStore } from './tablePlan'
import { describeError } from '../utils/errorMessages'
import { useI18n } from '../i18n'
import { netTotalCents, hasOrderDiscount } from '../utils/cartAmounts'
import type { HeldPart, HeldParts } from '../utils/heldParts'
import { hintAfter, triesWithoutPin, type PinGrantHint } from '../utils/pinGrant'

/** The order is signed either way; a slip that did not print must be
 * told to the bar / kitchen by hand. */
export function warnIfSlipFailed(outcome: SlipOutcome | null | undefined) {
  if (outcome === 'failed') {
    useToastStore().show(useI18n().t('kitchen_slip_failed'), 'warning', 8000)
  }
}

/** Run a cart action; a refusal (order paid meanwhile, product no
 * longer available, session closed) is shown instead of being lost. */
async function reported(action: () => Promise<void>): Promise<boolean> {
  try {
    await action()
    return true
  } catch (e) {
    useToastStore().show(describeError(e), 'error', 5000)
    return false
  }
}

/** How long the "Annuler" of a table move stays on screen (UX audit C8). */
export const MOVE_UNDO_TTL_MS = 5000

/** An add to the cart: the line it went to and the units it gave. `seq` grows with each one. */
export interface LineAdd {
  seq: number
  lineId: number
  itemId: number
  units: number
}

export const useOrderStore = defineStore('order', () => {
  const currentOrder = ref<Order | null>(null)
  const lines = ref<CartLine[]>([])
  // The last add (grid, code, digit window, dialogs): the beep and the line flash follow it.
  const lastAdd = ref<LineAdd | null>(null)
  let addSeq = 0
  function noteAdd(lineId: number, itemId: number, units: number): LineAdd {
    const add = { seq: ++addSeq, lineId, itemId, units }
    lastAdd.value = add
    return add
  }
  const openOrders = ref<Order[]>([])
  // The number of the day and the customer of the current takeaway (UX audit T2); null for a table.
  const takeawayTag = ref<ipc.TakeawayTag | null>(null)
  // The covers (guests) of the current table order (UX audit T5); null when none were set.
  const covers = ref<number | null>(null)
  // An unpaid online order being collected through the checkout (UX audit O2): it is not the active
  // order of the engine and leaving it registers nothing; the sale goes through the handover.
  const onlineCollect = ref<{ method: 'cash' | 'card' } | null>(null)
  // The payment parts taken by hand for the order, not signed yet (UX audit P4): they stay when the
  // operator goes back to the order from the checkout.
  const heldParts = ref<HeldParts | null>(null)
  // Client-side lookup: menu_item_id → category_id
  const itemCategoryMap = ref<Map<number, number>>(new Map())

  // Courses (kitchen printing only): the setting and the courses of the
  // current order that wait for "Envoyer la suite".
  const courseSettings = ref<ipc.CourseSettings>({ enabled: false, category_courses: [] })
  const heldCourses = ref<ipc.HeldCourse[]>([])
  const coursesEnabled = computed(() => courseSettings.value.enabled)
  /** The next course waiting at the current order, 1 to 3. */
  const nextHeldCourse = computed(() => heldCourses.value[0]?.course ?? null)

  /** The amount due: the lines' net amounts, till discounts taken off. */
  const totalCents = computed(() => netTotalCents(lines.value))

  /** An order discount (not a free line) applies to the order. */
  const orderDiscounted = computed(() => hasOrderDiscount(lines.value))

  /** Units ordered but not yet sent to the bar / kitchen (dine-in: they
   * go out with the next P; takeaway: with the sale). */
  const unsentCount = computed(() =>
    lines.value.reduce((sum, line) => sum + Math.max(0, line.unsent_quantity ?? 0), 0),
  )

  /** A dine-in order that holds lines may move to another table. */
  const canTransfer = computed(
    () =>
      currentOrder.value?.order_type === 'dine_in' &&
      currentOrder.value.status === 'open' &&
      currentOrder.value.table_number !== null &&
      lines.value.length > 0,
  )
  // Counts the table moves signed (a move and its undo): the Tables screen reads the plan again on each.
  const tableMoves = ref(0)

  async function startOrder(tableNumber: number | null) {
    const auth = useAuthStore()
    if (!auth.currentUser) throw new Error('Not logged in')

    currentOrder.value = await ipc.createOrder({
      order_type: tableNumber !== null && tableNumber !== 0 ? 'dine_in' : 'takeaway',
      table_number: tableNumber,
      user_insz: auth.currentUser.insz,
    })
    // A busy table returns its open order: show its lines.
    lines.value =
      tableNumber !== null && tableNumber !== 0
        ? await ipc.listCartLines(currentOrder.value.id)
        : []
    await ipc.setActiveOrder(currentOrder.value.id)
    await loadCourses()
    await loadTakeawayTag()
    await loadCovers()
  }

  /** Read the tag of the current order: a takeaway has its number of the day, a table has none.
   * A till that does not answer simply shows no number. */
  async function loadTakeawayTag() {
    const current = currentOrder.value
    if (!current || current.order_type !== 'takeaway') {
      takeawayTag.value = null
      return
    }
    try {
      const tag = await ipc.getTakeawayTag(current.id)
      if (currentOrder.value?.id === current.id) takeawayTag.value = tag
    } catch {
      takeawayTag.value = null
    }
  }

  /** Read the covers of the current order: only a table order has any. A till that does not
   * answer shows none. */
  async function loadCovers() {
    const current = currentOrder.value
    if (!current || current.order_type !== 'dine_in') {
      covers.value = null
      return
    }
    try {
      const stored = await ipc.getOrderCovers(current.id)
      if (currentOrder.value?.id === current.id) covers.value = stored
    } catch {
      covers.value = null
    }
  }

  /** Set (1 to 30) or clear (null) the covers of the current table order. A refusal is shown as
   * a toast; answers whether it was stored. */
  function setCovers(count: number | null) {
    return reported(async () => {
      const current = currentOrder.value
      if (!current) throw new Error('No active order')
      covers.value = await ipc.setOrderCovers(current.id, count)
    })
  }

  /** Put (or, with empty values, clear) the name and the phone of the customer of the current
   * takeaway. A refusal is shown as a toast; answers whether it was stored. */
  function setTakeawayCustomer(name: string | null, phone: string | null) {
    return reported(async () => {
      const current = currentOrder.value
      if (!current) throw new Error('No active order')
      takeawayTag.value = await ipc.setTakeawayCustomer(current.id, name, phone)
    })
  }

  /**
   * Show an unpaid online order at the checkout (UX audit O2): its lines and total are read, but it
   * does not become the engine's active order, so a lock or leaving the screen registers nothing.
   * `method` is the one the checkout starts on.
   */
  async function beginOnlineCollect(orderId: number, method: 'cash' | 'card') {
    const [order, orderLines] = await Promise.all([
      ipc.getOrder(orderId),
      ipc.listCartLines(orderId),
    ])
    resetLocal()
    currentOrder.value = order
    lines.value = orderLines
    onlineCollect.value = { method }
  }

  /** The parts kept for order `orderId` (none for another order). */
  function heldPartsFor(orderId: number): HeldPart[] {
    return heldParts.value?.orderId === orderId ? heldParts.value.parts : []
  }

  /** Keep `parts` (taken by hand, unsigned) for order `orderId`; none forgets them. */
  function holdParts(orderId: number, parts: HeldPart[]) {
    heldParts.value = parts.length > 0 ? { orderId, parts: parts.map((p) => ({ ...p })) } : null
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
    ])
    currentOrder.value = order
    lines.value = orderLines
    await ipc.setActiveOrder(orderId)
    await loadCourses()
    await loadTakeawayTag()
    await loadCovers()
  }

  /** Read the courses setting, then the held courses of the order. A
   * till that does not answer simply has courses off. */
  async function loadCourses() {
    try {
      courseSettings.value = await ipc.getCourseSettings()
    } catch {
      courseSettings.value = { enabled: false, category_courses: [] }
    }
    await refreshCourses()
  }

  /** With courses on, the engine owns each line's course and state: read
   * the cart and the held courses again after any change. */
  async function refreshCourses() {
    const current = currentOrder.value
    if (!courseSettings.value.enabled || !current) {
      heldCourses.value = []
      return
    }
    try {
      lines.value = await ipc.listCartLines(current.id)
      heldCourses.value = await ipc.listHeldCourses(current.id)
    } catch {
      // The cart stays as it is; the next action reads it again.
    }
  }

  /** Change a line's course (refused once a slip carried it). */
  function setLineCourse(lineId: number, course: number) {
    return reported(async () => {
      await ipc.setLineCourse(lineId, course)
      await refreshCourses()
    })
  }

  /** "Envoyer la suite": sign what is unsent, release the next held
   * course and print it. A refusal (FDM down, nothing held) is thrown for
   * the caller's toast. */
  async function sendNextCourse(): Promise<ipc.NextCourse | null> {
    const current = currentOrder.value
    if (!current) return null
    try {
      const next = await ipc.sendNextCourse(current.id)
      warnIfSlipFailed(next.signed)
      warnIfSlipFailed(next.slip)
      return next
    } finally {
      await refreshCourses()
      if (currentOrder.value) currentOrder.value = await ipc.getOrder(currentOrder.value.id)
    }
  }

  /** `unitPriceCents` is the operator-entered price of a free-price
   * product; fixed-price products take the catalogue price. */
  function addItem(item: MenuItem, quantity = 1, unitPriceCents?: number) {
    return addItemTracked(item, quantity, unitPriceCents).then((add) => add !== null)
  }

  /** Like [addItem], answering the add (its line and units), or null when it was refused. */
  async function addItemTracked(
    item: MenuItem,
    quantity = 1,
    unitPriceCents?: number,
  ): Promise<LineAdd | null> {
    let add: LineAdd | null = null
    const ok = await reported(async () => {
      add = await addItemUnreported(item, quantity, unitPriceCents)
    })
    return ok ? add : null
  }

  async function addItemUnreported(
    item: MenuItem,
    quantity: number,
    unitPriceCents?: number,
  ): Promise<LineAdd> {
    if (!currentOrder.value) throw new Error('No active order')

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
            (l.modifiers_json ?? '') === '' &&
            (l.price_changes?.length ?? 0) === 0 &&
            (l.kitchen_note ?? '') === '',
        )
    if (existing) {
      await updateQuantityUnreported(existing.id, existing.quantity + quantity)
      return noteAdd(existing.id, item.id, quantity)
    }

    const line = await ipc.addOrderLine({
      order_id: currentOrder.value.id,
      menu_item_id: item.id,
      quantity,
      modifiers_json: null,
      ...(item.open_price ? { unit_price_cents: unitPriceCents } : {}),
    })
    return lineAdded(item, line, quantity)
  }

  /**
   * Digit window: [delta] more units on the line an add just gave, as extra
   * units of the same line through the quantity command. The units already
   * sent to the kitchen never change: the quantity stays above them. A line
   * removed meanwhile does nothing. Answers false when refused.
   */
  async function addUnits(lineId: number, delta: number): Promise<boolean> {
    const line = lines.value.find((l) => l.id === lineId)
    if (!line) return false
    const sent = line.quantity - Math.max(0, line.unsent_quantity ?? 0)
    const before = line.quantity
    const quantity = Math.max(before + delta, sent + 1)
    const ok =
      quantity === before ? true : await reported(() => updateQuantityUnreported(lineId, quantity))
    if (ok) noteAdd(lineId, line.menu_item_id, quantity - before)
    return ok
  }

  /** A line another command added (a product with options): show it. */
  function addLineWith(
    item: MenuItem,
    quantity: number,
    create: (orderId: number) => Promise<OrderLine>,
  ) {
    return reported(async () => {
      if (!currentOrder.value) throw new Error('No active order')
      await lineAdded(item, await create(currentOrder.value.id), quantity)
    })
  }

  async function lineAdded(item: MenuItem, line: OrderLine, quantity: number): Promise<LineAdd> {
    if (!currentOrder.value) return noteAdd(line.id, item.id, quantity)
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
      kitchen_state: 'unsent',
    })
    itemCategoryMap.value.set(item.id, item.category_id)

    currentOrder.value = await ipc.getOrder(currentOrder.value.id)
    await refreshCourses()
    return noteAdd(line.id, item.id, quantity)
  }

  /** Sell a gift voucher: a fixed-quantity line of the system product. */
  function addGiftVoucher(amountCents: number) {
    return reported(async () => {
      const current = currentOrder.value
      if (!current) throw new Error('No active order')
      const line = await ipc.addGiftVoucherLine(current.id, amountCents)
      lines.value.push({
        ...line,
        name_fr: null,
        name_zh: null,
        unsent_quantity: 0,
        discount_cents: 0,
        price_changes: [],
        kitchen_note: null,
        course: 0,
        kitchen_state: 'printed',
      })
      currentOrder.value = await ipc.getOrder(current.id)
      await refreshCourses()
    })
  }

  function removeLine(lineId: number) {
    return reported(() => removeLineUnreported(lineId))
  }

  async function removeLineUnreported(lineId: number) {
    await ipc.removeOrderLine(lineId)
    lines.value = lines.value.filter((l) => l.id !== lineId)
    if (currentOrder.value) {
      currentOrder.value = await ipc.getOrder(currentOrder.value.id)
    }
    await refreshCourses()
  }

  /** Write (or clear, with an empty text) the kitchen note of a line. A
   * refusal is shown as a toast; a note slip that did not print is warned
   * like a failed order slip. */
  function setLineNote(lineId: number, note: string) {
    return reported(async () => {
      warnIfSlipFailed(await ipc.setLineNote(lineId, note))
      const line = lines.value.find((l) => l.id === lineId)
      if (line) line.kitchen_note = note.trim() === '' ? null : note.trim()
    })
  }

  /** The chips of the kitchen note dialog (UX audit C11), read each time the dialog opens. A till
   * that does not answer simply shows none: typing a note still works. */
  async function loadQuickNotes(): Promise<string[]> {
    try {
      return (await ipc.getQuickNotes()).notes
    } catch {
      return []
    }
  }

  function updateQuantity(lineId: number, quantity: number) {
    return reported(() => updateQuantityUnreported(lineId, quantity))
  }

  /** Quantities never go below 1 here (UX guide §4.3): removing a line
   * is its own action (`removeLine`). */
  async function updateQuantityUnreported(lineId: number, quantity: number) {
    if (quantity < 1) return
    await ipc.updateOrderLineQuantity(lineId, quantity)
    const line = lines.value.find((l) => l.id === lineId)
    if (line) {
      line.unsent_quantity = (line.unsent_quantity ?? 0) + (quantity - line.quantity)
      line.quantity = quantity
    }
    if (currentOrder.value) {
      currentOrder.value = await ipc.getOrder(currentOrder.value.id)
    }
    await refreshCourses()
  }

  async function loadOpenOrders() {
    openOrders.value = await ipc.listOpenOrders()
  }

  function getCategoryId(menuItemId: number): number | undefined {
    return itemCategoryMap.value.get(menuItemId)
  }

  /**
   * The operator leaves the order (another screen, another order, or
   * "send to kitchen"). The engine registers what was ordered (event P)
   * for an open dine-in order and does nothing for a takeaway or an order
   * that is not open; on failure the entries stay pending and are sent at
   * the next registration. A refusal is thrown for the caller's toast.
   */
  async function leave(reason: ipc.LeaveReason) {
    const current = currentOrder.value
    if (!current) return
    if (onlineCollect.value) {
      // An online order at the checkout was never the engine's active order: nothing to register.
      clearOrder()
      return
    }
    warnIfSlipFailed(await ipc.leaveOrder(current.id, reason))
    // Only a dine-in order sent anything: the badge shows the unsent units.
    if (current.order_type === 'dine_in') markAllSent()
    await refreshCourses()
  }

  /** Everything ordered so far went out with a signed P (or the sale). */
  function markAllSent() {
    for (const line of lines.value) line.unsent_quantity = 0
  }

  /** Provisional bill ("addition provisoire", §1.4), printed PRO FORMA. */
  async function preBill() {
    if (!currentOrder.value) throw new Error('No active order')
    await preBillOrder(currentOrder.value.id)
  }

  /** The provisional bill of any open table order, from the cart or from the Tables screen
   * (UX audit T7). Entries not yet registered are sent as an order first, so when the order
   * is the one on screen its lines are marked sent. */
  async function preBillOrder(orderId: number) {
    warnIfSlipFailed(await ipc.signPreBill(orderId))
    if (currentOrder.value?.id === orderId) {
      markAllSent()
      await refreshCourses()
    }
  }

  /** Move an open dine-in order to a free table (event P, cost center
   * change). A refusal is thrown for the caller's toast; both slips are
   * reported like "send to kitchen". */
  async function transferOrder(orderId: number, toTable: number) {
    const result = await ipc.transferTable(orderId, toTable)
    warnIfSlipFailed(result.kitchenSlip)
    warnIfSlipFailed(result.transferSlip)
    if (currentOrder.value?.id === orderId) {
      // What was unsent went out with the P that came first.
      markAllSent()
      currentOrder.value = await ipc.getOrder(orderId)
    }
    tableMoves.value++
    return result
  }

  /** UX audit C8: the move in one tap. The order goes to the free table `to` at
   * once (no confirmation); a toast "Table 4 déplacée en 9" stays 5 s with an
   * "Annuler" that moves it back, a second transfer signed like any other.
   * A refusal is thrown for the caller's toast. */
  async function moveTable(orderId: number, from: number, to: number) {
    const result = await transferOrder(orderId, to)
    const { t } = useI18n()
    const plan = useTablePlanStore()
    useToastStore().showWithAction(
      t('transfer_moved', { from: plan.label(from), to: plan.label(to) }),
      { label: t('transfer_undo'), run: () => undoMove(orderId, from, to) },
      MOVE_UNDO_TTL_MS,
    )
    return result
  }

  async function undoMove(orderId: number, from: number, to: number) {
    const { t } = useI18n()
    try {
      await transferOrder(orderId, from)
      const plan = useTablePlanStore()
      useToastStore().show(
        t('transfer_moved', { from: plan.label(to), to: plan.label(from) }),
        'success',
      )
    } catch (e) {
      useToastStore().show(describeError(e), 'error', 6000)
    }
  }

  /** Take the engine's view of the order again after a discount: the
   * lines (net amounts, price changes, what the P registered) and the total. */
  async function reloadFromEngine(orderId: number) {
    const [order, orderLines] = await Promise.all([
      ipc.getOrder(orderId),
      ipc.listCartLines(orderId),
    ])
    if (currentOrder.value?.id !== orderId) return
    currentOrder.value = order
    lines.value = orderLines
  }

  /** The engine's answer about the kept manager PIN (UX audit P8), see
   * `utils/pinGrant.ts`: a hint, the engine stays the judge. */
  const pinGrantHint = ref<PinGrantHint | null>(null)

  /** Try the next discount without the PIN: the engine said it keeps one. */
  function keepsManagerPin(): boolean {
    return triesWithoutPin(pinGrantHint.value, currentOrder.value?.id, Date.now())
  }

  /** The engine answered `pin_required`: the hint was stale. */
  function forgetManagerPin() {
    pinGrantHint.value = null
  }

  async function discountDone(orderId: number, result: DiscountResult) {
    pinGrantHint.value = hintAfter(orderId, result.pinGrantMs, Date.now())
    warnIfSlipFailed(result.kitchenSlip)
    if ((result.registrationError ?? '') !== '') {
      useToastStore().show(useI18n().t('discount_registration_failed'), 'warning', 8000)
    }
    await reloadFromEngine(orderId)
  }

  /** Give a discount on the order on screen (manager, confirmed by `pin`,
   * or by the kept PIN when `pin` is null: `pin_required` is thrown when the
   * engine kept none). A refusal is thrown (the PIN dialog shows it). The
   * engine's answer says whether the kept PIN was used and for how long it
   * still stands. */
  async function applyDiscount(input: DiscountInput, pin: string | null): Promise<DiscountResult> {
    const current = currentOrder.value
    const insz = useAuthStore().currentUser?.insz
    if (!current || insz === undefined || insz === '') throw new Error('No active order')
    const result = await ipc.applyDiscount(current.id, input, insz, pin)
    await discountDone(current.id, result)
    return result
  }

  /** Remove the order discount (`lineId` null) or the free line `lineId`
   * (`pin` null: the kept PIN, as for `applyDiscount`). */
  async function removeDiscount(
    lineId: number | null,
    pin: string | null,
  ): Promise<DiscountResult> {
    const current = currentOrder.value
    const insz = useAuthStore().currentUser?.insz
    if (!current || insz === undefined || insz === '') throw new Error('No active order')
    const result = await ipc.removeDiscount(current.id, lineId, insz, pin)
    await discountDone(current.id, result)
    return result
  }

  /** Close the current order unpaid, confirmed by the manager's PIN.
   * A refusal is thrown (the PIN dialog shows it); on success the
   * order is signed and closed, and the cart is cleared. */
  async function closeUnpaid(pin: string) {
    const current = currentOrder.value
    const insz = useAuthStore().currentUser?.insz
    if (!current || insz === undefined || insz === '') throw new Error('No active order')
    const result = await ipc.closeUnpaid(current.id, insz, pin)
    // The error code as the backend sent it, for describeError.
    if (!result.success) throw new Error(result.error ?? '')
    warnIfSlipFailed(result.kitchen_slip)
    clearOrder()
  }

  /** Forget the local order state only: after a lock the engine has
   * already registered and cleared its active order. */
  function resetLocal() {
    currentOrder.value = null
    takeawayTag.value = null
    covers.value = null
    onlineCollect.value = null
    heldParts.value = null
    lines.value = []
    lastAdd.value = null
    heldCourses.value = []
    itemCategoryMap.value.clear()
  }

  function clearOrder() {
    resetLocal()
    // Best effort: the engine drops it on the next set_active_order anyway.
    void ipc.setActiveOrder(null).catch(() => {})
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
    takeawayTag,
    covers,
    setCovers,
    onlineCollect,
    beginOnlineCollect,
    heldParts,
    heldPartsFor,
    holdParts,
    setTakeawayCustomer,
    lastAdd,
    loadQuickNotes,
    addItem,
    addItemTracked,
    addUnits,
    addLineWith,
    addGiftVoucher,
    removeLine,
    updateQuantity,
    setLineNote,
    loadOpenOrders,
    leave,
    preBill,
    preBillOrder,
    transferOrder,
    moveTable,
    canTransfer,
    tableMoves,
    applyDiscount,
    removeDiscount,
    keepsManagerPin,
    forgetManagerPin,
    closeUnpaid,
    clearOrder,
    resetLocal,
    getCategoryId,
  }
})
