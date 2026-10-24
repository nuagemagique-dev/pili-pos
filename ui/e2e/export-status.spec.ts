import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  DEFAULT_USER,
  EXPORT_STATUS_DISABLED,
  ipcCalls,
  loginAndGoto,
  overrideIPC,
} from './fixtures/mock-ipc'

// The nightly export panel in Settings (manager only). The engine
// commands export_status / export_run_now are mocked.
const UP_TO_DATE = {
  ...EXPORT_STATUS_DISABLED,
  enabled: true,
  endpoint: 'https://backup.example.test/ingest',
  keepDays: 30,
  lastExportId: 'exp-2026-10-08',
  lastPreparedAt: '2026-10-08T21:58:00Z',
  lastUploadedId: 'exp-2026-10-08',
  lastUploadedAt: '2026-10-08T22:30:05Z',
  kept: 12,
}

const PENDING = {
  ...UP_TO_DATE,
  lastUploadedId: 'exp-2026-10-06',
  lastUploadedAt: '2026-10-06T22:30:05Z',
  pending: 2,
  nextAttemptAt: '2026-10-09T01:15:00Z',
  lastError: 'HTTP 503',
  lastErrorAt: '2026-10-09T00:45:00Z',
}

async function openSettings(page: Page, status: object | null, role = 'admin') {
  const user = { ...DEFAULT_USER, role }
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    ...(status ? { export_status: () => status } : {}),
  })
  await loginAndGoto(page, '/settings')
}

const panel = (page: Page) => page.getByTestId('export-panel')

test.describe('Nightly export panel', () => {
  test('not configured: the state only, no retry', async ({ page }) => {
    await openSettings(page, null)
    await expect(panel(page).getByRole('heading')).toHaveText('Export nocturne')
    await expect(panel(page)).toContainText('Non configuré')
    await expect(panel(page).getByRole('button')).toHaveCount(0)
    await expect(panel(page).locator('.rows')).toHaveCount(0)
  })

  test('invalid configuration shows the problem', async ({ page }) => {
    await openSettings(page, { ...EXPORT_STATUS_DISABLED, problem: 'endpoint must be https' })
    await expect(panel(page)).toContainText('Configuration invalide')
    await expect(panel(page)).toContainText('endpoint must be https')
    await expect(panel(page).getByRole('button')).toHaveCount(0)
  })

  test('up to date: last upload in Brussels time, no error', async ({ page }) => {
    await openSettings(page, UP_TO_DATE)
    await expect(panel(page)).toContainText('À jour')
    await expect(panel(page)).toContainText('https://backup.example.test/ingest')
    await expect(panel(page)).toContainText('exp-2026-10-08')
    // 22:30 UTC on the 8th is 00:30 on the 9th in Brussels (CEST).
    await expect(panel(page).locator('.row', { hasText: 'Dernier envoi' })).toContainText(
      /09\/10.*00:30/u,
    )
    await expect(panel(page)).toContainText('Conservés : 12 sur 30 jours')
    await expect(panel(page).locator('.error-row')).toHaveCount(0)
    await expect(panel(page)).not.toContainText('Abandonnés')
    await expect(panel(page).getByRole('button', { name: 'Réessayer maintenant' })).toBeVisible()
  })

  test('pending with an error shows both and the next attempt', async ({ page }) => {
    await openSettings(page, { ...PENDING, dropped: 1 })
    await expect(panel(page)).toContainText('2 exports en attente')
    await expect(panel(page).locator('.error-row')).toContainText('HTTP 503')
    await expect(panel(page).locator('.error-row')).toContainText(/09\/10.*02:45/u)
    await expect(panel(page).locator('.row', { hasText: 'Prochain essai' })).toContainText(
      /09\/10.*03:15/u,
    )
    await expect(panel(page).locator('.row', { hasText: 'Abandonnés' })).toContainText('1')
  })

  test('Retry now asks the engine, then shows the new status', async ({ page }) => {
    await openSettings(page, PENDING)
    await expect(panel(page)).toContainText('2 exports en attente')
    // The engine answers at once; the status read after it is the new one.
    await overrideIPC(page, 'export_status', UP_TO_DATE)
    await panel(page).getByRole('button', { name: 'Réessayer maintenant' }).click()
    await expect.poll(() => ipcCalls(page, 'export_run_now')).toEqual([{}])
    await expect(panel(page)).toContainText('À jour')
    await expect(panel(page)).not.toContainText('HTTP 503')
    await expect(panel(page).getByRole('button', { name: 'Réessayer maintenant' })).toBeEnabled()
  })

  test('the status is read again every 30 seconds', async ({ page }) => {
    await page.clock.install()
    await openSettings(page, UP_TO_DATE)
    await expect(panel(page)).toContainText('À jour')
    const before = (await ipcCalls(page, 'export_status')).length
    expect(before).toBeGreaterThanOrEqual(1)
    await page.clock.fastForward(31_000)
    await expect
      .poll(async () => (await ipcCalls(page, 'export_status')).length)
      .toBeGreaterThan(before)
  })

  test('a refused call shows the engine error with the existing presenter', async ({ page }) => {
    await openSettings(page, PENDING)
    await overrideIPC(page, 'export_run_now', { __reject: 'forbidden' })
    await panel(page).getByRole('button', { name: 'Réessayer maintenant' }).click()
    await expect(panel(page).getByRole('alert')).toHaveText('Action réservée à un autre profil.')
    // The last known status stays on screen.
    await expect(panel(page)).toContainText('2 exports en attente')
  })

  test('absent for a manager-less session', async ({ page }) => {
    await openSettings(page, PENDING, 'cashier')
    await expect(page.locator('.worker-name').first()).toBeVisible()
    await expect(panel(page)).toHaveCount(0)
    expect(await ipcCalls(page, 'export_status')).toEqual([])
  })

  test('English and Chinese titles after switching the language', async ({ page }) => {
    await openSettings(page, PENDING)
    await page.locator('.lang-btn', { hasText: 'English' }).click()
    await expect(panel(page).getByRole('heading')).toHaveText('Nightly export')
    await expect(panel(page)).toContainText('2 exports waiting')
    await expect(panel(page).getByRole('button', { name: 'Retry now' })).toBeVisible()
    await page.locator('.lang-btn', { hasText: '中文' }).click()
    await expect(panel(page).getByRole('heading')).toHaveText('夜间导出')
    await expect(panel(page).getByRole('button', { name: '立即重试' })).toBeVisible()
  })
})
