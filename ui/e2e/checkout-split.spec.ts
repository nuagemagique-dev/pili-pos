import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  ipcCalls,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc';

// A sale paid in several parts: card and meal vouchers for an exact
// amount each, cash for the rest (the only part rounded to 5 cents).
// The screen sends the parts to `checkout`; nothing is signed until
// the rest is paid. The manual's rounding products: Dame Blanche
// 11,52 and Tapas Variation 3,34, a sale of 14,86.

const LINES = [
  { ...DEFAULT_ORDER_LINE, id: 1, menu_item_id: 25003, unit_price_cents: 1152, vat_code: 'B', name: 'Dame Blanche' },
  { ...DEFAULT_ORDER_LINE, id: 2, menu_item_id: 28007, unit_price_cents: 334, vat_code: 'B', name: 'Tapas Variation' },
];

// The signed sale the done screen shows (its change line needs a ticket).
const SALE = {
  success: true,
  ticket: {
    restaurant_name: 'Restaurant Exemple', restaurant_address: 'Liège', restaurant_vat: 'BE0499999960',
    pos_id: 'CFOD0061234567', terminal_id: 'POS1', pos_datetime: new Date().toISOString(),
    ticket_number: 1, event_label: 'N', is_training: false, lines: [], subtotal: 14.86, total: 14.86,
    vat_breakdown: [], payments: [], rounding: null, fdm_ref: null, short_signature: null,
    verification_url: null, footer: [], ticket_type_label: 'TICKET DE CAISSE TVA', is_refund: false, lang: 'fr',
  },
  error: null, fdm_connected: true, fiscal_event_id: 1, delivery: null, kitchen_slip: 'printed',
};

async function toPayment(page: Page) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1486 }),
    list_order_lines: () => LINES,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => SALE,
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.checkout-btn').click();
  await page.waitForURL(/checkout/);
  // Cash alone: 14,86 is paid 14,85.
  await expect(page.locator('.total-value')).toHaveText('14,85');
}

/** Card or meal vouchers: clear the prefilled rest, type an amount. */
async function exactPart(page: Page, button: string, digits: string | null) {
  await page.locator(button).click();
  if (digits !== null) {
    await page.locator('.nk-clear').click();
    for (const d of digits) await page.locator('.nk', { hasText: d }).first().click();
  }
  await page.locator('.card-confirm').click();
}

async function payCash(page: Page, digits: string) {
  await page.locator('.cash-btn').click();
  for (const d of digits) await page.locator('.nk', { hasText: d }).first().click();
  await page.locator('.act-btn.primary', { hasText: 'Valider' }).click();
}

test('[ROUND4] meal vouchers 11,52 then the rest in cash, rounded to 3,35', async ({ page }) => {
  await toPayment(page);
  await exactPart(page, '.meal-btn', '11,52');
  await expect(page.locator('.total-label')).toHaveText('Reste à payer');
  await expect(page.locator('.total-value')).toHaveText('3,35');
  await expect(page.locator('.paid-part').nth(1)).toContainText('11,52');

  await payCash(page, '5');
  await expect(page.locator('.success-icon')).toBeVisible();
  await expect(page.locator('.receipt-change')).toContainText('1,65');
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'meal_voucher', amount_cents: 1152 },
    { method: 'cash' },
  ]);
});

test('[ROUND5] meal vouchers, card, then 10 € cash for the 7,75 left', async ({ page }) => {
  await toPayment(page);
  await exactPart(page, '.meal-btn', '3,13');
  await exactPart(page, '.card-btn', '3,97');
  await expect(page.locator('.total-value')).toHaveText('7,75');
  await payCash(page, '10');
  await expect(page.locator('.receipt-change')).toContainText('2,25');
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'meal_voucher', amount_cents: 313 },
    { method: 'card', amount_cents: 397 },
    { method: 'cash' },
  ]);
});

test('[UC4119] card for part, meal vouchers for the rest: no cash, no change', async ({ page }) => {
  await toPayment(page);
  await exactPart(page, '.card-btn', '10');
  // The meal voucher amount starts at the rest, 4,86.
  await page.locator('.meal-btn').click();
  await expect(page.locator('.exact-value')).toHaveText('4,86');
  await page.locator('.card-confirm').click();
  await expect(page.locator('.success-icon')).toBeVisible();
  await expect(page.locator('.receipt-change')).toHaveCount(0);
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'card', amount_cents: 1000 },
    { method: 'meal_voucher', amount_cents: 486 },
  ]);
});

test('meal vouchers for the whole sale sign at once', async ({ page }) => {
  await toPayment(page);
  await exactPart(page, '.meal-btn', null);
  await expect(page.locator('.success-icon')).toBeVisible();
  expect((await ipcCalls(page, 'checkout'))[0].payments).toEqual([
    { method: 'meal_voucher', amount_cents: 1486 },
  ]);
});

test('an amount over the rest cannot be confirmed', async ({ page }) => {
  await toPayment(page);
  await page.locator('.card-btn').click();
  await page.locator('.nk-clear').click();
  for (const d of '20') await page.locator('.nk', { hasText: d }).first().click();
  await expect(page.locator('.exact-too-high')).toBeVisible();
  await expect(page.locator('.card-confirm')).toBeDisabled();
  expect(await ipcCalls(page, 'checkout')).toHaveLength(0);
});

test('the parts taken can be cancelled before anything is signed', async ({ page }) => {
  await toPayment(page);
  await exactPart(page, '.card-btn', '5');
  await expect(page.locator('.paid-part')).toHaveCount(2);
  await page.locator('.parts-clear').click();
  await expect(page.locator('.paid-part')).toHaveCount(0);
  await expect(page.locator('.total-value')).toHaveText('14,85');
  expect(await ipcCalls(page, 'checkout')).toHaveLength(0);
});
