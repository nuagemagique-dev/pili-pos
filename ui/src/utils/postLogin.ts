import type { Router } from 'vue-router'

/** Where a successful login goes (PIN, INSZ or badge): the order screen,
 * which sends a till without an open order to the tables. */
export function goAfterLogin(router: Router) {
  return router.push({ name: 'order' })
}
