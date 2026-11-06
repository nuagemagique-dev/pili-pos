import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  overrideIPC,
  loginAndGoto,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CART_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'
import { fillWithKeypad } from './fixtures/keypad'

// UX audit T1 and T5. A plan can name zones with letters ("A 1-3, B 4-6", named "A · Salle" and
// "B · Terrasse"): the Tables screen groups the tables under a header per zone, and a table reads
// "A-2" on the cards, in the order header, the cart title, the table picker and the move toast.
// A plan without letters is unchanged ("2"). T5: an occupied table card shows the initial of the
// user who opened the order and the covers; the covers are set from the cart header ("Couverts",
// a pad from 1 to 30, 0 clears) and are the usual number of equal parts at checkout.

const ZONED_PLAN = {
  plan: 'A 1-3, B 4-6',
  table_count: 6,
  zone_names: 'A · Salle, B · Terrasse',
  zones: [
    { letter: 'A', name: 'Salle', numbers: [1, 2, 3] },
    { letter: 'B', name: 'Terrasse', numbers: [4, 5, 6] },
  ],
  log: [],
}

const zonedTables =
  (occupied: Record<number, { orderId: number; initial?: string; covers?: number }> = {}) =>
  () => {
    const all = []
    for (let n = 1; n <= 6; n++) {
      const zone = n <= 3 ? 'A' : 'B'
      const busy = occupied[n]
      all.push({
        tableNumber: n,
        zone,
        zoneName: zone === 'A' ? 'Salle' : 'Terrasse',
        label: `${zone}-${n}`,
        openedByInitial: busy ? (busy.initial ?? 'Z') : null,
        covers: busy?.covers ?? null,
        orderId: busy?.orderId ?? null,
        totalCents: busy ? 2400 : 0,
        lineCount: busy ? 2 : 0,
        status: busy ? 'occupied' : 'free',
        openedAt: null,
        preBillPrinted: false,
        heldCourse: null,
      })
    }
    return all
  }

const ZONED_ORDER = { ...DEFAULT_ORDER, table_number: 2 }

async function toTables(page: Page, extra: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_table_plan: () => ZONED_PLAN,
    list_tables: zonedTables({ 1: { orderId: 41, initial: 'A', covers: 4 }, 5: { orderId: 55 } }),
    ...extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
}

async function toOrder(page: Page, extra: Record<string, unknown> = {}) {
  await toTables(page, {
    create_order: () => ZONED_ORDER,
    get_order: () => ({ ...ZONED_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [{ ...DEFAULT_CART_LINE, id: 1 }],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...extra,
  })
  await page.getByTestId('table-2').click()
  await page.waitForURL(/order/u)
}

test.describe('Zones on the Tables screen', () => {
  test('groups the tables under a header per zone and writes "A-2"', async ({ page }) => {
    await toTables(page)
    await expect(page.getByTestId('zone-A')).toHaveText('A · Salle')
    await expect(page.getByTestId('zone-B')).toHaveText('B · Terrasse')
    await expect(page.locator('.table-card')).toHaveCount(6)
    await expect(page.getByTestId('table-2').locator('.table-num')).toHaveText('A-2')
    await expect(page.getByTestId('table-5').locator('.table-num')).toHaveText('B-5')
    // Each header is followed by its own tables.
    const zoneA = page.locator('.zone', { has: page.getByTestId('zone-A') })
    await expect(zoneA.locator('.table-num')).toHaveText(['A-1', 'A-2', 'A-3'])
    const zoneB = page.locator('.zone', { has: page.getByTestId('zone-B') })
    await expect(zoneB.locator('.table-num')).toHaveText(['B-4', 'B-5', 'B-6'])
  })

  test('a zone without a name shows its letter, a plan without letters shows no header', async ({
    page,
  }) => {
    await toTables(page, {
      list_tables: () =>
        [1, 2].map((n) => ({
          tableNumber: n,
          zone: 'A',
          zoneName: null,
          label: `A-${n}`,
          openedByInitial: null,
          covers: null,
          orderId: null,
          totalCents: 0,
          lineCount: 0,
          status: 'free',
          openedAt: null,
          preBillPrinted: false,
          heldCourse: null,
        })),
    })
    await expect(page.getByTestId('zone-A')).toHaveText('A')

    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await expect(page.locator('.zone-header')).toHaveCount(0)
    await expect(page.locator('.table-num').first()).toHaveText('1')
    await expect(page.locator('.table-card')).toHaveCount(20)
  })

  test('an occupied card shows the initial of the server and the covers', async ({ page }) => {
    await toTables(page)
    const busy = page.getByTestId('table-1')
    await expect(busy.getByTestId('server-badge')).toHaveText('A')
    await expect(busy.getByTestId('table-covers')).toContainText('4 couv.')
    // No covers set: no covers on the card, the badge still shows.
    const other = page.getByTestId('table-5')
    await expect(other.getByTestId('server-badge')).toHaveText('Z')
    await expect(other.getByTestId('table-covers')).toHaveCount(0)
    // A free card has neither.
    await expect(page.getByTestId('table-3').getByTestId('server-badge')).toHaveCount(0)
  })

  test('the move mode names the tables with their zone', async ({ page }) => {
    await toTables(page)
    await page.getByTestId('move-toggle').click()
    await expect(page.getByTestId('move-hint')).toContainText('table à déplacer')
    await page.getByTestId('table-1').click()
    await expect(page.getByTestId('move-hint')).toContainText('A-1')
    await page.getByTestId('table-6').click()
    await expect(page.getByText('Table A-1 déplacée en B-6')).toBeVisible()
    expect(await ipcCalls(page, 'transfer_table')).toHaveLength(1)
  })
})

test.describe('Zones on the order screen', () => {
  test('the header, the cart title and the table picker write the zone', async ({ page }) => {
    await toOrder(page)
    await expect(page.getByTestId('header-table')).toHaveText('Table A-2')
    await expect(page.locator('.cart-header h2')).toHaveText('Commande · Table A-2')

    await page.getByTestId('header-table').click()
    const picker = page.getByRole('dialog', { name: 'Déplacer' })
    await expect(picker.locator('.title')).toContainText('Table A-2')
    await expect(picker.getByTestId('pick-table-5')).toContainText('B-5')
    await expect(picker.getByTestId('pick-table-2')).toHaveCount(0)
    // 5 is busy, 4 is free: the move and its toast name both tables.
    await picker.getByTestId('pick-table-4').click()
    await expect(page.getByText('Table A-2 déplacée en B-4')).toBeVisible()
    expect(await ipcCalls(page, 'transfer_table')).toEqual([
      expect.objectContaining({ toTable: 4 }),
    ])
  })

  test('the undo toast names the tables back', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('header-table').click()
    await page.getByTestId('pick-table-6').click()
    await page.getByTestId('toast-action').click()
    await expect(page.getByText('Table B-6 déplacée en A-2')).toBeVisible()
  })
})

test.describe('Covers', () => {
  test('the cart header asks for the covers on a pad and the chip shows them', async ({ page }) => {
    await toOrder(page)
    const chip = page.getByTestId('cart-covers')
    await expect(chip).toHaveText('Couverts')
    await expect(chip).toBeVisible()
    const box = await chip.boundingBox()
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(48)

    await chip.click()
    const dialog = page.getByRole('dialog', { name: /Couverts/u })
    await expect(dialog).toBeVisible()
    await dialog.locator('.key', { hasText: /^4$/u }).click()
    await expect(dialog.getByTestId('number-entry-value')).toHaveText('4')
    await dialog.getByTestId('number-entry-ok').click()
    await expect(chip).toHaveText('4 couverts')
    expect(await ipcCalls(page, 'set_order_covers')).toEqual([{ orderId: 1, covers: 4 }])
  })

  test('one cover is singular and 0 clears them', async ({ page }) => {
    await toOrder(page)
    const chip = page.getByTestId('cart-covers')
    await chip.click()
    await page.locator('.key', { hasText: /^1$/u }).click()
    await page.getByTestId('number-entry-ok').click()
    await expect(chip).toHaveText('1 couvert')

    await chip.click()
    await page.locator('.key-clear').click()
    await page.locator('.key', { hasText: /^0$/u }).click()
    await page.getByTestId('number-entry-ok').click()
    await expect(chip).toHaveText('Couverts')
    expect(await ipcCalls(page, 'set_order_covers')).toEqual([
      { orderId: 1, covers: 1 },
      { orderId: 1, covers: null },
    ])
  })

  test('the pad refuses more than 30', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('cart-covers').click()
    await page.locator('.key', { hasText: /^3$/u }).click()
    await page.locator('.key', { hasText: /^1$/u }).click()
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    await page.locator('.key-back').click()
    await page.locator('.key', { hasText: /^0$/u }).click()
    await expect(page.getByTestId('number-entry-ok')).toBeEnabled()
  })

  test('a refusal of the engine is shown and the chip keeps its value', async ({ page }) => {
    await toOrder(page, { get_order_covers: () => 3 })
    const chip = page.getByTestId('cart-covers')
    await expect(chip).toHaveText('3 couverts')
    await overrideIPC(page, 'set_order_covers', { __reject: 'order_not_open' })
    await chip.click()
    await page.locator('.key-clear').click()
    await page.locator('.key', { hasText: /^5$/u }).click()
    await page.getByTestId('number-entry-ok').click()
    await expect(page.getByTestId('toast-stack')).toContainText('clôturée')
    await expect(chip).toHaveText('3 couverts')
  })

  test('a takeaway has no covers chip', async ({ page }) => {
    await toTables(page, {
      create_order: () => ({ ...DEFAULT_ORDER, order_type: 'takeaway', table_number: null }),
      get_order: () => ({ ...DEFAULT_ORDER, order_type: 'takeaway', table_number: null }),
      list_cart_lines: () => [{ ...DEFAULT_CART_LINE, id: 1 }],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
    })
    await page.locator('.new-takeaway-btn').click()
    await page.waitForURL(/order/u)
    await expect(page.locator('.cart-header h2')).toBeVisible()
    await expect(page.getByTestId('cart-covers')).toHaveCount(0)
  })

  test('the covers are the usual number of equal parts at checkout', async ({ page }) => {
    await toOrder(page, { get_order_covers: () => 6 })
    await expect(page.getByTestId('cart-covers')).toHaveText('6 couverts')
    await page.locator('.checkout-btn').click()
    await page.waitForURL(/checkout/u)
    await expect(page.getByTestId('split-covers')).toHaveText('6 couverts')
    // Nothing is split by itself: the sale is still in one part.
    await expect(page.locator('.split-btn.active')).toHaveText('1')
    // 6 is not one of the four buttons: "Autre" is marked and its pad starts on 6.
    await expect(page.getByTestId('split-other')).toHaveClass(/covers-hint/u)
    await page.getByTestId('split-other').click()
    await expect(page.getByTestId('number-entry-value')).toHaveText('6')
    await page.getByTestId('number-entry-ok').click()
    await expect(page.getByTestId('split-other')).toContainText('6')
  })

  test('with 3 covers the button 3 is marked', async ({ page }) => {
    await toOrder(page, { get_order_covers: () => 3 })
    await page.locator('.checkout-btn').click()
    await page.waitForURL(/checkout/u)
    await expect(page.getByTestId('split-covers-hint')).toHaveText('3')
    await expect(page.getByTestId('split-covers-hint')).toHaveClass(/covers-hint/u)
  })
})

test.describe('Settings', () => {
  test('a manager writes the zones and their names', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      set_table_plan: () => ({
        plan: 'A 1-3, B 4-6',
        table_count: 6,
        zone_names: 'A · Salle, B · Terrasse',
        zones: [],
        log: [],
      }),
    })
    await loginAndGoto(page, '/settings')
    const panel = page.locator('.table-plan')
    await fillWithKeypad(page, panel.getByTestId('table-plan-input'), 'A 1-3, B 4-6')
    await fillWithKeypad(page, panel.getByTestId('zone-names-input'), 'A Salle, B Terrasse')
    await panel.locator('.table-plan-save').click()
    await expect(panel.getByTestId('zone-names-input')).toHaveText('A · Salle, B · Terrasse')
    await expect(panel.locator('.table-plan-current')).toHaveText('6 tables : A 1-3, B 4-6')
    expect(await ipcCalls(page, 'set_table_plan')).toEqual([
      { plan: 'A 1-3, B 4-6', zoneNames: 'A Salle, B Terrasse' },
    ])
  })

  test('a wrong zone says how to write the plan', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] })
    await loginAndGoto(page, '/settings')
    const panel = page.locator('.table-plan')
    await overrideIPC(page, 'set_table_plan', { __reject: 'table_plan_invalid' })
    await fillWithKeypad(page, panel.getByTestId('table-plan-input'), 'A 1-9, B 5-12')
    await panel.locator('.table-plan-save').click()
    await expect(panel.locator('.add-error')).toContainText('zones')
  })
})
