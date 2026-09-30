import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import {
  hubriseListWebshopOrders,
  hubrisePatchOrderStatus,
  hubriseSetConfirmedTime,
  hubriseSyncCatalog,
  printWebshopCustomer,
  printWebshopKitchen,
  webshopAcceptOrder,
  webshopCloseCancelledOrder,
  webshopCompleteOrder,
  webshopHandoverOrder,
  webshopRefundOrder,
  webshopRejectOrder,
  type WebshopOrderSummary,
} from '../api/ipc'
import { useI18n } from '../i18n'
import { useToastStore } from './toast'
import type { CheckoutResult, CheckoutPaymentMethod } from '../types'
import { describeError, errorCode } from '../utils/errorMessages'

/**
 * Default preparation window in minutes, applied automatically on
 * Accepter. The TSB webshop delivery bucket gets a longer default
 * because TSB's own driver covers the transit, while aggregators
 * (Deliveroo / Uber Eats / Takeaway) handle their own driver logistics.
 * Collection orders use the base prep window.
 */
function defaultPrepMinutes(order: WebshopOrderSummary): number {
  const isTsbWeb = (order.source?.toLowerCase() ?? '').includes('web')
  if (isTsbWeb && order.service_type === 'delivery') return 40
  return 30
}

interface WebshopOrdersState {
  orders: WebshopOrderSummary[]
  loading: boolean
  error: string | null
  listenersAttached: boolean
}

// Pure: with HubRise off nothing calls it, and the certification
// bundle drops the store and its webshop commands.
export const useWebshopOrdersStore = /* #__PURE__ */ defineStore('webshopOrders', {
  state: (): WebshopOrdersState => ({
    orders: [],
    loading: false,
    error: null,
    listenersAttached: false,
  }),

  getters: {
    // Orders still waiting for an operator decision (accept/reject).
    // Local `status === 'open'` is not a useful signal here - every
    // ingested webshop row starts as 'open', so including it would
    // count accepted/in-prep orders too.
    pendingCount(state): number {
      return state.orders.filter(
        (o) => o.hubrise_status === 'new' || o.hubrise_status === 'received',
      ).length
    },
    inProgressCount(state): number {
      return state.orders.filter(
        (o) =>
          o.hubrise_status === 'accepted' ||
          o.hubrise_status === 'in_preparation',
      ).length
    },
    readyCount(state): number {
      return state.orders.filter(
        (o) => o.hubrise_status === 'awaiting_collection',
      ).length
    },
  },

  actions: {
    async fetchOrders() {
      this.loading = true
      this.error = null
      try {
        this.orders = await hubriseListWebshopOrders()
      } catch (err) {
        this.error = describeError(err)
      } finally {
        this.loading = false
      }
    },

    async patchStatus(hubriseOrderId: string, newStatus: string) {
      try {
        await hubrisePatchOrderStatus(hubriseOrderId, newStatus)
        // Optimistic update; the poller will eventually reconcile.
        const target = this.orders.find(
          (o) => o.hubrise_order_id === hubriseOrderId,
        )
        if (target) {
          target.hubrise_status = newStatus
        }
      } catch (err) {
        this.error = describeError(err)
        throw err
      }
    },

    /**
     * Accepter: the backend signs the order (N if prepaid, P if
     * unpaid) and tells HubRise `accepted` with the target time: the
     * restaurant default unless HubRise already set one.
     */
    async acceptOrder(order: WebshopOrderSummary) {
      const minutes = defaultPrepMinutes(order)
      const confirmed =
        order.confirmed_time ?? new Date(Date.now() + minutes * 60_000).toISOString()
      await this.run(async () => {
        const result = await webshopAcceptOrder(order.id, confirmed)
        if (!result.kitchen_printed) this.warnKitchen()
      })
    },

    async rejectOrder(order: WebshopOrderSummary) {
      await this.run(() => webshopRejectOrder(order.id))
    },

    /** Encaisser an unpaid order, cash or card for its total. */
    async collectOrder(order: WebshopOrderSummary, method: CheckoutPaymentMethod) {
      const payments =
        method === 'cash' ? [{ method }] : [{ method, amount_cents: order.total_cents }]
      await this.runSale(() => webshopHandoverOrder(order.id, payments))
    },

    /** A prepaid order leaves: its sale is already signed. */
    async completeOrder(order: WebshopOrderSummary) {
      await this.run(() => webshopCompleteOrder(order.id))
    },

    /** Rembourser: the logged-in manager confirms the refund. */
    async refundOrder(order: WebshopOrderSummary, managerInsz: string) {
      await this.runSale(() => webshopRefundOrder(order.id, managerInsz))
    },

    /** Clôturer an unpaid order cancelled after its P. */
    async closeCancelledOrder(order: WebshopOrderSummary) {
      await this.runSale(() => webshopCloseCancelledOrder(order.id))
    },

    /** Resynchroniser: fetch the catalog, then the orders it resolves. */
    async resync() {
      await this.run(() => hubriseSyncCatalog())
    },

    /** Run a board action, show its error, reload the board. */
    async run(action: () => Promise<unknown>) {
      const toast = useToastStore()
      try {
        await action()
      } catch (err) {
        toast.show(describeError(err), 'error')
        throw err
      } finally {
        await this.fetchOrders()
      }
    },

    /** [`run`] for an action that signs a sale and answers with it. */
    async runSale(action: () => Promise<CheckoutResult>) {
      await this.run(async () => {
        const result = await action()
        if (!result.success) throw result.error ?? 'internal'
      })
    },

    warnKitchen() {
      const toast = useToastStore()
      const { t } = useI18n()
      toast.show(t('webshop_toast_printer_missing'), 'warning')
    },

    async startPreparation(hubriseOrderId: string) {
      return this.patchStatus(hubriseOrderId, 'in_preparation')
    },
    async markReady(hubriseOrderId: string) {
      return this.patchStatus(hubriseOrderId, 'awaiting_collection')
    },
    async markOutForDelivery(hubriseOrderId: string) {
      return this.patchStatus(hubriseOrderId, 'in_delivery')
    },

    /**
     * Push the HubRise `confirmed_time` forward by `minutes`. The
     * increment stacks on top of the current `confirmed_time` (so
     * tapping +15 twice moves the target by 30 min) and clamps the
     * base to "now" when the existing time is already in the past,
     * so a stale target can never be reinforced.
     */
    async delayByMinutes(hubriseOrderId: string, minutes: number) {
      const toast = useToastStore()
      const { t } = useI18n()
      const target = this.orders.find(
        (o) => o.hubrise_order_id === hubriseOrderId,
      )
      const now = Date.now()
      const currentMs = target?.confirmed_time
        ? new Date(target.confirmed_time).getTime()
        : now
      const baseMs = Number.isFinite(currentMs)
        ? Math.max(currentMs, now)
        : now
      const iso = new Date(baseMs + minutes * 60_000).toISOString()
      try {
        await hubriseSetConfirmedTime(hubriseOrderId, iso)
        if (target) target.confirmed_time = iso
        toast.show(`${t('webshop_toast_delay_ok')} +${minutes} min`, 'success')
      } catch (err) {
        toast.show(describeError(err), 'error')
        throw err
      }
    },

    async printKitchen(orderId: number) {
      const toast = useToastStore()
      const { t } = useI18n()
      try {
        await printWebshopKitchen(orderId)
        toast.show(t('webshop_toast_print_ok'), 'success')
      } catch (err) {
        if (errorCode(err) === 'printer_not_configured') {
          toast.show(t('webshop_toast_printer_missing'), 'warning')
        } else {
          toast.show(describeError(err), 'error')
        }
      }
    },

    async printCustomer(orderId: number) {
      const toast = useToastStore()
      const { t } = useI18n()
      try {
        await printWebshopCustomer(orderId)
        toast.show(t('webshop_toast_print_ok'), 'success')
      } catch (err) {
        if (errorCode(err) === 'printer_not_configured') {
          toast.show(t('webshop_toast_printer_missing'), 'warning')
        } else {
          toast.show(describeError(err), 'error')
        }
      }
    },

    async attachListeners() {
      if (this.listenersAttached) return
      this.listenersAttached = true

      await listen('webshop-order-received', async () => {
        await this.fetchOrders()
      })
      await listen('webshop-order-updated', async () => {
        await this.fetchOrders()
      })
    },
  },
})
