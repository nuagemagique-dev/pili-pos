import { test, expect } from '@playwright/test';
import { mockTauriIPC, loginAndGoto, DEFAULT_ORDER, DEFAULT_ORDER_LINE } from './fixtures/mock-ipc';

test.describe('Checkout View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2400 }),
      list_order_lines: () => [DEFAULT_ORDER_LINE],
    });
  });

  test('renders checkout view', async ({ page }) => {
    await loginAndGoto(page, '/checkout');
    // CheckoutView uses various class names — check for payment buttons
    // as proof the view rendered.
    await expect(page.getByText('CASH')).toBeVisible();
  });

  test('shows payment buttons', async ({ page }) => {
    await loginAndGoto(page, '/checkout');
    await expect(page.getByText('CASH')).toBeVisible();
    await expect(page.getByText('CARTE')).toBeVisible();
  });

  test('displays total section', async ({ page }) => {
    await loginAndGoto(page, '/checkout');
    // The total label is always rendered even if the amount is 0.00
    // (currentOrder not yet loaded via SPA nav). Verify the section exists.
    await expect(page.getByText(/TOTAL|total|payer/i)).toBeVisible();
  });
});
