import { test, expect } from '@playwright/test';
import { mockTauriIPC, DEFAULT_USER, ipcCalls, overrideIPC } from './fixtures/mock-ipc';

test.describe('Settings View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    });
    await page.goto('/#/settings');
  });

  test('shows training mode toggle', async ({ page }) => {
    await expect(page.locator('.toggle')).toBeVisible();
  });

  test('shows language switcher with 3 options', async ({ page }) => {
    await expect(page.locator('.lang-btn')).toHaveCount(3);
  });

  test('shows worker list with test user', async ({ page }) => {
    await expect(page.locator('.worker-name')).toContainText('Bruno Moyse');
  });

  test('shows add worker form with i18n placeholders', async ({ page }) => {
    const nameInput = page.locator('input').first();
    await expect(nameInput).toHaveAttribute('placeholder', 'Nom');
  });

  test('a new user gets the chosen role (Gérant / Serveur / Apprenti)', async ({ page }) => {
    await expect(page.getByRole('radio', { name: 'Serveur' })).toHaveAttribute('aria-checked', 'true');
    await page.getByPlaceholder('Nom').fill('Nouvel apprenti');
    await page.locator('input[inputmode="numeric"]').fill('99012345667');
    await page.getByRole('radio', { name: 'Apprenti' }).click();
    await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
    await expect.poll(() => ipcCalls(page, 'create_user')).toEqual([
      { input: { insz: '99012345667', name: 'Nouvel apprenti', role: 'waiter' } },
    ]);
  });

  test('deactivating a user takes two taps', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER, { ...DEFAULT_USER, insz: '99012345667', name: 'Parti', role: 'waiter' }],
      deactivate_user: () => undefined,
    });
    await page.goto('/#/settings');
    const row = page.locator('.worker-row', { hasText: 'Parti' });
    await row.getByRole('button', { name: 'Désactiver' }).click();
    expect(await ipcCalls(page, 'deactivate_user')).toEqual([]);
    await row.getByRole('button', { name: 'Confirmer ?' }).click();
    await expect.poll(() => ipcCalls(page, 'deactivate_user')).toEqual([{ insz: '99012345667' }]);
  });

  test('training mode is refused while an order is open, and says so', async ({ page }) => {
    await overrideIPC(page, 'set_training_mode', {
      __reject: 'open_orders_training',
    });
    await page.locator('.toggle').click();
    await expect(page.locator('.toast')).toContainText('Encaissez ou videz les commandes ouvertes');
    await expect(page.locator('.toggle')).toContainText('INACTIF');
  });

  test('shows POS info section', async ({ page }) => {
    await expect(page.getByText('CFOD0061234567')).toBeVisible();
    await expect(page.getByText('POS1')).toBeVisible();
  });

  test('shows HubRise section', async ({ page }) => {
    // HubRise heading may be below the fold - scroll to it.
    const heading = page.getByRole('heading', { name: 'HubRise' });
    await heading.scrollIntoViewIfNeeded();
    await expect(heading).toBeVisible();
  });

  test('shows HubRise disconnected status', async ({ page }) => {
    // The `.status-off` span renders inside HubRise section
    const statusSpan = page.locator('.status-off');
    await expect(statusSpan).toBeVisible();
  });

  test('HubRise OAuth flow - authorize + paste code + connected', async ({ page }) => {
    // 1. Start disconnected
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      hubrise_get_connection_status: () => ({ connected: false }),
      hubrise_start_oauth_flow: () => 'https://manager.hubrise.com/oauth2/v1/authorize?client_id=test',
      hubrise_complete_oauth: () => ({
        connected: true,
        account_id: 'acc-1',
        account_name: 'Restaurant Exemple',
        location_id: 'loc-1',
        location_name: 'Liège',
        catalog_id: 'cat-1',
        catalog_name: 'Menu principal',
        customer_list_id: null,
        customer_list_name: null,
      }),
    });
    await page.goto('/#/settings');

    // 2. Click "Ouvrir l'URL d'autorisation HubRise"
    const authorizeBtn = page.getByText("Ouvrir l'URL d'autorisation HubRise");
    await authorizeBtn.scrollIntoViewIfNeeded();
    await expect(authorizeBtn).toBeVisible();
    // Note: clicking opens a browser tab via Tauri shell - mocked, so no real navigation

    // 3. Paste a code and complete
    const codeInput = page.locator('input[placeholder]').last();
    await codeInput.fill('test-auth-code');
    const completeBtn = page.getByText('Valider la connexion');
    await completeBtn.click();

    // 4. Verify status transitions to connected
    await expect(page.locator('.status-ok')).toBeVisible();
  });

  test('HubRise connected shows account info', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      hubrise_get_connection_status: () => ({
        connected: true,
        account_id: 'acc-1',
        account_name: 'Restaurant Exemple',
        location_id: 'loc-1',
        location_name: 'Liège',
        catalog_id: 'cat-1',
        catalog_name: 'Menu principal',
        customer_list_id: null,
        customer_list_name: null,
      }),
    });
    // beforeEach already opened /#/settings, so a goto to the same hash
    // would not reload the page and the new mock would never install.
    await page.reload();
    await expect(page.locator('.status-ok')).toBeVisible();
  });
});
