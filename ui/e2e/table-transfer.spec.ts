import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  live,
  overrideIPC,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CART_LINE,
  tapUserTile,
} from './fixtures/mock-ipc'

// Moving an open order to another table (command transfer_table), from
// the cart and from the Tables screen's "Déplacer une table" mode.

const tables =
  (occupied: Record<number, number> = {}) =>
  () => {
    const all = []
    for (let n = 1; n <= 20; n++) {
      const orderId = occupied[n] ?? null
      all.push({
        tableNumber: n,
        orderId,
        totalCents: orderId ? 2400 : 0,
        lineCount: orderId ? 2 : 0,
        status: orderId ? 'occupied' : 'free',
        openedAt: null,
        preBillPrinted: false,
      })
    }
    return all
  }

async function openTable1(page: Page, extra: Record<string, any> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_tables: tables({ 4: 44 }),
    ...extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

test.describe('From the cart', () => {
  test('lists the free tables, current one and busy ones left out, and moves the order', async ({
    page,
  }) => {
    await openTable1(page)
    await page.getByTestId('transfer-table').click()

    const picker = page.getByRole('dialog', { name: 'Changer de table' })
    await expect(picker).toBeVisible()
    await expect(picker.getByTestId('pick-table-1')).toHaveCount(0)
    await expect(picker.getByTestId('pick-table-4')).toHaveCount(0)
    await expect(picker.locator('.pick')).toHaveCount(18)
    // Nothing is picked yet.
    await expect(picker.getByTestId('transfer-confirm')).toBeDisabled()

    await picker.getByTestId('pick-table-5').click()
    await picker.getByTestId('transfer-confirm').click()

    await expect(picker).toBeHidden()
    await expect(page.getByText('Commande déplacée vers la table 5')).toBeVisible()
    const calls = await ipcCalls(page, 'transfer_table')
    expect(calls).toHaveLength(1)
    expect(calls[0]).toMatchObject({ toTable: 5 })
  })

  test('shows the refusal in French', async ({ page }) => {
    await openTable1(page)
    await overrideIPC(page, 'transfer_table', { __reject: 'table_transfer_target_busy' })
    await page.getByTestId('transfer-table').click()
    await page.getByTestId('pick-table-6').click()
    await page.getByTestId('transfer-confirm').click()
    await expect(page.getByText('Cette table a déjà une commande en cours')).toBeVisible()
  })

  test('warns when the kitchen notice did not print', async ({ page }) => {
    await openTable1(page, {
      transfer_table: live(({ orderId, toTable }: any) => ({
        orderId,
        fromTable: 1,
        toTable,
        kitchenSlip: 'nothing_to_print',
        transferSlip: 'failed',
      })),
    })
    await page.getByTestId('transfer-table').click()
    await page.getByTestId('pick-table-7').click()
    await page.getByTestId('transfer-confirm').click()
    await expect(page.getByText('Bon de commande non imprimé')).toBeVisible()
  })

  test('the cart header shows the new table and the pending lines are marked sent', async ({
    page,
  }) => {
    const pending = { ...DEFAULT_CART_LINE, quantity: 2, unsent_quantity: 2 }
    await openTable1(page, { list_cart_lines: () => [pending] })
    await expect(page.locator('.cart-header h2')).toHaveText(/Table 1$/u)
    await expect(page.locator('.pending-label')).toBeVisible()
    await expect(page.locator('.order-line.pending')).toHaveCount(1)

    // The engine registers what was unsent before it moves the table.
    await overrideIPC(page, 'get_order', { ...DEFAULT_ORDER, table_number: 5, total_cents: 2400 })
    await page.getByTestId('transfer-table').click()
    await page.getByTestId('pick-table-5').click()
    await page.getByTestId('transfer-confirm').click()

    await expect(page.getByText('Commande déplacée vers la table 5')).toBeVisible()
    await expect(page.locator('.cart-header h2')).toHaveText(/Table 5$/u)
    await expect(page.locator('.pending-label')).toHaveCount(0)
    await expect(page.locator('.order-line.pending')).toHaveCount(0)
    await expect(page.locator('.order-line')).toHaveCount(1)
  })

  test('is hidden while the order has no line, and shown once it has one', async ({ page }) => {
    await openTable1(page, { list_cart_lines: () => [] })
    await expect(page.locator('.cart-empty')).toBeVisible()
    await expect(page.getByTestId('transfer-table')).toHaveCount(0)

    await page.locator('.item-btn').first().click()
    await expect(page.locator('.order-line')).toHaveCount(1)
    await expect(page.getByTestId('transfer-table')).toBeVisible()
  })

  test('says so when no table is free, and moves nothing', async ({ page }) => {
    // Every table is busy but table 1, the order's own.
    const busy: Record<number, number> = {}
    for (let n = 2; n <= 20; n++) busy[n] = 100 + n
    await openTable1(page, { list_tables: tables(busy) })
    await page.getByTestId('transfer-table').click()

    const picker = page.getByTestId('table-picker')
    await expect(picker).toBeVisible()
    await expect(picker).toContainText('Aucune table libre')
    await expect(picker.locator('.pick')).toHaveCount(0)
    await expect(picker.getByTestId('transfer-confirm')).toBeDisabled()
    expect(await ipcCalls(page, 'transfer_table')).toHaveLength(0)
  })

  test('Retour and the backdrop close the picker without a call', async ({ page }) => {
    await openTable1(page)
    await page.getByTestId('transfer-table').click()
    const picker = page.getByTestId('table-picker')
    await picker.getByTestId('pick-table-5').click()
    await picker.getByRole('button', { name: 'Retour' }).click()
    await expect(picker).toHaveCount(0)

    await page.getByTestId('transfer-table').click()
    await expect(picker).toBeVisible()
    await page.mouse.click(5, 5)
    await expect(picker).toHaveCount(0)
    expect(await ipcCalls(page, 'transfer_table')).toHaveLength(0)
  })

  test('a failed table list shows an error and no picker', async ({ page }) => {
    await openTable1(page)
    await overrideIPC(page, 'list_tables', { __reject: 'some_code' })
    await page.getByTestId('transfer-table').click()
    await expect(page.locator('.toast-error')).toBeVisible()
    await expect(page.getByTestId('table-picker')).toHaveCount(0)
    expect(await ipcCalls(page, 'transfer_table')).toHaveLength(0)
  })

  test('is absent for a takeaway', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_cart_lines: () => [],
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.getByRole('button', { name: 'Nouvelle vente à emporter' }).click()
    await page.waitForURL(/order/u)
    await expect(page.getByTestId('transfer-table')).toHaveCount(0)
  })
})

test.describe('From the Tables screen', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_tables: tables({ 3: 42, 4: 44 }),
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
  })

  test('source, then target, then confirmation', async ({ page }) => {
    await page.getByTestId('move-toggle').click()
    await expect(page.getByTestId('move-hint')).toHaveText('Touchez la table à déplacer')

    // A free table is no source: tapping it does nothing.
    await page.locator('.table-card', { hasText: 'Libre' }).first().click({ force: true })
    await expect(page.getByTestId('move-hint')).toHaveText('Touchez la table à déplacer')
    await expect(page).toHaveURL(/tables/u)

    await page.locator('.table-card.occupied').first().click()
    await expect(page.getByTestId('move-hint')).toHaveText('Table 3 : touchez la table libre')
    // A busy table is no target.
    await page.locator('.table-card.occupied').nth(1).click({ force: true })
    await expect(page.getByTestId('move-confirm')).toHaveCount(0)

    await page.locator('.table-card .table-num', { hasText: /^7$/u }).click()
    const confirm = page.getByTestId('move-confirm')
    await expect(confirm).toContainText('Déplacer la table 3 vers la table 7 ?')
    await page.getByTestId('move-validate').click()

    await expect(confirm).toBeHidden()
    await expect(page.getByText('Commande déplacée vers la table 7')).toBeVisible()
    const calls = await ipcCalls(page, 'transfer_table')
    expect(calls).toHaveLength(1)
    expect(calls[0]).toMatchObject({ orderId: 42, toTable: 7 })
    // The mode is left: tapping a table opens it again.
    await expect(page.getByTestId('move-hint')).toHaveCount(0)
  })

  test('a refusal shows the error, leaves the mode and reads the plan again', async ({ page }) => {
    await overrideIPC(page, 'transfer_table', { __reject: 'table_transfer_target_busy' })
    const before = (await ipcCalls(page, 'list_tables')).length

    await page.getByTestId('move-toggle').click()
    await page.locator('.table-card.occupied').first().click()
    await page.locator('.table-card .table-num', { hasText: /^7$/u }).click()
    await page.getByTestId('move-validate').click()

    await expect(page.getByText('Cette table a déjà une commande en cours')).toBeVisible()
    await expect(page.getByTestId('move-hint')).toHaveCount(0)
    await expect(page.getByTestId('move-confirm')).toHaveCount(0)
    await expect.poll(async () => (await ipcCalls(page, 'list_tables')).length).toBe(before + 1)
  })

  test('a move reads the plan again: the target shows occupied', async ({ page }) => {
    const card = (n: number) =>
      page.locator('.table-card', {
        has: page.locator('.table-num', { hasText: new RegExp(`^${n}$`, 'u') }),
      })
    await expect(card(7)).not.toHaveClass(/occupied/u)
    await overrideIPC(page, 'list_tables', tables({ 4: 44, 7: 42 })())

    await page.getByTestId('move-toggle').click()
    await page.locator('.table-card.occupied').first().click()
    await page.locator('.table-card .table-num', { hasText: /^7$/u }).click()
    await page.getByTestId('move-validate').click()

    await expect(page.getByText('Commande déplacée vers la table 7')).toBeVisible()
    await expect(card(7)).toHaveClass(/occupied/u)
    await expect(card(3)).not.toHaveClass(/occupied/u)
  })

  test('cancelling the confirmation or the mode moves nothing', async ({ page }) => {
    await page.getByTestId('move-toggle').click()
    await page.locator('.table-card.occupied').first().click()
    await page.locator('.table-card .table-num', { hasText: /^8$/u }).click()
    await page.getByTestId('move-confirm').getByRole('button', { name: 'Retour' }).click()
    await expect(page.getByTestId('move-confirm')).toHaveCount(0)

    await page.getByTestId('move-toggle').click()
    await expect(page.getByTestId('move-hint')).toHaveCount(0)
    expect(await ipcCalls(page, 'transfer_table')).toHaveLength(0)
  })
})
