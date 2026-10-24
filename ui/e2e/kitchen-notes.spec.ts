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

// The kitchen note of a cart line (command set_line_note): any staff
// member writes it on the on-screen text keypad, at most 60 characters.
// The cart shows it under the name; the dialog says when it prints. The
// engine owns every rule (validation, printing); the screen collects the
// text and reports a refusal or a slip that did not print.

const CASHIER = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa Serveuse', role: 'cashier' }

// Still to send to the kitchen, and already sent.
const UNSENT = { ...DEFAULT_CART_LINE, id: 1, quantity: 2, unsent_quantity: 2 }
const SENT = { ...DEFAULT_CART_LINE, id: 1, quantity: 2, unsent_quantity: 0 }
const NOTED = { ...UNSENT, kitchen_note: 'sans wasabi' }

async function toOrder(page: Page, lines: unknown[] = [UNSENT], user = DEFAULT_USER) {
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2400 }),
    list_cart_lines: () => lines,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

/** Type `text` on the keypad: letters, digits, the space bar and symbols. */
async function type(page: Page, text: string) {
  const pad = page.locator('.keypad-modal')
  for (const ch of text) {
    if (ch === ' ') await pad.locator('.key-space').click()
    else
      await pad
        .locator('.key', {
          hasText: new RegExp(`^${ch.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')}$`, 'u'),
        })
        .click()
  }
}

test.describe('Note button', () => {
  test('every line has one, for a cashier too', async ({ page }) => {
    await toOrder(page, [UNSENT, { ...UNSENT, id: 2 }], CASHIER)
    await expect(page.getByTestId('line-note')).toHaveCount(2)
    await expect(page.getByTestId('line-note').first()).toHaveText('Note')
  })

  test('the cart shows the note under the name', async ({ page }) => {
    await toOrder(page, [NOTED])
    await expect(page.getByTestId('line-kitchen-note')).toHaveText('sans wasabi')
  })

  test('a line without a note shows none', async ({ page }) => {
    await toOrder(page)
    await expect(page.getByTestId('line-kitchen-note')).toHaveCount(0)
  })
})

test.describe('Note dialog', () => {
  test('opens empty with a counter and says the note prints with the order', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('line-note').click()
    const pad = page.locator('.keypad-modal')
    await expect(pad).toContainText('Note pour la cuisine')
    await expect(pad.getByTestId('keypad-counter')).toHaveText('0/60')
    await expect(pad.getByTestId('keypad-hint')).toHaveText('Imprimée en cuisine avec la commande')
  })

  test('says the note prints now once the line went to the kitchen', async ({ page }) => {
    await toOrder(page, [SENT])
    await page.getByTestId('line-note').click()
    await expect(page.getByTestId('keypad-hint')).toHaveText('Imprimée en cuisine maintenant')
  })

  test('writing a note calls set_line_note and shows it in the cart', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('line-note').click()
    await type(page, 'sans wasabi')
    await expect(page.getByTestId('keypad-display')).toContainText('sans wasabi')
    await expect(page.getByTestId('keypad-counter')).toHaveText('11/60')
    await page.locator('.keypad-modal .act-btn.primary').click()

    await expect(page.locator('.keypad-modal')).toHaveCount(0)
    expect(await ipcCalls(page, 'set_line_note')).toEqual([{ lineId: 1, note: 'sans wasabi' }])
    await expect(page.getByTestId('line-kitchen-note')).toHaveText('sans wasabi')
    await expect(page.getByTestId('line-note')).toHaveClass(/filled/u)
  })

  test('accents and punctuation are on the keypad', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('line-note').click()
    await type(page, 'caf')
    await page.locator('.keypad-modal .key', { hasText: /^é$/u }).click()
    await page.locator('.keypad-modal .act-btn.primary').click()
    expect(await ipcCalls(page, 'set_line_note')).toEqual([{ lineId: 1, note: 'café' }])
  })

  test('stops at 60 characters', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('line-note').click()
    const pad = page.locator('.keypad-modal')
    for (let i = 0; i < 6; i++) await type(page, 'abcdefghijk')
    await expect(pad.getByTestId('keypad-counter')).toHaveText('60/60')
    await expect(pad.getByTestId('keypad-display')).toHaveText(`${'abcdefghijk'.repeat(5)}abcde`)
  })

  test('a noted line opens with its note; clearing it removes the note', async ({ page }) => {
    await toOrder(page, [NOTED])
    await page.getByTestId('line-note').click()
    const pad = page.locator('.keypad-modal')
    await expect(pad.getByTestId('keypad-display')).toContainText('sans wasabi')
    await pad.getByRole('button', { name: 'Effacer' }).click()
    await pad.locator('.act-btn.primary').click()
    expect(await ipcCalls(page, 'set_line_note')).toEqual([{ lineId: 1, note: '' }])
    await expect(page.getByTestId('line-kitchen-note')).toHaveCount(0)
  })

  test('cancelling sends nothing', async ({ page }) => {
    await toOrder(page)
    await page.getByTestId('line-note').click()
    await type(page, 'abc')
    await page.locator('.keypad-modal .act-btn.secondary').first().click()
    await expect(page.locator('.keypad-modal')).toHaveCount(0)
    expect(await ipcCalls(page, 'set_line_note')).toHaveLength(0)
    await expect(page.getByTestId('line-kitchen-note')).toHaveCount(0)
  })
})

test.describe('Outcomes', () => {
  test('a note slip that did not print is warned', async ({ page }) => {
    await toOrder(page, [SENT])
    await overrideIPC(page, 'set_line_note', 'failed')
    await page.getByTestId('line-note').click()
    await type(page, 'sans wasabi')
    await page.locator('.keypad-modal .act-btn.primary').click()
    await expect(page.locator('.toast', { hasText: 'Bon de commande non imprimé' })).toBeVisible()
    // The note is stored all the same.
    await expect(page.getByTestId('line-kitchen-note')).toHaveText('sans wasabi')
  })

  test('a refusal is shown and the cart keeps its note', async ({ page }) => {
    await toOrder(page)
    await overrideIPC(page, 'set_line_note', { __reject: 'note_too_long' })
    await page.getByTestId('line-note').click()
    await type(page, 'abc')
    await page.locator('.keypad-modal .act-btn.primary').click()
    await expect(page.locator('.toast', { hasText: '60 caractères au plus' })).toBeVisible()
    await expect(page.getByTestId('line-kitchen-note')).toHaveCount(0)
  })
})

test.describe('Adding the same product again', () => {
  test('makes a new line when the line has a note', async ({ page }) => {
    await toOrder(page, [NOTED])
    await page.locator('.item-btn').first().click()
    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBe(1)
    expect(await ipcCalls(page, 'update_order_line_quantity')).toHaveLength(0)
  })

  test('adds to a plain line', async ({ page }) => {
    await toOrder(page, [UNSENT])
    await page.locator('.item-btn').first().click()
    await expect
      .poll(async () => (await ipcCalls(page, 'update_order_line_quantity')).length)
      .toBe(1)
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0)
  })
})
