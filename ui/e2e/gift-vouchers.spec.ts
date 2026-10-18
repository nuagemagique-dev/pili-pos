import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC, ipcCalls, live, DEFAULT_USER, DEFAULT_ORDER, DEFAULT_ORDER_LINE, DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM, tapUserTile, loginAndGoto,
} from './fixtures/mock-ipc';

// Multi-purpose gift vouchers: the Settings panel, the "Bon
// cadeau" button and the voucher line on the order screen, paying with a
// voucher at checkout, the print outcomes on the done screen, and the
// Reports tab (lookup, list, DUPLICATA reprint).

const ON = { enabled: true, validity_months: 12, max_amount_cents: 50000 };

const VOUCHER = {
  id: 7,
  code_masked: '**** **** 9095',
  face_value_cents: 5000,
  balance_cents: 3000,
  issued_on: '2026-10-01',
  expires_on: '2027-10-01',
  status: 'active',
  training: false,
  movements: [
    { kind: 'issue', amount_cents: 5000, date: '2026-10-01', ticket: 'N 12' },
    { kind: 'redeem', amount_cents: 2000, date: '2026-10-05', ticket: 'N 40' },
  ],
};

const VOUCHER_LINE = {
  ...DEFAULT_ORDER_LINE, id: 5, menu_item_id: 990001, unit_price_cents: 2500, vat_code: 'X',
  name: 'Bon cadeau', department_id: 'GV', department_name: 'Bon cadeau',
};

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

async function toOrder(page: Page, extra: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    get_gift_voucher_settings: () => ON,
    ...extra,
  } as any);
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

test.describe('Selling a gift voucher', () => {
  test('the button is absent while the setting is off', async ({ page }) => {
    await toOrder(page, { get_gift_voucher_settings: () => ({ ...ON, enabled: false }) });
    await expect(page.getByTestId('gift-voucher-sell')).toHaveCount(0);
  });

  test('a waiter is not offered the button (the engine sells to a cashier or a manager only)', async ({ page }) => {
    const waiter = { ...DEFAULT_USER, role: 'waiter' };
    await toOrder(page, { login: () => waiter, list_users: () => [waiter] });
    await expect(page.locator('.item-btn').first()).toBeVisible();
    await expect(page.getByTestId('gift-voucher-sell')).toHaveCount(0);
    expect(await ipcCalls(page, 'add_gift_voucher_line')).toHaveLength(0);
  });

  test('a cashier is offered the button', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' };
    await toOrder(page, { login: () => cashier, list_users: () => [cashier] });
    await expect(page.getByTestId('gift-voucher-sell')).toBeVisible();
  });

  test('the button asks for an amount and adds a voucher line without quantity buttons', async ({ page }) => {
    await toOrder(page, {
      list_cart_lines: live(() => ((window as any).__sold ? [(window as any).__line] : [])),
      add_gift_voucher_line: live((args: any) => {
        const w = window as any;
        w.__line = {
          id: 5, order_id: args.orderId, menu_item_id: 990001, quantity: 1, unit_price_cents: args.amountCents,
          vat_code: 'X', name: 'Bon cadeau', modifiers_json: null, created_at: new Date().toISOString(),
          department_id: 'GV', department_name: 'Bon cadeau', name_fr: null, name_zh: null,
          unsent_quantity: 0, course: 0, kitchen_state: 'printed',
        };
        w.__sold = true;
        return w.__line;
      }),
    });
    await page.getByTestId('gift-voucher-sell').click();
    await expect(page.getByRole('dialog', { name: 'Vendre un bon cadeau' })).toBeVisible();
    for (const d of '2500') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await expect(page.getByTestId('open-price-amount')).toContainText('25,00');
    await page.getByRole('button', { name: 'Valider' }).click();

    const calls = await ipcCalls(page, 'add_gift_voucher_line');
    expect(calls).toHaveLength(1);
    expect(calls[0].amountCents).toBe(2500);
    await expect(page.getByTestId('voucher-badge')).toHaveText('Bon');
    await expect(page.locator('.qty-btn')).toHaveCount(0);
  });

  test('a voucher line has its badge and no course chip when courses are on', async ({ page }) => {
    await toOrder(page, {
      get_course_settings: () => ({ enabled: true, category_courses: [] }),
      list_cart_lines: () => [
        { ...VOUCHER_LINE, name_fr: null, name_zh: null, unsent_quantity: 0, course: 0, kitchen_state: 'printed' },
        { ...DEFAULT_ORDER_LINE, id: 6, name_fr: null, name_zh: null, unsent_quantity: 0, course: 0, kitchen_state: 'printed' },
      ],
    });
    await expect(page.getByTestId('voucher-badge')).toBeVisible();
    await expect(page.getByTestId('course-chip-5')).toHaveCount(0);
    // A dish line keeps its chip.
    await expect(page.getByTestId('course-chip-6')).toBeVisible();
  });

  test('an engine refusal is shown', async ({ page }) => {
    await toOrder(page, { add_gift_voucher_line: live(() => { throw 'gift_voucher_amount_invalid|max=500,00'; }) });
    await page.getByTestId('gift-voucher-sell').click();
    for (const d of '99999') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByRole('button', { name: 'Valider' }).click();
    await expect(page.getByText('500,00')).toBeVisible();
  });
});

async function toCheckout(page: Page, lines: unknown[], extra: Record<string, unknown> = {}) {
  await toOrder(page, {
    list_cart_lines: () => lines,
    checkout: () => SALE,
    ...extra,
  });
  await page.locator('.checkout-btn').click();
  await page.waitForURL(/checkout/);
}

test.describe('Paying with a gift voucher', () => {
  const LINES = [{ ...DEFAULT_ORDER_LINE, id: 1, unit_price_cents: 1200 }];

  test('the method is hidden while the setting is off', async ({ page }) => {
    await toCheckout(page, LINES, { get_gift_voucher_settings: () => ({ ...ON, enabled: false }) });
    await expect(page.getByTestId('method-gift_voucher')).toHaveCount(0);
    await expect(page.getByTestId('method-meal_voucher')).toBeVisible();
  });

  test('the method is hidden, with the meal vouchers, when the sale sells a voucher', async ({ page }) => {
    await toCheckout(page, [...LINES, VOUCHER_LINE]);
    await expect(page.getByTestId('method-cash')).toBeVisible();
    await expect(page.getByTestId('method-card')).toBeVisible();
    await expect(page.getByTestId('method-gift_voucher')).toHaveCount(0);
    await expect(page.getByTestId('method-meal_voucher')).toHaveCount(0);
  });

  test('a voucher found by its code pays the smaller of balance and rest, the rest goes in cash', async ({ page }) => {
    await toCheckout(page, [{ ...DEFAULT_ORDER_LINE, id: 1, unit_price_cents: 4500 }], {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 4500 }),
      lookup_gift_voucher: () => VOUCHER,
    });
    await page.getByTestId('method-gift_voucher').click();
    await expect(page.getByTestId('gv-dialog')).toBeVisible();
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await expect(page.getByTestId('gv-code')).toHaveText('1234 5678 9095');
    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-status')).toHaveText('Valide');
    await expect(page.getByTestId('gv-balance')).toContainText('30,00');
    await expect(page.getByTestId('gv-amount')).toContainText('30,00');
    await page.getByTestId('gv-apply').click();

    await expect(page.locator('.paid-part').nth(0)).toContainText('**** **** 9095');
    await expect(page.locator('.total-value')).toHaveText('15,00');
    await page.locator('.cash-btn').click();
    await page.locator('.shortcut.exact').click();
    await page.locator('.validate-btn').click();
    await expect(page.locator('.success-icon')).toBeVisible();

    const call = (await ipcCalls(page, 'checkout'))[0];
    expect(call.payments).toEqual([
      { method: 'gift_voucher', amount_cents: 3000, gift_voucher_id: 7 },
      { method: 'cash' },
    ]);
  });

  test('a voucher that covers the sale closes it and the balance slip is told', async ({ page }) => {
    await toCheckout(page, LINES, {
      lookup_gift_voucher: () => VOUCHER,
      checkout: () => ({ ...SALE, gift_voucher_prints: [{ voucher_id: 7, kind: 'balance', outcome: 'printed' }] }),
    });
    await page.getByTestId('method-gift_voucher').click();
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-lookup').click();
    await page.getByTestId('gv-apply').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    await expect(page.getByTestId('gv-prints')).toContainText('Solde imprimé');
    expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
      { method: 'gift_voucher', amount_cents: 1200, gift_voucher_id: 7 },
    ]);
  });

  test('an unknown, expired or throttled code is shown and nothing is added', async ({ page }) => {
    await toCheckout(page, LINES);
    await page.getByTestId('method-gift_voucher').click();
    for (const d of '000000000000') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-error')).toBeVisible();
    await expect(page.getByTestId('gv-apply')).toHaveCount(0);
  });

  test('an expired voucher cannot be applied', async ({ page }) => {
    await toCheckout(page, LINES, { lookup_gift_voucher: () => ({ ...VOUCHER, status: 'expired' }) });
    await page.getByTestId('method-gift_voucher').click();
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-status')).toHaveText('Expiré');
    await expect(page.getByTestId('gv-apply')).toHaveCount(0);
  });

  test('a voucher of the other mode is shown as not usable and cannot be applied', async ({ page }) => {
    await toCheckout(page, LINES, {
      lookup_gift_voucher: () => ({ ...VOUCHER, status: 'not_usable', training: true }),
    });
    await page.getByTestId('method-gift_voucher').click();
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-status')).toHaveText('Non utilisable (autre mode)');
    await expect(page.getByTestId('gv-apply')).toHaveCount(0);
  });

  test('the code is capped at 12 digits and the lookup needs all 12', async ({ page }) => {
    await toCheckout(page, LINES, { lookup_gift_voucher: () => VOUCHER });
    await page.getByTestId('method-gift_voucher').click();
    for (const d of '12345678901') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await expect(page.getByTestId('gv-lookup')).toBeDisabled();
    for (const d of '234') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await expect(page.getByTestId('gv-code')).toHaveText('1234 5678 9012');
    await expect(page.getByTestId('gv-lookup')).toBeEnabled();

    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-found')).toBeVisible();
    // Editing the code forgets the previous answer.
    await page.locator('.numpad .key-back').click();
    await expect(page.getByTestId('gv-found')).toHaveCount(0);
    await expect(page.getByTestId('gv-apply')).toHaveCount(0);
    await expect(page.getByTestId('gv-code')).toHaveText('1234 5678 901');
    await expect(page.getByTestId('gv-lookup')).toBeDisabled();
  });

  test('the expiry is shown as dd.mm.yyyy and a training voucher is tagged', async ({ page }) => {
    await toCheckout(page, LINES, { lookup_gift_voucher: live(() => (window as any).__v) });
    await page.evaluate((v) => { (window as any).__v = v; }, VOUCHER);
    await page.getByTestId('method-gift_voucher').click();
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-found')).toContainText('01.10.2027');
    await expect(page.getByTestId('gv-found')).not.toContainText("Bon d'entraînement");

    await page.evaluate((v) => { (window as any).__v = v; }, { ...VOUCHER, training: true });
    await page.locator('.numpad .key-back').click();
    await page.locator('.numpad .key', { hasText: /^5$/ }).click();
    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-found')).toContainText("Bon d'entraînement");
    await expect(page.getByTestId('gv-found')).toContainText('01.10.2027');
  });

  test('the same voucher applied twice counts once', async ({ page }) => {
    await toCheckout(page, [{ ...DEFAULT_ORDER_LINE, id: 1, unit_price_cents: 10000 }], {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 10000 }),
      lookup_gift_voucher: () => VOUCHER,
    });
    const apply = async () => {
      await page.getByTestId('method-gift_voucher').click();
      for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
      await page.getByTestId('gv-lookup').click();
      await page.getByTestId('gv-apply').click();
    };
    await apply();
    await expect(page.locator('.paid-part')).toHaveCount(1);
    await expect(page.locator('.total-value')).toHaveText('70,00');
    await apply();
    await expect(page.getByTestId('gv-dialog')).toHaveCount(0);
    await expect(page.locator('.paid-part')).toHaveCount(1);
    await expect(page.locator('.paid-part')).toContainText('**** **** 9095');
    await expect(page.locator('.total-value')).toHaveText('70,00');
  });

  test('a slip that did not print is told on the done screen', async ({ page }) => {
    await toCheckout(page, [...LINES, VOUCHER_LINE], {
      checkout: () => ({ ...SALE, gift_voucher_prints: [{ voucher_id: 8, kind: 'voucher', outcome: 'failed' }] }),
    });
    await page.locator('.shortcut.exact').click();
    await page.locator('.validate-btn').click();
    await expect(page.getByTestId('gv-prints')).toContainText('Bon non imprimé');
  });
});

test.describe('Gift voucher settings', () => {
  test('a manager switches vouchers on and saves the validity and the maximum', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_gift_voucher_settings: live(() => (window as any).__gv ?? { enabled: false, validity_months: 12, max_amount_cents: 50000 }),
      set_gift_voucher_settings: live((args: any) => { (window as any).__gv = args.input; return args.input; }),
    } as any);
    await loginAndGoto(page, '/settings');
    const panel = page.getByTestId('gv-settings');
    await expect(panel.locator('h2')).toHaveText('Bons cadeaux');
    await expect(panel.getByTestId('gv-enabled')).toContainText('Inactif');
    await expect(panel.getByTestId('gv-settings-save')).toBeDisabled();

    await panel.getByTestId('gv-enabled').click();
    await panel.getByTestId('gv-validity').fill('24');
    await panel.getByTestId('gv-max').fill('250');
    await panel.getByTestId('gv-settings-save').click();
    await expect(panel.getByTestId('gv-settings-saved')).toBeVisible();
    expect((await ipcCalls(page, 'set_gift_voucher_settings'))[0].input).toEqual({
      enabled: true, validity_months: 24, max_amount_cents: 25000,
    });
  });
});

test.describe('Gift voucher settings refusal', () => {
  test('a validity outside 12 to 36 months is refused and told', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_gift_voucher_settings: () => ON,
      set_gift_voucher_settings: live(() => { throw 'gift_voucher_settings_invalid|field=validity_months'; }),
    } as any);
    await loginAndGoto(page, '/settings');
    const panel = page.getByTestId('gv-settings');
    await panel.getByTestId('gv-validity').fill('40');
    await panel.getByTestId('gv-settings-save').click();
    await expect(panel.getByTestId('gv-settings-error')).toContainText('Réglage des bons cadeaux invalide (validity_months).');
    await expect(panel.getByTestId('gv-settings-saved')).toHaveCount(0);
    expect((await ipcCalls(page, 'set_gift_voucher_settings'))[0].input.validity_months).toBe(40);
  });
});

test.describe('Gift voucher switch-off', () => {
  test('the refusal while a sold voucher can still be redeemed is told and the setting stays on', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_gift_voucher_settings: () => ON,
      set_gift_voucher_settings: live(() => { throw 'gift_vouchers_outstanding|count=2'; }),
    } as any);
    await loginAndGoto(page, '/settings');
    const panel = page.getByTestId('gv-settings');
    await panel.getByTestId('gv-enabled').click();
    await panel.getByTestId('gv-settings-save').click();
    await expect(panel.getByTestId('gv-settings-error')).toContainText('2 bon(s) vendu(s)');
    await expect(panel.getByTestId('gv-settings-saved')).toHaveCount(0);
    expect((await ipcCalls(page, 'set_gift_voucher_settings'))[0].input.enabled).toBe(false);
  });
});

test.describe('Gift vouchers in the reports', () => {
  async function openTab(page: Page, extra: Record<string, unknown> = {}) {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_gift_voucher_settings: () => ON,
      ...extra,
    } as any);
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Bons cadeaux' }).click();
  }

  test('the tab is absent while the setting is off', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] } as any);
    await loginAndGoto(page, '/reports');
    await expect(page.getByRole('tab', { name: 'Bons cadeaux' })).toHaveCount(0);
  });

  test('a cashier can look a voucher up but gets no list and no reprint', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' };
    await openTab(page, { login: () => cashier, list_users: () => [cashier], lookup_gift_voucher: () => VOUCHER });
    await expect(page.getByTestId('gv-report-lookup')).toBeVisible();
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-report-lookup').click();
    await expect(page.getByTestId('gv-report-found')).toContainText('**** **** 9095');
    await expect(page.getByTestId('gv-list')).toHaveCount(0);
    await expect(page.getByTestId('gv-empty')).toHaveCount(0);
    await expect(page.locator('[data-testid^="gv-filter-"]')).toHaveCount(0);
    expect(await ipcCalls(page, 'list_gift_vouchers')).toHaveLength(0);
  });

  test('a code is looked up and its masked form shown', async ({ page }) => {
    await openTab(page, { lookup_gift_voucher: () => VOUCHER });
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-report-lookup').click();
    await expect(page.getByTestId('gv-report-found')).toContainText('**** **** 9095');
    await expect(page.getByTestId('gv-report-found')).toContainText('30,00');
  });

  test('the manager sees the list, filters it and reprints a voucher', async ({ page }) => {
    await openTab(page, {
      list_gift_vouchers: live((args: any) => (args.status && args.status !== 'active' ? [] : [{
        id: 7, code_masked: '**** **** 9095', face_value_cents: 5000, balance_cents: 3000, issued_on: '2026-10-01',
        expires_on: '2027-10-01', status: 'active', training: false,
        movements: [{ kind: 'issue', amount_cents: 5000, date: '2026-10-01', ticket: 'N 12' }, { kind: 'redeem', amount_cents: 2000, date: '2026-10-05', ticket: 'N 40' }],
      }])),
    });
    await expect(page.getByTestId('gv-row-7')).toContainText('30,00 / 50,00');
    await page.getByTestId('gv-row-7').locator('.voucher-head').click();
    await expect(page.getByTestId('gv-row-7')).toContainText('N 40');
    await page.getByTestId('gv-reprint-7').click();
    expect(await ipcCalls(page, 'print_gift_voucher')).toEqual([{ voucherId: 7 }]);
    await expect(page.getByText('Bon réimprimé (DUPLICATA)')).toBeVisible();

    await page.getByTestId('gv-filter-used').click();
    await expect(page.getByTestId('gv-empty')).toBeVisible();
    expect((await ipcCalls(page, 'list_gift_vouchers')).at(-1)).toEqual({ status: 'used' });
  });
});
