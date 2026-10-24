import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  overrideIPC,
  loginAndGoto,
  ipcCalls,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
} from './fixtures/mock-ipc'

// Digital ticket delivery (paper / email / both): the medium picker on
// checkout/refund/copy, the on-screen email keypad, the "done" screen
// outcome, the needsPaper cert-fix print action, and the Settings
// "Tickets numériques" panel.

type Handlers = Parameters<typeof mockTauriIPC>[1]

const SAMPLE_TICKET = {
  restaurant_name: 'The FPS Finances Experience',
  restaurant_address: 'Boulevard Albert II 33, 1030 Brussels',
  restaurant_vat: 'BE0000000097',
  pos_id: 'CPOS0031234567',
  terminal_id: 'POS-1-BAR',
  pos_datetime: new Date().toISOString(),
  ticket_number: 1,
  event_label: 'N',
  is_training: false,
  lines: [],
  subtotal: 12,
  total: 12,
  vat_breakdown: [],
  payments: [],
  rounding: null,
  fdm_ref: null,
  short_signature: null,
  verification_url: null,
  footer: [],
  ticket_type_label: 'TICKET',
  is_refund: false,
  lang: 'fr',
}

async function goToCheckoutMethodStep(page: Page, overrides: Handlers = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [DEFAULT_ORDER_LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...overrides,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
  await page.locator('.item-btn').first().click()
  await page.locator('.checkout-btn').click()
  await page.waitForURL(/checkout/u)
}

async function payExact(page: Page) {
  await page.locator('.cash-btn').click()
  await page.locator('.shortcut.exact').click()
  await page.locator('.validate-btn').click()
  await expect(page.locator('.success-icon')).toBeVisible()
}

async function typeAndConfirmEmail(page: Page, local: string, domain: string) {
  for (const ch of local) {
    await page.locator('.key', { hasText: ch }).click()
  }
  await page.locator('.quick-key', { hasText: domain }).click()
  await page.locator('.act-btn.primary').click()
  await page.getByRole('button', { name: "Confirmer l'adresse" }).click()
}

// ── Picker visibility ────────────────────────────────────────────

test.describe('Ticket medium picker - visibility', () => {
  test('hidden when the terminal has no mail setup; checkout sends no delivery', async ({
    page,
  }) => {
    await goToCheckoutMethodStep(page)
    await expect(page.locator('.medium-picker')).toHaveCount(0)

    await payExact(page)

    const calls = await ipcCalls(page, 'checkout')
    expect(calls).toHaveLength(1)
    expect(calls[0].delivery).toBeUndefined()
  })

  test('visible when configured, defaulting to Papier', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await expect(page.locator('.medium-picker')).toBeVisible()
    await expect(page.locator('.medium-btn.active')).toContainText('Papier')
    await expect(page.locator('.medium-btn', { hasText: 'Les deux' })).toBeVisible()
  })

  test('"E-mail" (digital only) hidden when not allowed', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await expect(page.locator('.medium-btn', { hasText: 'E-mail' })).toHaveCount(0)
  })

  test('"E-mail" (digital only) hidden when allowed but unreachable', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: false, allowDigitalOnly: true }),
    })
    await expect(page.locator('.medium-btn', { hasText: 'E-mail' })).toHaveCount(0)
  })

  test('"E-mail" (digital only) shown when allowed and reachable', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: true }),
    })
    await expect(page.locator('.medium-btn', { hasText: 'E-mail' })).toBeVisible()
  })

  test('shows an unreachable note; hides it once reachable', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: false, allowDigitalOnly: true }),
    })
    await expect(page.locator('.medium-note')).toBeVisible()
  })

  test('no unreachable note when the mail service answers', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: true }),
    })
    await expect(page.locator('.medium-note')).toHaveCount(0)
  })
})

// ── Email keypad ─────────────────────────────────────────────────

test.describe('Email keypad', () => {
  test.beforeEach(async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await page.locator('.medium-btn', { hasText: 'Les deux' }).click()
    await expect(page.locator('.keypad-overlay')).toBeVisible()
  })

  test('typing letters and digits builds the address', async ({ page }) => {
    for (const ch of 'ab12') await page.locator('.key', { hasText: ch }).click()
    await expect(page.locator('.email-display')).toContainText('ab12')
  })

  test('backspace removes the last character', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await page.locator('.key', { hasText: 'b' }).click()
    await page.locator('.key-backspace').click()
    await expect(page.locator('.email-display')).toContainText('a')
    await expect(page.locator('.email-display')).not.toContainText('ab')
  })

  test('clear empties the buffer', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await page.locator('.act-btn.secondary', { hasText: 'Effacer' }).click()
    await expect(page.locator('.email-placeholder')).toBeVisible()
  })

  test('quick keys append common domain chunks', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await page.locator('.quick-key', { hasText: '@gmail.com' }).click()
    await expect(page.locator('.email-display')).toContainText('a@gmail.com')
  })

  test('an incomplete address blocks the confirm step', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await expect(page.locator('.email-error')).toBeVisible()
    await expect(page.locator('.act-btn.primary')).toBeDisabled()
  })

  test('a complete address enables the confirm step', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await page.locator('.quick-key', { hasText: '@gmail.com' }).click()
    await expect(page.locator('.email-error')).toHaveCount(0)
    await expect(page.locator('.act-btn.primary')).toBeEnabled()
  })

  test('confirmation screen shows the address in large type', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await page.locator('.quick-key', { hasText: '@gmail.com' }).click()
    await page.locator('.act-btn.primary').click()
    await expect(page.getByRole('heading', { name: "Confirmer l'adresse" })).toBeVisible()
    await expect(page.locator('.email-confirm-display')).toHaveText('a@gmail.com')
  })

  test('"Modifier" returns to typing without losing the address', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await page.locator('.quick-key', { hasText: '@gmail.com' }).click()
    await page.locator('.act-btn.primary').click()
    await page.locator('.act-btn.secondary', { hasText: 'Modifier' }).click()
    await expect(page.locator('.email-display')).toContainText('a@gmail.com')
  })

  test('cancelling the keypad returns the picker to Papier', async ({ page }) => {
    await page.locator('.act-btn.secondary', { hasText: 'Annuler' }).click()
    await expect(page.locator('.keypad-overlay')).toHaveCount(0)
    await expect(page.locator('.medium-btn.active')).toContainText('Papier')
  })

  test('cancelling from the confirmation screen also returns to Papier', async ({ page }) => {
    await page.locator('.key', { hasText: 'a' }).click()
    await page.locator('.quick-key', { hasText: '@gmail.com' }).click()
    await page.locator('.act-btn.primary').click()
    // Tapping outside the modal cancels (click.self on the overlay).
    await page.locator('.keypad-overlay').click({ position: { x: 5, y: 5 } })
    await expect(page.locator('.medium-btn.active')).toContainText('Papier')
  })
})

// ── Checkout sends the chosen delivery ───────────────────────────

test.describe('Checkout request', () => {
  test('sends { medium: "paper_digital", email } for "Les deux"', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await page.locator('.medium-btn', { hasText: 'Les deux' }).click()
    await typeAndConfirmEmail(page, 'a', '@gmail.com')
    await payExact(page)

    const calls = await ipcCalls(page, 'checkout')
    expect(calls[0].delivery).toEqual({ medium: 'paper_digital', email: 'a@gmail.com' })
  })

  test('sends { medium: "digital", email } for "E-mail"', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: true }),
    })
    await page.locator('.medium-btn', { hasText: 'E-mail' }).click()
    await typeAndConfirmEmail(page, 'a', '@gmail.com')
    await payExact(page)

    const calls = await ipcCalls(page, 'checkout')
    expect(calls[0].delivery).toEqual({ medium: 'digital', email: 'a@gmail.com' })
  })

  test('the request resets to Papier after a completed checkout', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await page.locator('.medium-btn', { hasText: 'Les deux' }).click()
    await typeAndConfirmEmail(page, 'a', '@gmail.com')
    await payExact(page)
    await page.locator('.done-actions .primary').click()
    // finish() routes to "order", but OrderView immediately bounces to
    // "tables" (no active order left - it was cleared on success), so
    // "tables" is the only settled URL worth waiting for here.
    await page.waitForURL(/tables/u)

    // Re-enter the checkout screen fresh: the store must be back at Papier.
    await page.evaluate(() => {
      window.location.hash = '#/checkout'
    })
    await page.waitForTimeout(300)
    await expect(page.locator('.medium-btn.active')).toContainText('Papier')
  })
})

// ── Done screen outcome ───────────────────────────────────────────

test.describe('Done screen - delivery outcome', () => {
  test('shows "Ticket imprimé" for a paper outcome', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      checkout: () => ({
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 1,
        delivery: {
          fiscalEventId: 1,
          deliveryId: null,
          medium: 'PAPER',
          email: 'not_requested',
          recipientHint: null,
          paperPrinted: true,
          needsPaper: false,
        },
      }),
    })
    await payExact(page)
    await expect(page.locator('.delivery-msg')).toContainText('Ticket imprimé')
  })

  test('shows the masked address for a sent outcome', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      checkout: () => ({
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 1,
        delivery: {
          fiscalEventId: 1,
          deliveryId: 5,
          medium: 'DIGITAL',
          email: 'sent',
          recipientHint: 'j***@e***.be',
          paperPrinted: false,
          needsPaper: false,
        },
      }),
    })
    await payExact(page)
    await expect(page.locator('.delivery-msg')).toContainText('j***@e***.be')
  })

  test('shows a queued message when the email is still pending', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      checkout: () => ({
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 1,
        delivery: {
          fiscalEventId: 1,
          deliveryId: 5,
          medium: 'PAPER_DIGITAL',
          email: 'queued',
          recipientHint: 'j***@e***.be',
          paperPrinted: true,
          needsPaper: false,
        },
      }),
    })
    await payExact(page)
    await expect(page.locator('.delivery-msg')).toContainText('nouvel essai automatique')
  })

  test('the old "reprint original" button no longer exists', async ({ page }) => {
    await goToCheckoutMethodStep(page)
    await payExact(page)
    await expect(page.getByRole('button', { name: 'Réimprimer' })).toHaveCount(0)
    await expect(page.locator('.done-actions button')).toHaveCount(1)
  })
})

// ── needsPaper: the one print action left on the done screen ────

test.describe('needsPaper - print the missing original', () => {
  function checkoutNeedingPaper() {
    return () => ({
      success: true,
      ticket: SAMPLE_TICKET,
      error: null,
      fdm_connected: true,
      fiscal_event_id: 7,
      delivery: {
        fiscalEventId: 7,
        deliveryId: null,
        medium: 'PAPER',
        email: 'not_requested',
        recipientHint: null,
        paperPrinted: false,
        needsPaper: true,
      },
    })
  }

  test('shows "Imprimer le ticket", calls print_missing_original, hides on success', async ({
    page,
  }) => {
    await goToCheckoutMethodStep(page, {
      checkout: checkoutNeedingPaper(),
      print_missing_original: () => undefined,
    })
    await payExact(page)

    const printBtn = page.getByRole('button', { name: 'Imprimer le ticket' })
    await expect(printBtn).toBeVisible()
    await printBtn.click()
    await expect(printBtn).toHaveCount(0)

    const calls = await ipcCalls(page, 'print_missing_original')
    expect(calls).toHaveLength(1)
    expect(calls[0].fiscalEventId).toBe(7)
    expect(calls[0].ticket).toBeUndefined()
  })

  test('shows an error toast on failure and keeps the button', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      checkout: checkoutNeedingPaper(),
    })
    await payExact(page)
    // A handler that always throws is expressed as the '__THROW__'
    // sentinel via overrideIPC - a plain throwing function would only
    // throw once, at mock setup time (see mock-ipc.ts's `live()` doc).
    await overrideIPC(page, 'print_missing_original', '__THROW__')

    const printBtn = page.getByRole('button', { name: 'Imprimer le ticket' })
    await printBtn.click()
    await expect(page.locator('.toast-error')).toBeVisible()
    await expect(printBtn).toBeVisible()
  })

  test('the button is absent when the ticket does not need paper', async ({ page }) => {
    await goToCheckoutMethodStep(page, {
      checkout: () => ({
        success: true,
        ticket: SAMPLE_TICKET,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 8,
        delivery: {
          fiscalEventId: 8,
          deliveryId: null,
          medium: 'PAPER',
          email: 'not_requested',
          recipientHint: null,
          paperPrinted: true,
          needsPaper: false,
        },
      }),
    })
    await payExact(page)
    await expect(page.getByRole('button', { name: 'Imprimer le ticket' })).toHaveCount(0)
  })
})

// ── Refund flow ────────────────────────────────────────────────────

test.describe('Refund flow - ticket delivery', () => {
  const SAMPLE_REFUNDABLE = [
    {
      fiscalEventId: 10,
      ticketNumber: 42,
      orderId: 5,
      tableNumber: 3,
      totalCents: 1200,
      createdAt: new Date().toISOString(),
    },
  ]

  async function openRefundItems(page: Page, overrides: Handlers = {}) {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_refundable_events: () => SAMPLE_REFUNDABLE,
      list_cart_lines: () => [DEFAULT_ORDER_LINE],
      ...overrides,
    })
    await loginAndGoto(page, '/reports')
    await page.getByRole('tab', { name: 'Tickets' }).click()
    await page.locator('.ticket-row', { hasText: '#42' }).locator('.refund-btn').click()
    await page.locator('.refund-line').first().click()
  }

  test('offers the picker before confirming, defaulting to Papier', async ({ page }) => {
    await openRefundItems(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await page.locator('.refund-confirm-btn').click()
    await expect(page.locator('.medium-picker')).toBeVisible()
    await expect(page.locator('.medium-btn.active')).toContainText('Papier')
  })

  test('sends the chosen delivery with refund_checkout', async ({ page }) => {
    await openRefundItems(page, {
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await page.locator('.refund-confirm-btn').click() // -> delivery step
    await page.locator('.medium-btn', { hasText: 'Les deux' }).click()
    await typeAndConfirmEmail(page, 'a', '@gmail.com')
    await page.locator('.refund-confirm-btn').click() // -> actually refunds

    const calls = await ipcCalls(page, 'refund_checkout')
    expect(calls).toHaveLength(1)
    expect(calls[0].delivery).toEqual({ medium: 'paper_digital', email: 'a@gmail.com' })
  })

  test('shows the delivery outcome after a successful refund', async ({ page }) => {
    await openRefundItems(page, {
      refund_checkout: () => ({
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 20,
        delivery: {
          fiscalEventId: 20,
          deliveryId: null,
          medium: 'PAPER',
          email: 'not_requested',
          recipientHint: null,
          paperPrinted: true,
          needsPaper: false,
        },
      }),
    })
    await page.locator('.refund-confirm-btn').click() // -> delivery step
    await page.locator('.refund-confirm-btn').click() // -> refund (Papier default)
    await expect(page.locator('.delivery-msg')).toContainText('Ticket imprimé')
  })
})

// ── Copy flow ──────────────────────────────────────────────────────

test.describe('Copy flow - ticket delivery', () => {
  const SAMPLE_Z = [
    {
      id: 1,
      reportDate: '2026-04-11',
      sequenceNumber: 1,
      createdAt: '2026-04-11T22:15:00Z',
      fiscalEventId: 77,
    },
  ]

  test('offers the picker before signing', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_z_reports: () => SAMPLE_Z,
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await loginAndGoto(page, '/reports')
    await page.locator('.z-history-btn').click()
    await page.locator('.reprint-btn').click()
    await expect(page.locator('.medium-picker')).toBeVisible()
    await expect(page.locator('.medium-btn.active')).toContainText('Papier')
  })

  test('sends the chosen delivery with copy_ticket', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_z_reports: () => SAMPLE_Z,
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await loginAndGoto(page, '/reports')
    await page.locator('.z-history-btn').click()
    await page.locator('.reprint-btn').click()
    await page.locator('.medium-btn', { hasText: 'Les deux' }).click()
    await typeAndConfirmEmail(page, 'a', '@gmail.com')
    await page.getByRole('button', { name: 'Valider' }).click()

    const calls = await ipcCalls(page, 'copy_ticket')
    expect(calls).toHaveLength(1)
    expect(calls[0].fiscalEventId).toBe(77)
    expect(calls[0].delivery).toEqual({ medium: 'paper_digital', email: 'a@gmail.com' })
  })

  test('shows the delivery outcome from CopyResult.delivery', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_z_reports: () => SAMPLE_Z,
      copy_ticket: () => ({
        success: true,
        error: null,
        ticket: null,
        fiscalEventId: 77,
        delivery: {
          fiscalEventId: 77,
          deliveryId: 3,
          medium: 'DIGITAL',
          email: 'sent',
          recipientHint: 'a***@b***.be',
          paperPrinted: false,
          needsPaper: false,
        },
      }),
    })
    await loginAndGoto(page, '/reports')
    await page.locator('.z-history-btn').click()
    await page.locator('.reprint-btn').click()
    await page.getByRole('button', { name: 'Valider' }).click()
    await expect(page.locator('.delivery-msg')).toContainText('a***@b***.be')
  })
})

test.describe('Ticket copy (event C) and refund - one list of tickets', () => {
  const TICKETS = [
    {
      fiscalEventId: 41,
      eventLabel: 'T',
      ticketNumber: 9,
      tableNumber: null,
      totalCents: 300,
      createdAt: '2026-09-25 12:30:00',
    },
    {
      fiscalEventId: 40,
      eventLabel: 'N',
      ticketNumber: 8,
      tableNumber: null,
      totalCents: -1200,
      createdAt: '2026-09-25 12:20:00',
    },
    {
      fiscalEventId: 39,
      eventLabel: 'P',
      ticketNumber: 7,
      tableNumber: 4,
      totalCents: 2800,
      createdAt: '2026-09-25 12:10:00',
    },
    {
      fiscalEventId: 38,
      eventLabel: 'N',
      ticketNumber: 6,
      tableNumber: 4,
      totalCents: 5200,
      createdAt: '2026-09-25 12:00:00',
    },
  ]
  // Only the sale #6 can be refunded.
  const REFUNDABLE = [
    {
      fiscalEventId: 38,
      ticketNumber: 6,
      orderId: 5,
      tableNumber: 4,
      totalCents: 5200,
      createdAt: '2026-09-25 12:00:00',
    },
  ]

  async function openTicketsTab(page: Page, overrides: Handlers = {}) {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_copyable_tickets: () => TICKETS,
      list_refundable_events: () => REFUNDABLE,
      ...overrides,
    })
    await loginAndGoto(page, '/reports')
    await page.getByRole('tab', { name: 'Tickets' }).click()
  }

  test('lists each ticket with its kind, table and own total', async ({ page }) => {
    await openTicketsTab(page)
    const rows = page.locator('.ticket-row')
    await expect(rows).toHaveCount(4)
    await expect(rows.locator('.rt-kind')).toHaveText([
      'Formation',
      'Remboursement',
      'Addition',
      'Vente',
    ])
    await expect(rows.nth(2)).toContainText('#7')
    await expect(rows.nth(2)).toContainText('T4')
    await expect(rows.nth(1).locator('.rt-total')).toContainText('-12,00')
    // SQLite UTC 12:00 is 14:00 in Brussels (summer time).
    await expect(rows.nth(3).locator('.rt-time')).toHaveText('14:00')
  })

  test('every ticket can be copied, only the refundable sale refunded', async ({ page }) => {
    await openTicketsTab(page)
    await expect(page.locator('.copy-ticket-btn')).toHaveCount(4)
    await expect(page.locator('.ticket-row .refund-btn')).toHaveCount(1)
    await expect(
      page.locator('.ticket-row', { hasText: '#6' }).locator('.refund-btn'),
    ).toBeVisible()
  })

  test('copies the chosen ticket, on paper by default', async ({ page }) => {
    await openTicketsTab(page)
    await page.locator('.copy-ticket-btn').nth(3).click()
    await expect(page.locator('.refund-modal h3')).toHaveText('Copie de ticket')
    await page.getByRole('button', { name: 'Valider' }).click()
    await expect.poll(() => ipcCalls(page, 'copy_ticket')).toHaveLength(1)
    const calls = await ipcCalls(page, 'copy_ticket')
    expect(calls[0].fiscalEventId).toBe(38)
    expect(calls[0].employeeInsz).toBe(DEFAULT_USER.insz)
    expect(calls[0].delivery).toBeUndefined()
  })

  test('back from the copy picker signs nothing', async ({ page }) => {
    await openTicketsTab(page)
    await page.locator('.copy-ticket-btn').first().click()
    await page.locator('.refund-modal').getByRole('button', { name: 'Retour' }).click()
    await expect(page.locator('.refund-modal')).toHaveCount(0)
    expect(await ipcCalls(page, 'copy_ticket')).toHaveLength(0)
  })

  test('back from the refund items signs nothing', async ({ page }) => {
    await openTicketsTab(page)
    await page.locator('.ticket-row .refund-btn').click()
    await page.locator('.refund-modal').getByRole('button', { name: 'Retour' }).click()
    await expect(page.locator('.refund-modal')).toHaveCount(0)
    expect(await ipcCalls(page, 'refund_checkout')).toHaveLength(0)
  })

  test('an empty list says so', async ({ page }) => {
    await openTicketsTab(page, {
      list_copyable_tickets: () => [],
      list_refundable_events: () => [],
    })
    await expect(page.locator('.tickets-empty')).toHaveText('Aucun ticket récent')
  })

  test('a refused listing is shown, and no copy is offered', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] })
    await loginAndGoto(page, '/reports')
    await overrideIPC(page, 'list_copyable_tickets', { __reject: 'forbidden' })
    await page.getByRole('tab', { name: 'Tickets' }).click()
    await expect(
      page.locator('.toast', { hasText: 'Action réservée à un autre profil.' }),
    ).toBeVisible()
    await expect(page.locator('.copy-ticket-btn')).toHaveCount(0)
    await expect(page.locator('.refund-modal')).toHaveCount(0)
  })

  test('the Z history copy keeps its own title', async ({ page }) => {
    await openTicketsTab(page, {
      list_z_reports: () => [
        {
          id: 1,
          reportDate: '2026-09-24',
          sequenceNumber: 1,
          createdAt: '2026-09-24T22:15:00Z',
          fiscalEventId: 77,
        },
      ],
    })
    await page.locator('.copy-ticket-btn').first().click()
    await page.locator('.refund-modal').getByRole('button', { name: 'Retour' }).click()
    await page.getByRole('tab', { name: 'Journée' }).click()
    await page.locator('.z-history-btn').click()
    await page.locator('.reprint-btn').click()
    await expect(page.locator('.refund-modal h3')).toHaveText('Réimprimer')
  })
})

// ── Settings - digital tickets panel ─────────────────────────────

test.describe('Settings - digital tickets panel', () => {
  const SAMPLE_DELIVERIES = [
    {
      id: 1,
      fiscalEventId: 10,
      medium: 'DIGITAL',
      recipientHint: 'j***@e***.be',
      status: 'sent',
      attempts: 1,
      lastError: null,
      fallbackPrinted: false,
      createdAt: '2026-04-11T22:15:00Z',
    },
    {
      id: 2,
      fiscalEventId: 11,
      medium: 'PAPER_DIGITAL',
      recipientHint: 'a***@b***.com',
      status: 'failed',
      attempts: 3,
      lastError: 'timeout',
      fallbackPrinted: true,
      createdAt: '2026-04-11T20:00:00Z',
    },
  ]

  test('lists deliveries with masked hints and statuses only', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: true }),
      list_ticket_deliveries: () => SAMPLE_DELIVERIES,
    })
    await page.goto('/#/settings')
    await expect(page.locator('.delivery-row')).toHaveCount(2)
    await expect(page.locator('.delivery-hint').first()).toHaveText('j***@e***.be')
    await expect(page.locator('.delivery-status-badge').first()).toContainText('Envoyé')
    await expect(page.locator('.delivery-status-badge').last()).toContainText('Échec')
    await expect(page.getByText('timeout')).toBeVisible()
  })

  test('shows mail configured/reachable status', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      check_mail_status: () => ({ configured: true, reachable: true, allowDigitalOnly: false }),
    })
    await page.goto('/#/settings')
    await expect(page.locator('.mail-status-pill.mail-status-ok')).toHaveCount(2)
  })

  test('shows "not configured" when the terminal has no mail setup', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    })
    await page.goto('/#/settings')
    await expect(page.locator('.mail-status-pill.mail-status-off')).toBeVisible()
  })

  test('refresh button reloads the delivery list', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_ticket_deliveries: () => SAMPLE_DELIVERIES,
    })
    await page.goto('/#/settings')
    await page.locator('.refresh-btn').click()
    expect(await ipcCalls(page, 'list_ticket_deliveries')).toHaveLength(2)
  })

  test('empty state when there are no deliveries', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    })
    await page.goto('/#/settings')
    await expect(page.locator('.deliveries-section .empty')).toBeVisible()
  })
})
