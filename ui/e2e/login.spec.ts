import { test, expect } from '@playwright/test';
import { mockTauriIPC, overrideIPC, DEFAULT_USER, DEFAULT_USER_WITH_PIN } from './fixtures/mock-ipc';

test.describe('Login View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
    });
    await page.goto('/');
  });

  test('shows user tile with name and role', async ({ page }) => {
    await expect(page.locator('.user-btn')).toBeVisible();
    await expect(page.locator('.user-name')).toContainText('Bruno Moyse');
    await expect(page.locator('.user-role')).toContainText('admin');
  });

  test('quick login without PIN redirects to tables', async ({ page }) => {
    await page.click('.user-btn');
    // No PIN → direct login → redirect to /tables (no current order)
    await expect(page).toHaveURL(/tables/);
  });

  test('shows PIN overlay when user has PIN', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER_WITH_PIN],
      login: () => DEFAULT_USER_WITH_PIN,
    });
    await page.goto('/');
    await page.click('.user-btn');
    await expect(page.locator('.pin-overlay')).toBeVisible();
    await expect(page.locator('.pin-title')).toBeVisible();
    // 4 empty dots
    await expect(page.locator('.pin-dot')).toHaveCount(4);
  });

  test('correct PIN logs in', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER_WITH_PIN],
      login: () => DEFAULT_USER_WITH_PIN,
    });
    await page.goto('/');
    await page.click('.user-btn');
    // Enter 1111
    for (const d of ['1', '1', '1', '1']) {
      await page.click(`.nk:has-text("${d}")`);
    }
    // The 4th digit auto-submits the PIN.
    await expect(page).toHaveURL(/tables/);
  });

  test('wrong PIN shows error and clears dots', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER_WITH_PIN],
    });
    await page.goto('/');
    // Override login to reject AFTER page load (so the mock is
    // installed in the live page context, not pre-serialized).
    await overrideIPC(page, 'login', '__THROW__');
    await page.click('.user-btn');
    for (const d of ['9', '9', '9', '9']) {
      await page.click(`.nk:has-text("${d}")`);
    }
    // The 4th digit auto-submits the PIN.
    await expect(page.locator('.error-msg')).toBeVisible();
    // Dots should be cleared (no filled class)
    await expect(page.locator('.pin-dot.filled')).toHaveCount(0);
  });

  test('INSZ numpad shows on "Autre" tap', async ({ page }) => {
    await page.click('.other-link');
    await expect(page.locator('.numpad-section')).toBeVisible();
    await expect(page.locator('.insz-display')).toBeVisible();
  });

  test('empty user list shows numpad directly', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [],
    });
    await page.goto('/');
    await expect(page.locator('.numpad-section')).toBeVisible();
  });
});
