import { defineStore } from 'pinia'
import { listen } from '@tauri-apps/api/event'
import {
  hubriseListWebshopOrders,
  hubrisePatchOrderStatus,
  hubriseSetConfirmedTime,
  printWebshopCustomer,
  printWebshopKitchen,
  type WebshopOrderSummary,
} from '../api/ipc'
import { useI18n } from '../i18n'
import { useToastStore } from './toast'
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

export const useWebshopOrdersStore = defineStore('webshopOrders', {
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

    async acceptOrder(hubriseOrderId: string) {
      // Step 1 - status transition as before.
      await this.patchStatus(hubriseOrderId, 'accepted')

      // Step 2 - seed confirmed_time with the restaurant default so
      // aggregator drivers and the customer get a target ETA the
      // moment we accept. Respect any value already set upstream
      // (e.g. HubRise pre-populated it) and never overwrite it.
      const target = this.orders.find(
        (o) => o.hubrise_order_id === hubriseOrderId,
      )
      if (target && !target.confirmed_time) {
        const minutes = defaultPrepMinutes(target)
        const iso = new Date(Date.now() + minutes * 60_000).toISOString()
        try {
          await hubriseSetConfirmedTime(hubriseOrderId, iso)
          target.confirmed_time = iso
        } catch (err) {
          // Best-effort: Accepter stays green even if this secondary
          // PATCH fails; the operator can still tap the delay chips.
          console.error('auto-set confirmed_time failed:', err)
        }
      }
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
    async completeDelivery(hubriseOrderId: string) {
      return this.patchStatus(hubriseOrderId, 'completed')
    },
    async rejectOrder(hubriseOrderId: string) {
      return this.patchStatus(hubriseOrderId, 'rejected')
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
