import { test, expect } from '@playwright/test'
import {
  mockTauriIPC,
  loginAndGoto,
  ipcCalls,
  DEFAULT_USER,
  tapUserTile,
} from './fixtures/mock-ipc'

// Card payments the terminal approved that no sale records yet
// (command list_unsettled_card_payments): the Z tells the operator and
// is never blocked; the table plan and the takeaway list mark their orders.

const TABLE_PAYMENT = {
  payment_id: 1,
  order_id: 42,
  order_type: 'dine_in',
  table_number: 4,
  amount_cents: 1700,
  brand: 'Bancontact',
  created_at: '2026-10-13 12:00:00',
}
const TAKEAWAY_PAYMENT = {
  payment_id: 2,
  order_id: 43,
  order_type: 'takeaway',
  table_number: null,
  amount_cents: 950,
  brand: null,
  created_at: '2026-10-13 12:05:00',
}

test.describe('Z with charged cards not yet settled', () => {
  test('lists them above the Z button', async ({ page }) => {
    await mockTauriIPC(page, {
      list_unsettled_card_payments: () => [TABLE_PAYMENT, TAKEAWAY_PAYMENT],
    })
    await loginAndGoto(page, '/reports')
    const warning = page.getByTestId('z-unsettled')
    await expect(warning).toContainText(
      'Paiements par carte encaissés mais pas encore enregistrés dans une vente :',
    )
    await expect(warning.locator('li')).toHaveText([
      'Table 4 · 17,00 € · Bancontact',
      'À emporter · 9,50 €',
    ])
  })

  test('is absent when every card is settled', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await expect(page.locator('.z-btn')).toBeVisible()
    await expect(page.getByTestId('z-unsettled')).toHaveCount(0)
  })

  test('never blocks the Z, and is read again once it is done', async ({ page }) => {
    await mockTauriIPC(page, { list_unsettled_card_payments: () => [TABLE_PAYMENT] })
    await loginAndGoto(page, '/reports')
    await expect(page.getByTestId('z-unsettled')).toBeVisible()
    await page.locator('.z-btn').click()
    await page.locator('.z-btn').click()
    await expect.poll(() => ipcCalls(page, 'generate_z_report')).toHaveLength(1)
    // Read at the opening, when the Z is armed (so it is fresh), and after the Z.
    await expect
      .poll(async () => (await ipcCalls(page, 'list_unsettled_card_payments')).length)
      .toBeGreaterThanOrEqual(3)
  })
})

test.describe('Orders with a charged card', () => {
  test('are marked on the table plan and the takeaway list', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_tables: () => [
        { tableNumber: 1, orderId: 42, totalCents: 1700, lineCount: 2, status: 'occupied' },
        { tableNumber: 2, orderId: 44, totalCents: 800, lineCount: 1, status: 'occupied' },
        { tableNumber: 3, orderId: null, totalCents: 0, lineCount: 0, status: 'free' },
      ],
      list_open_takeaways: () => [
        { orderId: 43, totalCents: 950, lineCount: 1, createdAt: '2026-10-13T12:05:00Z' },
        { orderId: 45, totalCents: 300, lineCount: 1, createdAt: '2026-10-13T12:10:00Z' },
      ],
      list_unsettled_card_payments: () => [{ ...TABLE_PAYMENT, table_number: 1 }, TAKEAWAY_PAYMENT],
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await expect(page.locator('.table-card').nth(0).getByTestId('card-charged')).toHaveText(
      'Carte encaissée',
    )
    await expect(page.locator('.table-card').nth(1).getByTestId('card-charged')).toHaveCount(0)
    await expect(page.locator('.takeaway-card').nth(0).getByTestId('card-charged')).toBeVisible()
    await expect(page.locator('.takeaway-card').nth(1).getByTestId('card-charged')).toHaveCount(0)
  })
})
