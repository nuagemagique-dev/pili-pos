import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  emitTauriEvent,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
} from './fixtures/mock-ipc'

// Session screens (UX audit L4, L5, T4): the idle lock is announced by
// the engine 30 s before it happens, the login screen says why the till
// is locked, login goes straight to the tables, and starting a takeaway
// has one label.

async function login(page: Page) {
  await mockTauriIPC(page, { list_users: () => [DEFAULT_USER] })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
}

test.describe('Idle lock warning (L4)', () => {
  test('the engine event shows a live countdown under the header', async ({ page }) => {
    await page.clock.install()
    await login(page)
    await expect(page.getByTestId('idle-banner')).toHaveCount(0)

    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: 30_000 })
    const banner = page.getByTestId('idle-banner')
    await expect(banner).toContainText(
      "Verrouillage dans 30 s · touchez l'écran pour rester connecté",
    )

    // The countdown is live.
    await page.clock.runFor(5_000)
    await expect(banner).toContainText('Verrouillage dans 25 s')
    await page.clock.runFor(20_000)
    await expect(banner).toContainText('Verrouillage dans 5 s')
  })

  test('the banner sits under the header and takes no tap', async ({ page }) => {
    await login(page)
    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: 30_000 })
    const banner = page.getByTestId('idle-banner')
    await expect(banner).toBeVisible()
    const header = await page.locator('.app-header').boundingBox()
    const box = await banner.boundingBox()
    expect(box!.y).toBeGreaterThanOrEqual(header!.y + header!.height - 1)
    // The header buttons stay reachable while the warning shows.
    await page.getByRole('button', { name: 'Verrouiller' }).click()
    await page.waitForURL(/#\/$/u)
  })

  test('a touch is forwarded to the engine and hides the banner', async ({ page }) => {
    await login(page)
    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: 30_000 })
    await expect(page.getByTestId('idle-banner')).toBeVisible()
    // Past the one-per-second throttle of the activity forwarding.
    await page.waitForTimeout(1100)
    const before = (await ipcCalls(page, 'user_activity')).length
    await page.mouse.click(600, 400)
    await expect.poll(async () => (await ipcCalls(page, 'user_activity')).length).toBe(before + 1)
    await expect(page.getByTestId('idle-banner')).toHaveCount(0)
  })

  test('the engine ending the warning (active again) hides the banner', async ({ page }) => {
    await login(page)
    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: 30_000 })
    await expect(page.getByTestId('idle-banner')).toBeVisible()
    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: null })
    await expect(page.getByTestId('idle-banner')).toHaveCount(0)
  })

  test('the UI has no lock timer of its own: a warning alone never locks', async ({ page }) => {
    await page.clock.install()
    await login(page)
    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: 30_000 })
    await page.clock.runFor(60_000)
    await expect(page).toHaveURL(/tables/u)
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(0)
  })

  test('after an idle lock the login screen says why, until someone logs in', async ({ page }) => {
    await login(page)
    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: 30_000 })
    await emitTauriEvent(page, 'session-locked', {
      reason: 'idle',
      orderError: null,
      idleAfterMs: 300_000,
    })
    await page.waitForURL(/#\/$/u)
    const notice = page.getByTestId('idle-locked-notice')
    await expect(notice).toHaveText("Caisse verrouillée après 5 min d'inactivité")
    // The countdown is gone with the session.
    await expect(page.getByTestId('idle-banner')).toHaveCount(0)

    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.getByRole('button', { name: 'Verrouiller' }).click()
    await page.waitForURL(/#\/$/u)
    // A manual lock says nothing about inactivity.
    await expect(page.getByTestId('idle-locked-notice')).toHaveCount(0)
  })

  test('a manual lock event shows no inactivity line', async ({ page }) => {
    await login(page)
    await emitTauriEvent(page, 'session-locked', {
      reason: 'manual',
      orderError: null,
      idleAfterMs: null,
    })
    await page.waitForURL(/#\/$/u)
    await expect(page.getByTestId('idle-locked-notice')).toHaveCount(0)
  })
})

test.describe('Login goes straight to the tables (L5)', () => {
  test('the order screen is never drawn after a login', async ({ page }) => {
    await login(page)
    await expect(page).toHaveURL(/tables/u)
    // The order screen loads the menu categories when it mounts.
    expect(await ipcCalls(page, 'get_categories')).toHaveLength(0)
  })
})

test.describe('One label for the takeaway (T4)', () => {
  test('the header button and the tables panel both say "À emporter"', async ({ page }) => {
    await login(page)
    await expect(page.locator('.header-nav .nav-btn', { hasText: 'À emporter' })).toBeVisible()
    await expect(page.locator('.new-takeaway-btn')).toHaveText('À emporter')
  })

  test('the panel title keeps the count of sales in progress', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
      list_open_takeaways: () => [
        { orderId: 11, createdAt: new Date().toISOString(), lineCount: 2, totalCents: 1500 },
        { orderId: 12, createdAt: new Date().toISOString(), lineCount: 1, totalCents: 700 },
      ],
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await expect(page.getByTestId('takeaways-title')).toHaveText('À emporter en cours (2)')
  })

  test('the panel button starts a takeaway sale', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
      create_order: () => ({ ...DEFAULT_ORDER, order_type: 'takeaway', table_number: null }),
    })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.locator('.new-takeaway-btn').click()
    await page.waitForURL(/order/u)
    await expect(page.locator('.header-takeaway')).toBeVisible()
  })
})
