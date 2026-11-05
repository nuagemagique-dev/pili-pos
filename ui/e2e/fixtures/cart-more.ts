// Helpers for the cart's "Plus" menu (UX audit C3): transfer, order
// discount and "customer left" live behind it.
import { expect, type Page } from '@playwright/test'

/** Opens the "Plus" menu of the cart header. */
export async function openMore(page: Page) {
  await page.getByTestId('cart-more-open').click()
  await expect(page.getByTestId('cart-more')).toBeVisible()
}

/** Closes the "Plus" menu without choosing anything. */
export async function closeMore(page: Page) {
  await page.getByTestId('cart-more-close').click()
  await expect(page.getByTestId('cart-more')).toHaveCount(0)
}

/** Opens "Plus" and taps one of its rows by test id. */
export async function tapMore(page: Page, testId: string) {
  await openMore(page)
  await page.getByTestId(testId).click()
}

/**
 * Proves a row of the "Plus" menu is not offered: either the whole menu is
 * absent, or it is open on a list without that row.
 */
export async function expectNoMoreRow(page: Page, testId: string) {
  if ((await page.getByTestId('cart-more-open').count()) === 0) return
  await openMore(page)
  await expect(page.getByTestId(testId)).toHaveCount(0)
  await closeMore(page)
}

/** Proves a row of the "Plus" menu is offered. */
export async function expectMoreRow(page: Page, testId: string) {
  await openMore(page)
  await expect(page.getByTestId(testId)).toBeVisible()
  await closeMore(page)
}
