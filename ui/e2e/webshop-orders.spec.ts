import { test, expect, type Page } from '@playwright/test'
import { mockTauriIPC, loginAndGoto, DEFAULT_USER, live, ipcCalls } from './fixtures/mock-ipc'

const SAMPLE_ORDER = {
  id: 1,
  hubrise_order_id: 'abc12345-6789',
  status: 'open',
  hubrise_status: 'new',
  source: 'Deliveroo',
  service_type: 'delivery',
  total_cents: 2850,
  created_at: '2026-04-12 12:00:00',
  items: [
    {
      product_name: 'Maki saumon x8',
      quantity: 2,
      unit_price_cents: 1200,
      line_total_cents: 2400,
      options: [],
      customer_notes: null,
    },
    {
      product_name: 'Miso soup',
      quantity: 1,
      unit_price_cents: 450,
      line_total_cents: 450,
      options: ['Sauce soja'],
      customer_notes: 'Extra hot',
    },
  ],
  customer_notes: 'Ring the bell',
  customer_name: 'Alice Martin',
  confirmed_time: null,
  customer_address_1: null,
  customer_address_2: null,
  customer_postal_code: null,
  customer_city: null,
  webshop: {
    order_id: 1,
    hubrise_order_id: 'abc12345-6789',
    channel: 'deliveroo',
    channel_kind: 'platform',
    service_type: 'delivery',
    payment_mode: 'prepaid',
    payment_type: 'APP' as string | null,
    fiscal_state: 'pending',
    resolution: 'ok',
    attention: [] as { code: string; sku_ref?: string }[],
    warnings: [],
  },
}

type Order = typeof SAMPLE_ORDER

/**
 * The board over a small in-page backend: the fiscal commands move the
 * orders as the Rust commands do, and the list reads them back.
 */
async function board(
  page: Page,
  orders: unknown[],
  role = 'admin',
  extra: Record<string, (args?: any) => any> = {},
) {
  const user = { ...DEFAULT_USER, role }
  await page.addInitScript((o) => {
    ;(window as any).__webshop = o
  }, orders)
  await mockTauriIPC(page, {
    ...extra,
    login: () => user,
    list_users: () => [user],
    print_webshop_kitchen: () => undefined,
    // Fresh JSON on every call, as the real IPC answers.
    hubrise_list_webshop_orders: live(() => JSON.parse(JSON.stringify((window as any).__webshop))),
    webshop_accept_order: live(({ req }: any) => {
      const o = (window as any).__webshop.find((x: any) => x.id === req.order_id)
      const prepaid = o.webshop.payment_mode === 'prepaid'
      o.webshop.fiscal_state = prepaid ? 'signed_n' : 'signed_p'
      o.hubrise_status = 'accepted'
      o.confirmed_time = req.confirmed_time
      return {
        fiscal_state: o.webshop.fiscal_state,
        fiscal_event_id: 1,
        sale: null,
        kitchen_printed: true,
      }
    }),
    webshop_reject_order: live(({ orderId }: any) => {
      const o = (window as any).__webshop.find((x: any) => x.id === orderId)
      o.webshop.fiscal_state = 'rejected'
      o.hubrise_status = 'rejected'
    }),
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
    webshop_complete_order: live(({ orderId }: any) => {
      ;(window as any).__webshop.find((x: any) => x.id === orderId).hubrise_status = 'completed'
    }),
    webshop_refund_order: live(({ req }: any) => {
      ;(window as any).__webshop.find((x: any) => x.id === req.order_id).webshop.fiscal_state =
        'refunded'
      return {
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 3,
        delivery: null,
      }
    }),
    webshop_close_cancelled_order: live(({ orderId }: any) => {
      ;(window as any).__webshop.find((x: any) => x.id === orderId).webshop.fiscal_state = 'closed'
      return {
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 4,
        delivery: null,
      }
    }),
    hubrise_sync_catalog: live(() => {
      for (const o of (window as any).__webshop) {
        if (o.webshop?.resolution === 'needs_review') {
          o.webshop.resolution = 'ok'
          o.webshop.attention = []
        }
      }
      return null
    }),
  })
  await loginAndGoto(page, '/webshop-orders')
}

/** A copy of the sample order with a different fiscal state. */
function variant(
  id: number,
  fields: Partial<Order>,
  webshop: Partial<Order['webshop']> = {},
): Order {
  return {
    ...SAMPLE_ORDER,
    id,
    hubrise_order_id: `hr-${id}`,
    ...fields,
    webshop: { ...SAMPLE_ORDER.webshop, order_id: id, hubrise_order_id: `hr-${id}`, ...webshop },
  }
}

test.describe('Webshop Orders View', () => {
  test.beforeEach(async ({ page }) => {
    await board(page, [SAMPLE_ORDER])
  })

  test('renders order card', async ({ page }) => {
    await expect(page.locator('.card')).toHaveCount(1)
  })

  test('shows channel logo for Deliveroo', async ({ page }) => {
    await expect(page.locator('.channel-logo')).toBeVisible()
  })

  test('shows order items', async ({ page }) => {
    await expect(page.locator('.item-main')).toHaveCount(2)
  })

  test('shows customer name', async ({ page }) => {
    await expect(page.getByText('Alice Martin')).toBeVisible()
  })

  test('shows order notes', async ({ page }) => {
    await expect(page.getByText('Ring the bell')).toBeVisible()
  })

  test('shows total', async ({ page }) => {
    await expect(page.locator('.total-value')).toContainText('28,50')
  })

  test('one column per stage, new order in "Nouvelles"', async ({ page }) => {
    await expect(page.locator('.column')).toHaveCount(3)
    await expect(page.locator('.column-new .card')).toHaveCount(1)
    await expect(page.locator('.column-new .column-count')).toHaveText('1')
    await expect(page.locator('.column-new .key-time')).toContainText('il y a')
  })

  test('Accepter button visible', async ({ page }) => {
    await expect(page.locator('.btn-primary')).toContainText('Accepter')
  })

  test('Accepter moves the card to "En cuisine" with delay chips', async ({ page }) => {
    await page.locator('.btn-primary').click()
    await expect(page.locator('.column-kitchen .card')).toHaveCount(1)
    await expect(page.locator('.column-kitchen .chip-delay')).toHaveCount(3)
    await expect(page.locator('.column-kitchen .btn-primary')).toContainText('Préparer')
  })

  test('pending badge shows count', async ({ page }) => {
    await expect(page.locator('.nav-pending-badge')).toContainText('1')
  })
})

const at = (status: string, extra: Record<string, unknown> = {}) => ({
  ...SAMPLE_ORDER,
  hubrise_status: status,
  ...extra,
  webshop: { ...SAMPLE_ORDER.webshop, fiscal_state: 'signed_n' },
})

test.describe('Webshop Orders View - stages', () => {
  const ORDERS = [
    at('in_preparation', {
      id: 2,
      hubrise_order_id: 'prep-1',
      confirmed_time: '2026-04-12T17:50:00Z',
    }),
    at('awaiting_collection', {
      id: 3,
      hubrise_order_id: 'ready-1',
      source: 'Webshop',
      customer_address_1: 'Rue Léopold 12',
      customer_postal_code: '4000',
      customer_city: 'Liège',
    }),
    at('completed', { id: 4, hubrise_order_id: 'done-1' }),
    at('rejected', { id: 5, hubrise_order_id: 'done-2' }),
  ]

  test.beforeEach(async ({ page }) => {
    await board(page, ORDERS)
  })

  test('orders land in their stage column', async ({ page }) => {
    await expect(page.locator('.column-new .card')).toHaveCount(0)
    await expect(page.locator('.column-kitchen .card')).toHaveCount(1)
    await expect(page.locator('.column-ready .card')).toHaveCount(1)
    await expect(page.locator('.column-kitchen .btn-primary')).toContainText('Prête')
    await expect(page.locator('.column-kitchen .key-time')).toContainText('prêt à 19:50')
  })

  test('own webshop delivery shows address and "En livraison"', async ({ page }) => {
    const card = page.locator('.column-ready .card')
    await expect(card.locator('.webshop-badge')).toBeVisible()
    await expect(card.locator('.delivery-address')).toContainText('Rue Léopold 12, 4000 Liège')
    await expect(card.locator('.btn-primary')).toContainText('En livraison')
    await expect(card.locator('.chip-delay')).toHaveCount(0)
  })

  test('Réimprimer reveals kitchen and customer tickets', async ({ page }) => {
    const card = page.locator('.column-kitchen .card')
    await expect(card.locator('.print-row')).toHaveCount(0)
    await card.locator('.btn-reprint').click()
    await expect(card.locator('.print-row .btn-ghost')).toHaveCount(2)
    await card.locator('.print-row .btn-ghost').first().click()
    await expect(page.getByText("Ticket envoyé à l'imprimante")).toBeVisible()
    await expect(card.locator('.print-row')).toHaveCount(0)
  })

  test('Terminées toggles finished orders in the third column', async ({ page }) => {
    const toggle = page.locator('.done-toggle')
    await expect(toggle).toContainText('2')
    await toggle.click()
    await expect(page.locator('.column-done .card')).toHaveCount(2)
    await expect(page.locator('.column-done .btn-primary')).toHaveCount(0)
    await page.locator('.done-toggle').click()
    await expect(page.locator('.column-ready .card')).toHaveCount(1)
  })
})

test.describe('Webshop Orders View - signing', () => {
  test('Accepter signs the order with a target time, never a bare status', async ({ page }) => {
    await board(page, [SAMPLE_ORDER])
    await expect(page.getByTestId('pay-badge')).toHaveText('Payée')
    await page.locator('.btn-primary').click()
    await expect(page.locator('.column-kitchen .card')).toHaveCount(1)
    const calls = await ipcCalls(page, 'webshop_accept_order')
    expect(calls).toHaveLength(1)
    expect(calls[0].req.order_id).toBe(1)
    expect(Date.parse(calls[0].req.confirmed_time)).toBeGreaterThan(Date.now())
    expect(await ipcCalls(page, 'hubrise_patch_order_status')).toHaveLength(0)
  })

  test('an unpaid order is marked "À encaisser"', async ({ page }) => {
    await board(page, [
      variant(
        2,
        { source: 'Website' },
        { payment_mode: 'unpaid', payment_type: null, channel_kind: 'webshop' },
      ),
    ])
    await expect(page.getByTestId('pay-badge')).toHaveText('À encaisser')
  })

  test('an order to check cannot be accepted until the catalog resolves it', async ({ page }) => {
    await board(page, [
      variant(
        3,
        {},
        { resolution: 'needs_review', attention: [{ code: 'unknown_product', sku_ref: 'X' }] },
      ),
    ])
    const card = page.locator('.column-new .card')
    await expect(card.getByTestId('review')).toContainText('À vérifier')
    await expect(card.getByTestId('review')).toContainText('Produit absent du menu.')
    await expect(card.locator('.btn-primary')).toHaveCount(0)
    await card.getByRole('button', { name: 'Resynchroniser' }).click()
    expect(await ipcCalls(page, 'hubrise_sync_catalog')).toHaveLength(1)
    await expect(card.getByTestId('review')).toHaveCount(0)
    await expect(card.locator('.btn-primary')).toContainText('Accepter')
  })

  test('a refused signature shows the reason and the order stays new', async ({ page }) => {
    await board(page, [SAMPLE_ORDER])
    await page.evaluate(() => {
      const tauri = (window as any).__TAURI_INTERNALS__
      const prev = tauri.invoke
      tauri.invoke = (cmd: string, args?: any) =>
        cmd === 'webshop_accept_order' ? Promise.reject('webshop_needs_review') : prev(cmd, args)
    })
    await page.locator('.btn-primary').click()
    await expect(page.getByText('Commande à vérifier')).toBeVisible()
    await expect(page.locator('.column-new .card')).toHaveCount(1)
  })

  test('Refuser rejects through the fiscal command', async ({ page }) => {
    await board(page, [SAMPLE_ORDER])
    await page.getByRole('button', { name: 'Refuser' }).click()
    expect(await ipcCalls(page, 'webshop_reject_order')).toEqual([{ orderId: 1 }])
    await expect(page.locator('.column-new .card')).toHaveCount(0)
  })

  test('an unpaid order ready for collection is cashed in at the till', async ({ page }) => {
    await board(page, [
      variant(
        4,
        { hubrise_status: 'awaiting_collection', service_type: 'collection', source: 'Website' },
        {
          payment_mode: 'unpaid',
          payment_type: null,
          fiscal_state: 'signed_p',
          channel_kind: 'webshop',
        },
      ),
    ])
    const card = page.locator('.column-ready .card')
    await expect(card.locator('.btn-primary')).toContainText('Encaisser espèces')
    await card.getByRole('button', { name: 'Encaisser carte' }).click()
    const calls = await ipcCalls(page, 'webshop_handover_order')
    expect(calls[0].req).toEqual({
      order_id: 4,
      payments: [{ method: 'card', amount_cents: 2850 }],
    })
    await expect(page.locator('.column-ready .card')).toHaveCount(0)
  })

  test('a prepaid order ready for collection is completed without a sale', async ({ page }) => {
    await board(page, [
      variant(5, { hubrise_status: 'awaiting_collection' }, { fiscal_state: 'signed_n' }),
    ])
    await page.locator('.column-ready .btn-primary').click()
    expect(await ipcCalls(page, 'webshop_complete_order')).toEqual([{ orderId: 5 }])
    expect(await ipcCalls(page, 'webshop_handover_order')).toHaveLength(0)
  })

  test('a manager refunds an order cancelled after its sale', async ({ page }) => {
    await board(page, [
      variant(6, { hubrise_status: 'cancelled' }, { fiscal_state: 'refund_pending' }),
    ])
    const card = page.locator('.column-new .card')
    await expect(card.getByTestId('cancelled-banner')).toContainText('remboursement')
    await card.locator('.btn-primary', { hasText: 'Rembourser' }).click()
    const calls = await ipcCalls(page, 'webshop_refund_order')
    expect(calls[0].req).toEqual({ order_id: 6, employee_insz: DEFAULT_USER.insz })
    await expect(page.locator('.column-new .card')).toHaveCount(0)
  })

  test('a cashier sees the refund waiting for a manager', async ({ page }) => {
    await board(
      page,
      [variant(7, { hubrise_status: 'cancelled' }, { fiscal_state: 'refund_pending' })],
      'cashier',
    )
    const card = page.locator('.column-new .card')
    await expect(card.getByTestId('cancelled-banner')).toContainText('Un manager doit se connecter')
    await expect(card.locator('.btn-primary')).toHaveCount(0)
  })

  test('an unpaid order cancelled after its P is closed', async ({ page }) => {
    await board(page, [
      variant(
        8,
        { hubrise_status: 'cancelled', source: 'Website' },
        {
          payment_mode: 'unpaid',
          payment_type: null,
          fiscal_state: 'cancel_pending',
          channel_kind: 'webshop',
        },
      ),
    ])
    const card = page.locator('.column-new .card')
    await expect(card.getByTestId('cancelled-banner')).toContainText('0,00')
    await card.locator('.btn-primary', { hasText: 'Clôturer' }).click()
    expect(await ipcCalls(page, 'webshop_close_cancelled_order')).toEqual([{ orderId: 8 }])
  })

  test('a change after the sale is shown, the sale stands', async ({ page }) => {
    await board(page, [
      variant(
        9,
        { hubrise_status: 'in_preparation' },
        { fiscal_state: 'signed_n', attention: [{ code: 'changed_after_signing' }] },
      ),
    ])
    await expect(page.getByTestId('changed-banner')).toContainText('la vente signée reste valable')
  })

  test('an order unanswered for 5 minutes raises the alarm, a fresh one does not', async ({
    page,
  }) => {
    const sqlite = (d: Date) => d.toISOString().slice(0, 19).replace('T', ' ')
    await board(page, [
      variant(10, { created_at: sqlite(new Date(Date.now() - 7 * 60_000)) }),
      variant(11, { created_at: sqlite(new Date()) }),
    ])
    await expect(page.getByTestId('stale-alarm')).toContainText('1 commande(s) en ligne attendent')
    await expect(page.locator('.key-time-alarm')).toHaveCount(1)
    await page.locator('.column-new .card').first().locator('.btn-primary').click()
    await expect(page.getByTestId('stale-alarm')).toHaveCount(0)
  })

  test('the pending badge counts cancelled orders waiting for staff', async ({ page }) => {
    await board(page, [
      SAMPLE_ORDER,
      variant(12, { hubrise_status: 'cancelled' }, { fiscal_state: 'refund_pending' }),
      variant(13, { hubrise_status: 'accepted' }, { fiscal_state: 'signed_n' }),
    ])
    await expect(page.locator('.nav-pending-badge')).toContainText('2')
  })
})
