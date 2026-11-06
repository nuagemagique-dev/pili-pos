import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CART_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
} from './fixtures/mock-ipc'
import { expectMoreRow, expectNoMoreRow, openMore } from './fixtures/cart-more'

// UX audit C4 and C3. While lines wait for the kitchen, "Envoyer en cuisine" is the
// Volt primary of the cart footer with a pill counting the lines, and PAYER is an
// outlined secondary that still shows the total. Once everything is sent PAYER is the
// primary again. The rare cart actions (transfer, discount, customer left) sit behind
// the "Plus" button of the cart header.

const SENT = { ...DEFAULT_CART_LINE, id: 1, quantity: 1, unsent_quantity: 0 }
const PENDING = (id: number, extra: Record<string, unknown> = {}) => ({
  ...DEFAULT_CART_LINE,
  id,
  menu_item_id: id,
  name: `Plat ${id}`,
  name_fr: `Plat ${id}`,
  quantity: 2,
  unsent_quantity: 2,
  ...extra,
})

async function openTable(
  page: Page,
  lines: unknown[],
  opts: { role?: string; orderType?: string } = {},
) {
  const user = { ...DEFAULT_USER, role: opts.role ?? 'admin' }
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    create_order: () => ({
      ...DEFAULT_ORDER,
      order_type: opts.orderType ?? 'dine_in',
      table_number: opts.orderType === 'takeaway' ? null : 1,
    }),
    get_order: () => ({
      ...DEFAULT_ORDER,
      order_type: opts.orderType ?? 'dine_in',
      table_number: opts.orderType === 'takeaway' ? null : 1,
      total_cents: 4730,
    }),
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

test.describe('C4 contextual primary action', () => {
  test('lines to send: Envoyer en cuisine is the Volt primary with a pill, PAYER is secondary', async ({
    page,
  }) => {
    await openTable(page, [SENT, PENDING(2), PENDING(3)])
    const primary = page.getByTestId('send-primary')
    await expect(primary).toContainText('Envoyer en cuisine')
    await expect(page.getByTestId('send-pill')).toHaveText('2 lignes')
    await expect(primary).toHaveCSS('background-color', 'rgb(215, 255, 58)')

    const pay = page.getByTestId('pay-secondary')
    await expect(pay).toContainText('PAYER')
    await expect(pay).toContainText('60,00 €')
    await expect(pay).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)')
    // Addition stays a secondary button.
    await expect(page.locator('.pre-bill-btn')).toBeVisible()
  })

  test('the pill says "1 ligne" for one line', async ({ page }) => {
    await openTable(page, [SENT, PENDING(2)])
    await expect(page.getByTestId('send-pill')).toHaveText('1 ligne')
  })

  test('sending gives the primary back to PAYER', async ({ page }) => {
    await openTable(page, [SENT, PENDING(2)])
    await page.getByTestId('send-primary').click()
    expect(await ipcCalls(page, 'leave_order')).toHaveLength(1)
    await expect(page.getByTestId('send-primary')).toHaveCount(0)
    await expect(page.getByTestId('pay-secondary')).toHaveCount(0)
    const pay = page.locator('.checkout-btn')
    await expect(pay).toHaveCount(1)
    await expect(pay).toContainText('PAYER')
    await expect(pay).toHaveCSS('background-color', 'rgb(215, 255, 58)')
    await expect(page.locator('.kitchen-btn')).toBeDisabled()
  })

  test('PAYER with unsent lines goes to the checkout at once, no dialog', async ({ page }) => {
    await openTable(page, [PENDING(2)])
    await page.getByTestId('pay-secondary').click()
    await expect(page).toHaveURL(/checkout/u)
    await expect(page.getByRole('dialog')).toHaveCount(0)
  })

  test('lines held back by a course do not count: PAYER stays the primary', async ({ page }) => {
    await openTable(page, [SENT, PENDING(2, { kitchen_state: 'held', course: 2 })])
    await expect(page.getByTestId('send-primary')).toHaveCount(0)
    await expect(page.locator('.checkout-btn')).toHaveCSS('background-color', 'rgb(215, 255, 58)')
  })

  test('an apprentice has Envoyer en cuisine as primary, even with nothing to send', async ({
    page,
  }) => {
    await openTable(page, [SENT], { role: 'waiter' })
    const primary = page.getByTestId('send-primary')
    await expect(primary).toBeVisible()
    await expect(primary).toBeDisabled()
    await expect(page.getByTestId('send-pill')).toHaveCount(0)
    await expect(page.locator('.checkout-btn')).toHaveCount(0)
    await expect(page.locator('.pre-bill-btn')).toHaveCount(0)
  })

  test('an apprentice with lines to send sees the pill and can send', async ({ page }) => {
    await openTable(page, [PENDING(2), PENDING(3)], { role: 'waiter' })
    await expect(page.getByTestId('send-pill')).toHaveText('2 lignes')
    await page.getByTestId('send-primary').click()
    expect(await ipcCalls(page, 'leave_order')).toHaveLength(1)
  })

  test('a takeaway keeps PAYER as its primary, no kitchen button', async ({ page }) => {
    await openTable(page, [PENDING(2)], { orderType: 'takeaway' })
    await expect(page.getByTestId('send-primary')).toHaveCount(0)
    await expect(page.locator('.kitchen-btn')).toHaveCount(0)
    await expect(page.locator('.checkout-btn')).toHaveCSS('background-color', 'rgb(215, 255, 58)')
  })
})

test.describe('C3 the Plus menu', () => {
  test('a manager finds Déplacer, Remise and Client parti behind Plus', async ({ page }) => {
    await openTable(page, [SENT])
    // Gone from the footer.
    await expect(page.locator('.cart-footer').getByText('Client parti sans payer')).toHaveCount(0)
    await expect(page.locator('.cart-footer').getByText('Remise')).toHaveCount(0)
    const more = page.getByTestId('cart-more-open')
    await expect(more).toContainText('Plus')
    const box = await more.boundingBox()
    expect(box?.height).toBeGreaterThanOrEqual(48)
    await openMore(page)
    const menu = page.getByTestId('cart-more')
    await expect(menu.getByTestId('transfer-table')).toHaveText('Déplacer')
    await expect(menu.getByTestId('discount-open')).toHaveText('Remise')
    await expect(menu.getByTestId('unpaid-close')).toHaveText('Client parti sans payer')
  })

  test('the manager PIN and its hint stay on Client parti', async ({ page }) => {
    await openTable(page, [SENT])
    await openMore(page)
    await page.getByTestId('unpaid-close').click()
    await expect(page.getByTestId('cart-more')).toHaveCount(0)
    await expect(page.locator('.pin-dialog .hint')).toContainText('sans paiement')
  })

  test('a server only gets Déplacer, like before', async ({ page }) => {
    await openTable(page, [SENT], { role: 'cashier' })
    await expectMoreRow(page, 'transfer-table')
    await expectNoMoreRow(page, 'discount-open')
    await expectNoMoreRow(page, 'unpaid-close')
  })

  test('an apprentice has no Plus button when no action applies', async ({ page }) => {
    // An apprentice may move a table too, so look at an empty order: no action at all.
    await openTable(page, [], { role: 'waiter' })
    await expect(page.getByTestId('cart-more-open')).toHaveCount(0)
  })

  test('Fermer closes the menu without doing anything', async ({ page }) => {
    await openTable(page, [SENT])
    await openMore(page)
    await page.getByTestId('cart-more-close').click()
    await expect(page.getByTestId('cart-more')).toHaveCount(0)
    expect(await ipcCalls(page, 'close_unpaid')).toHaveLength(0)
  })
})
