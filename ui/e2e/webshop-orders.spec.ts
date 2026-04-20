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
    await expect(page.locator('.total-value')).toContainText('28.50');
  });

  test('filter tabs visible', async ({ page }) => {
    await expect(page.locator('.filter-chip')).toHaveCount(7);
  });

  test('Accepter button visible', async ({ page }) => {
    await expect(page.locator('.btn-primary')).toContainText('Accepter');
  });

  test('pending badge shows count', async ({ page }) => {
    await expect(page.locator('.nav-pending-badge')).toContainText('1');
  });
});
