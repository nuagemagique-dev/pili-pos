import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  overrideIPC,
  loginAndGoto,
  ipcCalls,
  emitTauriEvent,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM, tapUserTile, live } from './fixtures/mock-ipc';

// Fiscal side effects of the operator's navigation: an order left unpaid
// is registered with the FDM (event P, POS spec §1.3), the "Addition"
// button signs a pre-bill (§1.4), and every FDM refusal is shown.

type Handlers = Parameters<typeof mockTauriIPC>[1];

async function openTable(page: Page, overrides: Handlers = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_cart_lines: () => [DEFAULT_ORDER_LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...overrides,
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.item-btn').first().click();
  await expect(page.locator('.cart-lines')).not.toContainText('Aucun article');
}

const navButton = (page: Page, name: string) =>
  page.locator('.header-nav .nav-btn', { hasText: name });

// A takeaway is a direct sale: it ends in an N (FAQ Q205). Leaving it
// unpaid registers nothing; it stays a draft listed on the Tables
// screen until it is paid or closed unpaid. Order 501 is table 1,
// order 502 the takeaway.
const orderOf = (id: number) => ({
  id,
  table_number: id === 502 ? null : 1,
  order_type: id === 502 ? 'takeaway' : 'dine_in',
  status: 'open',
  user_insz: '94060242576',
  created_at: new Date().toISOString(),
  closed_at: null,
  subtotal_cents: 0,
  total_cents: 0,
});

test.describe('Takeaway drafts (direct sale)', () => {
  test('[FAQ_Q205] leaving an unpaid takeaway leaves the registration to the engine', async ({ page }) => {
    await openTable(page, {
      create_order: live(({ input }: any = {}) => ({
        id: input?.order_type === 'takeaway' ? 502 : 501,
        table_number: input?.table_number ?? null,
        order_type: input?.order_type ?? 'dine_in',
        status: 'open', user_insz: '94060242576', created_at: new Date().toISOString(),
        closed_at: null, subtotal_cents: 0, total_cents: 0,
      })),
      get_order: live(({ orderId }: any = {}) => ({
        id: orderId,
        table_number: orderId === 502 ? null : 1,
        order_type: orderId === 502 ? 'takeaway' : 'dine_in',
        status: 'open', user_insz: '94060242576', created_at: new Date().toISOString(),
        closed_at: null, subtotal_cents: 0, total_cents: 0,
      })),
    });
    await navButton(page, 'À emporter').click();
    await expect(page.locator('.header-takeaway')).toBeVisible();
    await page.locator('.item-btn').first().click();
    await navButton(page, 'Tables').click();
    await page.waitForURL(/tables/);

    // The UI leaves both orders through the engine; registering is the
    // engine's rule (a table yes, a takeaway never).
    const left = (await ipcCalls(page, 'leave_order')).map((c) => `${c.orderId}:${c.reason}`);
    expect(left).toEqual(['501:switch_order', '502:navigate']);
    expect(await ipcCalls(page, 'sign_table_order')).toHaveLength(0);
  });

  test('[FAQ_Q205] an unpaid takeaway is listed on the tables and resumes', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_open_takeaways: () => [
        { orderId: 502, totalCents: 608, lineCount: 1, createdAt: '2026-09-26 10:05:00' },
      ],
      get_order: () => orderOf(502),
      list_cart_lines: () => [DEFAULT_ORDER_LINE],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);

    const card = page.locator('.takeaway-card');
    await expect(card).toHaveCount(1);
    await expect(card).toContainText('6,08');
    await expect(card).toContainText('12:05');
    await card.click();
    await page.waitForURL(/order/);
    await expect(page.locator('.header-takeaway')).toBeVisible();
    expect((await ipcCalls(page, 'get_order')).map((c) => c.orderId)).toContain(502);
  });

  test('no takeaway in progress, none listed', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await expect(page.locator('.table-card').first()).toBeVisible();
    await expect(page.locator('.takeaway-card')).toHaveCount(0);
  });
});

test.describe('Table order registration (P)', () => {
  test('[FAQ_Q205] leaving the order for the tables registers it', async ({ page }) => {
    await openTable(page);
    await navButton(page, 'Tables').click();
    await page.waitForURL(/tables/);

    const calls = await ipcCalls(page, 'leave_order');
    expect(calls).toHaveLength(1);
    expect(calls[0].orderId).toBeGreaterThan(0);
    expect(calls[0].reason).toBe('navigate');
  });

  test('leaving for the reports registers it', async ({ page }) => {
    await openTable(page);
    await navButton(page, 'Rapports').click();
    await page.waitForURL(/reports/);
    expect(await ipcCalls(page, 'leave_order')).toMatchObject([{ reason: 'navigate' }]);
  });

  test('leaving for the settings registers it', async ({ page }) => {
    await openTable(page);
    await navButton(page, 'Config').click();
    await page.waitForURL(/settings/);
    expect(await ipcCalls(page, 'leave_order')).toMatchObject([{ reason: 'navigate' }]);
  });

  test('going to the payment does not register it (the sale carries it)', async ({ page }) => {
    await openTable(page);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    expect(await ipcCalls(page, 'leave_order')).toHaveLength(0);
  });

  test('leaving the payment screen for another screen registers it', async ({ page }) => {
    await openTable(page);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.evaluate(() => { window.location.hash = '#/tables'; });
    await page.waitForURL(/tables/);
    expect(await ipcCalls(page, 'leave_order')).toMatchObject([{ reason: 'navigate' }]);
  });

  test('[FAQ_Q205] switching to a takeaway registers the table first', async ({ page }) => {
    await openTable(page);
    await navButton(page, 'À emporter').click();

    // The mock's create_order is static, so check the calls themselves:
    // the table is registered before the takeaway order is created.
    await expect.poll(() => ipcCalls(page, 'create_order').then((c) => c.length)).toBe(2);
    const order = await page.evaluate(() =>
      ((window as any).__ipcCalls as Array<{ cmd: string; args: any }>)
        .filter((c) => c.cmd === 'leave_order' || c.cmd === 'create_order')
        .map((c) => c.cmd === 'create_order' ? `create:${c.args.input.order_type}` : 'sign'),
    );
    expect(order).toEqual(['create:dine_in', 'sign', 'create:takeaway']);
    expect(await ipcCalls(page, 'leave_order')).toMatchObject([{ reason: 'switch_order' }]);
  });

  test('[FAQ_Q205] locking the screen registers it', async ({ page }) => {
    await openTable(page);
    await navButton(page, 'Verrouiller').click();
    await page.waitForURL(/#\/$/);
    // The engine registers the open order while locking: the UI does not.
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(1);
    expect(await ipcCalls(page, 'leave_order')).toHaveLength(0);
    expect(await ipcCalls(page, 'logout')).toHaveLength(0);
  });

  test('an FDM refusal keeps the operator on the order and says so', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'leave_order', '__THROW__');
    await navButton(page, 'Tables').click();

    await expect(page.locator('.toast-error')).toContainText('Commande non enregistrée');
    await expect(page).toHaveURL(/order/);
  });

  test('an FDM refusal does not start the takeaway', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'leave_order', '__THROW__');
    await navButton(page, 'À emporter').click();

    await expect(page.locator('.toast-error')).toContainText('Commande non enregistrée');
    await expect(page.locator('.header-table')).toBeVisible();
    await expect(page.locator('.header-takeaway')).toHaveCount(0);
  });

  test('an FDM refusal never blocks locking the screen', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'lock_session', { orderError: 'FDM_UNREACHABLE' });
    await navButton(page, 'Verrouiller').click();
    await page.waitForURL(/#\/$/);
    await expect(page.locator('.user-btn').first()).toBeVisible();
    await expect(page.locator('.toast-error')).toContainText('Commande non enregistrée');
  });
});

test.describe('Leaving and locking through the engine', () => {
  test('leaving a table registers it through leave_order with the reason navigate', async ({ page }) => {
    // One id everywhere: the order is whichever the mocks return.
    const table = { ...DEFAULT_ORDER, id: 501, total_cents: 1200 };
    await openTable(page, { create_order: () => table, get_order: () => table });
    await navButton(page, 'Tables').click();
    await page.waitForURL(/tables/);
    expect(await ipcCalls(page, 'leave_order')).toEqual([{ orderId: 501, reason: 'navigate' }]);
    expect(await ipcCalls(page, 'sign_table_order')).toHaveLength(0);
  });

  test('a refused leave keeps the operator on the order with the toast', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'leave_order', { __reject: 'FDM_UNREACHABLE' });
    await navButton(page, 'Rapports').click();
    await expect(page.locator('.toast-error')).toContainText('Commande non enregistrée');
    await expect(page).toHaveURL(/order/);
  });

  test('the lock button calls lock_session and returns to the login screen', async ({ page }) => {
    await openTable(page);
    await navButton(page, 'Verrouiller').click();
    await page.waitForURL(/#\/$/);
    await expect(page.locator('.user-btn').first()).toBeVisible();
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(1);
    expect(await ipcCalls(page, 'logout')).toHaveLength(0);
    await expect(page.locator('.toast-error')).toHaveCount(0);
  });

  test('the lock button shows the toast when the order could not be registered', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'lock_session', { orderError: 'FDM_UNREACHABLE' });
    await navButton(page, 'Verrouiller').click();
    await page.waitForURL(/#\/$/);
    await expect(page.locator('.toast-error')).toContainText('Commande non enregistrée');
    await expect(page.locator('.user-btn').first()).toBeVisible();
  });

  test('a session-locked event returns to the login screen from the order screen', async ({ page }) => {
    await openTable(page);
    await emitTauriEvent(page, 'session-locked', { reason: 'idle', orderError: null });
    await page.waitForURL(/#\/$/);
    await expect(page.locator('.user-btn').first()).toBeVisible();
    await expect(page.locator('.toast-error')).toHaveCount(0);
    // The engine locked it: the UI does not lock again.
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(0);
  });

  test('a session-locked event with an order error shows the toast', async ({ page }) => {
    await openTable(page);
    await emitTauriEvent(page, 'session-locked', { reason: 'idle', orderError: 'FDM_UNREACHABLE' });
    await page.waitForURL(/#\/$/);
    await expect(page.locator('.toast-error')).toContainText('Commande non enregistrée');
  });

  test('opening a table sets the active order in the engine', async ({ page }) => {
    await openTable(page, { create_order: () => ({ ...DEFAULT_ORDER, id: 777 }) });
    const calls = await ipcCalls(page, 'set_active_order');
    expect(calls[0]).toEqual({ orderId: 777 });
  });

  test('clearing the order after payment sets the active order to null', async ({ page }) => {
    await openTable(page);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.locator('.card-btn').click();
    await page.locator('.card-confirm').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    const calls = await ipcCalls(page, 'set_active_order');
    expect(calls[calls.length - 1]).toEqual({ orderId: null });
  });
});

test.describe('Pre-bill ("Addition")', () => {
  test('signs a pre-bill for the table and confirms it', async ({ page }) => {
    await openTable(page);
    const button = page.getByRole('button', { name: 'Addition' });
    await expect(button).toBeEnabled();
    await button.click();

    await expect(page.locator('.toast-success')).toContainText('Addition provisoire imprimée');
    const calls = await ipcCalls(page, 'sign_pre_bill');
    expect(calls).toHaveLength(1);
    expect(calls[0].orderId).toBeGreaterThan(0);
    // The order stays open on screen: a pre-bill is not a payment.
    await expect(page).toHaveURL(/order/);
  });

  test('an FDM refusal is shown', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'sign_pre_bill', '__THROW__');
    await page.getByRole('button', { name: 'Addition' }).click();
    await expect(page.locator('.toast-error')).toContainText('Addition non enregistrée');
  });

  test('is not offered for takeaways', async ({ page }) => {
    const takeaway = { ...DEFAULT_ORDER, order_type: 'takeaway', table_number: null };
    await openTable(page, { create_order: () => takeaway, get_order: () => takeaway });
    await expect(page.locator('.header-takeaway')).toBeVisible();
    await expect(page.locator('.checkout-btn')).toBeEnabled();
    await expect(page.getByRole('button', { name: 'Addition' })).toHaveCount(0);
  });

  test('is disabled while the table has no lines', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_cart_lines: () => [],
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await page.locator('.table-card').first().click();
    await page.waitForURL(/order/);
    await expect(page.getByRole('button', { name: 'Addition' })).toBeDisabled();
  });
});

test.describe('[VI_BAR] Bar / kitchen order slip', () => {
  const SLIP_WARNING = 'Bon de commande non imprimé';

  test('[VI_BAR] a slip that did not print is told, the order stays registered', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'leave_order', 'failed');
    await navButton(page, 'Tables').click();
    await page.waitForURL(/tables/);
    await expect(page.locator('.toast-warning')).toContainText(SLIP_WARNING);
    await expect(page.locator('.toast-error')).toHaveCount(0);
  });

  test('[VI_BAR] a printed slip needs no message', async ({ page }) => {
    await openTable(page);
    await navButton(page, 'Tables').click();
    await page.waitForURL(/tables/);
    expect(await ipcCalls(page, 'leave_order')).toHaveLength(1);
    await expect(page.locator('.toast-warning')).toHaveCount(0);
  });

  test('[VI_BAR] a pre-bill whose slip failed says so next to its confirmation', async ({ page }) => {
    await openTable(page);
    await overrideIPC(page, 'sign_pre_bill', 'failed');
    await page.getByRole('button', { name: 'Addition' }).click();
    await expect(page.locator('.toast-success')).toContainText('Addition provisoire imprimée');
    await expect(page.locator('.toast-warning')).toContainText(SLIP_WARNING);
  });

  test('[VI_BAR] a sale whose slip failed is still paid, with the warning', async ({ page }) => {
    await openTable(page, {
      checkout: () => ({
        success: true,
        ticket: null,
        error: null,
        fdm_connected: true,
        fiscal_event_id: 1,
        delivery: null,
        kitchen_slip: 'failed',
      }),
    });
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.locator('.card-btn').click();
    await page.locator('.card-confirm').click();
    await expect(page.locator('.success-icon')).toBeVisible();
    await expect(page.locator('.toast-warning')).toContainText(SLIP_WARNING);
  });
});

test.describe('FDM refusals on presence and cash events', () => {
  test('a presence-failed event is reported but does not block the login', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await emitTauriEvent(page, 'presence-failed', {
      insz: DEFAULT_USER.insz,
      clockIn: true,
      error: 'FDM error: offline',
    });
    await expect(page.locator('.toast-error')).toContainText('Pointage non enregistré');
    await expect(page.locator('.toast-error')).toContainText('Erreur boîte noire. Contactez votre technicien.');
  });

  test('a successful login shows no clock-in error and the UI sends no work_in', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await page.waitForTimeout(300);
    expect(await ipcCalls(page, 'work_in')).toHaveLength(0);
    await expect(page.locator('.toast-error')).toHaveCount(0);
  });
});

test.describe('Cash events (F)', () => {
  const REFUSED = { success: false, error: 'FDM error: offline' };

  async function gotoCashTab(page: Page, overrides: Handlers = {}) {
    await mockTauriIPC(page, overrides);
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Caisse' }).click();
  }

  async function openCash(page: Page, overrides: Handlers = {}) {
    await gotoCashTab(page, overrides);
    await page.locator('.cash-preset').first().click();
  }

  test('a recorded cash movement is confirmed and sent once', async ({ page }) => {
    await openCash(page);
    await page.locator('.cash-submit').click();
    await expect(page.locator('.toast-success')).toContainText('Mouvement enregistré');

    const calls = await ipcCalls(page, 'money_in_out');
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ amountCents: 1000, direction: 'in', reason: 'fond_de_caisse' });
  });

  test('a cash out is sent with its direction', async ({ page }) => {
    await openCash(page);
    await page.locator('.cash-dir-out').click();
    await page.locator('.cash-reason', { hasText: 'Retrait' }).click();
    await page.locator('.cash-submit').click();
    await expect.poll(() => ipcCalls(page, 'money_in_out')).toHaveLength(1);
    const calls = await ipcCalls(page, 'money_in_out');
    expect(calls[0]).toMatchObject({ amountCents: 1000, direction: 'out', reason: 'retrait' });
  });

  test('a refused cash movement is shown as an error, never as recorded', async ({ page }) => {
    await openCash(page, { money_in_out: () => REFUSED });
    await page.locator('.cash-submit').click();

    await expect(page.locator('.toast-error')).toContainText('Erreur boîte noire. Contactez votre technicien.');
    await expect(page.locator('.toast-success')).toHaveCount(0);
    // The amount stays selected so the operator can retry.
    await expect(page.locator('.cash-preset.active')).toHaveCount(1);
  });

  test('a refused drawer opening is shown as an error', async ({ page }) => {
    await openCash(page, { open_drawer_fiscal: () => REFUSED });
    await page.locator('.drawer-btn').click();
    await expect(page.locator('.toast-error')).toContainText('Erreur boîte noire. Contactez votre technicien.');
    await expect(page.locator('.toast-success')).toHaveCount(0);
  });

  test('[M133] the counted cash is declared with the amount typed', async ({ page }) => {
    await gotoCashTab(page);
    await page.locator('.declaration-btn').click();
    for (const key of ['3', '4', '7', '2', '0']) {
      await page.locator('.numpad button', { hasText: new RegExp(`^${key}$`) }).click();
    }
    await expect(page.getByTestId('open-price-amount')).toContainText('347,20');
    await page.locator('.btn-confirm').click();

    await expect(page.locator('.toast-success')).toContainText('Contenu du tiroir déclaré');
    const calls = await ipcCalls(page, 'money_in_out');
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ amountCents: 34720, direction: 'declaration' });
  });

  test('[M133] an empty drawer is declared as 0', async ({ page }) => {
    await gotoCashTab(page);
    await page.locator('.declaration-btn').click();
    await expect(page.locator('.btn-confirm')).toBeEnabled();
    await page.locator('.btn-confirm').click();
    await expect.poll(() => ipcCalls(page, 'money_in_out')).toHaveLength(1);
    const calls = await ipcCalls(page, 'money_in_out');
    expect(calls[0]).toMatchObject({ amountCents: 0, direction: 'declaration' });
  });

  test('cash buttons stay disabled until an amount is chosen', async ({ page }) => {
    await gotoCashTab(page);
    await expect(page.locator('.cash-submit')).toBeDisabled();
    await page.locator('.cash-dir-out').click();
    await expect(page.locator('.cash-submit')).toBeDisabled();
    await page.locator('.cash-submit').click({ force: true });
    expect(await ipcCalls(page, 'money_in_out')).toHaveLength(0);
  });
});
