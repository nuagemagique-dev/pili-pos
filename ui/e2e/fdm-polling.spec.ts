import { test, expect, type Page } from '@playwright/test'
import { mockTauriIPC, tapUserTile, ipcCalls, emitTauriEvent, fdmHealth } from './fixtures/mock-ipc'

// The engine monitors the FDM while a session is open (30 s while it
// answers, backing off to 5 min while it does not; rules and tests in
// crates/pos-engine/src/commands/fdm_monitor.rs) and emits "fdm-status".
// The UI only shows what the engine reports: it never polls, and the
// only call it makes is the explicit "check now" (retry button, login
// screen start-up check).

const calls = async (page: Page, cmd: string) => (await ipcCalls(page, cmd)).length

async function openTable(page: Page) {
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/)
  await expect(page.locator('.fdm')).toBeVisible()
}

/** On a table's order screen (or, with `openOrder` false, on the
 * Tables screen: an FDM that is blocking veils the sales screens). */
async function onOrderScreen(page: Page, openOrder = true) {
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/)
  if (openOrder) await openTable(page)
  else await expect(page.locator('.fdm')).toBeVisible()
}

test('the FDM indicator follows the fdm-status events of the engine', async ({ page }) => {
  await mockTauriIPC(page)
  await onOrderScreen(page)
  await expect(page.locator('.fdm.ok')).toBeVisible()

  await emitTauriEvent(
    page,
    'fdm-status',
    fdmHealth({
      severity: 'warning',
      code: 'BUFFER_NEAR_FULL',
      mandatory: true,
    }),
  )
  await expect(page.locator('.fdm.warn')).toBeVisible()

  await emitTauriEvent(
    page,
    'fdm-status',
    fdmHealth({
      connected: false,
      answered: false,
      severity: 'blocking',
      code: 'FDM_NOT_OPERATIONAL',
      mandatory: true,
    }),
  )
  await expect(page.locator('.fdm.err')).toBeVisible()

  await emitTauriEvent(page, 'fdm-status', fdmHealth())
  await expect(page.locator('.fdm.ok')).toBeVisible()
  // The old status command is not used any more.
  expect(await calls(page, 'check_fdm_status')).toBe(0)
})

test('the blocking veil counts down to the next check of the engine', async ({ page }) => {
  await page.clock.install()
  await mockTauriIPC(page)
  await onOrderScreen(page, false)
  // Hold the clock, so the countdown only moves when the test says.
  await page.clock.pauseAt((await page.evaluate(() => Date.now())) + 1_000)
  const next = await page.evaluate(() => new Date(Date.now() + 60_000).toISOString())
  await emitTauriEvent(
    page,
    'fdm-status',
    fdmHealth({
      connected: false,
      answered: false,
      severity: 'blocking',
      code: 'FDM_NOT_OPERATIONAL',
      mandatory: true,
      nextCheckAt: next,
    }),
  )
  const veil = page.locator('.fdm-veil .code')
  await expect(veil).toContainText('nouvel essai automatique dans')
  // The countdown is refreshed every second.
  await page.clock.runFor(1_000)
  await expect(veil).toContainText('nouvel essai automatique dans 59 s')
  await page.clock.runFor(9_000)
  await expect(veil).toContainText('nouvel essai automatique dans 50 s')
  await page.clock.runFor(20_000)
  await expect(veil).toContainText('nouvel essai automatique dans 30 s')
  // The UI checks nothing by itself while it waits.
  expect(await calls(page, 'check_fdm_status')).toBe(0)

  // "Réessayer maintenant" asks the engine to check at once.
  const before = await calls(page, 'fdm_check_now')
  // (the mocked engine answers healthy)
  await page.locator('.retry-btn').click()
  await expect(page.locator('.fdm-veil')).toHaveCount(0)
  await expect(page.locator('.fdm.ok')).toBeVisible()
  expect((await ipcCalls(page, 'fdm_check_now')).length).toBe(before + 1)
})

test('changing screens makes no FDM call from the UI', async ({ page }) => {
  await page.clock.install()
  await mockTauriIPC(page)
  await onOrderScreen(page)
  const n = await calls(page, 'fdm_check_now')
  await page.evaluate(() => {
    window.location.hash = '/tables'
  })
  await page.waitForURL(/tables/)
  await openTable(page)
  // Every screen shows the indicator (AppHeader): the engine's monitor
  // goes on through the screen changes and the UI asks for nothing,
  // not even after 30 s.
  await page.clock.runFor(30_000)
  await page.clock.runFor(30_000)
  expect(await calls(page, 'fdm_check_now')).toBe(n)
  expect(await calls(page, 'check_fdm_status')).toBe(0)
})
