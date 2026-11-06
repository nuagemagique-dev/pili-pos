import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  loginAndGoto,
  tapUserTile,
  emitTauriEvent,
  ipcCalls,
  live,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// UX audit O2 (an unpaid online order is collected at the checkout, with change), O3 (a banner
// on every staff screen and a repeating chime for online orders nobody answered) and P4 (going
// back to the order from the checkout keeps the parts taken by hand).

const SALE = {
  success: true,
  ticket: null,
  error: null,
  fdm_connected: true,
  fiscal_event_id: 2,
  delivery: null,
  kitchen_slip: 'nothing_to_print',
}

const sqlite = (d: Date) => d.toISOString().slice(0, 19).replace('T', ' ')

/** An online order, unpaid and ready to be collected (its P is signed). */
function unpaidOrder(id: number, over: Record<string, unknown> = {}) {
  return {
    id,
    hubrise_order_id: `hr-${id}`,
    status: 'open',
    hubrise_status: 'awaiting_collection',
    source: 'Website',
    service_type: 'collection',
    total_cents: 2850,
    created_at: sqlite(new Date(Date.now() - 20 * 60_000)),
    items: [
      {
        product_name: 'Maki saumon x8',
        quantity: 2,
        unit_price_cents: 1200,
        line_total_cents: 2400,
        options: [],
        customer_notes: null,
      },
    ],
    customer_notes: null,
    customer_name: 'Alice Martin',
    confirmed_time: null,
    customer_address_1: null,
    customer_address_2: null,
    customer_postal_code: null,
    customer_city: null,
    webshop: {
      order_id: id,
      hubrise_order_id: `hr-${id}`,
      channel: 'website',
      channel_kind: 'webshop',
      service_type: 'collection',
      payment_mode: 'unpaid',
      payment_type: null,
      fiscal_state: 'signed_p',
      resolution: 'ok',
      attention: [],
      warnings: [],
    },
    ...over,
  }
}

const ONLINE_ORDER = {
  ...DEFAULT_ORDER,
  id: 4,
  order_type: 'takeaway',
  table_number: null,
  subtotal_cents: 2850,
  total_cents: 2850,
}
const ONLINE_LINE = {
  ...DEFAULT_ORDER_LINE,
  id: 41,
  order_id: 4,
  quantity: 1,
  unit_price_cents: 2850,
  vat_code: 'C',
  name: 'Maki saumon x8',
  name_fr: 'Maki saumon x8',
  name_zh: null,
  unsent_quantity: 0,
  discount_cents: 0,
  price_changes: [],
  kitchen_note: null,
  online: true,
  course: 0,
  kitchen_state: 'printed',
}

/** The board over a small in-page backend, as in webshop-orders.spec.ts, plus the order the checkout reads. */
async function board(
  page: Page,
  orders: unknown[],
  extra: Record<string, (args?: any) => any> = {},
  role = 'admin',
) {
  const user = { ...DEFAULT_USER, role }
  await page.addInitScript((o) => {
    ;(window as any).__webshop = o
  }, orders)
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ONLINE_ORDER,
    list_cart_lines: () => [ONLINE_LINE],
    hubrise_list_webshop_orders: live(() => JSON.parse(JSON.stringify((window as any).__webshop))),
    webshop_handover_order: live(({ req }: any) => {
      const o = (window as any).__webshop.find((x: any) => x.id === req.order_id)
      o.webshop.fiscal_state = 'signed_n'
      o.hubrise_status = 'completed'
      return {
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 2,
        delivery: null,
      }
    }),
    checkout: () => SALE,
    ...extra,
  })
  await loginAndGoto(page, '/webshop-orders')
}

test.describe('O2: collecting an unpaid online order at the checkout', () => {
  test('Encaisser espèces opens the checkout on Exact, with the change, and sells through the handover', async ({
    page,
  }) => {
    await board(page, [unpaidOrder(4)])
    const card = page.locator('.column-ready .card')
    await card.locator('.btn-primary').click()

    // Nothing is taken silently: the checkout shows the total, Exact already typed.
    await expect(page).toHaveURL(/checkout/u)
    await expect(page.locator('.cash-btn')).toHaveClass(/active/u)
    await expect(page.locator('.entry-value')).toHaveText('28,50')
    await expect(page.getByTestId('header-online')).toBeVisible()
    expect(await ipcCalls(page, 'webshop_handover_order')).toHaveLength(0)

    // 30,00 handed over: 1,50 of change.
    for (const d of '30') await page.locator('.nk', { hasText: d }).first().click()
    await expect(page.locator('.change-value')).toContainText('1,50')
    await page.locator('.validate-btn').click()

    await expect(page.getByTestId('done-change')).toContainText('1,50')
    const [call] = await ipcCalls(page, 'webshop_handover_order')
    expect(call.req).toEqual({ order_id: 4, payments: [{ method: 'cash' }] })
    // The handover is the sale: no plain checkout, and the order was never the engine's active one.
    expect(await ipcCalls(page, 'checkout')).toHaveLength(0)
    expect((await ipcCalls(page, 'set_active_order')).filter((c) => c.orderId !== null)).toEqual([])

    // The sale returns to the online orders screen, where the card is gone.
    await page.getByTestId('done-finish').click()
    await expect(page).toHaveURL(/webshop-orders/u)
    await expect(page.locator('.column-ready .card')).toHaveCount(0)
  })

  test('an idle lock at the online checkout does not bring the order back: the same user lands on Tables (T6)', async ({
    page,
  }) => {
    await board(page, [unpaidOrder(4)])
    await page.locator('.column-ready .card .btn-primary').click()
    await expect(page).toHaveURL(/checkout/u)

    await emitTauriEvent(page, 'session-locked', {
      reason: 'idle',
      orderError: null,
      idleAfterMs: 300_000,
    })
    await page.waitForURL(/#\/$/u)
    await tapUserTile(page, 'Bruno')

    // Not the editable order screen of an online order, and no order made active again.
    await page.waitForURL(/tables/u)
    expect((await ipcCalls(page, 'set_active_order')).filter((c) => c.orderId !== null)).toEqual([])
  })

  test('Encaisser carte opens the checkout with Carte selected and sells the card for the total', async ({
    page,
  }) => {
    await board(page, [unpaidOrder(4)])
    await page
      .locator('.column-ready .card')
      .getByRole('button', { name: 'Encaisser carte' })
      .click()

    await expect(page).toHaveURL(/checkout/u)
    await expect(page.locator('.card-btn')).toHaveClass(/active/u)
    expect(await ipcCalls(page, 'webshop_handover_order')).toHaveLength(0)
    await page.locator('.validate-btn').click()

    await expect(page.locator('.success-icon')).toBeVisible()
    const [call] = await ipcCalls(page, 'webshop_handover_order')
    expect(call.req).toEqual({ order_id: 4, payments: [{ method: 'card', amount_cents: 2850 }] })
  })

  test('the checkout of an online order can be split in parts, all carried by the handover', async ({
    page,
  }) => {
    await board(page, [unpaidOrder(4)])
    await page.locator('.column-ready .card .btn-primary').click()
    await page.locator('.meal-btn').click()
    for (const d of '10') await page.locator('.nk', { hasText: d }).first().click()
    await page.locator('.card-confirm').click()
    await expect(page.locator('.total-value')).toHaveText('18,50')
    // The method stays on meal vouchers for the next part: the rest is paid in cash.
    await page.locator('.cash-btn').click()
    await page.locator('.validate-btn').click()

    await expect(page.locator('.success-icon')).toBeVisible()
    const [call] = await ipcCalls(page, 'webshop_handover_order')
    expect(call.req.payments).toEqual([
      { method: 'meal_voucher', amount_cents: 1000 },
      { method: 'cash' },
    ])
  })

  test('Retour à la commande goes back to the board and signs nothing', async ({ page }) => {
    await board(page, [unpaidOrder(4)])
    await page.locator('.column-ready .card .btn-primary').click()
    await expect(page).toHaveURL(/checkout/u)
    await page.getByTestId('back-to-order').click()

    await expect(page).toHaveURL(/webshop-orders/u)
    await expect(page.locator('.column-ready .card')).toHaveCount(1)
    expect(await ipcCalls(page, 'webshop_handover_order')).toHaveLength(0)
    // An online order is not a table: leaving it registers nothing with the engine.
    expect(await ipcCalls(page, 'leave_order')).toHaveLength(0)
  })

  test('a refused sale shows the reason on the checkout and the order stays to collect', async ({
    page,
  }) => {
    await board(
      page,
      [unpaidOrder(4)],
      {
        webshop_handover_order: () => ({
          ...SALE,
          success: false,
          error: 'webshop_already_handled',
        }),
      },
      'admin',
    )
    await page.locator('.column-ready .card .btn-primary').click()
    await page.locator('.validate-btn').click()
    await expect(page.locator('.error-msg')).toBeVisible()
    await expect(page.locator('.success-icon')).toHaveCount(0)
  })

  test('an Apprenti cannot start the collection: both buttons stay disabled', async ({ page }) => {
    await board(page, [unpaidOrder(4)], {}, 'waiter')
    const card = page.locator('.column-ready .card')
    await expect(card.locator('.btn-primary')).toBeDisabled()
    await expect(card.getByRole('button', { name: 'Encaisser carte' })).toBeDisabled()
  })
})

test.describe('O3: late online orders on every staff screen', () => {
  const late = (id: number, minutes: number) =>
    unpaidOrder(id, {
      hubrise_status: 'new',
      created_at: sqlite(new Date(Date.now() - minutes * 60_000)),
      webshop: {
        ...unpaidOrder(id).webshop,
        fiscal_state: 'pending',
        payment_mode: 'prepaid',
      },
    })

  test('a banner under the header of the Tables screen counts them, Voir opens the board', async ({
    page,
  }) => {
    await board(page, [late(10, 7), late(11, 9), late(12, 1)])
    await page.getByRole('button', { name: 'Tables' }).click()
    await expect(page).toHaveURL(/tables/u)

    const banner = page.getByTestId('late-online-banner')
    await expect(banner).toContainText('2 commandes en ligne attendent depuis plus de 5 min')
    await page.getByTestId('late-online-view').click()
    await expect(page).toHaveURL(/webshop-orders/u)
    // The board keeps its own alarm, not the banner.
    await expect(page.getByTestId('late-online-banner')).toHaveCount(0)
    await expect(page.getByTestId('stale-alarm')).toBeVisible()
  })

  test('one late order reads in the singular, and a fresh one raises nothing', async ({ page }) => {
    await board(page, [late(10, 6), late(12, 1)])
    await page.getByRole('button', { name: 'Tables' }).click()
    await expect(page.getByTestId('late-online-banner')).toContainText(
      '1 commande en ligne attend depuis plus de 5 min',
    )
  })

  test('no banner while every order is fresh or answered', async ({ page }) => {
    await board(page, [late(12, 1), unpaidOrder(13, { hubrise_status: 'accepted' })])
    await page.getByRole('button', { name: 'Tables' }).click()
    await expect(page.locator('.table-card').first()).toBeVisible()
    await expect(page.getByTestId('late-online-banner')).toHaveCount(0)
  })

  test('the banner shows on the order screen too, and is gone once the order is answered', async ({
    page,
  }) => {
    await board(page, [late(10, 8)], {
      webshop_accept_order: live(({ req }: any) => {
        const o = (window as any).__webshop.find((x: any) => x.id === req.order_id)
        o.hubrise_status = 'accepted'
        o.webshop.fiscal_state = 'signed_n'
        return { fiscal_state: 'signed_n', fiscal_event_id: 1, sale: null, kitchen_printed: true }
      }),
    })
    await page.getByRole('button', { name: 'À emporter' }).click()
    await expect(page).toHaveURL(/order/u)
    await expect(page.getByTestId('late-online-banner')).toBeVisible()

    // Answer it from the board: the banner leaves every screen.
    await page.getByTestId('late-online-view').click()
    await page.locator('.column-new .card .btn-primary').click()
    await page.getByRole('button', { name: 'Tables' }).click()
    await expect(page.locator('.table-card').first()).toBeVisible()
    await expect(page.getByTestId('late-online-banner')).toHaveCount(0)
  })

  test('the chime plays when an order turns late and again every 2 minutes', async ({ page }) => {
    await page.clock.install({ time: new Date('2026-11-06T12:00:00Z') })
    await page.addInitScript(() => {
      // A fake Web Audio: every chime creates two oscillators (two notes).
      const w = window as unknown as { __notes: number; AudioContext: unknown }
      w.__notes = 0
      w.AudioContext = class {
        currentTime = 0
        destination = {}
        createOscillator() {
          w.__notes++
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
    const notes = () => page.evaluate(() => (window as unknown as { __notes: number }).__notes)
    // Arrived 10 minutes ago: already late when the till reads the board.
    await board(page, [{ ...late(10, 10), created_at: '2026-11-06 11:50:00' }])
    await page.clock.runFor(10_000)
    await expect.poll(notes).toBe(2)

    await page.clock.runFor(60_000)
    expect(await notes()).toBe(2)
    await page.clock.runFor(60_000)
    await expect.poll(notes).toBe(4)
    await page.clock.runFor(120_000)
    await expect.poll(notes).toBe(6)
  })
})

test.describe('P4: back to the order keeps the parts taken by hand', () => {
  const LINES = [
    {
      ...DEFAULT_ORDER_LINE,
      id: 1,
      menu_item_id: 25003,
      unit_price_cents: 1152,
      vat_code: 'B',
      name: 'Dame Blanche',
      name_fr: 'Dame Blanche',
      unsent_quantity: 0,
    },
    {
      ...DEFAULT_ORDER_LINE,
      id: 2,
      menu_item_id: 28007,
      unit_price_cents: 334,
      vat_code: 'B',
      name: 'Tapas Variation',
      name_fr: 'Tapas Variation',
      unsent_quantity: 0,
    },
  ]

  async function toPayment(page: Page) {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      // The same order everywhere, as the engine answers for one table.
      create_order: () => ({ ...DEFAULT_ORDER, total_cents: 1486 }),
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1486 }),
      list_cart_lines: () => LINES,
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
      remove_order_line: () => null,
      checkout: () => SALE,
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.locator('.table-card').first().click()
    await page.waitForURL(/order/u)
    await page.locator('.checkout-btn').click()
    await page.waitForURL(/checkout/u)
  }

  async function takeMealVoucher(page: Page, digits: string) {
    await page.locator('.meal-btn').click()
    for (const d of digits) await page.locator('.nk', { hasText: d }).first().click()
    await page.locator('.card-confirm').click()
  }

  test('"Retour à la commande" is always there, "Annuler les parts" only once a part is taken', async ({
    page,
  }) => {
    await toPayment(page)
    await expect(page.getByTestId('back-to-order')).toHaveText('Retour à la commande')
    await expect(page.getByTestId('parts-clear')).toHaveCount(0)

    await takeMealVoucher(page, '5')
    await expect(page.getByTestId('back-to-order')).toBeVisible()
    await expect(page.getByTestId('parts-clear')).toHaveText('Annuler les parts')
  })

  test('going back and returning shows the same parts, and the cart says how many', async ({
    page,
  }) => {
    await toPayment(page)
    await takeMealVoucher(page, '5')
    await expect(page.locator('.paid-part')).toHaveCount(1)
    // The meal voucher stays selected: the rest is exact until cash is chosen.
    await expect(page.locator('.total-value')).toHaveText('9,86')

    await page.getByTestId('back-to-order').click()
    await expect(page).toHaveURL(/order/u)
    await expect(page.getByTestId('held-parts')).toHaveText('Parts déjà saisies : 1')

    await page.locator('.checkout-btn').click()
    await expect(page).toHaveURL(/checkout/u)
    await expect(page.locator('.paid-part')).toHaveCount(1)
    await expect(page.locator('.paid-part')).toContainText('5,00')
    await expect(page.locator('.total-value')).toHaveText('9,85')
    // Nothing was signed on the way.
    expect(await ipcCalls(page, 'checkout')).toHaveLength(0)

    // The kept parts are the ones sent when the sale is signed.
    await page.locator('.validate-btn').click()
    await expect(page.locator('.success-icon')).toBeVisible()
    expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
      { method: 'meal_voucher', amount_cents: 500 },
      { method: 'cash' },
    ])
  })

  test('a cash part taken before going back is still taken, and cash stays offered', async ({
    page,
  }) => {
    await toPayment(page)
    // Two equal parts: the first one is paid in cash.
    await page.locator('.split-btn', { hasText: '2' }).click()
    await page.locator('.cash-btn').click()
    await page.locator('.validate-btn').click()
    await expect(page.locator('.paid-part')).toHaveCount(1)
    await expect(page.locator('.cash-btn')).toHaveClass(/active/u)

    await page.getByTestId('back-to-order').click()
    await page.locator('.checkout-btn').click()
    await expect(page.locator('.paid-part')).toHaveCount(1)
    await expect(page.locator('.cash-btn')).toBeVisible()
  })

  test('"Annuler les parts" forgets them for good, also on the cart', async ({ page }) => {
    await toPayment(page)
    await takeMealVoucher(page, '5')
    await page.getByTestId('parts-clear').click()
    await expect(page.locator('.paid-part')).toHaveCount(0)
    await expect(page.locator('.total-value')).toHaveText('14,85')

    await page.getByTestId('back-to-order').click()
    await expect(page.getByTestId('held-parts')).toHaveCount(0)
    await page.locator('.checkout-btn').click()
    await expect(page.locator('.paid-part')).toHaveCount(0)
  })

  test('a part that no longer fits after the order shrank is dropped, with a toast', async ({
    page,
  }) => {
    await toPayment(page)
    await takeMealVoucher(page, '5')
    await page.getByTestId('back-to-order').click()

    // Remove the 11,52 line: 3,34 is left to pay, the 5,00 part cannot stand.
    await page.locator('.order-line', { hasText: 'Dame Blanche' }).click()
    await page.getByTestId('line-remove').click()
    await expect(page.locator('.order-line', { hasText: 'Dame Blanche' })).toHaveCount(0)

    await expect(page.getByTestId('held-parts')).toHaveText('Parts déjà saisies : 1')
    await page.locator('.checkout-btn').click()
    await expect(page).toHaveURL(/checkout/u)
    await expect(page.locator('.toast')).toContainText('les parts saisies ont été annulées')
    await expect(page.locator('.paid-part')).toHaveCount(0)
    await expect(page.locator('.total-value')).toHaveText('3,35')
  })
})
