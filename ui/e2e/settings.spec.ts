import { test, expect } from '@playwright/test'
import {
  mockTauriIPC,
  DEFAULT_USER,
  ipcCalls,
  loginAndGoto,
  overrideIPC,
  live,
  shiftsInit,
  fdmHealth,
  fdmMocks,
  settingsWith,
} from './fixtures/mock-ipc'

test.describe('Settings View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    })
    await page.goto('/#/settings')
  })

  test('shows training mode toggle', async ({ page }) => {
    await expect(page.locator('.toggle')).toBeVisible()
  })

  test('shows language switcher with 3 options', async ({ page }) => {
    await expect(page.locator('.lang-btn')).toHaveText(['Français', '中文', 'English'])
    await expect(page.locator('.lang-btn.active')).toHaveText('Français')
  })

  test('products are shown by their menu code by default, names are a switch away', async ({
    page,
  }) => {
    await expect(page.locator('.label-btn')).toHaveText(['Nom', 'Code'])
    await expect(page.locator('.label-btn.active')).toHaveText('Code')
    await page.locator('.label-btn', { hasText: 'Nom' }).click()
    await expect(page.locator('.label-btn.active')).toHaveText('Nom')
    await page.reload()
    await expect(page.locator('.label-btn.active')).toHaveText('Nom')
  })

  test('a manager sets the table plan as ranges', async ({ page }) => {
    await loginAndGoto(page, '/settings')
    const panel = page.locator('.table-plan')
    await expect(panel.locator('h2')).toHaveText('Plan de salle')
    await expect(panel.locator('.table-plan-current')).toHaveText('20 tables : 1-20')
    await overrideIPC(page, 'set_table_plan', { plan: '1-12, 20-25', table_count: 18, log: [] })
    await panel.locator('input').fill('20-25,1-12')
    await panel.locator('.table-plan-save').click()
    await expect(panel.locator('.table-plan-current')).toHaveText('18 tables : 1-12, 20-25')
    await expect(panel.locator('input')).toHaveValue('1-12, 20-25')
  })

  test('a wrong table plan says how to write it', async ({ page }) => {
    await loginAndGoto(page, '/settings')
    const panel = page.locator('.table-plan')
    await overrideIPC(page, 'set_table_plan', { __reject: 'table_plan_invalid' })
    await panel.locator('input').fill('A1')
    await panel.locator('.table-plan-save').click()
    await expect(panel.locator('.add-error')).toContainText('1-12, 20-25')
  })

  test('an employee without a PIN is flagged', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [
        DEFAULT_USER,
        { ...DEFAULT_USER, insz: '99012345667', name: 'Wei', role: 'cashier', has_pin: false },
      ],
    })
    await page.reload()
    await expect(
      page.locator('.worker-row', { hasText: 'Bruno Moyse' }).locator('.worker-meta'),
    ).toHaveText('Gérant · PIN défini')
    const meta = page.locator('.worker-row', { hasText: 'Wei' }).locator('.worker-meta')
    await expect(meta).toHaveText('Serveur · sans PIN')
    await expect(meta).toHaveClass(/no-pin/)
  })

  test('shows worker list with test user', async ({ page }) => {
    await expect(page.locator('.worker-name')).toContainText('Bruno Moyse')
  })

  test('shows add worker form with i18n placeholders', async ({ page }) => {
    const nameInput = page.locator('input').first()
    await expect(nameInput).toHaveAttribute('placeholder', 'Nom')
  })

  test('a new user gets the chosen role (Gérant / Serveur / Apprenti)', async ({ page }) => {
    await expect(page.getByRole('radio', { name: 'Serveur' })).toHaveAttribute(
      'aria-checked',
      'true',
    )
    await page.getByPlaceholder('Nom').fill('Nouvel apprenti')
    await page.locator('input[inputmode="numeric"]').fill('99012345667')
    await page.getByRole('radio', { name: 'Apprenti' }).click()
    await page.getByRole('button', { name: 'Ajouter l’employé' }).click()
    await expect
      .poll(() => ipcCalls(page, 'create_user'))
      .toEqual([{ input: { insz: '99012345667', name: 'Nouvel apprenti', role: 'waiter' } }])
  })

  test('a new manager gets a PIN, confirmed by the manager', async ({ page }) => {
    await loginAndGoto(page, '/settings')
    await page.getByPlaceholder('Nom').fill('Gérant 2')
    await page.locator('input[inputmode="numeric"]').fill('99012345667')
    await page.getByRole('radio', { name: 'Gérant' }).click()
    await page.getByRole('button', { name: 'Ajouter l’employé' }).click()
    await expect(page.locator('.pin-dialog .who')).toHaveText('Gérant 2')
    for (const pin of ['2468', '2468']) {
      for (const d of pin)
        await page.locator('.pin-dialog').getByRole('button', { name: d, exact: true }).click()
    }
    await expect(page.locator('.pin-dialog .who')).toHaveText('Bruno Moyse')
    for (const d of '4321')
      await page.locator('.pin-dialog').getByRole('button', { name: d, exact: true }).click()
    await expect
      .poll(() => ipcCalls(page, 'create_user'))
      .toEqual([
        {
          input: {
            insz: '99012345667',
            name: 'Gérant 2',
            role: 'admin',
            initialPin: '2468',
            operatorPin: '4321',
          },
        },
      ])
  })

  test('deactivating a user takes two taps', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [
        DEFAULT_USER,
        { ...DEFAULT_USER, insz: '99012345667', name: 'Parti', role: 'waiter' },
      ],
      deactivate_user: () => undefined,
    })
    // beforeEach already opened /#/settings: reload so the new mock installs.
    await page.reload()
    const row = page.locator('.worker-row', { hasText: 'Parti' })
    await row.getByRole('button', { name: 'Désactiver' }).click()
    expect(await ipcCalls(page, 'deactivate_user')).toEqual([])
    await row.getByRole('button', { name: 'Confirmer ?' }).click()
    await expect.poll(() => ipcCalls(page, 'deactivate_user')).toEqual([{ insz: '99012345667' }])
  })

  test('training mode is refused while an order is open, and says so', async ({ page }) => {
    await overrideIPC(page, 'set_training_mode', {
      __reject: 'open_orders_training',
    })
    await page.locator('.toggle').click()
    await expect(page.locator('.toast')).toContainText('Encaissez ou videz les commandes ouvertes')
    await expect(page.locator('.toggle')).toContainText('Inactif')
  })

  test('shows POS info section', async ({ page }) => {
    await expect(page.getByText('CPOS0031234567')).toBeVisible()
    await expect(page.getByText('POS-1-BAR')).toBeVisible()
  })

  test('shows HubRise section', async ({ page }) => {
    // HubRise heading may be below the fold - scroll to it.
    const heading = page.getByRole('heading', { name: 'HubRise' })
    await heading.scrollIntoViewIfNeeded()
    await expect(heading).toBeVisible()
  })

  test('shows HubRise disconnected status', async ({ page }) => {
    // The `.status-off` span renders inside HubRise section
    const statusSpan = page.locator('.status-off')
    await expect(statusSpan).toBeVisible()
  })

  test('HubRise OAuth flow - authorize + paste code + connected', async ({ page }) => {
    // 1. Start disconnected
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      hubrise_get_connection_status: () => ({ connected: false }),
      hubrise_start_oauth_flow: () => ({
        url: 'https://manager.hubrise.com/oauth2/v1/authorize?client_id=test',
        qr_svg:
          '<svg xmlns="http://www.w3.org/2000/svg" data-qr="1" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>',
      }),
      // The catalog comes with the connection: the panel shows it at once.
      hubrise_status: live(() => ({
        connected: !!(window as any).__hubriseConnected,
        outbox: { pending: 0, parked: 0 },
        parked_events: 0,
        catalog: (window as any).__hubriseConnected
          ? {
              source: 'hubrise',
              manages_menu: true,
              applied_at: '2026-10-03 09:30:00',
              rejected_at: null,
              issues: [],
            }
          : null,
        catalog_awaiting_adoption: false,
      })),
      hubrise_complete_oauth: live(
        () =>
          ((window as any).__hubriseConnected = true) && {
            connected: true,
            account_id: 'acc-1',
            account_name: 'Restaurant Exemple',
            location_id: 'loc-1',
            location_name: 'Liège',
            catalog_id: 'cat-1',
            catalog_name: 'Menu principal',
            customer_list_id: null,
            customer_list_name: null,
          },
      ),
    })
    await page.goto('/#/settings')

    // 2. "Se connecter à HubRise": the kiosk has no browser, so the
    // login link shows as a QR code, with the URL spelled out.
    const authorizeBtn = page.getByText('Se connecter à HubRise')
    await authorizeBtn.scrollIntoViewIfNeeded()
    await authorizeBtn.click()
    // A dialog over the screen: the HubRise card is too short for it.
    const dialog = page.getByRole('dialog', { name: 'Se connecter à HubRise' })
    await expect(dialog.locator('svg[data-qr]')).toBeVisible()
    await expect(dialog).toContainText('Scannez ce code avec un téléphone')
    await expect(dialog).toContainText(
      'https://manager.hubrise.com/oauth2/v1/authorize?client_id=test',
    )

    // 3. No code, no connection; type the code HubRise shows.
    const completeBtn = dialog.getByRole('button', { name: 'Valider la connexion' })
    await expect(completeBtn).toBeDisabled()
    await dialog.getByPlaceholder("Code d'autorisation").fill('test-auth-code')
    await completeBtn.click()

    // 4. Connected: the dialog closes, the status shows.
    await expect(dialog).toBeHidden()
    await expect(page.locator('.status-ok')).toBeVisible()
    await expect(page.getByTestId('menu-managed')).toContainText('Menu géré par HubRise')
  })

  test('a refused HubRise code keeps the dialog open with its error', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      hubrise_get_connection_status: () => ({ connected: false }),
      hubrise_start_oauth_flow: () => ({
        url: 'https://manager.hubrise.com/oauth2/v1/authorize?client_id=test',
        qr_svg:
          '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>',
      }),
    })
    await page.goto('/#/settings')
    await overrideIPC(page, 'hubrise_complete_oauth', { __reject: 'hubrise_secret_unreadable' })
    const btn = page.getByText('Se connecter à HubRise')
    await btn.scrollIntoViewIfNeeded()
    await btn.click()
    const dialog = page.getByRole('dialog', { name: 'Se connecter à HubRise' })
    await dialog.getByPlaceholder("Code d'autorisation").fill('bad-code')
    await dialog.getByRole('button', { name: 'Valider la connexion' }).click()
    await expect(dialog).toContainText('Le secret HubRise est illisible')
    // "Retour" closes it; nothing is connected.
    await dialog.getByRole('button', { name: 'Retour' }).click()
    await expect(dialog).toBeHidden()
    await expect(page.locator('.status-ok')).toHaveCount(0)
  })

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
    })
    // beforeEach already opened /#/settings, so a goto to the same hash
    // would not reload the page and the new mock would never install.
    await page.reload()
    await expect(page.locator('.status-ok')).toBeVisible()
  })
})

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
}

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
  catalog: {
    source: 'hubrise',
    manages_menu: true,
    applied_at: '2026-10-03 09:30:00',
    rejected_at: null,
    issues: [],
  },
  catalog_awaiting_adoption: false,
}

async function hubriseSettings(
  page: import('@playwright/test').Page,
  overrides: Record<string, unknown>,
) {
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
  })
  // The manager's sections need a logged-in manager.
  await loginAndGoto(page, '/settings')
}

test.describe('Settings - HubRise catalog and channels', () => {
  test('the menu is shown as managed by HubRise, with its last sync', async ({ page }) => {
    await hubriseSettings(page, {})
    await expect(page.getByTestId('menu-managed')).toContainText('Menu géré par HubRise')
    await expect(page.getByTestId('catalog-synced')).toContainText('Synchronisé')
  })

  test('a refused catalog lists its issues', async ({ page }) => {
    await hubriseSettings(page, {
      hubrise_status: () => ({
        ...HEALTHY,
        catalog: {
          ...HEALTHY.catalog,
          rejected_at: '2026-10-03 10:00:00',
          issues: [
            { code: 'unknown_vat_rate', key: 'MAKI-6' },
            { code: 'missing_name', key: 'COCA' },
          ],
        },
      }),
    })
    const refused = page.getByTestId('catalog-refused')
    await expect(refused).toContainText('Catalogue refusé')
    await expect(refused).toContainText('Taux de TVA inconnu')
    await expect(refused).toContainText('MAKI-6')
    await expect(page.locator('.lights .light', { hasText: 'HubRise' })).toHaveClass(/err/)
  })

  test('Resynchroniser fetches the catalog', async ({ page }) => {
    await hubriseSettings(page, { hubrise_sync_catalog: () => null })
    await page.locator('.catalog-row').getByRole('button', { name: 'Resynchroniser' }).click()
    expect(await ipcCalls(page, 'hubrise_sync_catalog')).toHaveLength(1)
    await expect(page.getByText('Catalogue synchronisé')).toBeVisible()
  })

  test('a refused resync says so and lists the issues it came with', async ({ page }) => {
    await hubriseSettings(page, {
      // The engine answers the rejection (it is not an error) and the
      // health read after it shows the stored issues.
      hubrise_sync_catalog: live(() => {
        ;(window as any).__catalogRefused = true
        return { outcome: 'rejected', issues: [{ code: 'unknown_vat_rate', key: 'MAKI-6' }] }
      }),
      hubrise_status: live(() => ({
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
        catalog: (window as any).__catalogRefused
          ? {
              source: 'hubrise',
              manages_menu: true,
              applied_at: '2026-10-03 09:30:00',
              rejected_at: '2026-10-03 10:05:00',
              issues: [{ code: 'unknown_vat_rate', key: 'MAKI-6' }],
            }
          : {
              source: 'hubrise',
              manages_menu: true,
              applied_at: '2026-10-03 09:30:00',
              rejected_at: null,
              issues: [],
            },
        catalog_awaiting_adoption: false,
      })),
    })
    await expect(page.getByTestId('catalog-refused')).toHaveCount(0)
    await page.locator('.catalog-row').getByRole('button', { name: 'Resynchroniser' }).click()
    expect(await ipcCalls(page, 'hubrise_sync_catalog')).toHaveLength(1)
    await expect(page.locator('.toast-error', { hasText: 'Catalogue refusé' })).toBeVisible()
    await expect(page.getByText('Catalogue synchronisé')).toHaveCount(0)
    const refused = page.getByTestId('catalog-refused')
    await expect(refused).toContainText('Taux de TVA inconnu')
    await expect(refused).toContainText('MAKI-6')
  })

  test('the first catalog waits for the manager to pick the products to keep', async ({ page }) => {
    await hubriseSettings(page, {
      hubrise_status: () => ({ ...HEALTHY, catalog: null, catalog_awaiting_adoption: true }),
      hubrise_adoption_preview: () => [
        { menu_item_id: 1, key: 'MAKI-6', name: 'Maki saumon', price_cents: 650 },
        { menu_item_id: 2, key: 'COCA', name: 'Coca-Cola', price_cents: 300 },
      ],
      hubrise_adopt_catalog: () => ({
        outcome: 'applied',
        hash: 'h',
        active_items: 2,
        deactivated_items: 0,
      }),
    })
    const list = page.getByTestId('adoptions')
    await expect(list.locator('input[type=checkbox]:checked')).toHaveCount(2)
    await list.locator('label', { hasText: 'Coca-Cola' }).locator('input').uncheck()
    await page.getByRole('button', { name: 'Reprendre le menu' }).click()
    const calls = await ipcCalls(page, 'hubrise_adopt_catalog')
    expect(calls[0].adoptions).toEqual([
      { menu_item_id: 1, key: 'MAKI-6', name: 'Maki saumon', price_cents: 650 },
    ])
  })

  test('a channel is added as a platform and its kind changes its payment', async ({ page }) => {
    await hubriseSettings(page, { hubrise_set_channel: () => 1 })
    const channels = page.getByTestId('channels')
    await expect(channels.locator('.channel')).toHaveCount(2)
    await page.locator('input[placeholder="Nom du canal"]').fill('Table QR')
    await page.getByRole('button', { name: 'Ajouter un canal' }).click()
    await channels
      .locator('.channel', { hasText: 'website' })
      .locator('select')
      .first()
      .selectOption('platform')
    const calls = await ipcCalls(page, 'hubrise_set_channel')
    expect(calls[0].channel).toEqual({
      channel: 'Table QR',
      kind: 'platform',
      payment_type: 'APP',
      charge_policy: 'exclude',
    })
    expect(calls[1].channel).toEqual({
      channel: 'website',
      kind: 'platform',
      payment_type: 'APP',
      charge_policy: 'include',
    })
    await expect(page.getByText('1 commande(s) débloquée(s)').first()).toBeVisible()
  })

  test('a refused token turns the light red and says so', async ({ page }) => {
    await hubriseSettings(page, {
      hubrise_status: () => ({ ...HEALTHY, auth_error_at: '2026-10-03 10:00:00' }),
    })
    await expect(page.getByText('HubRise refuse la connexion')).toBeVisible()
    await expect(page.locator('.lights .light', { hasText: 'HubRise' })).toHaveClass(/err/)
  })
})

test.describe('Settings - the HubRise switch', () => {
  test('a manager switches HubRise off: the panel and the board go, the change is logged', async ({
    page,
  }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] })
    await loginAndGoto(page, '/settings')
    const panel = page.getByTestId('integrations')
    await expect(panel.locator('.pill')).toHaveText('Activé')
    await expect(page.locator('.hubrise-section')).toBeVisible()
    await expect(page.getByRole('button', { name: /En ligne/ })).toBeVisible()

    await panel.getByRole('button', { name: 'Désactiver' }).click()
    await expect(panel.locator('.pill')).toHaveText('Désactivé')
    expect((await ipcCalls(page, 'set_hubrise_enabled'))[0]).toMatchObject({ enabled: false })
    await expect(panel.locator('.log li').first()).toContainText('gérant 85073003328')
    await expect(page.locator('.hubrise-section')).toHaveCount(0)
    await expect(page.getByRole('button', { name: /En ligne/ })).toHaveCount(0)
  })

  test('switching off waits for the online orders: the refusal says how many', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] })
    await loginAndGoto(page, '/settings')
    await overrideIPC(page, 'set_hubrise_enabled', {
      __reject: 'hubrise_online_orders_open|count=2',
    })
    await page.getByTestId('integrations').getByRole('button', { name: 'Désactiver' }).click()
    await expect(
      page.getByText("Terminez d'abord les 2 commande(s) en ligne en cours."),
    ).toBeVisible()
    await expect(page.getByTestId('integrations').locator('.pill')).toHaveText('Activé')
  })

  test('with HubRise off the online board is out of reach', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_integrations: () => ({ hubrise_enabled: false, online_orders_open: 0, hubrise_log: [] }),
    })
    await loginAndGoto(page, '/webshop-orders')
    await expect(page).toHaveURL(/#\/tables/)
    await expect(page.getByRole('button', { name: /En ligne/ })).toHaveCount(0)
  })

  test('off with online orders still to settle, the board stays reachable', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_integrations: () => ({ hubrise_enabled: false, online_orders_open: 1, hubrise_log: [] }),
    })
    await loginAndGoto(page, '/settings')
    await expect(page.getByTestId('integrations')).toContainText(
      '1 commande(s) en ligne à terminer.',
    )
    await expect(page.getByRole('button', { name: /En ligne/ })).toBeVisible()
  })
  test('the alerts, courses and gift voucher panels are for a manager only', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' }
    await mockTauriIPC(page, { login: () => cashier, list_users: () => [cashier] })
    await loginAndGoto(page, '/settings')
    await expect(page.locator('.workers-section')).toBeVisible()
    await expect(page.getByTestId('alerts-panel')).toHaveCount(0)
    await expect(page.getByTestId('courses-panel')).toHaveCount(0)
    await expect(page.getByTestId('gv-settings')).toHaveCount(0)
    // The panels load their settings when they mount: none did.
    expect(await ipcCalls(page, 'get_owner_alerts')).toHaveLength(0)
    expect(await ipcCalls(page, 'list_owner_alert_log')).toHaveLength(0)
    expect(await ipcCalls(page, 'get_course_settings')).toHaveLength(0)
  })

  test('a manager sees the alerts, courses and gift voucher panels', async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] })
    await loginAndGoto(page, '/settings')
    await expect(page.getByTestId('alerts-panel')).toBeVisible()
    await expect(page.getByTestId('courses-panel')).toBeVisible()
    await expect(page.getByTestId('gv-settings')).toBeVisible()
    expect(await ipcCalls(page, 'get_owner_alerts')).toHaveLength(1)
    expect(await ipcCalls(page, 'get_course_settings')).not.toHaveLength(0)
    expect(await ipcCalls(page, 'get_gift_voucher_settings')).not.toHaveLength(0)
  })

  test('the employees stay in view beside the courses and gift voucher panels', async ({
    page,
  }) => {
    const users = ['Bruno Moyse', 'Wei', 'Lea', 'Tom'].map((name, i) => ({
      ...DEFAULT_USER,
      insz: `9906024257${i}`,
      name,
      role: i === 0 ? 'admin' : 'cashier',
    }))
    await mockTauriIPC(page, { login: () => users[0], list_users: () => users })
    await loginAndGoto(page, '/settings')
    await expect(page.getByTestId('courses-panel')).toBeVisible()
    await expect(page.getByTestId('gv-settings')).toBeVisible()
    const rows = page.locator('.worker-row')
    await expect(rows).toHaveCount(4)
    await expect(rows.first()).toBeInViewport()
    await expect(page.locator('.add-worker .add-btn')).toBeInViewport()
    // The list keeps room for at least one row and scrolls inside its panel
    // (nothing squeezes it to zero when the manager panels are drawn).
    const listBox = await page.locator('.worker-list').boundingBox()
    expect(listBox && listBox.height).toBeGreaterThanOrEqual(72)
    await rows.last().scrollIntoViewIfNeeded()
    await expect(rows.last()).toBeInViewport()
    // The manager panels share the column with the list, below it.
    const list = await page.locator('.workers-section').boundingBox()
    const panels = await page.locator('.manager-panels').boundingBox()
    expect(list && panels && panels.y >= list.y + list.height - 1).toBeTruthy()
  })
})

test.describe('Presence registration by the manager (event S)', () => {
  const WEI = { insz: '99012345667', name: 'Wei', role: 'cashier', active: true, has_pin: true }
  const COOK = {
    insz: '88010112345',
    name: 'Chen',
    role: 'waiter',
    active: true,
    has_pin: false,
    presence_only: true,
  }
  const row = (page: import('@playwright/test').Page, name: string) =>
    page.locator('.worker-row', { hasText: name })

  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER, WEI, COOK] })
  })

  test('the panel says who is on shift and who is not, and offers the right button', async ({
    page,
  }) => {
    await shiftsInit(page, [{ userInsz: WEI.insz, startedAt: '2026-10-17T12:02:00+02:00' }])
    await loginAndGoto(page, '/settings')
    await expect(page.locator('.workers-section .panel-text')).toContainText(
      'Personnel sans accès à la caisse',
    )
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText(
      'En service depuis 12:02',
    )
    await expect(row(page, 'Wei').getByTestId('presence-btn')).toHaveText('Départ')
    await expect(row(page, 'Chen').getByTestId('presence-state')).toHaveText('Hors service')
    await expect(row(page, 'Chen').getByTestId('presence-btn')).toHaveText('Arrivée')
    // The operator's own row has no presence button.
    await expect(row(page, 'Bruno').getByTestId('presence-btn')).toHaveCount(0)
  })

  test('clock in then clock out, with a toast each time and the state refreshed', async ({
    page,
  }) => {
    await loginAndGoto(page, '/settings')
    await row(page, 'Chen').getByTestId('presence-btn').click()
    await expect(page.locator('.toast')).toContainText('Arrivée enregistrée · Chen')
    await expect
      .poll(() => ipcCalls(page, 'clock_in_employee'))
      .toEqual([{ employeeInsz: COOK.insz }])
    await expect(row(page, 'Chen').getByTestId('presence-state')).toContainText('En service depuis')
    await row(page, 'Chen').getByTestId('presence-btn').click()
    await expect(page.locator('.toast', { hasText: 'Départ enregistré · Chen' })).toBeVisible()
    await expect
      .poll(() => ipcCalls(page, 'clock_out_employee'))
      .toEqual([{ employeeInsz: COOK.insz }])
    await expect(row(page, 'Chen').getByTestId('presence-state')).toHaveText('Hors service')
  })

  test('an FDM failure is shown and the state does not change', async ({ page }) => {
    await loginAndGoto(page, '/settings')
    await overrideIPC(page, 'clock_in_employee', { success: false, error: 'FDM injoignable' })
    await row(page, 'Wei').getByTestId('presence-btn').click()
    // The FDM error goes through the §V mapping.
    await expect(page.locator('.toast')).toContainText('Erreur boîte noire')
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText('Hors service')
  })

  test('a refusal code is shown as its text', async ({ page }) => {
    await loginAndGoto(page, '/settings')
    await overrideIPC(page, 'clock_in_employee', { __reject: 'presence_already_in' })
    await row(page, 'Wei').getByTestId('presence-btn').click()
    await expect(page.locator('.toast')).toContainText('Cet employé est déjà en service')
  })

  test('the buttons are off while the FDM is not connected', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER, WEI, COOK],
      ...fdmMocks(fdmHealth({ connected: false, severity: 'blocking', code: 'FDM_DOWN' })),
    })
    await loginAndGoto(page, '/settings')
    await expect(page.getByTestId('presence-block')).toHaveText(
      'FDM indisponible : pointage impossible',
    )
    await expect(row(page, 'Wei').getByTestId('presence-btn')).toBeDisabled()
    await expect(row(page, 'Chen').getByTestId('presence-btn')).toBeDisabled()
  })

  test('the buttons are off in training mode, with the reason', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER, WEI, COOK],
      get_settings: settingsWith({ trainingMode: true }),
    })
    await loginAndGoto(page, '/settings')
    await expect(page.getByTestId('presence-block')).toContainText('Mode formation : le pointage')
    await expect(row(page, 'Wei').getByTestId('presence-btn')).toBeDisabled()
  })

  test('a presence-only row has its own label, no PIN button, and keeps Désactiver', async ({
    page,
  }) => {
    await loginAndGoto(page, '/settings')
    const cook = row(page, 'Chen')
    await expect(cook.locator('.worker-meta')).toHaveText('Présence seule')
    await expect(cook.getByRole('button', { name: 'Code PIN' })).toHaveCount(0)
    await expect(cook.getByRole('button', { name: 'Désactiver' })).toBeVisible()
    await expect(row(page, 'Wei').getByRole('button', { name: 'Code PIN' })).toBeVisible()
  })

  test('a presence-only employee is added without a PIN dialog', async ({ page }) => {
    await loginAndGoto(page, '/settings')
    await page.getByPlaceholder('Nom').fill('Plongeur')
    await page.locator('input[inputmode="numeric"]').fill('77010112345')
    await page.getByRole('radio', { name: 'Présence seule' }).click()
    await page.getByRole('button', { name: 'Ajouter l’employé' }).click()
    await expect
      .poll(() => ipcCalls(page, 'create_user'))
      .toEqual([
        { input: { insz: '77010112345', name: 'Plongeur', role: 'waiter', presenceOnly: true } },
      ])
    await expect(page.locator('.pin-dialog')).toHaveCount(0)
  })
  test('a cashier sees no presence button and no presence note, but still the state line', async ({
    page,
  }) => {
    const CASHIER = {
      ...DEFAULT_USER,
      insz: '12345678958',
      name: 'Lena',
      role: 'cashier',
      has_pin: true,
    }
    await mockTauriIPC(page, {
      login: () => CASHIER,
      list_users: () => [CASHIER, WEI, COOK],
      ...fdmMocks(fdmHealth({ connected: false, severity: 'blocking', code: 'FDM_DOWN' })),
    })
    await shiftsInit(page, [{ userInsz: WEI.insz, startedAt: '2026-10-17T12:02:00+02:00' }])
    await loginAndGoto(page, '/settings')
    await expect(row(page, 'Wei')).toBeVisible()
    await expect(page.getByTestId('presence-btn')).toHaveCount(0)
    await expect(page.getByTestId('presence-block')).toHaveCount(0)
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText(
      'En service depuis 12:02',
    )
    await expect(row(page, 'Chen').getByTestId('presence-state')).toHaveText('Hors service')
  })

  test('one presence call at a time: every button is off while one runs, a second tap is ignored', async ({
    page,
  }) => {
    await loginAndGoto(page, '/settings')
    // Hold the clock-in until the test releases it.
    await page.evaluate(() => {
      const w = window as any
      const tauri = w.__TAURI_INTERNALS__
      const prev = tauri.invoke
      tauri.invoke = (cmd: string, args?: any) => {
        if (cmd !== 'clock_in_employee') return prev(cmd, args)
        w.__ipcCalls.push({ cmd, args: args ?? {} })
        return new Promise((resolve) => {
          w.__release = () => {
            w.__shifts = (w.__shifts ?? []).concat([
              { userInsz: args.employeeInsz, startedAt: '2026-10-17T12:02:00+02:00' },
            ])
            resolve({ success: true, error: null })
          }
        })
      }
    })
    await row(page, 'Chen').getByTestId('presence-btn').click()
    await expect(row(page, 'Chen').getByTestId('presence-btn')).toBeDisabled()
    await expect(row(page, 'Wei').getByTestId('presence-btn')).toBeDisabled()
    // A tap on the disabled button does nothing.
    await row(page, 'Wei').getByTestId('presence-btn').click({ force: true })
    expect(await ipcCalls(page, 'clock_in_employee')).toEqual([{ employeeInsz: COOK.insz }])
    await page.evaluate(() => (window as any).__release())
    await expect(page.locator('.toast')).toContainText('Arrivée enregistrée · Chen')
    await expect(row(page, 'Wei').getByTestId('presence-btn')).toBeEnabled()
    await expect(row(page, 'Chen').getByTestId('presence-btn')).toBeEnabled()
    expect(await ipcCalls(page, 'clock_in_employee')).toEqual([{ employeeInsz: COOK.insz }])
  })

  test('adding or deactivating an employee re-reads the open shifts and keeps them', async ({
    page,
  }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER, WEI, COOK],
      deactivate_user: () => undefined,
    })
    await shiftsInit(page, [{ userInsz: WEI.insz, startedAt: '2026-10-17T12:02:00+02:00' }])
    await loginAndGoto(page, '/settings')
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText(
      'En service depuis 12:02',
    )
    const afterLoad = (await ipcCalls(page, 'list_open_shifts')).length

    await page.getByPlaceholder('Nom').fill('Plongeur')
    await page.locator('input[inputmode="numeric"]').fill('77010112345')
    await page.getByRole('radio', { name: 'Présence seule' }).click()
    await page.getByRole('button', { name: 'Ajouter l’employé' }).click()
    await expect.poll(() => ipcCalls(page, 'create_user')).toHaveLength(1)
    await expect
      .poll(async () => (await ipcCalls(page, 'list_open_shifts')).length)
      .toBe(afterLoad + 1)
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText(
      'En service depuis 12:02',
    )

    await row(page, 'Chen').getByRole('button', { name: 'Désactiver' }).click()
    await row(page, 'Chen').getByRole('button', { name: 'Confirmer ?' }).click()
    await expect.poll(() => ipcCalls(page, 'deactivate_user')).toEqual([{ insz: COOK.insz }])
    await expect
      .poll(async () => (await ipcCalls(page, 'list_open_shifts')).length)
      .toBe(afterLoad + 2)
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText(
      'En service depuis 12:02',
    )
  })

  test('deactivating an employee on shift is refused with its text and the row stays', async ({
    page,
  }) => {
    await shiftsInit(page, [{ userInsz: WEI.insz, startedAt: '2026-10-17T12:02:00+02:00' }])
    await loginAndGoto(page, '/settings')
    await overrideIPC(page, 'deactivate_user', { __reject: 'deactivate_on_shift' })
    await row(page, 'Wei').getByRole('button', { name: 'Désactiver' }).click()
    await row(page, 'Wei').getByRole('button', { name: 'Confirmer ?' }).click()
    await expect(page.locator('.add-error')).toHaveText(
      "Cet employé est en service : enregistrez d'abord son départ",
    )
    await expect(row(page, 'Wei')).toBeVisible()
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText(
      'En service depuis 12:02',
    )
  })

  test('the presence texts follow the language (Chinese)', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'zh'))
    await shiftsInit(page, [{ userInsz: WEI.insz, startedAt: '2026-10-17T12:02:00+02:00' }])
    await loginAndGoto(page, '/settings')
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText('12:02 起上班中')
    await expect(row(page, 'Chen').getByTestId('presence-state')).toHaveText('未上班')
    await expect(row(page, 'Wei').getByTestId('presence-btn')).toHaveText('下班打卡')
    await expect(row(page, 'Chen').getByTestId('presence-btn')).toHaveText('上班打卡')
    await expect(row(page, 'Chen').locator('.worker-meta')).toHaveText('仅考勤')
    await row(page, 'Chen').getByTestId('presence-btn').click()
    await expect(page.locator('.toast')).toContainText('已登记上班 · Chen')
  })

  test('the presence texts follow the language (English)', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'en'))
    await shiftsInit(page, [{ userInsz: WEI.insz, startedAt: '2026-10-17T12:02:00+02:00' }])
    await loginAndGoto(page, '/settings')
    await expect(row(page, 'Wei').getByTestId('presence-state')).toHaveText('On shift since 12:02')
    await expect(row(page, 'Chen').getByTestId('presence-state')).toHaveText('Off shift')
    await expect(row(page, 'Wei').getByTestId('presence-btn')).toHaveText('Clock out')
    await expect(row(page, 'Chen').getByTestId('presence-btn')).toHaveText('Clock in')
    await expect(row(page, 'Chen').locator('.worker-meta')).toHaveText('Presence only')
    await row(page, 'Chen').getByTestId('presence-btn').click()
    await expect(page.locator('.toast')).toContainText('Clock-in registered · Chen')
  })
})
