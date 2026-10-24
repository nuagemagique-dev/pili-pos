import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  emitTauriEvent,
  DEFAULT_USER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
} from './fixtures/mock-ipc'

// POS description v2.2, Message: a warning the FDM attaches to the
// response of a signed event is shown at least once per bookingDate.
// The backend forwards those messages as "fdm-sign-messages" (live on
// the module2 simulator: BUFFER_NEAR_FULL, TRANSACTION_UPLOAD_FAILED).

function signed(code: string, bookingDate: string, showPos = 'MANDATORY') {
  return {
    mutation: 'signSale',
    bookingDate,
    warnings: [{ message: `${code} text`, extensions: { category: 'SPF_FOD', code, showPos } }],
    informations: [],
  }
}

async function orderScreen(page: Page) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
  await expect(page.locator('.fdm.ok')).toBeVisible()
}

test('a warning in a sign response is shown, with its French message', async ({ page }) => {
  await orderScreen(page)
  await emitTauriEvent(page, 'fdm-sign-messages', signed('BUFFER_NEAR_FULL', '2026-09-27'))
  await expect(page.locator('.toast-warning')).toHaveText(
    'La boîte noire est presque pleine. Programmez une intervention.',
  )
  await expect(page.locator('.fdm.warn')).toBeVisible()
})

// A mandatory warning (showPos MANDATORY) stays on screen while the FDM
// reports it; an optional one closes itself. The sign response can bring
// either before the monitor's next status check.
test('a mandatory warning in a sign response keeps its banner open', async ({ page }) => {
  await orderScreen(page)
  await emitTauriEvent(page, 'fdm-sign-messages', signed('BUFFER_NEAR_FULL', '2026-09-27'))
  await expect(page.locator('.fdm-alert .msg')).toHaveText(
    'La boîte noire est presque pleine. Programmez une intervention.',
  )
  await expect(page.locator('.fdm-alert .countdown')).toHaveCount(0)
})

test('an optional warning in a sign response has a countdown', async ({ page }) => {
  await orderScreen(page)
  await emitTauriEvent(
    page,
    'fdm-sign-messages',
    signed('RTC_SYNC_FAILED', '2026-09-27', 'OPTIONAL'),
  )
  await expect(page.locator('.fdm-alert .countdown')).toHaveCount(1)
})

test('a mandatory warning replaces an optional one on the banner', async ({ page }) => {
  await orderScreen(page)
  await emitTauriEvent(
    page,
    'fdm-sign-messages',
    signed('RTC_SYNC_FAILED', '2026-09-27', 'OPTIONAL'),
  )
  await expect(page.locator('.fdm-alert .countdown')).toHaveCount(1)
  await emitTauriEvent(page, 'fdm-sign-messages', signed('CLIENT_CERT_EXPIRED', '2026-09-27'))
  await expect(page.locator('.fdm-alert .msg')).toHaveText(
    'Le certificat de la boîte noire a expiré. Contactez votre technicien.',
  )
  await expect(page.locator('.fdm-alert .countdown')).toHaveCount(0)
})

test('the same warning is shown once per bookingDate', async ({ page }) => {
  await page.clock.install()
  await orderScreen(page)
  await emitTauriEvent(
    page,
    'fdm-sign-messages',
    signed('TRANSACTION_UPLOAD_FAILED', '2026-09-27', 'OPTIONAL'),
  )
  await expect(page.locator('.toast-warning')).toHaveCount(1)
  await page.clock.runFor(10_000)
  await expect(page.locator('.toast-warning')).toHaveCount(0)

  await emitTauriEvent(
    page,
    'fdm-sign-messages',
    signed('TRANSACTION_UPLOAD_FAILED', '2026-09-27', 'OPTIONAL'),
  )
  await page.clock.runFor(500)
  await expect(page.locator('.toast-warning')).toHaveCount(0)

  await emitTauriEvent(
    page,
    'fdm-sign-messages',
    signed('TRANSACTION_UPLOAD_FAILED', '2026-09-28', 'OPTIONAL'),
  )
  await expect(page.locator('.toast-warning')).toHaveCount(1)
})

test('a vendor code without a French text shows the FDM message', async ({ page }) => {
  await orderScreen(page)
  await emitTauriEvent(page, 'fdm-sign-messages', signed('UNRECOGNIZED', '2026-09-27', 'OPTIONAL'))
  await expect(page.locator('.toast-warning')).toHaveText('UNRECOGNIZED text')
})
