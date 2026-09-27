import { test, expect } from '@playwright/test';
import { mockTauriIPC, loginAndGoto, DEFAULT_USER } from './fixtures/mock-ipc';

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
    { product_name: 'Maki saumon x8', quantity: 2, unit_price_cents: 1200, line_total_cents: 2400, options: [], customer_notes: null },
    { product_name: 'Miso soup', quantity: 1, unit_price_cents: 450, line_total_cents: 450, options: ['Sauce soja'], customer_notes: 'Extra hot' },
  ],
  customer_notes: 'Ring the bell',
  customer_name: 'Alice Martin',
  confirmed_time: null,
  customer_address_1: null,
  customer_address_2: null,
  customer_postal_code: null,
  customer_city: null,
};

test.describe('Webshop Orders View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      hubrise_list_webshop_orders: () => [SAMPLE_ORDER],
    });
    await loginAndGoto(page, '/webshop-orders');
  });

  test('renders order card', async ({ page }) => {
    await expect(page.locator('.card')).toHaveCount(1);
  });

  test('shows channel logo for Deliveroo', async ({ page }) => {
    await expect(page.locator('.channel-logo')).toBeVisible();
  });

  test('shows order items', async ({ page }) => {
    await expect(page.locator('.item-main')).toHaveCount(2);
  });

  test('shows customer name', async ({ page }) => {
    await expect(page.getByText('Alice Martin')).toBeVisible();
  });

  test('shows order notes', async ({ page }) => {
    await expect(page.getByText('Ring the bell')).toBeVisible();
  });

  test('shows total', async ({ page }) => {
    await expect(page.locator('.total-value')).toContainText('28,50');
  });

  test('one column per stage, new order in "Nouvelles"', async ({ page }) => {
    await expect(page.locator('.column')).toHaveCount(3);
    await expect(page.locator('.column-new .card')).toHaveCount(1);
    await expect(page.locator('.column-new .column-count')).toHaveText('1');
    await expect(page.locator('.column-new .key-time')).toContainText('il y a');
  });

  test('Accepter button visible', async ({ page }) => {
    await expect(page.locator('.btn-primary')).toContainText('Accepter');
  });

  test('Accepter moves the card to "En cuisine" with delay chips', async ({ page }) => {
    await page.locator('.btn-primary').click();
    await expect(page.locator('.column-kitchen .card')).toHaveCount(1);
    await expect(page.locator('.column-kitchen .chip-delay')).toHaveCount(3);
    await expect(page.locator('.column-kitchen .btn-primary')).toContainText('Préparer');
  });

  test('pending badge shows count', async ({ page }) => {
    await expect(page.locator('.nav-pending-badge')).toContainText('1');
  });
});

const at = (status: string, extra: Record<string, unknown> = {}) => ({
  ...SAMPLE_ORDER,
  hubrise_status: status,
  ...extra,
});

test.describe('Webshop Orders View - stages', () => {
  const ORDERS = [
    at('in_preparation', { id: 2, hubrise_order_id: 'prep-1', confirmed_time: '2026-04-12T17:50:00Z' }),
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
  ];

  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      hubrise_list_webshop_orders: () => ORDERS,
      print_webshop_kitchen: () => undefined,
    });
    await loginAndGoto(page, '/webshop-orders');
  });

  test('orders land in their stage column', async ({ page }) => {
    await expect(page.locator('.column-new .card')).toHaveCount(0);
    await expect(page.locator('.column-kitchen .card')).toHaveCount(1);
    await expect(page.locator('.column-ready .card')).toHaveCount(1);
    await expect(page.locator('.column-kitchen .btn-primary')).toContainText('Prête');
    await expect(page.locator('.column-kitchen .key-time')).toContainText('prêt à 19:50');
  });

  test('own webshop delivery shows address and "En livraison"', async ({ page }) => {
    const card = page.locator('.column-ready .card');
    await expect(card.locator('.webshop-badge')).toBeVisible();
    await expect(card.locator('.delivery-address')).toContainText('Rue Léopold 12, 4000 Liège');
    await expect(card.locator('.btn-primary')).toContainText('En livraison');
    await expect(card.locator('.chip-delay')).toHaveCount(0);
  });

  test('Réimprimer reveals kitchen and customer tickets', async ({ page }) => {
    const card = page.locator('.column-kitchen .card');
    await expect(card.locator('.print-row')).toHaveCount(0);
    await card.locator('.btn-reprint').click();
    await expect(card.locator('.print-row .btn-ghost')).toHaveCount(2);
    await card.locator('.print-row .btn-ghost').first().click();
    await expect(page.getByText("Ticket envoyé à l'imprimante")).toBeVisible();
    await expect(card.locator('.print-row')).toHaveCount(0);
  });

  test('Terminées toggles finished orders in the third column', async ({ page }) => {
    const toggle = page.locator('.done-toggle');
    await expect(toggle).toContainText('2');
    await toggle.click();
    await expect(page.locator('.column-done .card')).toHaveCount(2);
    await expect(page.locator('.column-done .btn-primary')).toHaveCount(0);
    await page.locator('.done-toggle').click();
    await expect(page.locator('.column-ready .card')).toHaveCount(1);
  });
});
