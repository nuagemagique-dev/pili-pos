import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  overrideIPC,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CART_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// UX audit T2: the numbered takeaway. Each takeaway of the till carries a number of the day
// ("E-61"); the operator may add a name and a phone from the header ("Ajouter un nom" opens the
// text keypad, then the numeric pad for the phone). The number and the name show in the header
// ("À emporter E-61 · Dupont") and on the takeaway cards of the Tables screen. The first number of
// the day is a manager's setting in the table plan panel. All local data, so the mocks answer
// what the engine does.

const TAKEAWAY = { ...DEFAULT_ORDER, order_type: 'takeaway', table_number: null }
const TAG = {
  orderId: 1,
  day: '2026-11-06',
  number: 61,
  label: 'E-61',
  customerName: null,
  customerPhone: null,
}

async function toTakeaway(page: Page, extra: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    create_order: () => TAKEAWAY,
    get_order: () => TAKEAWAY,
    list_cart_lines: () => [{ ...DEFAULT_CART_LINE, id: 1 }],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    get_takeaway_tag: () => TAG,
    ...extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.new-takeaway-btn').click()
  await page.waitForURL(/order/u)
}

async function typeName(page: Page, text: string) {
  for (const ch of text) {
    await page
      .locator('.keypad-modal .key', { hasText: new RegExp(`^${ch}$`, 'u') })
      .first()
      .click()
  }
}

async function tapDigits(page: Page, digits: string) {
  for (const d of digits) {
    await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
  }
}

test.describe('The takeaway header', () => {
  test('shows the number of the day next to the takeaway badge', async ({ page }) => {
    await toTakeaway(page)
    await expect(page.getByTestId('header-takeaway')).toHaveText('À emporter E-61')
    expect(await ipcCalls(page, 'get_takeaway_tag')).toEqual([{ orderId: 1 }])
  })

  test('keeps the plain badge when the till gives no number', async ({ page }) => {
    await toTakeaway(page, { get_takeaway_tag: () => null })
    await expect(page.getByTestId('header-takeaway')).toHaveText('À emporter')
    await expect(page.getByTestId('takeaway-customer-btn')).toHaveCount(0)
  })

  test('has no name button on a table', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_cart_lines: () => [{ ...DEFAULT_CART_LINE, id: 1 }],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.locator('.table-card').first().click()
    await page.waitForURL(/order/u)
    await expect(page.getByTestId('header-table')).toBeVisible()
    await expect(page.getByTestId('header-takeaway')).toHaveCount(0)
    await expect(page.getByTestId('takeaway-customer-btn')).toHaveCount(0)
  })

  test('"Ajouter un nom" is a 48 px button', async ({ page }) => {
    await toTakeaway(page)
    const button = page.getByTestId('takeaway-customer-btn')
    await expect(button).toHaveText('Ajouter un nom')
    expect((await button.boundingBox())?.height).toBeGreaterThanOrEqual(48)
  })

  test('the name goes through the text keypad, then the phone through the numeric pad', async ({
    page,
  }) => {
    await toTakeaway(page)
    await page.getByTestId('takeaway-customer-btn').click()
    await expect(page.locator('.keypad-modal')).toBeVisible()
    await typeName(page, 'dupont')
    await expect(page.getByTestId('keypad-display')).toHaveText('dupont')
    await page.locator('.keypad-modal .act-btn.primary').click()

    // The name is saved at once, the phone dialog follows.
    expect(await ipcCalls(page, 'set_takeaway_customer')).toEqual([
      { orderId: 1, name: 'dupont', phone: null },
    ])
    await expect(page.getByTestId('header-takeaway')).toHaveText('À emporter E-61 · dupont')
    await expect(page.getByTestId('phone-entry-value')).toHaveText('Pas de numéro')
    await tapDigits(page, '0470123456')
    await expect(page.getByTestId('phone-entry-value')).toHaveText('0470 12 34 56')
    await page.getByTestId('phone-entry-ok').click()

    expect(await ipcCalls(page, 'set_takeaway_customer')).toEqual([
      { orderId: 1, name: 'dupont', phone: null },
      { orderId: 1, name: 'dupont', phone: '0470 12 34 56' },
    ])
    await expect(page.getByTestId('phone-entry-value')).toHaveCount(0)
    await expect(page.getByTestId('takeaway-customer-btn')).toHaveText('Modifier le client')
  })

  test('"Passer" leaves the phone out', async ({ page }) => {
    await toTakeaway(page)
    await page.getByTestId('takeaway-customer-btn').click()
    await typeName(page, 'ana')
    await page.locator('.keypad-modal .act-btn.primary').click()
    await page.getByTestId('phone-entry-skip').click()
    await expect(page.getByTestId('phone-entry-value')).toHaveCount(0)
    expect(await ipcCalls(page, 'set_takeaway_customer')).toHaveLength(1)
    await expect(page.getByTestId('header-takeaway')).toHaveText('À emporter E-61 · ana')
  })

  test('cancelling the name changes nothing and skips the phone', async ({ page }) => {
    await toTakeaway(page)
    await page.getByTestId('takeaway-customer-btn').click()
    await typeName(page, 'ana')
    await page.locator('.keypad-modal .act-btn.secondary').first().click()
    await expect(page.locator('.keypad-modal')).toHaveCount(0)
    await expect(page.getByTestId('phone-entry-value')).toHaveCount(0)
    expect(await ipcCalls(page, 'set_takeaway_customer')).toHaveLength(0)
  })

  test('an existing customer opens the keypad with the name and keeps the phone', async ({
    page,
  }) => {
    await toTakeaway(page, {
      get_takeaway_tag: () => ({
        ...TAG,
        customerName: 'Dupont',
        customerPhone: '0470 12 34 56',
      }),
    })
    await expect(page.getByTestId('header-takeaway')).toHaveText('À emporter E-61 · Dupont')
    await page.getByTestId('takeaway-customer-btn').click()
    await expect(page.getByTestId('keypad-display')).toHaveText('Dupont')
    await page.getByRole('button', { name: 'Effacer' }).click()
    await typeName(page, 'ana')
    await page.locator('.keypad-modal .act-btn.primary').click()
    expect(await ipcCalls(page, 'set_takeaway_customer')).toEqual([
      { orderId: 1, name: 'ana', phone: '0470 12 34 56' },
    ])
    await expect(page.getByTestId('phone-entry-value')).toHaveText('0470 12 34 56')
  })

  test('a refusal of the engine is shown and the phone step does not open', async ({ page }) => {
    await toTakeaway(page)
    await overrideIPC(page, 'set_takeaway_customer', { __reject: 'takeaway_name_invalid|max=30' })
    await page.getByTestId('takeaway-customer-btn').click()
    await typeName(page, 'ana')
    await page.locator('.keypad-modal .act-btn.primary').click()
    await expect(page.locator('.toast', { hasText: '30 caractères au plus' })).toBeVisible()
    await expect(page.getByTestId('phone-entry-value')).toHaveCount(0)
    await expect(page.getByTestId('header-takeaway')).toHaveText('À emporter E-61')
  })
})

test.describe('The takeaway cards of the Tables screen', () => {
  test('lead with the number of the day and the name', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
      list_open_takeaways: () => [
        {
          orderId: 11,
          createdAt: new Date().toISOString(),
          lineCount: 2,
          totalCents: 1500,
          number: 61,
          label: 'E-61',
          customerName: 'Dupont',
        },
        {
          orderId: 12,
          createdAt: new Date().toISOString(),
          lineCount: 1,
          totalCents: 700,
          number: 62,
          label: 'E-62',
          customerName: null,
        },
        {
          orderId: 13,
          createdAt: new Date().toISOString(),
          lineCount: 1,
          totalCents: 400,
          number: null,
          label: null,
          customerName: null,
        },
      ],
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await expect(page.getByTestId('takeaway-number')).toHaveText(['E-61 · Dupont', 'E-62'])
    await expect(page.locator('.takeaway-card')).toHaveCount(3)
  })
})

test.describe('The first takeaway number in Settings', () => {
  async function toSettings(page: Page, extra: Record<string, unknown> = {}, user = DEFAULT_USER) {
    await mockTauriIPC(page, { login: () => user, list_users: () => [user], ...extra })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.evaluate(() => {
      window.location.hash = '/settings'
    })
    await page.waitForURL(/settings/u)
  }

  test('shows the current first number in the table plan panel', async ({ page }) => {
    await toSettings(page)
    await expect(page.getByTestId('takeaway-first-value')).toHaveText('E-61')
    await expect(
      page.getByTestId('takeaway-first').getByText('Premier numéro à emporter'),
    ).toBeVisible()
  })

  test('a manager changes it on the numeric pad and it is saved', async ({ page }) => {
    await toSettings(page)
    await page.getByTestId('takeaway-first-edit').click()
    await expect(page.getByTestId('number-entry-value')).toHaveText('61')
    await page.locator('.numpad .key-clear').click()
    await tapDigits(page, '100')
    await page.getByTestId('number-entry-ok').click()
    expect(await ipcCalls(page, 'set_takeaway_numbering')).toEqual([
      { input: { firstNumber: 100 } },
    ])
    await expect(page.getByTestId('takeaway-first-value')).toHaveText('E-100')
  })

  test('the numeric pad refuses 0 and numbers above 999', async ({ page }) => {
    await toSettings(page)
    await page.getByTestId('takeaway-first-edit').click()
    await page.locator('.numpad .key-clear').click()
    await tapDigits(page, '0')
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    await page.locator('.numpad .key-clear').click()
    await tapDigits(page, '999')
    await expect(page.getByTestId('number-entry-ok')).toBeEnabled()
  })

  test('a refusal of the engine is shown', async ({ page }) => {
    await toSettings(page)
    await overrideIPC(page, 'set_takeaway_numbering', {
      __reject: 'takeaway_number_invalid|min=1|max=999',
    })
    await page.getByTestId('takeaway-first-edit').click()
    await page.getByTestId('number-entry-ok').click()
    await expect(page.locator('.toast', { hasText: 'de 1 à 999' })).toBeVisible()
  })

  test('is for a manager only', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa', role: 'cashier' }
    await toSettings(page, {}, cashier)
    await expect(page.getByTestId('takeaway-first')).toHaveCount(0)
    expect(await ipcCalls(page, 'get_takeaway_numbering')).toHaveLength(0)
  })
})
