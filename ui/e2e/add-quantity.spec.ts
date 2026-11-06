import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
  live,
} from './fixtures/mock-ipc'

// C7 (a digit after an add = quantity, the XudoX order) and C9 (beep and
// flash on every add, switch in Settings).
const MAKI = { ...DEFAULT_MENU_ITEM, id: 1, name: 'Maki saumon', code: 'A12', price_cents: 1200 }
const COLA = { ...DEFAULT_MENU_ITEM, id: 2, name: 'Cola', code: 'B3', price_cents: 300 }
const BOWL = { ...DEFAULT_MENU_ITEM, id: 3, name: 'Menu libre', code: 'C1', open_price: true }

async function openOrder(page: Page) {
  await page.addInitScript(() => {
    localStorage.removeItem('pili-pos-code-pad')
    localStorage.removeItem('pili-pos-add-beep')
    // A fake Web Audio: every beep creates one oscillator.
    const w = window as unknown as { __beeps: number; AudioContext: unknown }
    w.__beeps = 0
    w.AudioContext = class {
      currentTime = 0
      destination = {}
      createOscillator() {
        w.__beeps++
        return {
          type: '',
          frequency: { value: 0 },
          connect: (n: unknown) => n,
          start() {},
          stop() {},
        }
      }
      createGain() {
        return {
          gain: { setValueAtTime() {}, exponentialRampToValueAtTime() {} },
          connect() {},
        }
      }
    }
  })
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [MAKI, COLA, BOWL],
    // One line per add, with its own id, the product of the tap and the quantity asked.
    add_order_line: live(({ input }: any) => {
      const w = window as any
      w.__lineId = (w.__lineId ?? 100) + 1
      return {
        id: w.__lineId,
        order_id: input.order_id,
        menu_item_id: input.menu_item_id,
        quantity: input.quantity,
        unit_price_cents: input.unit_price_cents ?? 500,
        vat_code: 'C',
        name: ({ 1: 'Maki saumon', 2: 'Cola', 3: 'Menu libre' } as any)[input.menu_item_id],
        modifiers_json: null,
        created_at: new Date().toISOString(),
        department_id: '1',
        department_name: 'Sushi',
      }
    }),
    update_order_line_quantity: () => null,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
  await page.getByTestId('code-pad-toggle').click()
  await expect(page.getByTestId('code-pad')).toBeVisible()
}

const tapProduct = (page: Page, name: string) =>
  page.locator('.item-btn', { hasText: name }).click()
const beeps = (page: Page) =>
  page.evaluate(() => (window as unknown as { __beeps: number }).__beeps)

test.describe('Digit after an add = quantity (C7)', () => {
  test('a digit after a grid tap gives that many of the product, on the same line', async ({
    page,
  }) => {
    await openOrder(page)
    await tapProduct(page, 'Maki saumon')
    await expect(page.locator('.order-line .line-qty')).toHaveText('1×')

    await page.getByTestId('code-key-3').click()

    await expect
      .poll(async () => (await ipcCalls(page, 'update_order_line_quantity')).length)
      .toBe(1)
    const [call] = await ipcCalls(page, 'update_order_line_quantity')
    expect(call.quantity).toBe(3)
    await expect(page.locator('.order-line')).toHaveCount(1)
    await expect(page.locator('.order-line .line-qty')).toHaveText('3×')
    await expect(page.getByTestId('code-pad-message')).toHaveText('Maki saumon × 3')
    // The digit did not start a code.
    await expect(page.getByTestId('code-pad-display')).not.toContainText('3')
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(1)
  })

  test('a second digit extends it (1, 2 = 12) from a physical keyboard too', async ({ page }) => {
    await openOrder(page)
    await tapProduct(page, 'Cola')

    await page.keyboard.press('1')
    await page.keyboard.press('2')

    await expect(page.locator('.order-line .line-qty')).toHaveText('12×')
    await expect(page.getByTestId('code-pad-message')).toHaveText('Cola × 12')
    const calls = await ipcCalls(page, 'update_order_line_quantity')
    expect(calls.at(-1)?.quantity).toBe(12)
  })

  test('the units a merged line already had are kept: only the extra ones are added', async ({
    page,
  }) => {
    await openOrder(page)
    await tapProduct(page, 'Maki saumon')
    await tapProduct(page, 'Maki saumon')
    await expect(page.locator('.order-line .line-qty')).toHaveText('2×')

    await page.getByTestId('code-key-3').click()

    // 2 on the line + (3 - 1) extra units from this add.
    await expect(page.locator('.order-line .line-qty')).toHaveText('4×')
    const calls = await ipcCalls(page, 'update_order_line_quantity')
    expect(calls.at(-1)?.quantity).toBe(4)
  })

  test('a code pad add can be corrected by a digit too', async ({ page }) => {
    await openOrder(page)
    for (const k of ['B', '3']) await page.getByTestId(`code-key-${k}`).click()
    await page.getByTestId('code-key-ok').click()
    await expect(page.locator('.order-line .line-qty')).toHaveText('1×')

    await page.getByTestId('code-key-5').click()

    await expect(page.locator('.order-line .line-qty')).toHaveText('5×')
    await expect(page.getByTestId('code-pad-message')).toHaveText('Cola × 5')
  })

  test('a digit under the units the add gave removes nothing; a longer number still extends', async ({
    page,
  }) => {
    await openOrder(page)
    for (const k of ['2', 'times']) await page.getByTestId(`code-key-${k}`).click()
    await tapProduct(page, 'Maki saumon')
    await expect(page.locator('.order-line .line-qty')).toHaveText('2×')

    await page.getByTestId('code-key-1').click()

    // The add already gave 2: the "1" leaves the line as it is and sends no quantity command.
    await expect(page.getByTestId('code-pad-message')).toHaveText('Maki saumon × 2')
    await expect(page.locator('.order-line .line-qty')).toHaveText('2×')
    expect(await ipcCalls(page, 'update_order_line_quantity')).toHaveLength(0)

    // "1" then "5" is 15.
    await page.getByTestId('code-key-5').click()
    await expect(page.locator('.order-line .line-qty')).toHaveText('15×')
    const calls = await ipcCalls(page, 'update_order_line_quantity')
    expect(calls.at(-1)?.quantity).toBe(15)
  })

  test('after 3 seconds a digit starts a code as today', async ({ page }) => {
    await page.clock.install()
    await openOrder(page)
    await tapProduct(page, 'Maki saumon')

    await page.clock.fastForward(3500)
    await page.getByTestId('code-key-3').click()

    await expect(page.getByTestId('code-pad-display')).toContainText('3')
    expect(await ipcCalls(page, 'update_order_line_quantity')).toHaveLength(0)
    await expect(page.locator('.order-line .line-qty')).toHaveText('1×')
  })

  test('a letter closes the window: the next digit is part of a code', async ({ page }) => {
    await openOrder(page)
    await tapProduct(page, 'Maki saumon')

    await page.getByTestId('code-key-A').click()
    await page.getByTestId('code-key-3').click()

    await expect(page.getByTestId('code-pad-display')).toContainText('A3')
    expect(await ipcCalls(page, 'update_order_line_quantity')).toHaveLength(0)
  })

  test('another product added moves the window to it', async ({ page }) => {
    await openOrder(page)
    await tapProduct(page, 'Maki saumon')
    await tapProduct(page, 'Cola')

    await page.getByTestId('code-key-2').click()

    await expect(page.locator('.order-line').nth(1).locator('.line-qty')).toHaveText('2×')
    await expect(page.locator('.order-line').nth(0).locator('.line-qty')).toHaveText('1×')
    const [call] = await ipcCalls(page, 'update_order_line_quantity')
    expect(call.lineId).toBe(102)
  })

  test('a product with a free price ignores the window', async ({ page }) => {
    await openOrder(page)
    await page.locator('.item-btn', { hasText: 'Menu libre' }).click()
    const dialog = page.getByRole('dialog', { name: 'Prix libre' })
    for (const d of ['2', '0', '0']) {
      await dialog.locator('.key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
    }
    await dialog.getByRole('button', { name: 'Valider' }).click()
    await expect(page.locator('.order-line')).toHaveCount(1)

    await page.getByTestId('code-key-3').click()

    // The digit starts a code instead of changing the quantity.
    await expect(page.getByTestId('code-pad-display')).toContainText('3')
    expect(await ipcCalls(page, 'update_order_line_quantity')).toHaveLength(0)
  })

  test('"3 ×" then a product still works', async ({ page }) => {
    await openOrder(page)
    for (const k of ['3', 'times']) await page.getByTestId(`code-key-${k}`).click()
    await tapProduct(page, 'Cola')

    await expect(page.locator('.order-line .line-qty')).toHaveText('3×')
    const [call] = await ipcCalls(page, 'add_order_line')
    expect(call.input.quantity).toBe(3)
  })

  test('the digit never reaches the kitchen note or a signed request: only the quantity command', async ({
    page,
  }) => {
    await openOrder(page)
    await tapProduct(page, 'Cola')
    await page.getByTestId('code-key-4').click()
    await expect(page.locator('.order-line .line-qty')).toHaveText('4×')

    const cmds = (await page.evaluate(() => (window as any).__ipcCalls)).map(
      (c: { cmd: string }) => c.cmd,
    )
    expect(cmds.filter((c: string) => /sign|note|send/u.test(c))).toEqual([])
  })
})

test.describe('Beep and flash on add (C9)', () => {
  test('every add beeps once and flashes its line; the digit window too', async ({ page }) => {
    await openOrder(page)
    expect(await beeps(page)).toBe(0)

    await tapProduct(page, 'Cola')
    await expect.poll(() => beeps(page)).toBe(1)
    await expect(page.locator('.order-line.flash-added')).toHaveCount(1)

    await page.getByTestId('code-key-2').click()
    await expect.poll(() => beeps(page)).toBe(2)

    for (const k of ['A', '1', '2']) await page.getByTestId(`code-key-${k}`).click()
    await page.getByTestId('code-key-ok').click()
    await expect.poll(() => beeps(page)).toBe(3)
    await expect(page.locator('.order-line.flash-added')).toHaveCount(2)
  })

  test('the switch in Settings turns the beep off for this terminal and is remembered', async ({
    page,
  }) => {
    await openOrder(page)
    await page.evaluate(() => {
      window.location.hash = '/settings'
    })
    await expect(page.getByTestId('add-beep-on')).toHaveClass(/active/u)
    await page.getByTestId('add-beep-off').click()
    await expect(page.getByTestId('add-beep-off')).toHaveClass(/active/u)
    expect(await page.evaluate(() => localStorage.getItem('pili-pos-add-beep'))).toBe('0')

    await page.evaluate(() => {
      window.location.hash = '/order'
    })
    await tapProduct(page, 'Cola')
    await expect(page.locator('.order-line.flash-added')).toHaveCount(1)
    expect(await beeps(page)).toBe(0)
  })
})
