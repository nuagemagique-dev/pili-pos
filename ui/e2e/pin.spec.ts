import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  loginAndGoto,
  ipcCalls,
  overrideIPC,
  DEFAULT_USER,
  tapUserTile,
} from './fixtures/mock-ipc'

// A PIN is what keeps one employee from opening another's session by
// tapping their tile: every user can set their own (tap their name on
// the order screen), a manager can set anyone's (Settings).

async function typePin(page: Page, pin: string) {
  const dialog = page.locator('.pin-dialog')
  for (const d of pin) await dialog.getByRole('button', { name: d, exact: true }).click()
}

test.describe('Own PIN - tap your name on the order screen', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, { login: () => DEFAULT_USER, list_users: () => [DEFAULT_USER] })
    await page.goto('/')
    await tapUserTile(page)
    await page.waitForURL(/tables/u)
    await page.locator('.table-card').first().click()
    await page.waitForURL(/order/u)
    await page.locator('.header-user').click()
  })

  test('the PIN is typed twice, then saved for the logged-in user', async ({ page }) => {
    await expect(page.locator('.pin-dialog .title')).toHaveText('Saisir le PIN')
    await expect(page.locator('.pin-dialog .who')).toHaveText('Bruno Moyse')
    await typePin(page, '1234')
    await expect(page.locator('.pin-dialog .title')).toHaveText('Confirmer le PIN')
    expect(await ipcCalls(page, 'set_pin')).toHaveLength(0)
    await typePin(page, '1234')

    await expect(page.locator('.pin-dialog')).toHaveCount(0)
    expect(await ipcCalls(page, 'set_pin')).toEqual([{ insz: DEFAULT_USER.insz, newPin: '1234' }])
    await expect(page.locator('.toast', { hasText: 'PIN modifié' })).toBeVisible()
  })

  test('two different PINs are refused and nothing is saved', async ({ page }) => {
    await typePin(page, '1234')
    await typePin(page, '5678')
    await expect(page.locator('.pin-dialog .error-msg')).toHaveText('Les PIN ne correspondent pas')
    await expect(page.locator('.pin-dialog .title')).toHaveText('Saisir le PIN')
    expect(await ipcCalls(page, 'set_pin')).toHaveLength(0)
  })

  test('a digit corrected before the fourth is not kept', async ({ page }) => {
    await typePin(page, '12')
    await page.locator('.pin-dialog .key-back').click()
    await typePin(page, '345')
    await typePin(page, '1345')
    expect(await ipcCalls(page, 'set_pin')).toEqual([{ insz: DEFAULT_USER.insz, newPin: '1345' }])
  })

  test('a refusal from the backend is shown and the dialog stays', async ({ page }) => {
    await overrideIPC(page, 'set_pin', { __reject: 'pin_format' })
    await typePin(page, '1234')
    await typePin(page, '1234')
    await expect(page.locator('.pin-dialog .error-msg')).toHaveText(
      'Le PIN doit compter exactement 4 chiffres.',
    )
    await expect(page.locator('.pin-dialog .title')).toHaveText('Saisir le PIN')
  })

  test('back closes without saving', async ({ page }) => {
    await typePin(page, '12')
    await page.locator('.pin-dialog').getByRole('button', { name: 'Retour' }).click()
    await expect(page.locator('.pin-dialog')).toHaveCount(0)
    expect(await ipcCalls(page, 'set_pin')).toHaveLength(0)
  })
})

test.describe('Employee PIN - set by a manager in Settings', () => {
  const WAITER = { ...DEFAULT_USER, insz: '99012345667', name: 'Apprenti', role: 'waiter' }

  test('the PIN button of a row sets that employee’s PIN', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER, WAITER],
    })
    await loginAndGoto(page, '/settings')
    const row = page.locator('.worker-row', { hasText: 'Apprenti' })
    await row.getByRole('button', { name: 'PIN' }).click()
    await expect(page.locator('.pin-dialog .who')).toHaveText('Apprenti')
    await typePin(page, '9876')
    await typePin(page, '9876')
    // The manager confirms with their own PIN.
    await expect(page.locator('.pin-dialog .who')).toHaveText(DEFAULT_USER.name)
    await typePin(page, '4321')
    await expect(page.locator('.pin-dialog')).toHaveCount(0)
    expect(await ipcCalls(page, 'set_pin')).toEqual([
      { insz: WAITER.insz, newPin: '9876', operatorPin: '4321' },
    ])
  })
})

test('the no-PIN warning at login says where to set one', async ({ page }) => {
  // A server may still log in without a PIN (a manager may not).
  const server = { ...DEFAULT_USER, role: 'cashier', has_pin: false }
  await mockTauriIPC(page, { login: () => server, list_users: () => [server] })
  await page.goto('/')
  await tapUserTile(page)
  await expect(page.locator('.toast', { hasText: 'touchez votre nom' })).toBeVisible()
})
