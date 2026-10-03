import { test, expect } from '@playwright/test';
import { mockTauriIPC, DEFAULT_USER, ipcCalls, loginAndGoto, overrideIPC } from './fixtures/mock-ipc';

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
    await expect(page.locator('.lang-btn')).toHaveText(['Français', '中文', 'English']);
    await expect(page.locator('.lang-btn.active')).toHaveText('Français');
  });

  test('an employee without a PIN is flagged', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER, { ...DEFAULT_USER, insz: '99012345667', name: 'Wei', role: 'cashier', has_pin: false }],
    });
    await page.reload();
    await expect(page.locator('.worker-row', { hasText: 'Bruno Moyse' }).locator('.worker-meta')).toHaveText('Gérant · PIN défini');
    const meta = page.locator('.worker-row', { hasText: 'Wei' }).locator('.worker-meta');
    await expect(meta).toHaveText('Serveur · sans PIN');
    await expect(meta).toHaveClass(/no-pin/);
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
    await page.getByRole('button', { name: 'Ajouter l’employé' }).click();
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
    // beforeEach already opened /#/settings: reload so the new mock installs.
    await page.reload();
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
    await expect(page.locator('.toggle')).toContainText('Inactif');
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
      hubrise_start_oauth_flow: () => ({
        url: 'https://manager.hubrise.com/oauth2/v1/authorize?client_id=test',
        qr_svg: '<svg xmlns="http://www.w3.org/2000/svg" data-qr="1" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>',
      }),
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

    // 2. "Se connecter à HubRise": the kiosk has no browser, so the
    // login link shows as a QR code, with the URL spelled out.
    const authorizeBtn = page.getByText('Se connecter à HubRise');
    await authorizeBtn.scrollIntoViewIfNeeded();
    await authorizeBtn.click();
    const panel = page.getByTestId('hubrise-authorize');
    await expect(panel.locator('svg[data-qr]')).toBeVisible();
    await expect(panel).toContainText('Scannez ce code avec un téléphone');
    await expect(panel).toContainText('https://manager.hubrise.com/oauth2/v1/authorize?client_id=test');

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

const CONNECTED = {
  connected: true,
  account_id: 'acc-1',
  account_name: 'Restaurant Exemple',
  location_id: 'loc-1',
  location_name: 'Liège',
  catalog_id: 'cat-1',
  catalog_name: 'Menu principal',
  customer_list_id: null,
  customer_list_name: null,
};

const HEALTHY = {
  connected: true,
  last_poll_at: '2026-10-03 10:00:00',
  auth_error_at: null,
  last_error: null,
  callback_registered_at: '2026-10-03 09:00:00',
  last_catalog_sync_at: '2026-10-03 09:30:00',
  outbox: { pending: 0, parked: 0 },
  outbox_error: null,
  parked_events: 0,
  stale_orders: 0,
  catalog: { source: 'hubrise', manages_menu: true, applied_at: '2026-10-03 09:30:00', rejected_at: null, issues: [] },
  catalog_awaiting_adoption: false,
};

async function hubriseSettings(page: import('@playwright/test').Page, overrides: Record<string, unknown>) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    hubrise_get_connection_status: () => CONNECTED,
    hubrise_status: () => HEALTHY,
    hubrise_list_channels: () => [
      { channel: 'deliveroo', kind: 'platform', payment_type: 'APP', charge_policy: 'include' },
      { channel: 'website', kind: 'webshop', payment_type: 'ONLINE', charge_policy: 'include' },
    ],
    ...(overrides as Record<string, never>),
  });
  // The manager's sections need a logged-in manager.
  await loginAndGoto(page, '/settings');
}

test.describe('Settings - HubRise catalog and channels', () => {
  test('the menu is shown as managed by HubRise, with its last sync', async ({ page }) => {
    await hubriseSettings(page, {});
    await expect(page.getByTestId('menu-managed')).toContainText('Menu géré par HubRise');
    await expect(page.getByTestId('catalog-synced')).toContainText('Synchronisé');
  });

  test('a refused catalog lists its issues', async ({ page }) => {
    await hubriseSettings(page, {
      hubrise_status: () => ({
        ...HEALTHY,
        catalog: {
          ...HEALTHY.catalog,
          rejected_at: '2026-10-03 10:00:00',
          issues: [{ code: 'unknown_vat_rate', key: 'MAKI-6' }, { code: 'missing_name', key: 'COCA' }],
        },
      }),
    });
    const refused = page.getByTestId('catalog-refused');
    await expect(refused).toContainText('Catalogue refusé');
    await expect(refused).toContainText('Taux de TVA inconnu');
    await expect(refused).toContainText('MAKI-6');
    await expect(page.locator('.lights .light', { hasText: 'HubRise' })).toHaveClass(/err/);
  });

  test('Resynchroniser fetches the catalog', async ({ page }) => {
    await hubriseSettings(page, { hubrise_sync_catalog: () => null });
    await page.locator('.catalog-row').getByRole('button', { name: 'Resynchroniser' }).click();
    expect(await ipcCalls(page, 'hubrise_sync_catalog')).toHaveLength(1);
    await expect(page.getByText('Catalogue synchronisé')).toBeVisible();
  });

  test('the first catalog waits for the manager to pick the products to keep', async ({ page }) => {
    await hubriseSettings(page, {
      hubrise_status: () => ({ ...HEALTHY, catalog: null, catalog_awaiting_adoption: true }),
      hubrise_adoption_preview: () => [
        { menu_item_id: 1, key: 'MAKI-6', name: 'Maki saumon', price_cents: 650 },
        { menu_item_id: 2, key: 'COCA', name: 'Coca-Cola', price_cents: 300 },
      ],
      hubrise_adopt_catalog: () => ({ outcome: 'applied', hash: 'h', active_items: 2, deactivated_items: 0 }),
    });
    const list = page.getByTestId('adoptions');
    await expect(list.locator('input[type=checkbox]:checked')).toHaveCount(2);
    await list.locator('label', { hasText: 'Coca-Cola' }).locator('input').uncheck();
    await page.getByRole('button', { name: 'Reprendre le menu' }).click();
    const calls = await ipcCalls(page, 'hubrise_adopt_catalog');
    expect(calls[0].adoptions).toEqual([{ menu_item_id: 1, key: 'MAKI-6', name: 'Maki saumon', price_cents: 650 }]);
  });

  test('a channel is added as a platform and its kind changes its payment', async ({ page }) => {
    await hubriseSettings(page, { hubrise_set_channel: () => 1 });
    const channels = page.getByTestId('channels');
    await expect(channels.locator('.channel')).toHaveCount(2);
    await page.locator('input[placeholder="Nom du canal"]').fill('Table QR');
    await page.getByRole('button', { name: 'Ajouter un canal' }).click();
    await channels.locator('.channel', { hasText: 'website' }).locator('select').first().selectOption('platform');
    const calls = await ipcCalls(page, 'hubrise_set_channel');
    expect(calls[0].channel).toEqual({ channel: 'Table QR', kind: 'platform', payment_type: 'APP', charge_policy: 'exclude' });
    expect(calls[1].channel).toEqual({ channel: 'website', kind: 'platform', payment_type: 'APP', charge_policy: 'include' });
    await expect(page.getByText('1 commande(s) débloquée(s)').first()).toBeVisible();
  });

  test('a refused token turns the light red and says so', async ({ page }) => {
    await hubriseSettings(page, {
      hubrise_status: () => ({ ...HEALTHY, auth_error_at: '2026-10-03 10:00:00' }),
    });
    await expect(page.getByText('HubRise refuse la connexion')).toBeVisible();
    await expect(page.locator('.lights .light', { hasText: 'HubRise' })).toHaveClass(/err/);
  });
});

test.describe('Settings - the HubRise switch', () => {
  test('a manager switches HubRise off: the panel and the board go, the change is logged', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] });
    await loginAndGoto(page, '/settings');
    const panel = page.getByTestId('integrations');
    await expect(panel.locator('.pill')).toHaveText('Activé');
    await expect(page.locator('.hubrise-section')).toBeVisible();
    await expect(page.getByRole('button', { name: /En ligne/ })).toBeVisible();

    await panel.getByRole('button', { name: 'Désactiver' }).click();
    await expect(panel.locator('.pill')).toHaveText('Désactivé');
    expect((await ipcCalls(page, 'set_hubrise_enabled'))[0]).toMatchObject({ enabled: false });
    await expect(panel.locator('.log li').first()).toContainText('gérant 85073003328');
    await expect(page.locator('.hubrise-section')).toHaveCount(0);
    await expect(page.getByRole('button', { name: /En ligne/ })).toHaveCount(0);
  });

  test('switching off waits for the online orders: the refusal says how many', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] });
    await loginAndGoto(page, '/settings');
    await overrideIPC(page, 'set_hubrise_enabled', { __reject: 'hubrise_online_orders_open|count=2' });
    await page.getByTestId('integrations').getByRole('button', { name: 'Désactiver' }).click();
    await expect(page.getByText('Terminez d\'abord les 2 commande(s) en ligne en cours.')).toBeVisible();
    await expect(page.getByTestId('integrations').locator('.pill')).toHaveText('Activé');
  });

  test('with HubRise off the online board is out of reach', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_integrations: () => ({ hubrise_enabled: false, online_orders_open: 0, hubrise_log: [] }),
    });
    await loginAndGoto(page, '/webshop-orders');
    await expect(page).toHaveURL(/#\/tables/);
    await expect(page.getByRole('button', { name: /En ligne/ })).toHaveCount(0);
  });

  test('off with online orders still to settle, the board stays reachable', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_integrations: () => ({ hubrise_enabled: false, online_orders_open: 1, hubrise_log: [] }),
    });
    await loginAndGoto(page, '/settings');
    await expect(page.getByTestId('integrations')).toContainText('1 commande(s) en ligne à terminer.');
    await expect(page.getByRole('button', { name: /En ligne/ })).toBeVisible();
  });
});
