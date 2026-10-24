import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  loginAndGoto,
  overrideIPC,
  ipcCalls,
  tapUserTile,
  DEFAULT_USER,
  live,
} from './fixtures/mock-ipc'

// Remote support (commands/support.rs): a manager starts RustDesk from
// Settings with their PIN, stops it there, and every screen shows a
// banner while it runs. At the login screen anyone may stop it.

const RUNNING = {
  available: true,
  running: true,
  startedBy: 'Bruno Moyse',
  startedAt: '2026-09-26T08:30:00Z',
}

async function typePin(page: Page, pin: string) {
  const dialog = page.locator('.pin-dialog')
  for (const d of pin) await dialog.getByRole('button', { name: d, exact: true }).click()
}

test('a manager starts a session with their PIN', async ({ page }) => {
  await mockTauriIPC(page)
  await loginAndGoto(page, '/settings')
  const section = page.locator('.support-section')
  await expect(section.locator('h2')).toHaveText('Assistance à distance')
  await section.getByRole('button', { name: 'Démarrer' }).click()

  await expect(page.locator('.pin-dialog .title')).toHaveText('Saisir le PIN')
  await expect(page.locator('.pin-dialog .who')).toHaveText(DEFAULT_USER.name)
  await typePin(page, '4321')

  await expect(page.locator('.pin-dialog')).toHaveCount(0)
  expect(await ipcCalls(page, 'start_remote_support')).toEqual([
    { insz: DEFAULT_USER.insz, pin: '4321' },
  ])
  await expect(page.locator('.toast', { hasText: 'Assistance à distance démarrée' })).toBeVisible()
  // 08:30 UTC is 10:30 in Brussels (summer time).
  await expect(section.locator('.support-state')).toContainText('10:30')
  await expect(section.locator('.support-state')).toContainText('Bruno Moyse')
  await expect(section.getByRole('button', { name: 'Arrêter la session' })).toBeVisible()
})

test('a wrong PIN starts nothing and says so', async ({ page }) => {
  await mockTauriIPC(page)
  await loginAndGoto(page, '/settings')
  await overrideIPC(page, 'start_remote_support', { __reject: 'pin_wrong' })
  await page.locator('.support-section').getByRole('button', { name: 'Démarrer' }).click()
  await typePin(page, '0000')
  await expect(page.locator('.pin-dialog .error-msg')).toHaveText('PIN incorrect')
  await expect(page.locator('.pin-dialog .pin-dot.filled')).toHaveCount(0)
  await expect(page.locator('.support-banner')).toHaveCount(0)
})

test('back closes the PIN dialog without starting', async ({ page }) => {
  await mockTauriIPC(page)
  await loginAndGoto(page, '/settings')
  await page.locator('.support-section').getByRole('button', { name: 'Démarrer' }).click()
  await page.locator('.pin-dialog').getByRole('button', { name: 'Retour' }).click()
  await expect(page.locator('.pin-dialog')).toHaveCount(0)
  expect(await ipcCalls(page, 'start_remote_support')).toHaveLength(0)
})

test('a running session is stopped from Settings', async ({ page }) => {
  await mockTauriIPC(page, { remote_support_status: () => RUNNING })
  await loginAndGoto(page, '/settings')
  const section = page.locator('.support-section')
  await section.getByRole('button', { name: 'Arrêter la session' }).click()
  expect(await ipcCalls(page, 'stop_remote_support')).toHaveLength(1)
  await expect(page.locator('.toast', { hasText: 'Assistance à distance arrêtée' })).toBeVisible()
  await expect(section.getByRole('button', { name: 'Démarrer' })).toBeVisible()
})

test('without RustDesk the section says it is not installed', async ({ page }) => {
  await mockTauriIPC(page, {
    remote_support_status: () => ({
      available: false,
      running: false,
      startedBy: null,
      startedAt: null,
    }),
  })
  await loginAndGoto(page, '/settings')
  const section = page.locator('.support-section')
  await expect(section.locator('.support-state')).toHaveText('Non installée sur cette caisse')
  await expect(section.getByRole('button')).toHaveCount(0)
})

test('every screen shows the banner while a session runs', async ({ page }) => {
  const server = { ...DEFAULT_USER, role: 'cashier', has_pin: false }
  await mockTauriIPC(page, {
    login: () => server,
    list_users: () => [server],
    remote_support_status: () => RUNNING,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await expect(page.locator('.support-banner')).toHaveText(
    'Assistance à distance en cours (démarrée par Bruno Moyse)',
    { timeout: 8000 },
  )
})

test('no banner without a session', async ({ page }) => {
  await mockTauriIPC(page)
  await loginAndGoto(page, '/tables')
  await page.waitForTimeout(5500)
  await expect(page.locator('.support-banner')).toHaveCount(0)
})

test('the login screen shows a running session, and anyone stops it', async ({ page }) => {
  await mockTauriIPC(page, {
    remote_support_running: live(() => ({
      running: !(window as any).__supportStopped,
      startedAt: '2026-09-26T08:30:00Z',
    })),
    stop_remote_support: live(() => {
      ;(window as any).__supportStopped = true
      return { available: true, running: false, startedBy: null, startedAt: null }
    }),
  })
  await page.goto('/')
  const banner = page.locator('.support-banner')
  await expect(banner).toContainText('Assistance à distance en cours', { timeout: 8000 })
  await expect(banner).not.toContainText('Bruno Moyse')

  await banner.getByRole('button', { name: 'Arrêter la session' }).click()
  expect(await ipcCalls(page, 'stop_remote_support')).toHaveLength(1)
  await expect(banner).toHaveCount(0)
  await page.waitForTimeout(5500)
  await expect(banner).toHaveCount(0)
})
