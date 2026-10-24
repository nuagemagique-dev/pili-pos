import { test, expect } from '@playwright/test'
import { mockTauriIPC, loginAndGoto, DEFAULT_ORDER, DEFAULT_ORDER_LINE } from './fixtures/mock-ipc'

test.describe('Checkout View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2400 }),
      list_cart_lines: () => [DEFAULT_ORDER_LINE],
    })
  })

  test('renders checkout view', async ({ page }) => {
    await loginAndGoto(page, '/checkout')
    // The payment method buttons are proof the view rendered.
    await expect(page.locator('.cash-btn')).toHaveText('Espèces')
  })

  test('shows payment buttons, cash selected by default (§7.1)', async ({ page }) => {
    await loginAndGoto(page, '/checkout')
    await expect(page.locator('.cash-btn')).toHaveClass(/active/)
    await expect(page.locator('.card-btn')).toHaveText('Carte')
    await expect(page.locator('.meal-btn')).toHaveText('Chèques-repas')
  })

  test('displays total section', async ({ page }) => {
    await loginAndGoto(page, '/checkout')
    // The amount can be 0,00 (currentOrder not loaded via SPA nav);
    // the "left to pay" block is always there.
    await expect(page.locator('.total-label')).toHaveText('Reste à payer')
    await expect(page.locator('.total-value')).toBeVisible()
  })
})
