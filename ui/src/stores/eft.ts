// The linked card payment terminal (EFT), as the screens use it.
//
// The engine rules (crates/pos-engine/src/commands/eft.rs): a payment
// is stored before the terminal is called; an approved unused payment
// of an order MUST be used by its sale (exact amount, its own card
// part); a payment waiting for its answer (`sent`, `unknown`) blocks the
// sale until `eft_check` gets the terminal's answer. This store holds
// the payments of the order on screen and drives the blocking overlay
// (EftOverlay.vue) of one terminal session at a time:
//
//   charge(order, cents)  ask the terminal, wait for the card
//   verify(payment)       ask the terminal where a payment stands
//
// Both resolve with the approved payment, or null (declined, cancelled,
// refused, or the operator closed the overlay: the payment, if it still
// waits for its answer, stays in `payments`).
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as ipc from '../api/ipc'
import type { EftPayment } from '../types'
import { describeError } from '../utils/errorMessages'
import { usePosStore } from './pos'
import { useToastStore } from './toast'

/** idle: no overlay; waiting: the terminal waits for the card; checking:
 * `eft_check` runs; result: the overlay shows the answer or a refusal. */
export type EftPhase = 'idle' | 'waiting' | 'checking' | 'result'

export const useEftStore = defineStore('eft', () => {
  const pos = usePosStore()

  const phase = ref<EftPhase>('idle')
  const amountCents = ref(0)
  /** The payment the overlay is about (its last answer). */
  const payment = ref<EftPayment | null>(null)
  /** Why nothing was charged, or why the check failed (readable text). */
  const failure = ref<string | null>(null)
  const abortRequested = ref(false)

  /** The order's approved unused payments and those awaiting an answer. */
  const payments = ref<EftPayment[]>([])
  let finish: ((p: EftPayment | null) => void) | null = null

  /** A terminal is linked (the installation's setting). */
  const linked = computed(() => Boolean(pos.settings?.paymentTerminal))
  /** Kind of the linked terminal, as in pos.json. */
  const kind = computed(() => pos.settings?.paymentTerminal ?? null)
  /** Card parts go to the terminal: linked, and not in training (the
   * engine refuses `eft_pay` there; card parts are typed by hand). */
  const active = computed(() => linked.value && !pos.trainingMode)

  const open = computed(() =>
    payments.value.filter((p) => p.status === 'sent' || p.status === 'unknown'),
  )
  const approved = computed(() => payments.value.filter((p) => p.status === 'approved' && !p.used))

  function record(p: EftPayment) {
    const others = payments.value.filter((x) => x.id !== p.id)
    const keep =
      p.status === 'sent' || p.status === 'unknown' || (p.status === 'approved' && !p.used)
    payments.value = (keep ? [...others, p] : others).sort((a, b) => a.id - b.id)
  }

  /** The order's payments, from the engine (recovery after a crash too). */
  async function load(orderId: number) {
    try {
      payments.value = await ipc.listEftPayments(orderId)
    } catch (e) {
      payments.value = []
      useToastStore().show(describeError(e), 'error')
    }
  }

  function clear() {
    payments.value = []
  }

  function close(result: EftPayment | null) {
    const done = finish
    finish = null
    phase.value = 'idle'
    payment.value = null
    failure.value = null
    abortRequested.value = false
    done?.(result)
  }

  /** The terminal answered `p`: approved closes the session, anything
   * else is shown. */
  function answered(p: EftPayment) {
    record(p)
    payment.value = p
    failure.value = null
    if (p.status === 'approved') {
      close(p)
      return
    }
    phase.value = 'result'
  }

  async function runCheck() {
    const current = payment.value
    if (!current) return
    phase.value = 'checking'
    failure.value = null
    try {
      answered(await ipc.eftCheck(current.id))
    } catch (e) {
      failure.value = describeError(e)
      phase.value = 'result'
    }
  }

  async function runCharge(orderId: number, cents: number) {
    try {
      answered(await ipc.eftPay(orderId, cents))
    } catch (e) {
      // Nothing was charged (or the request was refused first).
      payment.value = null
      failure.value = describeError(e)
      phase.value = 'result'
    }
  }

  /** Charge `cents` of the order on the terminal and wait for the card. */
  function charge(orderId: number, cents: number): Promise<EftPayment | null> {
    return new Promise((resolve) => {
      finish = resolve
      amountCents.value = cents
      payment.value = null
      failure.value = null
      abortRequested.value = false
      phase.value = 'waiting'
      void runCharge(orderId, cents)
    })
  }

  /** Ask the terminal where `p` (sent or unknown) stands. */
  function verify(p: EftPayment): Promise<EftPayment | null> {
    return new Promise((resolve) => {
      finish = resolve
      amountCents.value = p.amount_cents
      payment.value = p
      abortRequested.value = false
      void runCheck()
    })
  }

  /** The overlay's Vérifier button (the payment is unknown). */
  function check() {
    void runCheck()
  }

  /** A request: the pending `eft_pay` then ends cancelled, or approved
   * when the abort came too late. */
  async function abort() {
    if (abortRequested.value) return
    abortRequested.value = true
    try {
      await ipc.eftAbort()
    } catch (e) {
      abortRequested.value = false
      useToastStore().show(describeError(e), 'error')
    }
  }

  /** OK / Fermer on the overlay. */
  function dismiss() {
    close(null)
  }

  return {
    phase,
    amountCents,
    payment,
    failure,
    abortRequested,
    payments,
    linked,
    kind,
    active,
    open,
    approved,
    load,
    clear,
    charge,
    verify,
    check,
    abort,
    dismiss,
  }
})
