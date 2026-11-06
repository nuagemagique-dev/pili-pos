import type { Router } from 'vue-router'
import * as ipc from '../api/ipc'
import { useAuthStore } from '../stores/auth'
import { useOrderStore } from '../stores/order'

/** The order an idle lock interrupted, when it can still be taken up (UX audit T6): the same
 * user is back, and the order is still open. Anything else lands on Tables. */
async function reopenLockedOrder(): Promise<boolean> {
  const auth = useAuthStore()
  const insz = auth.currentUser?.insz
  if (insz === undefined) return false
  const orderId = auth.takeLockedOrder(insz)
  if (orderId === null) return false
  const order = useOrderStore()
  try {
    // A table order with nothing on it was voided when the engine registered it; a closed
    // one was paid meanwhile. Neither comes back.
    const wanted = await ipc.getOrder(orderId)
    if (wanted.status !== 'open') return false
    await order.loadOrder(orderId)
    return true
  } catch {
    order.resetLocal()
    return false
  }
}

/** Where a successful login goes (PIN, INSZ or badge): straight to the
 * tables, like the Android till (L5), so the order screen is never drawn
 * only to bounce back. The exceptions are the order the same user left
 * through an idle lock, which opens again (T6), and an order the screen
 * already holds (the operator is resuming it). */
export async function goAfterLogin(router: Router) {
  const back = await reopenLockedOrder()
  return router.push({ name: back || useOrderStore().currentOrder ? 'order' : 'tables' })
}
