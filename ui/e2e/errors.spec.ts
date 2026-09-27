import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  overrideIPC,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc';

// A backend refusal is a code (src-tauri/src/commands/errors.rs), shown
// in the operator's language, never as a raw English sentence.

async function openTable(page: Page) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => DEFAULT_ORDER,
    list_order_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

test('a refused item is explained, in French', async ({ page }) => {
  await openTable(page);
  await overrideIPC(page, 'add_order_line', { __reject: 'order_not_open' });
  await page.locator('.item-btn').first().click();
  await expect(page.locator('.toast', { hasText: 'Cette commande est déjà clôturée.' })).toBeVisible();
  await expect(page.locator('.cart-lines')).not.toContainText(DEFAULT_MENU_ITEM.name);
});

test('a code with a value shows the value', async ({ page }) => {
  await openTable(page);
  await overrideIPC(page, 'add_order_line', { __reject: 'product_unavailable|product=12' });
  await page.locator('.item-btn').first().click();
  await expect(page.locator('.toast', { hasText: "Le produit 12 n'est pas disponible." })).toBeVisible();
});

test('an unexpected error says so and keeps the detail for the technician', async ({ page }) => {
  await openTable(page);
  await overrideIPC(page, 'add_order_line', { __reject: 'database is locked' });
  await page.locator('.item-btn').first().click();
  await expect(
    page.locator('.toast', {
      hasText: 'Erreur inattendue. Contactez votre technicien. (database is locked)',
    }),
  ).toBeVisible();
});

test('the text follows the screen language', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'zh'));
  await openTable(page);
  await overrideIPC(page, 'add_order_line', { __reject: 'order_not_open' });
  await page.locator('.item-btn').first().click();
  await expect(page.locator('.toast', { hasText: '该订单已结束。' })).toBeVisible();
});

test('a refused table stays on the tables screen, with the reason', async ({ page }) => {
  await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await overrideIPC(page, 'create_order', { __reject: 'not_logged_in' });
  await page.locator('.table-card').first().click();
  await expect(page.locator('.toast', { hasText: 'Session fermée : reconnectez-vous.' })).toBeVisible();
  await expect(page).toHaveURL(/tables/);
});

test('a failed payment without detail says it failed', async ({ page }) => {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => ({
      success: false, ticket: null, error: null, fdm_connected: true, fiscal_event_id: null, delivery: null,
    }),
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.item-btn').first().click();
  await page.locator('.checkout-btn').click();
  await page.waitForURL(/checkout/);
  await page.locator('.card-btn').click();
  await page.locator('.card-confirm').click();
  await expect(page.locator('.checkout .error-msg')).toHaveText("L'opération a échoué.");
});
