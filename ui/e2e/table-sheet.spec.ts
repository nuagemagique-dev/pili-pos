import { test, expect, type Locator, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  overrideIPC,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  tapUserTile,
} from './fixtures/mock-ipc'

// UX audit T7: a long press (500 ms) on an occupied table card opens an action sheet
// "Ouvrir / Addition / Déplacer / Payer". A short tap still opens the table. The sheet runs
// the same commands as the cart (sign_pre_bill, transfer_table with the undo toast, the
// checkout), and the cashier rights hide "Addition" and "Payer" for a waiter.

const tables =
  (occupied: Record<number, number> = { 3: 42, 4: 44 }) =>
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

async function openTables(page: Page, role = 'admin') {
  const user = { ...DEFAULT_USER, role }
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    list_tables: tables(),
    set_active_order: () => null,
    get_order: () => ({ ...DEFAULT_ORDER, id: 42, total_cents: 2400 }),
    list_cart_lines: () => [DEFAULT_ORDER_LINE],
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
}

const card = (page: Page, n: number) => page.getByTestId(`table-${n}`)

/** A finger held on `target` for `ms`, then lifted where it is. */
async function hold(page: Page, target: Locator, ms: number) {
  const box = await target.boundingBox()
  if (!box) throw new Error('card not visible')
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
  await page.mouse.down()
  await page.waitForTimeout(ms)
  await page.mouse.up()
}

const sheet = (page: Page) => page.getByTestId('table-sheet')

test.describe('Long press on an occupied table', () => {
  test('opens the sheet after 500 ms and keeps it open when the finger lifts', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 3), 650)

    await expect(sheet(page)).toBeVisible()
    await expect(sheet(page)).toHaveAttribute('aria-label', 'Table 3')
    await expect(sheet(page).getByTestId('sheet-open')).toHaveText('Ouvrir')
    await expect(sheet(page).getByTestId('sheet-bill')).toHaveText('Addition')
    await expect(sheet(page).getByTestId('sheet-move')).toHaveText('Déplacer')
    await expect(sheet(page).getByTestId('sheet-pay')).toHaveText('Payer')
    // The lift of the finger neither opened the table nor closed the sheet.
    await expect(page).toHaveURL(/tables/u)
    await page.waitForTimeout(200)
    await expect(sheet(page)).toBeVisible()
  })

  test('every action is a 48 px target at least', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 3), 650)
    for (const id of ['sheet-open', 'sheet-bill', 'sheet-move', 'sheet-pay', 'sheet-close']) {
      const box = await sheet(page).getByTestId(id).boundingBox()
      expect(box?.height ?? 0).toBeGreaterThanOrEqual(48)
    }
  })

  test('a short tap still opens the table, even a held one under 500 ms', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 3), 300)
    await expect(sheet(page)).toHaveCount(0)
    await expect(page).toHaveURL(/order/u)
  })

  test('a free table has no sheet', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 7), 650)
    await expect(sheet(page)).toHaveCount(0)
    // A free table is started by the click that ends the hold, as always.
    await expect(page).toHaveURL(/order/u)
  })

  test('a finger that slides away is a scroll: no sheet', async ({ page }) => {
    await openTables(page)
    const box = await card(page, 3).boundingBox()
    if (!box) throw new Error('card not visible')
    await page.mouse.move(box.x + 40, box.y + 40)
    await page.mouse.down()
    await page.mouse.move(box.x + 40, box.y + 100, { steps: 4 })
    await page.waitForTimeout(650)
    await page.mouse.up()
    await expect(sheet(page)).toHaveCount(0)
  })

  test('raises no context menu', async ({ page }) => {
    await openTables(page)
    const prevented = await card(page, 3).evaluate((el) => {
      const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true })
      el.dispatchEvent(event)
      return event.defaultPrevented
    })
    expect(prevented).toBe(true)
  })

  test('the next tap after the sheet is a normal tap', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 3), 650)
    await page.getByTestId('sheet-close').click()
    await expect(sheet(page)).toHaveCount(0)
    await card(page, 3).click()
    await expect(page).toHaveURL(/order/u)
  })

  test('a tap outside the sheet closes it and does nothing else', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 3), 650)
    await page.mouse.click(5, 5)
    await expect(sheet(page)).toHaveCount(0)
    await expect(page).toHaveURL(/tables/u)
    expect(await ipcCalls(page, 'sign_pre_bill')).toHaveLength(0)
  })

  test('no sheet in the move mode', async ({ page }) => {
    await openTables(page)
    await page.getByTestId('move-toggle').click()
    await hold(page, card(page, 3), 650)
    await expect(sheet(page)).toHaveCount(0)
    // The tap that ended the hold chose the source, as in the mode.
    await expect(page.getByTestId('move-hint')).toHaveText('Table 3 : touchez la table libre')
  })
})

test.describe('The actions of the sheet', () => {
  test('Ouvrir loads the order and goes to it', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 3), 650)
    await page.getByTestId('sheet-open').click()

    await expect(page).toHaveURL(/order/u)
    expect(await ipcCalls(page, 'get_order')).toContainEqual(
      expect.objectContaining({ orderId: 42 }),
    )
    expect(await ipcCalls(page, 'set_active_order')).toContainEqual(
      expect.objectContaining({ orderId: 42 }),
    )
  })

  test('Addition signs the pre-bill of that order and stays on the plan', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 4), 650)
    await page.getByTestId('sheet-bill').click()

    await expect(page.getByText('Addition provisoire imprimée')).toBeVisible()
    await expect(sheet(page)).toHaveCount(0)
    await expect(page).toHaveURL(/tables/u)
    const calls = await ipcCalls(page, 'sign_pre_bill')
    expect(calls).toHaveLength(1)
    expect(calls[0]).toMatchObject({ orderId: 44 })
  })

  test('a refused Addition shows the error', async ({ page }) => {
    await openTables(page)
    await overrideIPC(page, 'sign_pre_bill', { __reject: 'fdm_unavailable' })
    await hold(page, card(page, 3), 650)
    await page.getByTestId('sheet-bill').click()
    await expect(page.locator('.toast-error')).toContainText('Addition non enregistrée')
    await expect(page).toHaveURL(/tables/u)
  })

  test('Déplacer picks this table as the source, then one tap on a free table moves it', async ({
    page,
  }) => {
    await openTables(page)
    await hold(page, card(page, 3), 650)
    await page.getByTestId('sheet-move').click()

    await expect(sheet(page)).toHaveCount(0)
    await expect(page.getByTestId('move-hint')).toHaveText('Table 3 : touchez la table libre')
    await card(page, 7).click()

    await expect(page.getByText('Table 3 déplacée en 7')).toBeVisible()
    await expect(page.getByTestId('toast-action')).toHaveText('Annuler')
    const calls = await ipcCalls(page, 'transfer_table')
    expect(calls).toHaveLength(1)
    expect(calls[0]).toMatchObject({ orderId: 42, toTable: 7 })
  })

  test('Payer loads the order and opens its checkout', async ({ page }) => {
    await openTables(page)
    await hold(page, card(page, 3), 650)
    await page.getByTestId('sheet-pay').click()

    await expect(page).toHaveURL(/checkout/u)
    expect(await ipcCalls(page, 'get_order')).toContainEqual(
      expect.objectContaining({ orderId: 42 }),
    )
  })

  test('a waiter is offered Ouvrir and Déplacer only', async ({ page }) => {
    await openTables(page, 'waiter')
    await hold(page, card(page, 3), 650)
    await expect(sheet(page).getByTestId('sheet-open')).toBeVisible()
    await expect(sheet(page).getByTestId('sheet-move')).toBeVisible()
    await expect(sheet(page).getByTestId('sheet-bill')).toHaveCount(0)
    await expect(sheet(page).getByTestId('sheet-pay')).toHaveCount(0)
  })

  test('a cashier is offered the four', async ({ page }) => {
    await openTables(page, 'cashier')
    await hold(page, card(page, 3), 650)
    await expect(sheet(page).locator('button.act')).toHaveCount(4)
  })
})
