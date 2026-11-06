import { test, expect } from '@playwright/test'
import { mockTauriIPC, DEFAULT_USER, ipcCalls, loginAndGoto } from './fixtures/mock-ipc'
import { fillWithKeypad, fillWithNumPad, keypad } from './fixtures/keypad'

// UX audit C13 / R6: no field of the till takes the OS keyboard (the Cage kiosk has none), every
// one opens an on-screen keypad. Settings fields are the ones that used to be native inputs.

test.describe('Keypad fields', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
    })
    await loginAndGoto(page, '/settings')
  })

  test('the settings page has no native text input left, bar the Chinese menu name', async ({
    page,
  }) => {
    const natives = await page
      .locator('input:not([type="checkbox"]):not([type="radio"]):not([readonly]), textarea')
      .count()
    expect(natives).toBe(0)
  })

  test('a text field opens the keypad, cancel keeps the value, validate sets it', async ({
    page,
  }) => {
    const name = page.getByTestId('new-worker-name')
    await expect(name).toHaveText('Nom')
    await name.click()
    await expect(keypad(page)).toBeVisible()
    await keypad(page).locator('.act-btn.secondary').first().click()
    await expect(keypad(page)).toHaveCount(0)
    await expect(name).toHaveText('Nom')

    await fillWithKeypad(page, name, 'Éric Van-Damme')
    await expect(name).toHaveText('Éric Van-Damme')
  })

  test('a text field takes at most its limit', async ({ page }) => {
    const channel = page.getByTestId('new-worker-name')
    await channel.click()
    for (let i = 0; i < 70; i++) await keypad(page).locator('.key', { hasText: /^a$/u }).click()
    await keypad(page).locator('.act-btn.primary').click()
    await expect(channel).toHaveText('a'.repeat(60))
  })

  test('the INSZ takes exactly 11 digits and keeps its leading zeros', async ({ page }) => {
    const insz = page.getByTestId('new-worker-insz')
    await insz.click()
    const ok = page.getByTestId('number-entry-ok')
    for (const d of '0104041234')
      await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
    await expect(ok).toBeDisabled()
    await page.locator('.numpad .key', { hasText: /^5$/u }).click()
    await expect(ok).toBeEnabled()
    await ok.click()
    await expect(insz).toHaveText('01040412345')
  })

  test('a whole number field is set on the number pad', async ({ page }) => {
    await fillWithNumPad(page, page.getByTestId('new-worker-insz'), '99040400456')
    await expect(page.getByTestId('new-worker-insz')).toHaveText('99040400456')
    expect(await ipcCalls(page, 'create_user')).toHaveLength(0)
  })
})
