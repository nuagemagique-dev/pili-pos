import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  ipcCalls,
  tapUserTile,
  eftScript,
  settingsWith,
  loginAndGoto,
  live,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  type EftStep,
  type EftKnown,
} from './fixtures/mock-ipc';

// The linked card payment terminal (EFT): a card part is charged on the
// terminal when Valider is pressed, the approved payment becomes a
// locked part, and the sale is signed with its eft_payment_id. The
// engine's rules are in crates/pos-engine/src/commands/eft.rs.

const SALE = {
  success: true,
  ticket: {
    restaurant_name: 'The FPS Finances Experience', restaurant_address: 'Boulevard Albert II 33, 1030 Brussels', restaurant_vat: 'BE0000000097',
    pos_id: 'CPOS0031234567', terminal_id: 'POS-1-BAR', pos_datetime: new Date().toISOString(),
    ticket_number: 1, event_label: 'N', is_training: false, lines: [], subtotal: 12, total: 12,
    vat_breakdown: [], payments: [], rounding: null, fdm_ref: null, short_signature: null,
    verification_url: null, footer: [], ticket_type_label: 'TICKET DE CAISSE T.V.A.', is_refund: false, lang: 'fr',
  },
  error: null, fdm_connected: true, fiscal_event_id: 1, delivery: null, kitchen_slip: 'printed',
};

// The order the till creates for the first table (mock create_order).
const ORDER = 2;

const LINE = { ...DEFAULT_ORDER_LINE, id: 1, unit_price_cents: 1200 };

interface Setup {
  terminal?: string;
  training?: boolean;
  pay?: EftStep[];
  check?: EftStep[];
  known?: EftKnown[];
}

/** The checkout of a 12,00 € order; the terminal as `setup` says. */
async function toCheckout(page: Page, setup: Setup = {}) {
  await eftScript(page, { pay: setup.pay, check: setup.check, known: setup.known });
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => SALE,
    get_settings: settingsWith({
      ...(setup.terminal ? { paymentTerminal: setup.terminal } : {}),
      trainingMode: setup.training ?? false,
    }),
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.checkout-btn').click();
  await page.waitForURL(/checkout/);
}

const overlay = (page: Page) => page.getByTestId('eft-overlay');

async function payByCard(page: Page) {
  await page.locator('.card-btn').click();
  await page.locator('.validate-btn').click();
}

test.describe('card payment on the linked terminal', () => {
  test('an approved terminal payment is a locked part and the sale carries its id', async ({ page }) => {
    await toCheckout(page, { terminal: 'worldline_terminal_api', pay: [{ status: 'approved', brand: 'Bancontact' }] });
    await page.locator('.card-btn').click();
    await expect(page.getByTestId('eft-mode')).toBeVisible();
    await expect(page.locator('.card-hint')).toHaveText('Le montant est envoyé au terminal à la validation');
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    expect(await ipcCalls(page, 'eft_pay')).toEqual([{ orderId: ORDER, amountCents: 1200 }]);
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1200, eft_payment_id: 100 }]);
  });

  test('the overlay blocks the screen while the terminal waits, and Annuler asks the terminal to stop', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      pay: [{ status: 'approved', hold: true }],
    });
    await payByCard(page);
    await expect(overlay(page)).toBeVisible();
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement sur le terminal');
    await expect(overlay(page)).toContainText('12,00 €');
    await expect(overlay(page)).toContainText('Le client présente sa carte sur le terminal');
    await page.getByTestId('eft-cancel').click();
    expect(await ipcCalls(page, 'eft_abort')).toHaveLength(1);
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement annulé');
    await page.getByTestId('eft-ok').click();
    await expect(overlay(page)).toBeHidden();
    // Nothing was signed; the part is still editable.
    expect(await ipcCalls(page, 'checkout')).toHaveLength(0);
    await expect(page.locator('.validate-btn')).toBeEnabled();
    await expect(page.getByTestId('eft-approved')).toHaveCount(0);
  });

  test('the terminal light is asked again when a payment starts and when it ends', async ({ page }) => {
    await toCheckout(page, { terminal: 'worldline_terminal_api', pay: [{ status: 'approved', hold: true }] });
    const asked = async () => (await ipcCalls(page, 'check_payment_terminal')).length;
    const before = await asked();
    await payByCard(page);
    await expect(overlay(page)).toBeVisible();
    await expect.poll(asked).toBeGreaterThan(before);
    const running = await asked();
    await page.getByTestId('eft-cancel').click();
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement annulé');
    await expect.poll(asked).toBeGreaterThan(running);
  });

  test('an abort that came too late ends approved: the sale goes on', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'ccv_payment_api',
      pay: [{ status: 'approved', hold: true, afterAbort: { status: 'approved', brand: 'Visa' } }],
    });
    await payByCard(page);
    await page.getByTestId('eft-cancel').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments[0]).toMatchObject({ method: 'card', eft_payment_id: 100 });
  });

  test('declined: the reason is shown, then manual entry sends the part without a payment id', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      pay: [{ status: 'declined', detail: 'Carte refusée par la banque' }],
    });
    await payByCard(page);
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement refusé');
    await expect(page.getByTestId('eft-detail')).toHaveText('Carte refusée par la banque');
    await page.getByTestId('eft-ok').click();
    await expect(overlay(page)).toBeHidden();
    await expect(page.getByTestId('eft-approved')).toHaveCount(0);
    // The terminal is down: the card is typed by hand.
    await page.getByTestId('eft-mode-manual').click();
    await expect(page.locator('.card-hint')).toHaveText('Encaisser sur le terminal');
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    expect(await ipcCalls(page, 'eft_pay')).toHaveLength(1);
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1200 }]);
  });

  test('a refusal before anything is charged shows the error text', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'ccv_opi',
      pay: [{ status: 'declined', reject: 'eft_unreachable|detail=timeout' }],
    });
    await payByCard(page);
    await expect(page.getByTestId('eft-failure')).toContainText('injoignable');
    await expect(page.getByTestId('eft-failure')).toContainText('timeout');
    await page.getByTestId('eft-ok').click();
    await expect(overlay(page)).toBeHidden();
    expect(await ipcCalls(page, 'checkout')).toHaveLength(0);
  });

  test('unknown answer: Vérifier asks the terminal, then the sale is signed', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      pay: [{ status: 'unknown', detail: 'no answer' }],
      check: [{ status: 'approved', brand: 'Visa' }],
    });
    await payByCard(page);
    await expect(page.getByTestId('eft-title')).toHaveText('Réponse du terminal inconnue');
    await expect(overlay(page)).toContainText('La vente attend la réponse du terminal');
    await page.getByTestId('eft-check').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    expect(await ipcCalls(page, 'eft_check')).toEqual([{ id: 100 }]);
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1200, eft_payment_id: 100 }]);
  });

  test('unknown and closed: the sale is blocked until the payment is checked from the list', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      pay: [{ status: 'unknown' }],
      check: [{ status: 'unknown' }, { status: 'approved' }],
    });
    await payByCard(page);
    await page.getByTestId('eft-ok').click();
    await expect(overlay(page)).toBeHidden();
    await expect(page.getByTestId('eft-open')).toContainText('Réponse inconnue');
    await expect(page.getByTestId('eft-blocking')).toBeVisible();
    await expect(page.locator('.validate-btn')).toBeDisabled();
    // Still unknown after a first check: still blocked.
    await page.getByTestId('eft-verify').click();
    await expect(page.getByTestId('eft-title')).toHaveText('Réponse du terminal inconnue');
    await page.getByTestId('eft-ok').click();
    await expect(page.locator('.validate-btn')).toBeDisabled();
    // Approved: it becomes the locked part, and the sale can be signed.
    await page.getByTestId('eft-verify').click();
    await expect(overlay(page)).toBeHidden();
    await expect(page.getByTestId('eft-open')).toHaveCount(0);
    await expect(page.locator('.paid-part')).toContainText('Bancontact');
    await expect(page.locator('.validate-btn')).toBeEnabled();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1200, eft_payment_id: 100 }]);
  });

  test('an approved unused payment is a locked part when the checkout reopens, and the sale uses it', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      known: [{ id: 7, order_id: ORDER, amount_cents: 1200, status: 'approved', brand: 'Bancontact' }],
    });
    expect(await ipcCalls(page, 'list_eft_payments')).toEqual([{ orderId: ORDER }]);
    const part = page.locator('.paid-part');
    await expect(part).toContainText('Bancontact');
    await expect(part).toContainText('Accepté');
    await expect(page.locator('.total-value')).toHaveText('0,00');
    // It cannot be cancelled: Retour, not "Annuler les paiements".
    await expect(page.locator('.parts-clear')).toHaveCount(0);
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    expect(await ipcCalls(page, 'eft_pay')).toHaveLength(0);
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1200, eft_payment_id: 7 }]);
  });

  test('a partial approved payment stays, the rest is paid in cash', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      known: [{ id: 7, order_id: ORDER, amount_cents: 500, status: 'approved' }],
    });
    await expect(page.locator('.paid-part')).toContainText('5,00');
    await expect(page.locator('.total-value')).toHaveText('7,00');
    await page.locator('.shortcut.exact').click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 500, eft_payment_id: 7 }, { method: 'cash' }]);
  });

  test('a payment waiting for its answer shows up at reopening with Vérifier', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      known: [{ id: 8, order_id: ORDER, amount_cents: 1200, status: 'unknown' }],
      check: [{ status: 'declined', detail: 'Carte refusée' }],
    });
    await expect(page.getByTestId('eft-open')).toBeVisible();
    await expect(page.locator('.validate-btn')).toBeDisabled();
    await page.getByTestId('eft-verify').click();
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement refusé');
    await page.getByTestId('eft-ok').click();
    // Declined: nothing was charged, the sale is free again.
    await expect(page.getByTestId('eft-open')).toHaveCount(0);
    await page.locator('.card-btn').click();
    await expect(page.locator('.validate-btn')).toBeEnabled();
  });

  test('two card parts are charged in turn, each locked', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      pay: [{ status: 'approved', brand: 'Bancontact' }, { status: 'approved', brand: 'Visa' }],
    });
    await page.locator('.card-btn').click();
    for (const d of '5') await page.locator('.nk', { hasText: d }).first().click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.paid-part')).toContainText('Bancontact');
    await expect(page.locator('.total-value')).toHaveText('7,00');
    await page.locator('.card-btn').click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    expect(await ipcCalls(page, 'eft_pay')).toEqual([
      { orderId: ORDER, amountCents: 500 },
      { orderId: ORDER, amountCents: 700 },
    ]);
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([
      { method: 'card', amount_cents: 500, eft_payment_id: 100 },
      { method: 'card', amount_cents: 700, eft_payment_id: 101 },
    ]);
  });

  test('a meal-voucher card is a locked part named Chèques-repas, still sent as a card part', async ({ page }) => {
    await toCheckout(page, {
      terminal: 'worldline_terminal_api',
      pay: [{ status: 'approved', brand: 'Edenred' }, { status: 'approved', brand: 'Bancontact' }],
    });
    await page.locator('.card-btn').click();
    for (const d of '5') await page.locator('.nk', { hasText: d }).first().click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.paid-part')).toContainText('Chèques-repas');
    await expect(page.locator('.paid-part')).toContainText('Edenred');
    await expect(page.locator('.paid-part')).not.toContainText('Carte');
    await expect(page.getByTestId('eft-approved')).toBeVisible();
    await page.locator('.card-btn').click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    const [call] = await ipcCalls(page, 'checkout');
    // The part stays a card part: the engine classifies it from the stored brand.
    expect(call.payments).toEqual([
      { method: 'card', amount_cents: 500, eft_payment_id: 100 },
      { method: 'card', amount_cents: 700, eft_payment_id: 101 },
    ]);
  });

  test('no terminal linked: the manual card flow is unchanged', async ({ page }) => {
    await toCheckout(page);
    await page.locator('.card-btn').click();
    await expect(page.getByTestId('eft-mode')).toHaveCount(0);
    await expect(page.locator('.card-hint')).toHaveText('Encaisser sur le terminal');
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    expect(await ipcCalls(page, 'eft_pay')).toHaveLength(0);
    expect(await ipcCalls(page, 'list_eft_payments')).toHaveLength(0);
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1200 }]);
  });

  test('training mode: card parts are typed by hand, the terminal is not asked', async ({ page }) => {
    await toCheckout(page, { terminal: 'worldline_terminal_api', training: true });
    await page.locator('.card-btn').click();
    await expect(page.getByTestId('eft-mode')).toHaveCount(0);
    await expect(page.locator('.card-hint')).toHaveText('Encaisser sur le terminal');
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    expect(await ipcCalls(page, 'eft_pay')).toHaveLength(0);
    const [call] = await ipcCalls(page, 'checkout');
    expect(call.payments).toEqual([{ method: 'card', amount_cents: 1200 }]);
  });
});

// ── Online orders ───────────────────────────────────────────────

const UNPAID = {
  id: 4,
  hubrise_order_id: 'hr-4',
  status: 'open',
  hubrise_status: 'awaiting_collection',
  source: 'Website',
  service_type: 'collection',
  total_cents: 2850,
  created_at: '2026-04-12 12:00:00',
  items: [{ product_name: 'Maki saumon x8', quantity: 2, unit_price_cents: 1200, line_total_cents: 2400, options: [], customer_notes: null }],
  customer_notes: null,
  customer_name: 'Alice Martin',
  confirmed_time: null,
  customer_address_1: null,
  customer_address_2: null,
  customer_postal_code: null,
  customer_city: null,
  webshop: {
    order_id: 4, hubrise_order_id: 'hr-4', channel: 'website', channel_kind: 'webshop', service_type: 'collection',
    payment_mode: 'unpaid', payment_type: null, fiscal_state: 'signed_p', resolution: 'ok', attention: [], warnings: [],
  },
};

async function webshopBoard(page: Page, setup: Setup) {
  await eftScript(page, { pay: setup.pay, check: setup.check, known: setup.known });
  await page.addInitScript((o) => {
    (window as any).__webshop = o;
  }, [UNPAID]);
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_settings: settingsWith(setup.terminal ? { paymentTerminal: setup.terminal } : {}),
    hubrise_list_webshop_orders: live(() => JSON.parse(JSON.stringify((window as any).__webshop))),
    webshop_handover_order: live(({ req }: any) => {
      const o = (window as any).__webshop.find((x: any) => x.id === req.order_id);
      o.webshop.fiscal_state = 'signed_n';
      o.hubrise_status = 'completed';
      return { success: true, ticket: null, error: null, fdm_connected: true, fiscal_event_id: 2, delivery: null };
    }),
  });
  await loginAndGoto(page, '/webshop-orders');
}

test.describe('online order collected by card on the terminal', () => {
  test('the total is charged on the terminal, then handed over with the payment id', async ({ page }) => {
    await webshopBoard(page, { terminal: 'worldline_terminal_api', pay: [{ status: 'approved', brand: 'Bancontact' }] });
    await page.locator('.column-ready .card').getByRole('button', { name: 'Encaisser carte' }).click();
    await expect(page.locator('.column-ready .card')).toHaveCount(0);
    expect(await ipcCalls(page, 'eft_pay')).toEqual([{ orderId: 4, amountCents: 2850 }]);
    const [call] = await ipcCalls(page, 'webshop_handover_order');
    expect(call.req).toEqual({ order_id: 4, payments: [{ method: 'card', amount_cents: 2850, eft_payment_id: 100 }] });
  });

  test('declined on the terminal: nothing is handed over', async ({ page }) => {
    await webshopBoard(page, { terminal: 'worldline_terminal_api', pay: [{ status: 'declined', detail: 'Solde insuffisant' }] });
    await page.locator('.column-ready .card').getByRole('button', { name: 'Encaisser carte' }).click();
    await expect(page.getByTestId('eft-title')).toHaveText('Paiement refusé');
    await page.getByTestId('eft-ok').click();
    await expect(page.locator('.column-ready .card')).toHaveCount(1);
    expect(await ipcCalls(page, 'webshop_handover_order')).toHaveLength(0);
  });

  test('an approved unused payment of the order is used, not charged again', async ({ page }) => {
    await webshopBoard(page, {
      terminal: 'worldline_terminal_api',
      known: [{ id: 9, order_id: 4, amount_cents: 2850, status: 'approved' }],
    });
    await page.locator('.column-ready .card').getByRole('button', { name: 'Encaisser carte' }).click();
    await expect(page.locator('.column-ready .card')).toHaveCount(0);
    expect(await ipcCalls(page, 'eft_pay')).toHaveLength(0);
    const [call] = await ipcCalls(page, 'webshop_handover_order');
    expect(call.req.payments).toEqual([{ method: 'card', amount_cents: 2850, eft_payment_id: 9 }]);
  });

  test('a payment waiting for its answer is checked before anything else', async ({ page }) => {
    await webshopBoard(page, {
      terminal: 'worldline_terminal_api',
      known: [{ id: 9, order_id: 4, amount_cents: 2850, status: 'unknown' }],
      check: [{ status: 'approved', brand: 'Visa' }],
    });
    await page.locator('.column-ready .card').getByRole('button', { name: 'Encaisser carte' }).click();
    await expect(page.locator('.column-ready .card')).toHaveCount(0);
    expect(await ipcCalls(page, 'eft_check')).toEqual([{ id: 9 }]);
    expect(await ipcCalls(page, 'eft_pay')).toHaveLength(0);
    const [call] = await ipcCalls(page, 'webshop_handover_order');
    expect(call.req.payments).toEqual([{ method: 'card', amount_cents: 2850, eft_payment_id: 9 }]);
  });

  test('without a terminal the card is collected as before', async ({ page }) => {
    await webshopBoard(page, {});
    await page.locator('.column-ready .card').getByRole('button', { name: 'Encaisser carte' }).click();
    await expect(page.locator('.column-ready .card')).toHaveCount(0);
    expect(await ipcCalls(page, 'eft_pay')).toHaveLength(0);
    const [call] = await ipcCalls(page, 'webshop_handover_order');
    expect(call.req.payments).toEqual([{ method: 'card', amount_cents: 2850 }]);
  });
});

// ── Settings ────────────────────────────────────────────────────

test.describe('settings: the linked terminal', () => {
  const cases: [string | undefined, string][] = [
    ['worldline_terminal_api', 'Worldline'],
    ['ccv_payment_api', 'CCV cloud'],
    ['ccv_opi', 'CCV réseau local'],
    [undefined, 'Aucun (saisie manuelle)'],
  ];
  for (const [kind, label] of cases) {
    test(`shows ${label}`, async ({ page }) => {
      await mockTauriIPC(page, {
        login: () => DEFAULT_USER,
        list_users: () => [DEFAULT_USER],
        get_settings: settingsWith(kind ? { paymentTerminal: kind } : {}),
      });
      await page.goto('/#/settings');
      const row = page.getByTestId('payment-terminal');
      await expect(row).toContainText('Terminal de paiement');
      await expect(row).toContainText(label);
    });
  }
});
