import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  tapUserTile,
  emitTauriEvent,
  fdmHealth,
  live,
} from './fixtures/mock-ipc'

// The chrome around every screen (UX audit lot 3): C5 toasts at the top
// centre, L3 the FDM veil under the header, L2 the status light sheet.

const HEADER = 64

/** The ticket printer answers what the test says (window.__printerOk), and each call is logged. */
const printerMock = {
  check_printer: live(() => (window as unknown as { __printerOk?: boolean }).__printerOk === true),
}

async function setPrinterOk(page: Page) {
  await page.evaluate(() => {
    ;(window as unknown as { __printerOk?: boolean }).__printerOk = true
  })
}

async function toTables(page: Page) {
  await mockTauriIPC(page, printerMock)
  await page.goto('/')
  await tapUserTile(page)
  await expect(page).toHaveURL(/tables/u)
}

/** Show a toast through the app's own store. */
async function showToast(page: Page, message: string, variant: string, ttl?: number) {
  await page.evaluate(
    ([m, v, t]) => {
      const app = (document.querySelector('#app') as any).__vue_app__
      app.config.globalProperties.$pinia._s.get('toast').show(m, v, t)
    },
    [message, variant, ttl] as const,
  )
}

test.describe('C5 toasts', () => {
  test('sit at the top centre, just under the header, 560 px wide at most', async ({ page }) => {
    await toTables(page)
    await showToast(page, 'Ticket imprimé', 'success')
    const toast = page.locator('.toast', { hasText: 'Ticket imprimé' })
    await expect(toast).toBeVisible()
    const box = (await toast.boundingBox())!
    expect(box.y).toBeGreaterThanOrEqual(HEADER)
    expect(box.y).toBeLessThan(HEADER + 40)
    expect(box.width).toBeLessThanOrEqual(560)
    const viewport = page.viewportSize()!
    expect(Math.abs(box.x + box.width / 2 - viewport.width / 2)).toBeLessThan(2)
  })

  test('stack downwards, newest at the bottom, three at most', async ({ page }) => {
    await toTables(page)
    for (const n of [1, 2, 3, 4, 5]) await showToast(page, `Message ${n}`, 'info', 60_000)
    const toasts = page.locator('.toast')
    await expect(toasts).toHaveCount(3)
    await expect(toasts.nth(0)).toHaveText('Message 3')
    await expect(toasts.nth(2)).toHaveText('Message 5')
    const first = (await toasts.nth(0).boundingBox())!
    const last = (await toasts.nth(2).boundingBox())!
    expect(last.y).toBeGreaterThan(first.y)
  })

  test('never cover the header buttons, even with a full stack', async ({ page }) => {
    await toTables(page)
    for (const n of [1, 2, 3]) await showToast(page, `Message ${n}`, 'error')
    const reports = page.locator('.nav-btn', { hasText: 'Rapports' })
    const header = (await reports.boundingBox())!
    const stack = (await page.locator('.toast').first().boundingBox())!
    expect(stack.y).toBeGreaterThanOrEqual(header.y + header.height)
    await reports.click()
    await expect(page).toHaveURL(/reports/u)
  })

  test('the container takes no tap and info toasts let taps through', async ({ page }) => {
    await toTables(page)
    await showToast(page, 'Information', 'info', 60_000)
    await expect(page.locator('.toast-stack')).toHaveCSS('pointer-events', 'none')
    const hit = await page.evaluate(() => {
      const r = document.querySelector('.toast')!.getBoundingClientRect()
      return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('.toast')
    })
    expect(hit).toBeNull()
  })

  test('info and success toasts go away by themselves', async ({ page }) => {
    await page.clock.install()
    await toTables(page)
    await showToast(page, 'Passager', 'info')
    await showToast(page, 'Enregistré', 'success', 2000)
    await expect(page.locator('.toast')).toHaveCount(2)
    await page.clock.runFor(3500)
    await expect(page.locator('.toast')).toHaveCount(0)
  })

  test('an error is never pushed out of view by newer messages', async ({ page }) => {
    await toTables(page)
    await showToast(page, 'Impression impossible', 'error')
    for (const n of [1, 2, 3]) await showToast(page, `Info ${n}`, 'info', 60_000)
    const toasts = page.locator('.toast')
    await expect(toasts).toHaveCount(3)
    await expect(toasts.nth(0)).toHaveText(/Impression impossible/u)
    await expect(toasts.nth(1)).toHaveText('Info 2')
    await expect(toasts.nth(2)).toHaveText('Info 3')
  })

  test('an error stays until it is tapped, and says so', async ({ page }) => {
    await page.clock.install()
    await toTables(page)
    await showToast(page, 'Impression impossible', 'error', 2000)
    const toast = page.locator('.toast-error')
    await expect(toast).toBeVisible()
    await expect(toast.locator('.toast-hint')).toHaveText('Touchez pour fermer')
    await page.clock.runFor(60_000)
    await expect(toast).toBeVisible()
    // The error is the only thing that takes the tap, and the tap closes it.
    const hit = await page.evaluate(() => {
      const r = document.querySelector('.toast-error')!.getBoundingClientRect()
      return Boolean(
        document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('.toast-error'),
      )
    })
    expect(hit).toBe(true)
    await toast.tap()
    await expect(page.locator('.toast')).toHaveCount(0)
  })
})

test.describe('L3 FDM veil under the header', () => {
  const BLOCKING = fdmHealth({
    connected: false,
    answered: false,
    severity: 'blocking',
    code: 'FDM_NOT_OPERATIONAL',
    mandatory: true,
  })

  test('starts at 64 px and leaves the header nav usable', async ({ page }) => {
    await toTables(page)
    await emitTauriEvent(page, 'fdm-status', BLOCKING)
    const veil = page.locator('.fdm-veil')
    await expect(veil).toBeVisible()
    const box = (await veil.boundingBox())!
    expect(box.y).toBe(HEADER)
    // The message, the code and the retry stay.
    await expect(veil.locator('.msg')).toContainText('boîte noire')
    await expect(veil.locator('.code')).toContainText('FDM_NOT_OPERATIONAL')
    await expect(veil.locator('.retry-btn')).toHaveText('Réessayer maintenant')

    // Rapports, Config, Tables and Verrouiller are reachable through it.
    await page.locator('.nav-btn', { hasText: 'Rapports' }).click()
    await expect(page).toHaveURL(/reports/u)
    await expect(page.locator('.fdm-veil')).toHaveCount(0)
    await page.locator('.nav-btn', { hasText: 'Config' }).click()
    await expect(page).toHaveURL(/settings/u)
    await page.locator('.nav-btn', { hasText: 'Tables' }).click()
    await expect(page).toHaveURL(/tables/u)
    await expect(page.locator('.fdm-veil')).toBeVisible()
    await page.locator('.nav-logout').click()
    await expect(page).toHaveURL(/login|\/$/u)
  })

  test('the idle lock countdown stays above the veil', async ({ page }) => {
    await toTables(page)
    await emitTauriEvent(page, 'idle-lock-warning', { lockInMs: 30_000 })
    await emitTauriEvent(page, 'fdm-status', BLOCKING)
    await expect(page.locator('.fdm-veil')).toBeVisible()
    const banner = page.getByTestId('idle-banner')
    await expect(banner).toBeVisible()
    const onTop = await page.evaluate(() => {
      const r = document.querySelector('[data-testid="idle-banner"]')!.getBoundingClientRect()
      const el = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
      return Boolean(el?.closest('[data-testid="idle-banner"]'))
    })
    expect(onTop).toBe(true)
    // The veil is gone: the lift goes with it.
    await emitTauriEvent(page, 'fdm-status', fdmHealth({}))
    await expect(page.locator('.fdm-veil')).toHaveCount(0)
    expect(
      await page.evaluate(() => document.documentElement.classList.contains('fdm-blocking')),
    ).toBe(false)
  })

  test('still blocks the sales screen under the header', async ({ page }) => {
    await toTables(page)
    await emitTauriEvent(page, 'fdm-status', BLOCKING)
    await expect(page.locator('.fdm-veil')).toBeVisible()
    const blocked = await page.evaluate(() => {
      const el = document.elementFromPoint(400, 300)
      return Boolean(el?.closest('.fdm-veil'))
    })
    expect(blocked).toBe(true)
  })
})

test.describe('L2 status light sheet', () => {
  test('each light is a button of 48 px with an aria-label', async ({ page }) => {
    await toTables(page)
    for (const key of ['fdm', 'ticket', 'kitchen']) {
      const light = page.getByTestId(`light-${key}`)
      await expect(light).toHaveJSProperty('tagName', 'BUTTON')
      const box = (await light.boundingBox())!
      expect(box.height).toBeGreaterThanOrEqual(48)
      expect(box.width).toBeGreaterThanOrEqual(48)
      await expect(light).toHaveAttribute('aria-label', /.+ : .+/u)
    }
    await expect(page.getByTestId('light-ticket')).toHaveAttribute(
      'aria-label',
      'Imprimante ticket : Ne répond pas',
    )
    await expect(page.getByTestId('light-fdm')).toHaveAttribute('aria-label', 'FDM : Fonctionne')
  })

  test('a tap opens the sheet: device, state, reason, time of the last check', async ({ page }) => {
    await toTables(page)
    await page.getByTestId('light-ticket').click()
    const sheet = page.getByTestId('device-sheet')
    await expect(sheet).toBeVisible()
    await expect(page.getByTestId('device-sheet-name')).toHaveText('Imprimante ticket')
    await expect(page.getByTestId('device-sheet-state')).toHaveText('Ne répond pas')
    await expect(page.getByTestId('device-sheet-reason')).toContainText('ne répond pas')
    await expect(page.getByTestId('device-sheet-checked')).toHaveText(
      /^Dernière vérification : \d{2}:\d{2}:\d{2}$/u,
    )
    await page.getByTestId('device-sheet-close').click()
    await expect(sheet).toHaveCount(0)
  })

  test('the reason of the terminal is in the sheet, not only in the tooltip', async ({ page }) => {
    await mockTauriIPC(page, {
      check_payment_terminal: () => ({
        configured: true,
        kind: 'ccv_opi',
        connected: false,
        busy: false,
        detail: 'connection refused',
      }),
    })
    await page.goto('/')
    await tapUserTile(page)
    await expect(page).toHaveURL(/tables/u)
    await page.getByTestId('light-terminal').click()
    await expect(page.getByTestId('device-sheet-reason')).toContainText(
      'injoignable: connection refused',
    )
  })

  test('the FDM light sheet is usable while the blocking veil is up', async ({ page }) => {
    await toTables(page)
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
    await expect(page.locator('.fdm-veil')).toBeVisible()
    await page.getByTestId('light-fdm').click()
    await expect(page.getByTestId('device-sheet')).toBeVisible()
    const retryOnTop = await page.evaluate(() => {
      const r = document
        .querySelector('[data-testid="device-sheet-retry"]')!
        .getBoundingClientRect()
      const el = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
      return Boolean(el?.closest('[data-testid="device-sheet"]'))
    })
    expect(retryOnTop).toBe(true)
    const before = (await ipcCalls(page, 'fdm_check_now')).length
    await page.getByTestId('device-sheet-retry').click()
    expect((await ipcCalls(page, 'fdm_check_now')).length).toBe(before + 1)
  })

  test('"Réessayer maintenant" runs only that device check again', async ({ page }) => {
    await toTables(page)
    await expect(page.getByTestId('light-ticket')).toHaveClass(/err/u)
    const ticketBefore = (await ipcCalls(page, 'check_printer')).length
    const kitchenBefore = (await ipcCalls(page, 'check_kitchen_printer')).length

    await page.getByTestId('light-ticket').click()
    await setPrinterOk(page)
    await page.getByTestId('device-sheet-retry').click()

    // The sheet shows the new answer and the light follows.
    await expect(page.getByTestId('device-sheet-state')).toHaveText('Fonctionne')
    await expect(page.getByTestId('light-ticket')).toHaveClass(/ok/u)
    expect((await ipcCalls(page, 'check_printer')).length).toBe(ticketBefore + 1)
    expect((await ipcCalls(page, 'check_kitchen_printer')).length).toBe(kitchenBefore)
  })

  test('the FDM light retries the FDM only', async ({ page }) => {
    await toTables(page)
    await emitTauriEvent(
      page,
      'fdm-status',
      fdmHealth({ severity: 'warning', code: 'BUFFER_NEAR_FULL', mandatory: true }),
    )
    await expect(page.getByTestId('light-fdm')).toHaveClass(/warn/u)
    await page.getByTestId('light-fdm').click()
    await expect(page.getByTestId('device-sheet-state')).toHaveText('À surveiller')
    await expect(page.getByTestId('device-sheet-reason')).toContainText('presque pleine')
    const before = (await ipcCalls(page, 'fdm_check_now')).length
    await page.getByTestId('device-sheet-retry').click()
    await expect(page.getByTestId('device-sheet-state')).toHaveText('Fonctionne')
    expect((await ipcCalls(page, 'fdm_check_now')).length).toBe(before + 1)
  })

  test('the start-up rows of the login screen open the same sheet', async ({ page }) => {
    await mockTauriIPC(page, printerMock)
    await page.goto('/')
    const row = page.getByTestId('check-ticket')
    await expect(row).toHaveJSProperty('tagName', 'BUTTON')
    expect((await row.boundingBox())!.height).toBeGreaterThanOrEqual(48)
    await row.click()
    await expect(page.getByTestId('device-sheet-name')).toHaveText('Imprimante ticket')
    await expect(page.getByTestId('device-sheet-state')).toHaveText('Ne répond pas')
    await expect(page.getByTestId('device-sheet-checked')).toContainText('Dernière vérification')

    const ticketBefore = (await ipcCalls(page, 'check_printer')).length
    const kitchenBefore = (await ipcCalls(page, 'check_kitchen_printer')).length
    await setPrinterOk(page)
    await page.getByTestId('device-sheet-retry').click()
    await expect(page.getByTestId('device-sheet-state')).toHaveText('Fonctionne')
    await expect(page.getByTestId('check-ticket')).toHaveClass(/ok/u)
    expect((await ipcCalls(page, 'check_printer')).length).toBe(ticketBefore + 1)
    expect((await ipcCalls(page, 'check_kitchen_printer')).length).toBe(kitchenBefore)
  })
})
