<script setup lang="ts">
import { describeError } from '../utils/errorMessages'
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useOrderStore, warnIfSlipFailed } from '../stores/order'
import { useDeliveryStore } from '../stores/delivery'
import { usePosStore } from '../stores/pos'
import { useEftStore } from '../stores/eft'
import { useToastStore } from '../stores/toast'
import type { CartLine, CheckoutResult, EftPayment, PaymentPart } from '../types'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import { formatPrice, round5 } from '../utils/format'
import { useBilingualName } from '../utils/names'
import { lineNetCents, discountTotalCents } from '../utils/cartAmounts'
import { DONE_AUTO_RETURN_MS, doneNeedsAttention } from '../utils/checkoutDone'
import { reconcileHeld, type HeldPart } from '../utils/heldParts'
import {
  allLeftPicked,
  cashDueCents,
  dishLines,
  paidUnits,
  pickedCents,
  pickedCount,
  pickedUnits,
  toggleLine,
  unitsFit,
  unitsLeft,
  withUnits,
  type DishPick,
  type DishUnits,
} from '../utils/dishSplit'
import AppHeader from '../components/AppHeader.vue'
import TicketMediumPicker from '../components/TicketMediumPicker.vue'
import DeliveryOutcomePanel from '../components/DeliveryOutcomePanel.vue'
import GiftVoucherDialog from '../components/GiftVoucherDialog.vue'
import NumberEntryDialog from '../components/NumberEntryDialog.vue'
import { useGiftVouchersStore, GIFT_VOUCHER_DEPARTMENT } from '../stores/giftVouchers'

const router = useRouter()
const order = useOrderStore()
const delivery = useDeliveryStore()
const pos = usePosStore()
const eft = useEftStore()
const giftVouchers = useGiftVouchersStore()
const toast = useToastStore()
const bilingual = useBilingualName()
const { t } = useI18n()

// A sale can be paid in several parts: card and meal vouchers for an
// exact amount each, cash for the rest (the only part rounded to
// 5 cents). Nothing is signed until the whole sale is paid; the
// backend settles the parts again and refuses a payment that does not
// add up.
//
// Cash: the signed request carries ONE cash line, rounded once (POS
// description v2.2 section 7.1: cash appears as "une seule ligne de
// paiement sur le ticket, ARRONDIE", the rounding of a ticket being
// 2 cents at most). Several friends can still pay in cash: each cash
// part keeps its own amount, given and change on this screen, and the
// payload merges them all in the single cash part without amount (the
// backend's "what is left"). The rounding of the merged line is shared
// between the cash parts (`cashDueCents`), so what each friend hands
// over adds up to the signed cash line.
//
// The equal split (UX guide §7.4) only helps with the amounts: staff
// pick the number of parts (1 to 4, or "Autre": 2 to 20), each part is
// pre-computed, and every part taken is one more entry of the same
// PaymentPart[] list. The split by dishes (UX audit P3) lets staff
// check the units of the cart a part pays (`utils/dishSplit.ts`); it
// is the same list of parts, the units never leave this screen.
type Method = 'cash' | 'card' | 'meal_voucher' | 'gift_voucher'
const ALL_METHODS: Method[] = ['cash', 'card', 'meal_voucher', 'gift_voucher']

interface TakenPart {
  method: Method
  cents: number
  /** Change handed back for a cash part, in cents. */
  changeCents: number
  /** Cash handed over for a cash part, in cents. */
  givenCents?: number
  /** The cart units a part pays when the sale is split by dishes (never sent to the engine). */
  units?: DishUnits[]
  /** A card part taken on the linked terminal: the approved payment it
   * settles. Such a part is locked (the card is charged): it cannot be
   * removed and its amount is fixed. */
  eftPaymentId?: number
  /** The card brand the terminal gave ("Bancontact"). */
  brand?: string | null
  /** The approved terminal card is a meal-voucher card: shown (and signed)
   * as meal vouchers, the part itself stays method card. */
  mealVoucher?: boolean
  /** A gift voucher part: the voucher it redeems, and its masked code. */
  giftVoucherId?: number
  codeMasked?: string
}

const step = ref<'pay' | 'processing' | 'done'>('pay')
const result = ref<CheckoutResult | null>(null)
const error = ref<string | null>(null)
// Change handed back, fixed when the sale is recorded: the done screen
// cannot recompute it, the cart (and so the total) is cleared by then.
const changeDueCents = ref(0)

// §7.1: cash selected by default, the exact amount already entered
// (PAYER → Encaisser).
// An unpaid online order collected from the board (UX audit O2) starts on the button that was
// tapped there; its sale goes through the handover command instead of a plain checkout.
const online = computed(() => order.onlineCollect !== null)
// The sale clears the order, and with it the online mode: remember it for the way out.
const wasOnline = order.onlineCollect !== null
const method = ref<Method>(order.onlineCollect?.method ?? 'cash')
const splitCount = ref(1)
// "Par plats" (UX audit P3): the units of the cart checked for the part being taken.
const splitMode = ref<'equal' | 'dishes'>('equal')
const dishMode = computed(() => splitMode.value === 'dishes')
const pick = ref<DishPick>({})
// "Autre": the pad for any number of equal parts.
const otherOpen = ref(false)
const parts = ref<TakenPart[]>([])
// The amount being typed ("12,50"): received for cash, charged for
// card / meal vouchers. `fresh`: set by a shortcut or a prefill, the
// next key replaces it instead of appending to it.
const entry = ref('')
const fresh = ref(false)
// Card parts: sent to the linked terminal, or typed by hand when it is down.
const cardManual = ref(false)

// The picker needs a fresh mail probe every time it's about to be
// shown - cheap, the backend caches the probe for 60s.
watch(
  step,
  (s) => {
    if (s === 'pay') delivery.refreshMailStatus()
  },
  { immediate: true },
)

// The amount due is the net total: till discounts are already taken off.
const totalCents = computed(() => order.totalCents)
const discountCents = computed(() => discountTotalCents(order.lines))
/** A sale that sells a gift voucher is paid in cash or by card only. */
const sellsVoucher = computed(() =>
  order.lines.some((l) => l.department_id === GIFT_VOUCHER_DEPARTMENT),
)
/** The gift voucher method needs the setting on; meal and gift vouchers
 * are hidden when the sale sells a voucher. */
const METHODS = computed<Method[]>(() =>
  ALL_METHODS.filter((m) =>
    sellsVoucher.value
      ? m === 'cash' || m === 'card'
      : m !== 'gift_voucher' || giftVouchers.enabled,
  ),
)
const paidCents = computed(() => parts.value.reduce((sum, p) => sum + p.cents, 0))
const restCents = computed(() => Math.max(0, totalCents.value - paidCents.value))
/** Cash of the cash parts already taken (exact amounts): the base of the shared 5-cent rounding. */
const cashTakenCents = computed(() =>
  parts.value.filter((p) => p.method === 'cash').reduce((sum, p) => sum + p.cents, 0),
)
/** A card part goes to the linked terminal (not typed by hand). */
const viaTerminal = computed(() => eft.active && !cardManual.value)
/** Every part is taken (card parts approved by the terminal): the sale
 * only waits to be signed, for instance after a failed signature or a
 * restart. */
const allPaid = computed(() => parts.value.length > 0 && restCents.value === 0)
/** A terminal payment waits for its answer: no sale until it is checked. */
const eftBlocked = computed(() => eft.open.length > 0)

/** `total` in `n` parts, the remainder cents on the first parts:
 * 6030 / 4 -> 1508 1508 1507 1507. */
function equalShares(total: number, n: number): number[] {
  const base = Math.floor(total / n)
  const extra = total - base * n
  return Array.from({ length: n }, (_, i) => base + (i < extra ? 1 : 0))
}

const shares = computed(() => equalShares(totalCents.value, splitCount.value))
const partIndex = computed(() => parts.value.length)

// ── Split by dishes ──
const dLines = computed(() => dishLines(order.lines))
const paid = computed(() => paidUnits(parts.value))
const pickedTotal = computed(() => pickedCount(pick.value))
const pickedAmount = computed(() => pickedCents(dLines.value, paid.value, pick.value))
/** Every unit not paid yet is checked: this part pays the whole rest. */
const everythingPicked = computed(
  () =>
    dishMode.value && pickedTotal.value > 0 && allLeftPicked(dLines.value, paid.value, pick.value),
)
/** The lines of the recap in dishes mode: what each one has left and what is checked. */
const dishRows = computed(() =>
  dLines.value.map((dl, i) => ({
    line: order.lines[i],
    left: unitsLeft(dl, paid.value),
    paidCount: paid.value.get(dl.id) ?? 0,
    picked: pick.value[dl.id] ?? 0,
  })),
)

const isLastPart = computed(() =>
  dishMode.value ? everythingPicked.value : partIndex.value >= splitCount.value - 1,
)
/** What the current part pays: its share (the units checked by dishes), the rest for the last one. */
const dueCents = computed(() => {
  if (dishMode.value) {
    return everythingPicked.value ? restCents.value : Math.min(pickedAmount.value, restCents.value)
  }
  return isLastPart.value
    ? restCents.value
    : Math.min(shares.value[partIndex.value], restCents.value)
})
/** A part by dishes needs a unit checked (or the sale only waits to be signed). */
const dishBlocked = computed(
  () =>
    dishMode.value &&
    !allPaid.value &&
    (pickedTotal.value === 0 || (dueCents.value === 0 && !everythingPicked.value)),
)
/** The cash to hand over for this part: its share of the rounding of the one cash line. */
const roundedDueCents = computed(() => cashDueCents(cashTakenCents.value, dueCents.value))

/** "Reste à payer": rounded when cash is about to pay all of it. */
const restShownCents = computed(() =>
  method.value === 'cash' && isLastPart.value
    ? cashDueCents(cashTakenCents.value, restCents.value)
    : restCents.value,
)

/** The most equal parts: the pad of "Autre" (2 to 20). */
const SPLIT_QUICK_MAX = 4
const SPLIT_OTHER_MIN = 2
const SPLIT_OTHER_MAX = 20

/** A split in more parts than cents would give a part of 0. */
function splitAllowed(n: number): boolean {
  return parts.value.length === 0 && (n === 1 || n <= totalCents.value)
}

/** "12,50" -> 1250 cents; 0 when empty or not a number. */
function toCents(input: string): number {
  if (!input) return 0
  const parsed = parseFloat(input.replace(',', '.'))
  return isNaN(parsed) ? 0 : Math.round(parsed * 100)
}

function toInput(cents: number): string {
  return (cents / 100).toFixed(2).replace('.', ',')
}

const entryCents = computed(() => toCents(entry.value))
const entryShown = computed(() => {
  if (!entry.value) return '0,00'
  return fresh.value ? formatPrice(entryCents.value) : entry.value
})

// Cash: what was received against the rounded part.
const canPayCash = computed(() => entryCents.value >= roundedDueCents.value)
const changeCents = computed(() => Math.max(0, entryCents.value - roundedDueCents.value))

// Card / meal vouchers: an amount up to the rest.
const exactTooHigh = computed(() => method.value !== 'cash' && entryCents.value > restCents.value)
const canConfirmExact = computed(() => entryCents.value > 0 && !exactTooHigh.value)

const canValidate = computed(
  () =>
    !eftBlocked.value &&
    !dishBlocked.value &&
    (allPaid.value || (method.value === 'cash' ? canPayCash.value : canConfirmExact.value)),
)

/** Likely amounts handed over (§7.3): the next 5 / 10 € above the
 * amount due, then bank notes above it. */
const quickAmounts = computed(() => {
  const due = roundedDueCents.value / 100
  const candidates = [Math.ceil(due / 5) * 5, Math.ceil(due / 10) * 10, 20, 50, 100, 200, 500]
  return [...new Set(candidates.filter((n) => n > due))].sort((a, b) => a - b).slice(0, 3)
})

function methodLabel(m: Method): string {
  return t(`method_${m}`)
}

/** What a part taken is called: a meal-voucher card is a meal voucher. */
function takenLabel(p: TakenPart): string {
  return methodLabel(p.mealVoucher ? 'meal_voucher' : p.method)
}

/** Every part: taken, the current one, and the ones to come. */
const partChips = computed(() => {
  if (dishMode.value) {
    // By dishes the parts to come are not known: the one being taken and what stays after it.
    const chips: { state: 'paid' | 'current' | 'next'; label: string }[] = parts.value.map((p) => ({
      state: 'paid',
      label: `${takenLabel(p)} · ${formatPrice(p.cents)} € ✓`,
    }))
    if (restCents.value > 0) {
      chips.push({
        state: 'current',
        label: `${methodLabel(method.value)} · ${formatPrice(dueCents.value)} €`,
      })
      if (restCents.value > dueCents.value) {
        chips.push({ state: 'next', label: `${formatPrice(restCents.value - dueCents.value)} €` })
      }
    }
    return chips
  }
  const count = Math.max(splitCount.value, parts.value.length + (restCents.value > 0 ? 1 : 0))
  const chips: { state: 'paid' | 'current' | 'next'; label: string }[] = []
  let left = restCents.value
  for (let i = 0; i < count; i++) {
    const taken = parts.value[i]
    if (taken) {
      chips.push({ state: 'paid', label: `${takenLabel(taken)} · ${formatPrice(taken.cents)} € ✓` })
      continue
    }
    const amount = i >= splitCount.value - 1 ? left : Math.min(shares.value[i], left)
    left -= amount
    chips.push(
      i === partIndex.value
        ? { state: 'current', label: `${methodLabel(method.value)} · ${formatPrice(amount)} €` }
        : { state: 'next', label: `${formatPrice(amount)} €` },
    )
  }
  return chips
})

const itemCount = computed(() => order.lines.reduce((sum, l) => sum + l.quantity, 0))
const orderTitle = computed(() => {
  const o = order.currentOrder
  const where = online.value
    ? t('online_orders')
    : o?.order_type === 'dine_in' && o.table_number
      ? `${t('table')} ${o.table_number}`
      : t('takeaway_badge')
  return `${t('order')} · ${where}`
})

/** Notes / modifiers of a line, when it has some (a JSON list of
 * strings, or a plain string). */
function lineNotes(line: CartLine): string | null {
  const raw = line.modifiers_json
  if (!raw) return null
  try {
    const parsed: unknown = JSON.parse(raw)
    if (Array.isArray(parsed)) return parsed.map(String).join(' · ') || null
    if (typeof parsed === 'string') return parsed || null
  } catch {
    /* not JSON: shown as is */
  }
  return raw
}

// ── Entry ──

/** A new part or method starts at the exact amount: the rounded cash
 * due for cash, the part for card / meal vouchers. The first key (or a
 * quick amount) replaces it. */
function prefill() {
  const cents = method.value === 'cash' ? roundedDueCents.value : dueCents.value
  entry.value = cents > 0 ? toInput(cents) : ''
  fresh.value = cents > 0
}
watch([method, partIndex, splitCount, totalCents, dueCents], prefill, { immediate: true })

/** Numpad: digits, one comma, 2 decimals, 6 chars. */
function handleKey(value: string) {
  if (fresh.value) {
    // A shortcut or a prefilled amount is replaced, not appended to.
    entry.value = ''
    fresh.value = false
    if (value === 'BS') return
  }
  if (value === 'BS') {
    entry.value = entry.value.slice(0, -1)
    return
  }
  if (value === ',') {
    // Only one comma allowed, and not as first char
    if (entry.value.includes(',') || !entry.value) return
    entry.value += ','
    return
  }
  // Max 2 digits after comma
  const commaIdx = entry.value.indexOf(',')
  if (commaIdx >= 0 && entry.value.length - commaIdx > 2) return
  // Max 6 chars total (e.g. "200,10")
  if (entry.value.length < 6) entry.value += value
}

function setExact() {
  entry.value = toInput(method.value === 'cash' ? roundedDueCents.value : dueCents.value)
  fresh.value = true
}

const exactActive = computed(
  () =>
    fresh.value &&
    entryCents.value === (method.value === 'cash' ? roundedDueCents.value : dueCents.value),
)

/** The amount the primary button collects: the rounded due for cash
 * (the received amount only decides the change), the typed amount for
 * card / vouchers. */
const collectCents = computed(() =>
  method.value === 'cash' ? roundedDueCents.value : entryCents.value,
)

/** "Encaisser 47,30 € en espèces", "Encaisser la part 2 · 15,77 €";
 * "Valider" when every part is taken and the sale only waits to be signed. */
const validateLabel = computed(() => {
  if (allPaid.value) return t('validate')
  const amount = formatPrice(collectCents.value)
  if (splitCount.value > 1 || dishMode.value) {
    return t('collect_part', { n: partIndex.value + 1, amount })
  }
  return t(`collect_${method.value}`, { amount })
})

/** The received amount, shown under the label only when it differs
 * from what is due (there is change to give back). */
const receivedHint = computed(
  () =>
    method.value === 'cash' && entryCents.value > 0 && entryCents.value !== roundedDueCents.value,
)

/** After a part left a rest: the row "Reste 12,30 € :" in front of the
 * methods still allowed. */
const showRest = computed(() => parts.value.length > 0 && restCents.value > 0)

function setQuickAmount(n: number) {
  entry.value = n.toString()
  fresh.value = true
}

function quickActive(n: number): boolean {
  return fresh.value && entryCents.value === n * 100
}

function selectMethod(m: Method) {
  if (m === 'gift_voucher') {
    redeeming.value = true
    return
  }
  method.value = m
}

// ── Gift voucher ──

const redeeming = ref(false)

/** The dialog found a usable voucher: it pays `cents` of the rest. */
async function takeVoucher(voucher: ipc.GiftVoucherView, cents: number) {
  redeeming.value = false
  if (parts.value.some((p) => p.giftVoucherId === voucher.id)) return
  const part: TakenPart = {
    method: 'gift_voucher',
    cents,
    changeCents: 0,
    giftVoucherId: voucher.id,
    codeMasked: voucher.code_masked,
    units: currentUnits(),
  }
  if (cents >= restCents.value) {
    await doCheckout([...parts.value, part])
    return
  }
  // The method selected stays (a voucher is not a method of its own).
  parts.value.push(part)
  pick.value = {}
}

// ── Split: equal parts, "Autre", by dishes ──

function selectSplit(n: number) {
  if (!splitAllowed(n)) return
  splitMode.value = 'equal'
  splitCount.value = n
  pick.value = {}
}

/** "Autre" is highlighted for a number of parts the four buttons do not give. */
const otherActive = computed(() => !dishMode.value && splitCount.value > SPLIT_QUICK_MAX)

/** The pad of "Autre" gave a number of equal parts. */
function takeOtherSplit(value: string) {
  otherOpen.value = false
  const n = Number(value)
  if (n >= SPLIT_OTHER_MIN && n <= SPLIT_OTHER_MAX) selectSplit(n)
}

function selectDishes() {
  if (parts.value.length > 0 || dishMode.value) return
  splitMode.value = 'dishes'
  splitCount.value = 1
  pick.value = {}
}

/** The units checked for the part being taken, when the sale is split by dishes. */
function currentUnits(): DishUnits[] | undefined {
  return dishMode.value ? pickedUnits(dLines.value, pick.value) : undefined
}

/** Tap on a dish: all its units left are checked, or none. */
function toggleDish(index: number) {
  const dl = dLines.value[index]
  if (dishMode.value && dl)
    pick.value = toggleLine(pick.value, { id: dl.id, left: unitsLeft(dl, paid.value) })
}

/** The - and + of a dish of several units: one unit more or less for this part. */
function stepDish(index: number, delta: number) {
  const dl = dLines.value[index]
  if (dishMode.value && dl) {
    pick.value = withUnits(
      pick.value,
      { id: dl.id, left: unitsLeft(dl, paid.value) },
      (pick.value[dl.id] ?? 0) + delta,
    )
  }
}

function clearParts() {
  // A card the terminal approved is charged: its part stays.
  parts.value = parts.value.filter((p) => p.eftPaymentId !== undefined)
  pick.value = {}
  error.value = null
  method.value = 'cash'
}

// ── Parts kept when going back to the order (UX audit P4) ──

/** The parts taken by hand: the ones "Annuler les parts" removes. */
const manualParts = computed(() => parts.value.filter((p) => p.eftPaymentId === undefined))

/** Keep the manual parts with the order: going back to it and returning shows them again. */
let restored = false
watch(
  parts,
  () => {
    const id = order.currentOrder?.id
    if (!restored || id === undefined) return
    order.holdParts(
      id,
      manualParts.value.map((p): HeldPart => ({
        method: p.method,
        cents: p.cents,
        changeCents: p.changeCents,
        givenCents: p.givenCents,
        giftVoucherId: p.giftVoucherId,
        codeMasked: p.codeMasked,
        units: p.units,
      })),
    )
  },
  { deep: true },
)

/** Bring the kept parts back, once the card payments the terminal approved are known. If the
 * order changed so that a part no longer fits in what is left, they are all dropped. */
function restoreParts() {
  const id = order.currentOrder?.id
  if (id !== undefined) {
    const locked = parts.value.reduce((sum, p) => sum + p.cents, 0)
    const kept = reconcileHeld(order.heldPartsFor(id), Math.max(0, totalCents.value - locked))
    // A part by dishes needs its units to still be in the cart.
    const unitsGone = !unitsFit(dLines.value, [...parts.value, ...kept.parts])
    if (kept.dropped || unitsGone) {
      toast.show(t('parts_dropped'), 'warning', 6000)
      kept.parts = []
    }
    if (kept.parts.some((p) => p.units !== undefined)) splitMode.value = 'dishes'
    parts.value = [...parts.value, ...kept.parts]
  }
  restored = true
  if (id !== undefined && parts.value.length === 0) order.holdParts(id, [])
}

// ── Payment ──

function payload(all: TakenPart[]): PaymentPart[] {
  const list: PaymentPart[] = all
    .filter((p) => p.method !== 'cash')
    .map((p) => {
      if (p.giftVoucherId !== undefined) {
        return { method: p.method, amount_cents: p.cents, gift_voucher_id: p.giftVoucherId }
      }
      return p.eftPaymentId === undefined
        ? { method: p.method, amount_cents: p.cents }
        : { method: p.method, amount_cents: p.cents, eft_payment_id: p.eftPaymentId }
    })
  // All the cash parts are ONE cash line: no amount, it pays what is left (and is rounded once).
  if (all.some((p) => p.method === 'cash')) list.push({ method: 'cash' })
  return list
}

/** Take the current part: the sale is signed once nothing is left to
 * pay, otherwise on to the next part. */
async function validate() {
  if (!canValidate.value) return
  if (allPaid.value) {
    await doCheckout(parts.value)
    return
  }
  const isCash = method.value === 'cash'
  const units = currentUnits()
  const part: TakenPart = isCash
    ? {
        method: 'cash',
        cents: dueCents.value,
        changeCents: changeCents.value,
        givenCents: entryCents.value,
        units,
      }
    : { method: method.value, cents: entryCents.value, changeCents: 0, units }
  if (method.value === 'card' && viaTerminal.value) {
    // The terminal charges the card; its approved payment becomes a
    // locked part. A refusal leaves the part editable.
    const approved = await takeOnTerminal(part.cents, units)
    if (!approved) return
    pick.value = {}
    if (restCents.value === 0) {
      await doCheckout(parts.value)
    }
    // Otherwise the method stays selected for the next part.
    return
  }
  if (part.cents >= restCents.value) {
    await doCheckout([...parts.value, part])
    return
  }
  parts.value.push(part)
  pick.value = {}
  // The method stays selected for the next part (UX audit P3); "Reste" offers the others.
}

// ── Linked card terminal ──

/** An approved payment becomes a locked card part, once. */
function adopt(p: EftPayment) {
  if (p.status !== 'approved' || p.used) return
  if (parts.value.some((x) => x.eftPaymentId === p.id)) return
  parts.value.push({
    method: 'card',
    cents: p.amount_cents,
    changeCents: 0,
    eftPaymentId: p.id,
    brand: p.brand,
    mealVoucher: p.meal_voucher,
  })
}

/** Charge `cents` on the terminal (the overlay waits for the card). */
async function takeOnTerminal(cents: number, units?: DishUnits[]): Promise<EftPayment | null> {
  if (!order.currentOrder) return null
  const approved = await eft.charge(order.currentOrder.id, cents)
  if (approved) {
    adopt(approved)
    // The part by dishes keeps the units it pays.
    const taken = parts.value.find((x) => x.eftPaymentId === approved.id)
    if (taken && units) taken.units = units
  }
  return approved
}

/** Vérifier: ask the terminal where a waiting payment stands. */
async function verifyOpen(p: EftPayment) {
  const answer = await eft.verify(p)
  if (answer) adopt(answer)
}

async function refreshEft() {
  if (eft.linked && order.currentOrder) await eft.load(order.currentOrder.id)
}

// An approved payment the engine knows (after a crash, a restart or a
// check) must be used by this sale: it appears as a locked part.
watch(
  () => eft.approved,
  (list) => list.forEach(adopt),
)

onMounted(async () => {
  if (!pos.settings) {
    try {
      await pos.loadSettings()
    } catch {
      /* the card stays manual */
    }
  }
  await refreshEft()
  eft.approved.forEach(adopt)
  restoreParts()
})
onUnmounted(() => eft.clear())

// Courses still held at the table: the sale does not print them. Warn
// before paying; "Envoyer maintenant" releases the next one.
const sendingNext = ref(false)
async function sendHeldNow() {
  if (sendingNext.value) return
  sendingNext.value = true
  error.value = null
  try {
    await order.sendNextCourse()
  } catch (e) {
    error.value = describeError(e)
  } finally {
    sendingNext.value = false
  }
}

async function doCheckout(all: TakenPart[]) {
  if (!order.currentOrder) return
  error.value = null
  step.value = 'processing'
  try {
    const req = delivery.request.medium === 'paper' ? undefined : delivery.request
    // An online order is sold by its handover (its P is closed by the sale N); the parts are the same.
    result.value = online.value
      ? await ipc.webshopHandoverOrder(order.currentOrder.id, payload(all), req)
      : await ipc.checkout(order.currentOrder.id, payload(all), req)
    if (result.value.success) {
      warnIfSlipFailed(result.value.kitchen_slip)
      void giftVouchers.load()
      // The change of the last cash part: the earlier ones were handed back when they were taken.
      changeDueCents.value = [...all].reverse().find((p) => p.method === 'cash')?.changeCents ?? 0
      // Clear the order from the store immediately so it can't be
      // re-submitted. The "done" screen reads from result.value.
      order.clearOrder()
      parts.value = []
      eft.clear()
      // Next customer starts from Papier.
      delivery.reset()
      step.value = 'done'
    } else {
      error.value = describeError(result.value.error)
      step.value = 'pay'
      void refreshEft()
    }
  } catch (e) {
    error.value = describeError(e)
    step.value = 'pay'
    void refreshEft()
  }
}

/** What became of the voucher slips (vouchers sold, balances left). */
const giftVoucherNotes = computed(() =>
  (result.value?.gift_voucher_prints ?? []).map((p, i) => {
    const ok = p.outcome === 'printed'
    const text = ok
      ? t(p.kind === 'balance' ? 'gv_print_balance_ok' : 'gv_print_voucher_ok')
      : p.outcome === 'no_printer'
        ? t('gv_print_no_printer')
        : t('gv_print_failed')
    return { key: `${p.voucher_id}-${p.kind}-${i}`, ok, text }
  }),
)

// The done screen returns to the tables by itself after two seconds,
// unless something must be read or decided (ticket not printed, digital
// ticket pending, FDM offline, gift voucher slip note).
const holdDone = computed(() => doneNeedsAttention(result.value))
let returnTimer: ReturnType<typeof setTimeout> | undefined
let leaving = false

function clearReturnTimer() {
  if (returnTimer !== undefined) clearTimeout(returnTimer)
  returnTimer = undefined
}

watch(step, (s) => {
  clearReturnTimer()
  if (s === 'done' && !holdDone.value) returnTimer = setTimeout(finish, DONE_AUTO_RETURN_MS)
})
onUnmounted(clearReturnTimer)

/** A tap anywhere leaves the done screen, unless it holds something to read. */
function onDoneTap() {
  if (!holdDone.value) finish()
}

function finish() {
  if (leaving) return
  leaving = true
  clearReturnTimer()
  result.value = null
  // The order is gone: straight to the tables, no detour through the
  // order screen. An online order goes back to the board it came from.
  void router.push({ name: wasOnline ? 'webshop-orders' : 'tables' })
}

/** "Retour à la commande": the order screen, or the board for an online order. The parts taken
 * by hand stay with the order. */
function goBack() {
  void router.push({ name: online.value ? 'webshop-orders' : 'order' })
}
</script>

<template>
  <div class="checkout">
    <AppHeader />

    <div v-if="step === 'pay'" class="pay-row">
      <section class="pay-main">
        <div
          v-if="order.nextHeldCourse !== null"
          class="held-warning"
          role="alert"
          data-testid="held-warning"
        >
          <span>{{
            t('checkout_held_warning', { course: t(`course_${order.nextHeldCourse}`) })
          }}</span>
          <button
            class="held-send"
            :disabled="sendingNext"
            data-testid="held-send-now"
            @click="sendHeldNow"
          >
            {{ t('checkout_send_now') }}
          </button>
        </div>
        <!-- §7.2: the three methods side by side -->
        <div class="methods-row">
          <span v-if="showRest" class="rest-methods-label" data-testid="rest-methods-label">{{
            t('rest_methods', { amount: formatPrice(restShownCents) })
          }}</span>
          <div class="methods">
            <button
              v-for="m in METHODS"
              :key="m"
              class="method-btn"
              :class="[
                m === 'cash' ? 'cash-btn' : m === 'card' ? 'card-btn' : 'meal-btn',
                { active: method === m },
              ]"
              :data-testid="`method-${m}`"
              @click="selectMethod(m)"
            >
              {{ methodLabel(m) }}
            </button>
          </div>
        </div>

        <!-- §7.4: equal split (1 to 4 or "Autre" 2 to 20) or by dishes (UX audit P3) -->
        <div class="split-row">
          <span class="row-label">{{ t('split_into') }}</span>
          <div class="segmented split-options">
            <button
              v-for="n in SPLIT_QUICK_MAX"
              :key="n"
              class="seg-btn split-btn"
              :class="{ active: !dishMode && splitCount === n }"
              :disabled="(dishMode || splitCount !== n) && !splitAllowed(n)"
              @click="selectSplit(n)"
            >
              {{ n }}
            </button>
            <button
              class="seg-btn split-other"
              :class="{ active: otherActive }"
              :disabled="!otherActive && parts.length > 0"
              data-testid="split-other"
              @click="otherOpen = true"
            >
              {{ otherActive ? t('split_other_n', { n: splitCount }) : t('split_other') }}
            </button>
          </div>
          <span class="split-hint">{{ t('equal_parts') }}</span>
          <div class="segmented split-dishes-seg">
            <button
              class="seg-btn split-dishes"
              :class="{ active: dishMode }"
              :disabled="!dishMode && parts.length > 0"
              data-testid="split-dishes"
              @click="selectDishes"
            >
              {{ t('split_dishes') }}
            </button>
          </div>
        </div>

        <div v-if="partChips.length > 1" class="part-chips">
          <div v-for="(chip, i) in partChips" :key="i" class="part-chip" :class="chip.state">
            <span class="chip-title">{{ t('part_n', { n: i + 1 }) }}</span>
            <span class="chip-label">{{ chip.label }}</span>
          </div>
        </div>

        <div class="entry-grid">
          <div class="entry-left">
            <div class="section-label">
              {{ method === 'cash' ? t('amount_received') : t('amount') }}
            </div>
            <button class="shortcut exact" :class="{ active: exactActive }" @click="setExact">
              <span>{{ t('exact') }}</span>
              <span class="exact-amounts">
                <span class="exact-main"
                  >{{ formatPrice(method === 'cash' ? roundedDueCents : dueCents) }} €</span
                >
                <span v-if="method === 'cash' && roundedDueCents !== dueCents" class="exact-sub">
                  {{ t('exact_rounded', { exact: formatPrice(dueCents) }) }}
                </span>
              </span>
            </button>

            <template v-if="method === 'cash'">
              <div class="quick-amounts">
                <button
                  v-for="n in quickAmounts"
                  :key="n"
                  class="shortcut quick"
                  :class="{ active: quickActive(n) }"
                  @click="setQuickAmount(n)"
                >
                  {{ n }} €
                </button>
              </div>
              <div class="change-box" :class="{ pending: !canPayCash }">
                <span class="change-label">{{ t('change') }}</span>
                <span class="change-value">{{
                  canPayCash ? `${formatPrice(changeCents)} €` : '-'
                }}</span>
              </div>
            </template>
            <template v-else>
              <p v-if="exactTooHigh" class="exact-too-high">{{ t('amount_over_rest') }}</p>
              <div
                v-if="method === 'card' && eft.active"
                class="segmented eft-mode"
                data-testid="eft-mode"
              >
                <button
                  class="seg-btn"
                  :class="{ active: !cardManual }"
                  data-testid="eft-mode-terminal"
                  @click="cardManual = false"
                >
                  {{ t('eft_mode_terminal') }}
                </button>
                <button
                  class="seg-btn"
                  :class="{ active: cardManual }"
                  data-testid="eft-mode-manual"
                  @click="cardManual = true"
                >
                  {{ t('eft_mode_manual') }}
                </button>
              </div>
              <p class="card-hint">
                {{
                  method === 'card' && eft.active && !cardManual
                    ? t('eft_hint_terminal')
                    : t('card_wait')
                }}
              </p>
            </template>
          </div>

          <div class="entry-right">
            <div class="entry-display">
              <span class="entry-label">{{ method === 'cash' ? t('received') : t('amount') }}</span>
              <span class="entry-value" :class="{ 'exact-value': method !== 'cash' }">{{
                entryShown
              }}</span>
            </div>
            <div class="numpad">
              <button
                v-for="d in ['7', '8', '9', '4', '5', '6', '1', '2', '3']"
                :key="d"
                class="nk"
                @click="handleKey(d)"
              >
                {{ d }}
              </button>
              <button class="nk nk-comma" @click="handleKey(',')">,</button>
              <button class="nk" @click="handleKey('0')">0</button>
              <button class="nk nk-back" @click="handleKey('BS')">&#9003;</button>
            </div>
          </div>
        </div>

        <!-- Note 05: ticket medium, only when email is configured -->
        <div v-if="delivery.mailStatus?.configured && !parts.length" class="ticket-row">
          <span class="row-label">{{ t('ticket_medium') }}</span>
          <TicketMediumPicker />
        </div>
      </section>

      <aside class="summary">
        <div class="summary-head">
          <h2>{{ orderTitle }}</h2>
          <span class="item-count">{{ t('items_count', { n: itemCount }) }}</span>
        </div>

        <p v-if="dishMode" class="dish-hint" data-testid="dish-hint">
          {{ pickedTotal > 0 ? t('dish_picked', { n: pickedTotal }) : t('dish_pick_hint') }}
        </p>
        <div class="summary-lines scroll-touch">
          <div
            v-for="(row, i) in dishMode ? dishRows : []"
            :key="`dish${row.line.id}`"
            class="sum-line dish-line"
            :class="{ 'dish-done': row.left === 0, 'dish-on': row.picked > 0 }"
            :data-testid="`dish-line-${row.line.id}`"
            @click="toggleDish(i)"
          >
            <button
              class="dish-check"
              role="checkbox"
              :aria-checked="row.picked > 0 && row.picked >= row.left"
              :disabled="row.left === 0"
              :data-testid="`dish-check-${row.line.id}`"
              @click.stop="toggleDish(i)"
            >
              <template v-if="row.picked > 0">{{ row.picked >= row.left ? '✓' : '–' }}</template>
            </button>
            <div class="line-name">
              <span class="name-primary" :class="{ code: bilingual(row.line).isCode }"
                >{{ row.line.quantity }}× {{ bilingual(row.line).primary }}</span
              >
              <span v-if="bilingual(row.line).secondary" class="name-secondary">{{
                bilingual(row.line).secondary
              }}</span>
              <span
                v-if="row.paidCount > 0"
                class="dish-paid"
                :data-testid="`dish-paid-${row.line.id}`"
                >{{
                  row.left === 0
                    ? t('dish_paid')
                    : t('dish_paid_of', { paid: row.paidCount, n: row.line.quantity })
                }}</span
              >
            </div>
            <span class="line-total">{{ formatPrice(lineNetCents(row.line)) }}</span>
            <div v-if="row.line.quantity > 1 && row.left > 0" class="dish-stepper" @click.stop>
              <button
                class="dish-step"
                :disabled="row.picked === 0"
                :aria-label="t('dish_less')"
                :data-testid="`dish-minus-${row.line.id}`"
                @click="stepDish(i, -1)"
              >
                −
              </button>
              <span class="dish-count" :data-testid="`dish-count-${row.line.id}`"
                >{{ row.picked }} / {{ row.left }}</span
              >
              <button
                class="dish-step"
                :disabled="row.picked >= row.left"
                :aria-label="t('dish_more')"
                :data-testid="`dish-plus-${row.line.id}`"
                @click="stepDish(i, 1)"
              >
                +
              </button>
            </div>
          </div>
          <div v-for="line in dishMode ? [] : order.lines" :key="line.id" class="sum-line">
            <span class="line-qty">{{ line.quantity }}×</span>
            <div class="line-name">
              <span
                class="name-primary"
                :class="{
                  zh: pos.language === 'zh' && !bilingual(line).isCode,
                  code: bilingual(line).isCode,
                }"
                >{{ bilingual(line).primary }}</span
              >
              <span v-if="bilingual(line).secondary" class="name-secondary">{{
                bilingual(line).secondary
              }}</span>
              <span v-if="lineNotes(line)" class="line-notes">{{ lineNotes(line) }}</span>
            </div>
            <span class="line-total">{{ formatPrice(lineNetCents(line)) }}</span>
            <div
              v-for="(c, i) in line.price_changes"
              :key="`${c.id}-${i}`"
              class="line-change"
              data-testid="summary-change"
            >
              <span>{{ c.name }}</span
              ><span>{{ formatPrice(c.amount_cents) }}</span>
            </div>
          </div>
        </div>

        <div class="summary-foot">
          <template v-if="discountCents !== 0">
            <div class="foot-row" data-testid="summary-subtotal">
              <span>{{ t('checkout_subtotal') }}</span
              ><span>{{ formatPrice(totalCents - discountCents) }} €</span>
            </div>
            <div class="foot-row discount-row" data-testid="summary-discount">
              <span>{{ t('checkout_discount') }}</span
              ><span>{{ formatPrice(discountCents) }} €</span>
            </div>
          </template>
          <div class="foot-row total-row">
            <span>{{ t('total') }}</span
            ><span>{{ formatPrice(totalCents) }} €</span>
          </div>
          <div
            v-for="(p, i) in parts"
            :key="p.eftPaymentId ?? `m${i}`"
            class="foot-row paid-part"
            :class="{ locked: p.eftPaymentId !== undefined }"
          >
            <span>
              {{ t('part_n', { n: i + 1 }) }} · {{ takenLabel(p) }}
              <template v-if="p.codeMasked"> · {{ p.codeMasked }}</template>
              <template v-if="p.method === 'cash' && p.changeCents > 0">
                ·
                <span class="cash-given" data-testid="part-cash-detail">{{
                  t('part_cash_detail', {
                    given: formatPrice(p.givenCents ?? 0),
                    change: formatPrice(p.changeCents),
                  })
                }}</span>
              </template>
              <template v-if="p.eftPaymentId !== undefined">
                <template v-if="p.brand"> · {{ p.brand }}</template>
                <span class="eft-badge" data-testid="eft-approved">{{ t('eft_approved') }}</span>
              </template>
            </span>
            <span>− {{ formatPrice(p.cents) }} €</span>
          </div>
          <div
            v-for="p in eft.open"
            :key="`open${p.id}`"
            class="foot-row eft-open"
            data-testid="eft-open"
          >
            <span
              >{{ methodLabel('card') }} · {{ formatPrice(p.amount_cents) }} € ·
              {{ t('eft_unknown_row') }}</span
            >
            <button class="eft-verify" data-testid="eft-verify" @click="verifyOpen(p)">
              {{ t('eft_check') }}
            </button>
          </div>
          <div class="rest-row">
            <span class="total-label">{{ t('rest_to_pay') }}</span>
            <span class="rest-amount"
              ><span class="total-value">{{ formatPrice(restShownCents) }}</span> €</span
            >
          </div>

          <p v-if="eftBlocked" class="error-msg" data-testid="eft-blocking">
            {{ t('eft_blocking') }}
          </p>
          <p v-if="error" class="error-msg shake">{{ error }}</p>

          <div class="foot-actions" :class="{ 'with-clear': manualParts.length > 0 }">
            <button
              v-if="manualParts.length > 0"
              class="back-btn parts-clear"
              data-testid="parts-clear"
              @click="clearParts"
            >
              {{ t('clear_payments') }}
            </button>
            <button class="back-btn" data-testid="back-to-order" @click="goBack">
              {{ t('back_to_order') }}
            </button>
            <button
              class="validate-btn"
              :class="method === 'cash' ? 'cash-confirm' : 'card-confirm'"
              :disabled="!canValidate"
              @click="validate"
            >
              <span class="validate-label">{{ validateLabel }}</span>
              <span v-if="receivedHint" class="validate-amount">
                {{ t('received_amount', { amount: formatPrice(entryCents) }) }}
              </span>
            </button>
          </div>
        </div>
      </aside>
    </div>

    <!-- Processing -->
    <div v-else-if="step === 'processing'" class="status-screen">
      <div class="checkout-center">
        <div class="spinner"></div>
        <p class="processing-text">{{ t('processing_fdm') }}</p>
      </div>
    </div>

    <!-- Done: leaves by itself after 2 s unless there is something to read -->
    <div v-else class="status-screen done-screen" data-testid="done-screen" @click="onDoneTap">
      <div class="checkout-center">
        <div class="success-icon">OK</div>
        <div v-if="changeDueCents > 0" class="receipt-change" data-testid="done-change">
          <span>{{ t('change') }}</span>
          <span class="receipt-change-value">{{ formatPrice(changeDueCents) }} €</span>
        </div>
        <div v-if="result?.ticket" class="receipt-mini">
          <div class="receipt-label">{{ result.ticket.ticket_type_label }}</div>
          <div class="receipt-total">
            {{ formatPrice(Math.round(result.ticket.total * 100)) }} €
          </div>
          <div v-if="result.ticket.fdm_ref" class="receipt-fdm">
            FDM: {{ result.ticket.fdm_ref.fdmId }}
          </div>
        </div>
        <div v-if="!result?.fdm_connected" class="fdm-warn">
          {{ t('fdm_no_sig') }}
        </div>
        <ul v-if="giftVoucherNotes.length" class="gv-notes" data-testid="gv-prints">
          <li v-for="note in giftVoucherNotes" :key="note.key" :class="note.ok ? 'ok' : 'bad'">
            {{ note.text }}
          </li>
        </ul>
        <DeliveryOutcomePanel
          :outcome="result?.delivery ?? null"
          :fiscal-event-id="result?.fiscal_event_id ?? null"
        />
        <div class="done-actions">
          <button class="act-btn primary" data-testid="done-finish" @click.stop="finish">
            {{ t('finish') }}
          </button>
        </div>
        <p v-if="!holdDone" class="done-hint" data-testid="done-auto-return">
          {{ t('done_auto_return') }}
        </p>
      </div>
    </div>

    <NumberEntryDialog
      v-if="otherOpen"
      :title="t('split_other_title')"
      initial=""
      :max-digits="2"
      :min="SPLIT_OTHER_MIN"
      :max="SPLIT_OTHER_MAX"
      @confirm="takeOtherSplit"
      @cancel="otherOpen = false"
    />

    <GiftVoucherDialog
      v-if="redeeming"
      :rest-cents="restCents"
      @confirm="takeVoucher"
      @cancel="redeeming = false"
    />
  </div>
</template>

<style scoped>
.checkout {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-primary);
}

button {
  transition: transform var(--tap-duration) ease;
}
button:active:not(:disabled) {
  transform: scale(var(--tap-scale));
}

.pay-row {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

/* ── Left: methods, split, amount entry ── */
.pay-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 20px;
  padding: 24px 32px;
  overflow-y: auto;
}

.methods-row {
  display: flex;
  align-items: center;
  gap: 16px;
}
.rest-methods-label {
  flex: none;
  font-size: 22px;
  font-weight: 800;
  color: var(--accent-warning);
  white-space: nowrap;
}
.methods {
  flex: 1;
  min-width: 0;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 12px;
}
.gv-notes {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 17px;
  font-weight: var(--font-weight-bold);
}
.gv-notes .ok {
  color: var(--accent-success);
}
.gv-notes .bad {
  color: var(--accent-danger);
}
.method-btn {
  height: 88px;
  border-radius: 14px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 22px;
  font-weight: 800;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}
.method-btn.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.method-btn:disabled {
  opacity: 0.35;
}

.row-label {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-secondary);
  min-width: 170px;
}
.split-row {
  display: flex;
  align-items: center;
  gap: 16px;
}
.split-hint {
  font-size: 17px;
  color: var(--text-secondary);
}

.segmented {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.split-options {
  width: 480px;
}
.split-dishes-seg {
  min-width: 180px;
}
.split-dishes {
  padding: 0 16px;
  font-family: var(--font-family);
  font-size: 17px;
}
.split-other {
  font-family: var(--font-family);
  font-size: 16px;
  white-space: nowrap;
}
.seg-btn {
  height: 48px;
  border-radius: 8px;
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
  font-family: var(--font-mono);
}
.seg-btn.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.seg-btn:disabled {
  color: var(--text-secondary);
  opacity: 0.5;
}

.eft-mode {
  width: 420px;
}
.eft-badge {
  margin-left: 8px;
  padding: 2px 8px;
  border-radius: 6px;
  background: var(--accent-success);
  color: var(--text-on-accent);
  font-family: var(--font-family);
  font-size: 13px;
  font-weight: 800;
  text-transform: uppercase;
}
.eft-open {
  align-items: center;
  color: var(--accent-warning);
}
.eft-verify {
  height: 48px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 17px;
  font-weight: 800;
}

.part-chips {
  display: flex;
  gap: 8px;
}
.part-chip {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 14px;
  border-radius: 10px;
}
.part-chip.paid {
  background: var(--bg-surface);
}
.part-chip.current {
  box-shadow: inset 0 0 0 2px var(--selected-bg);
}
.part-chip.next {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
.chip-title {
  font-family: var(--font-mono);
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.chip-label {
  font-size: 16px;
  font-weight: 700;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.entry-grid {
  flex: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 32px;
  min-height: 0;
}
.entry-left,
.entry-right {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-label {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.shortcut {
  height: 88px;
  border-radius: 10px;
  background: var(--bg-surface);
  color: var(--text-primary);
}
.shortcut.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.shortcut.exact {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  font-size: 24px;
  font-weight: 700;
}
.exact-amounts {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  line-height: 1.15;
  font-family: var(--font-mono);
}
.exact-main {
  font-size: 28px;
}
.exact-sub {
  font-size: 14px;
  color: var(--text-secondary);
}
.shortcut.exact.active .exact-sub {
  color: var(--selected-fg);
}

.quick-amounts {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}
.shortcut.quick {
  font-family: var(--font-mono);
  font-size: 28px;
  font-weight: 700;
}

/* §5.3: change due, 72px */
.change-box {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 28px;
  border-radius: 16px;
  border: 2px solid var(--accent-success);
  color: var(--accent-success);
}
.change-box.pending {
  border-color: var(--border);
  color: var(--text-secondary);
}
.change-label {
  font-size: 28px;
  font-weight: 800;
}
.change-value {
  font-family: var(--font-mono);
  font-size: 72px;
  font-weight: 700;
  line-height: 1;
}

.exact-too-high {
  color: var(--accent-danger);
  font-size: 17px;
  font-weight: 700;
}
.card-hint {
  font-size: 17px;
  color: var(--text-secondary);
}

.entry-display {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 72px;
  padding: 0 20px;
  border-radius: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}
.entry-label {
  font-size: 16px;
  color: var(--text-secondary);
}
.entry-value {
  font-family: var(--font-mono);
  font-size: 40px;
  font-weight: 700;
}

.numpad {
  display: grid;
  grid-template-columns: repeat(3, 120px);
  gap: 8px;
}
.nk {
  height: 80px;
  border-radius: 10px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 30px;
  font-weight: 700;
}
.nk:active {
  background: var(--bg-pressed);
}
.nk-comma {
  color: var(--text-secondary);
}
.nk-back {
  color: var(--accent-danger);
}

.ticket-row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding-top: 14px;
  border-top: 1px solid var(--border);
}
/* TicketMediumPicker is shared with the reports screen: restyled here
 * as the design's segmented control without touching the component. */
.ticket-row :deep(.medium-picker) {
  flex: 1;
  min-width: 0;
}
.ticket-row :deep(.medium-options) {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 1fr;
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.ticket-row :deep(.medium-btn) {
  height: 48px;
  border-radius: 8px;
  font-size: 18px;
  font-weight: 700;
  background: transparent;
  color: var(--text-primary);
}
.ticket-row :deep(.medium-btn.active) {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.ticket-row :deep(.medium-note) {
  text-align: left;
  font-size: 13px;
}

/* ── Right: the order, read-only ── */
.summary {
  width: 670px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
  min-height: 0;
}
.summary-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px 10px;
  border-bottom: 1px solid var(--border);
}
.summary-head h2 {
  font-size: 28px;
  font-weight: 800;
  line-height: 1.2;
}
.item-count {
  font-family: var(--font-mono);
  font-size: 15px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.summary-lines {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0 16px;
}
.sum-line {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 16px;
  min-height: 56px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
/* Split by dishes (UX audit P3): a checkbox, the dish, its net, and a - n / left + stepper under it */
.dish-hint {
  padding: 8px 16px;
  font-size: 15px;
  font-weight: 700;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}
.dish-line {
  grid-template-columns: 48px minmax(0, 1fr) auto;
  min-height: 64px;
  gap: 12px;
  cursor: pointer;
}
.dish-line.dish-on {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.dish-line.dish-done {
  opacity: 0.4;
  cursor: default;
}
.dish-check {
  width: 48px;
  height: 48px;
  border-radius: 8px;
  border: 2px solid var(--border);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 26px;
  font-weight: 800;
  line-height: 1;
}
.dish-on .dish-check {
  border-color: var(--selected-fg);
}
.dish-paid {
  font-size: 13px;
  font-weight: 700;
  color: var(--accent-success);
}
.dish-stepper {
  grid-column: 2 / -1;
  display: flex;
  align-items: center;
  gap: 12px;
}
.dish-step {
  width: 48px;
  height: 48px;
  border-radius: 8px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 26px;
  font-weight: 800;
}
.dish-step:disabled {
  opacity: 0.35;
}
.dish-count {
  min-width: 72px;
  text-align: center;
  font-family: var(--font-mono);
  font-size: 18px;
  font-weight: 700;
}
.cash-given {
  font-size: 14px;
}
.line-qty {
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: 700;
  color: var(--text-secondary);
  min-width: 36px;
}
.line-name {
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.name-primary {
  font-size: 17px;
  font-weight: 700;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.name-primary.zh {
  font-family: var(--font-zh);
}
.name-primary.code {
  font-family: var(--font-mono);
}
.name-secondary {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.line-notes {
  font-size: 13px;
  color: var(--accent-warning);
}
.line-total {
  font-family: var(--font-mono);
  font-size: 17px;
  font-weight: 700;
}
.line-change {
  grid-column: 2 / -1;
  display: flex;
  justify-content: space-between;
  font-family: var(--font-mono);
  font-size: 14px;
  color: var(--accent-warning);
}
.discount-row {
  color: var(--accent-warning);
}

.summary-foot {
  border-top: 1px solid var(--border);
  padding: 14px 16px 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.foot-row {
  display: flex;
  justify-content: space-between;
  font-family: var(--font-mono);
  font-size: 17px;
  color: var(--text-secondary);
}
.rest-row {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
}
.total-label {
  font-size: 20px;
  font-weight: 700;
}
.rest-amount {
  font-family: var(--font-mono);
  font-size: 44px;
  font-weight: 700;
  line-height: 1.1;
}

.held-warning {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 10px;
  font-size: 16px;
  font-weight: 700;
  color: var(--accent-warning);
  background: color-mix(in srgb, var(--accent-warning) 12%, transparent);
  border: 1px solid var(--accent-warning);
}
.held-send {
  height: 56px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  background: var(--selected-bg);
  color: var(--selected-fg);
  font-size: 16px;
  font-weight: 700;
  flex-shrink: 0;
}
.held-send:disabled {
  opacity: 0.4;
}

.error-msg {
  color: var(--accent-danger);
  font-size: 16px;
  font-weight: 700;
  padding: 10px 14px;
  background: color-mix(in srgb, var(--accent-danger) 12%, transparent);
  border: 1px solid var(--accent-danger);
  border-radius: 10px;
}

.foot-actions {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 8px;
  margin-top: 4px;
}
.foot-actions.with-clear {
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
}
.foot-actions.with-clear .validate-btn {
  grid-column: 1 / -1;
}
.foot-actions.with-clear .back-btn {
  height: 56px;
}
.back-btn {
  height: 80px;
  padding: 0 24px;
  border-radius: 10px;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 17px;
  font-weight: 700;
}
.validate-btn {
  height: 80px;
  padding: 0 24px;
  border-radius: 10px;
  background: var(--accent-primary);
  color: var(--text-on-accent);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.validate-btn:disabled {
  opacity: 0.35;
}
.validate-label {
  font-size: 22px;
  font-weight: 800;
  text-transform: uppercase;
}
.validate-amount {
  font-family: var(--font-mono);
  font-size: 18px;
  white-space: nowrap;
}

/* ── Processing / done ── */
.done-hint {
  margin: 0;
  font-size: 17px;
  color: var(--text-secondary);
}
.status-screen {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
.checkout-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  max-width: 600px;
  width: 100%;
}
.spinner {
  width: 56px;
  height: 56px;
  border: 4px solid var(--bg-surface);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.processing-text {
  color: var(--text-secondary);
  font-size: 18px;
}

.success-icon {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  background: var(--accent-success);
  color: var(--text-on-accent);
  font-size: 28px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
}
.receipt-mini {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  width: 100%;
}
.receipt-label {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: var(--text-secondary);
}
.receipt-total {
  font-family: var(--font-mono);
  font-size: 44px;
  font-weight: 700;
}
.receipt-change {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 28px;
  border-radius: 16px;
  border: 2px solid var(--accent-success);
  color: var(--accent-success);
  font-size: 28px;
  font-weight: 800;
}
.receipt-change-value {
  font-family: var(--font-mono);
  font-size: 72px;
  font-weight: 700;
  line-height: 1;
}
.receipt-fdm {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-secondary);
}
.fdm-warn {
  padding: 10px 16px;
  border: 1px solid var(--accent-warning);
  color: var(--accent-warning);
  border-radius: 10px;
  font-size: 16px;
  font-weight: 700;
}
.done-actions {
  display: flex;
  width: 100%;
}
.act-btn {
  flex: 1;
  height: 80px;
  font-size: 22px;
  font-weight: 800;
  border-radius: 10px;
}
.act-btn.primary {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
</style>
