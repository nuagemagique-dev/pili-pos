import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  DEFAULT_USER,
  ipcCalls,
  loginAndGoto,
  overrideIPC,
  live,
} from './fixtures/mock-ipc'

// The bot token lives in a file named by pos.json: the panel only learns
// whether it is there (`token_configured`). `live` handlers cannot close
// over the test, so the flag travels in a page global.
async function openPanel(page: Page, tokenConfigured: boolean) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_owner_alerts: live(() => {
      const w = window as any
      return (
        w.__alerts ?? {
          enabled: false,
          chat_id: '',
          label: '',
          language: 'fr',
          kinds: {
            fdm: true,
            printer: true,
            terminal: true,
            z_missing: true,
            online_waiting: true,
          },
          z_deadline_hour: 5,
          online_wait_minutes: 5,
          reminder_minutes: 60,
          token_configured: Boolean(w.__TOKEN),
        }
      )
    }),
    set_owner_alerts: live((args: any) => {
      const w = window as any
      w.__alerts = { ...args.input, token_configured: Boolean(w.__TOKEN) }
      return w.__alerts
    }),
  } as any)
  await page.addInitScript((t) => {
    ;(window as any).__TOKEN = t
  }, tokenConfigured)
  await loginAndGoto(page, '/settings')
  return page.locator('[data-testid="alerts-panel"]')
}

test.describe('Owner alerts panel', () => {
  test('is off by default and says the bot token is missing', async ({ page }) => {
    const panel = await openPanel(page, false)
    await expect(panel.locator('h2')).toHaveText('Alertes sur téléphone')
    await expect(panel.getByTestId('alerts-enabled')).toContainText('Inactives')
    await expect(panel.getByTestId('alerts-token')).toContainText('Robot Telegram absent')
    await expect(panel.getByTestId('alerts-test')).toBeDisabled()
    await expect(panel.getByTestId('alerts-log-empty')).toHaveText("Aucun message pour l'instant")
  })

  test('a manager turns the alerts on with a group chat id and saves', async ({ page }) => {
    const panel = await openPanel(page, true)
    await expect(panel.getByTestId('alerts-token')).toContainText('Robot Telegram configuré')
    await expect(panel.getByTestId('alerts-save')).toBeDisabled()

    await panel.getByTestId('alerts-enabled').click()
    await panel.getByTestId('alerts-chat-id').click()
    await page.getByTestId('number-entry-minus').click()
    for (const d of '10012345')
      await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
    await expect(page.getByTestId('number-entry-value')).toHaveText('-10012345')
    await page.getByTestId('number-entry-ok').click()
    await expect(panel.getByTestId('alerts-chat-id')).toHaveText('-10012345')

    await panel.getByTestId('alerts-lang-zh').click()
    await panel.getByTestId('alerts-kind-printer').click()
    await panel.getByTestId('alerts-label').fill('Salle 1')
    await panel.getByTestId('alerts-save').click()

    await expect(panel.getByTestId('alerts-saved')).toBeVisible()
    const calls = await ipcCalls(page, 'set_owner_alerts')
    expect(calls).toHaveLength(1)
    expect(calls[0].input).toMatchObject({
      enabled: true,
      chat_id: '-10012345',
      label: 'Salle 1',
      language: 'zh',
      kinds: { fdm: true, printer: false, terminal: true, z_missing: true, online_waiting: true },
      z_deadline_hour: 5,
      online_wait_minutes: 5,
      reminder_minutes: 60,
    })
  })

  test('numbers are entered on the keypad and kept in range', async ({ page }) => {
    const panel = await openPanel(page, true)
    await panel.getByTestId('alerts-online-wait').click()
    await expect(page.getByTestId('number-entry-minus')).toHaveCount(0)
    await page.getByTestId('number-entry-ok').click()
    // "5" is kept as it is: the dialog starts from the current value.
    await expect(panel.getByTestId('alerts-online-wait')).toHaveText('5 min')

    await panel.getByTestId('alerts-online-wait').click()
    await page.locator('.numpad .key-clear').click()
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    for (const d of '999')
      await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    await page.locator('.numpad .key-back').click()
    await page.locator('.numpad .key-back').click()
    await page.locator('.numpad .key', { hasText: /^2$/u }).click()
    await page.getByTestId('number-entry-ok').click()
    await expect(panel.getByTestId('alerts-online-wait')).toHaveText('92 min')

    await panel.getByTestId('alerts-reminder').click()
    await page.locator('.numpad .key-clear').click()
    await page.locator('.numpad .key', { hasText: /^0$/u }).click()
    await page.getByTestId('number-entry-ok').click()
    await expect(panel.getByTestId('alerts-reminder')).toHaveText('Jamais')
  })

  test('the test message goes out with the saved settings', async ({ page }) => {
    const panel = await openPanel(page, true)
    await overrideIPC(page, 'send_test_alert', { sent: true, detail: 'ok' })
    await overrideIPC(page, 'list_owner_alert_log', [
      {
        id: 2,
        kind: 'test',
        transition: 'test',
        detail: null,
        outcome: 'sent',
        http_status: 200,
        created_at: '2026-10-17 09:00:00',
      },
      {
        id: 1,
        kind: 'fdm',
        transition: 'raised',
        detail: null,
        outcome: 'suppressed',
        http_status: null,
        created_at: '2026-10-17 08:00:00',
      },
    ])
    await panel.getByTestId('alerts-chat-id').click()
    for (const d of '42')
      await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
    await page.getByTestId('number-entry-ok').click()
    await panel.getByTestId('alerts-test').click()

    await expect(panel.getByTestId('alerts-test-sent')).toHaveText('Message test envoyé')
    // The draft was saved first (an overridden command is not recorded).
    expect(await ipcCalls(page, 'set_owner_alerts')).toHaveLength(1)
    const rows = panel.locator('[data-testid="alerts-log"] .log-row')
    await expect(rows).toHaveCount(2)
    await expect(rows.nth(0)).toContainText('Test')
    await expect(rows.nth(0)).toContainText('Envoyé')
    await expect(rows.nth(1)).toContainText('Alerte')
    await expect(rows.nth(1)).toContainText('Retenu (limite horaire)')
  })

  test('a refused message says why', async ({ page }) => {
    const panel = await openPanel(page, true)
    await panel.getByTestId('alerts-chat-id').click()
    await page.locator('.numpad .key', { hasText: /^7$/u }).click()
    await page.getByTestId('number-entry-ok').click()
    await overrideIPC(page, 'send_test_alert', { __reject: 'alerts_send_failed|detail=HTTP 401' })
    await panel.getByTestId('alerts-test').click()
    await expect(panel.getByTestId('alerts-error')).toContainText('HTTP 401')
    await expect(panel.getByTestId('alerts-test-sent')).toHaveCount(0)
  })

  test('a setting the engine refuses is named', async ({ page }) => {
    const panel = await openPanel(page, true)
    await overrideIPC(page, 'set_owner_alerts', { __reject: 'alerts_invalid|field=chat_id' })
    await panel.getByTestId('alerts-enabled').click()
    await panel.getByTestId('alerts-save').click()
    await expect(panel.getByTestId('alerts-error')).toContainText('chat_id')
    await expect(panel.getByTestId('alerts-saved')).toHaveCount(0)
  })
  test('the Z deadline hour is kept between 0 and 12', async ({ page }) => {
    const panel = await openPanel(page, true)
    await expect(panel.getByTestId('alerts-z-hour')).toHaveText('5 h')
    await panel.getByTestId('alerts-z-hour').click()
    await page.locator('.numpad .key-clear').click()
    for (const d of '13')
      await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    await page.locator('.numpad .key-clear').click()
    await page.locator('.numpad .key', { hasText: /^7$/u }).click()
    await expect(page.getByTestId('number-entry-ok')).toBeEnabled()
    await page.getByTestId('number-entry-ok').click()
    await expect(panel.getByTestId('alerts-z-hour')).toHaveText('7 h')
    await panel.getByTestId('alerts-save').click()
    await expect(panel.getByTestId('alerts-saved')).toBeVisible()
    const calls = await ipcCalls(page, 'set_owner_alerts')
    expect(calls).toHaveLength(1)
    expect(calls[0].input.z_deadline_hour).toBe(7)
  })

  test('the test button needs a token and a chat id', async ({ page }) => {
    const panel = await openPanel(page, true)
    // The token is there, the chat id is not.
    await expect(panel.getByTestId('alerts-token')).toContainText('Robot Telegram configuré')
    await expect(panel.getByTestId('alerts-chat-id')).not.toHaveText(/\d/u)
    await expect(panel.getByTestId('alerts-test')).toBeDisabled()
    await panel.getByTestId('alerts-chat-id').click()
    await page.locator('.numpad .key', { hasText: /^5$/u }).click()
    await page.getByTestId('number-entry-ok').click()
    await expect(panel.getByTestId('alerts-test')).toBeEnabled()
  })

  for (const [code, text] of [
    ['config_alerts', 'Réglage des alertes propriétaire invalide'],
    ['alerts_token_missing', "Le jeton du robot Telegram n'est pas configuré"],
  ] as const) {
    test(`a refused read (${code}) shows the error and hides the form`, async ({ page }) => {
      await mockTauriIPC(page, {
        login: () => DEFAULT_USER,
        list_users: () => [DEFAULT_USER],
        get_owner_alerts: live(() => {
          throw (window as any).__REJECT
        }),
      } as any)
      await page.addInitScript((c) => {
        ;(window as any).__REJECT = c
      }, code)
      await loginAndGoto(page, '/settings')
      await expect(page.getByTestId('alerts-error')).toContainText(text)
      await expect(page.getByTestId('alerts-chat-id')).toHaveCount(0)
      expect(await ipcCalls(page, 'list_owner_alert_log')).toHaveLength(0)
    })
  }
})
