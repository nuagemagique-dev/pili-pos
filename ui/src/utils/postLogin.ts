import type { Router } from 'vue-router'
import { useOrderStore } from '../stores/order'

/** Where a successful login goes (PIN, INSZ or badge): straight to the
 * tables, like the Android till (L5), so the order screen is never drawn
 * only to bounce back. An order the screen already holds (the operator
 * is resuming it) is the one exception. */
export function goAfterLogin(router: Router) {
  return router.push({ name: useOrderStore().currentOrder ? 'order' : 'tables' })
}
