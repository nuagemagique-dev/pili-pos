import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  overrideIPC,
  ipcCalls,
  emitTauriEvent,
  tapUserTile,
  live,
  DEFAULT_USER,
  TEST_PIN,
} from './fixtures/mock-ipc'

// UX audit wave 2, lot 6: the user menu of the header (L1), the way back to the table after an idle
// lock (T6) and the language of each user (R5).

const SERVER = {
  ...DEFAULT_USER,
  insz: '84022899837',
  name: 'Serveur',
  role: 'cashier',
  has_pin: true,
}
const ON_SHIFT = [{ userInsz: DEFAULT_USER.insz, startedAt: '2026-09-26T09:02:00Z' }]

// `login` answers the user whose INSZ was sent.
const loginAsTapped = live(({ insz }: { insz: string }) =>
  insz === '84022899837'
    ? { insz, name: 'Serveur', role: 'cashier', active: true, has_pin: true }
    : { insz, name: 'Bruno Moyse', role: 'admin', active: true, has_pin: true },
)

async function loginToTables(page: Page, overrides: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    list_users: () => [DEFAULT_USER, SERVER],
    login: loginAsTapped,
    ...overrides,
  })
  await page.goto('/')
  await tapUserTile(page, 'Bruno')
  await page.waitForURL(/tables/u)
}

async function openUserMenu(page: Page) {
  await page.getByTestId('header-user').click()
  await expect(page.getByTestId('user-menu')).toBeVisible()
}

async function closeUserMenu(page: Page) {
  await page.getByTestId('user-menu-backdrop').click({ position: { x: 900, y: 600 } })
  await expect(page.getByTestId('user-menu')).toHaveCount(0)
}

test.describe('User menu (L1)', () => {
  test('the user button opens a menu with the language, the PIN, the end of shift and the lock', async ({
    page,
  }) => {
    await loginToTables(page)
    await expect(page.getByTestId('user-menu')).toHaveCount(0)
    await openUserMenu(page)
    const menu = page.getByTestId('user-menu')
    await expect(menu.getByTestId('user-menu-language')).toContainText('Ma langue')
    await expect(menu.getByTestId('user-lang-fr')).toHaveText('FR')
    await expect(menu.getByTestId('user-lang-zh')).toHaveText('中文')
    await expect(menu.getByTestId('user-lang-en')).toHaveText('EN')
    await expect(menu.getByTestId('user-menu-pin')).toHaveText('Changer mon PIN')
    await expect(menu.getByTestId('user-menu-end-shift')).toHaveText('Fin de service')
    await expect(menu.getByTestId('user-menu-lock')).toHaveText('Verrouiller')
    // Touch: every row is at least 48 px high.
    for (const id of ['user-menu-pin', 'user-menu-end-shift', 'user-menu-lock']) {
      const box = await menu.getByTestId(id).boundingBox()
      expect(box?.height ?? 0).toBeGreaterThanOrEqual(48)
    }
    // A tap outside closes it without doing anything.
    await closeUserMenu(page)
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(0)
  })

  test('"Changer mon PIN" opens the PIN dialog of the logged-in user', async ({ page }) => {
    await loginToTables(page)
    await openUserMenu(page)
    await page.getByTestId('user-menu-pin').click()
    await expect(page.getByTestId('user-menu')).toHaveCount(0)
    await expect(page.locator('.pin-dialog .who')).toHaveText('Bruno Moyse')
  })

  test('"Verrouiller" of the menu locks the till and keeps the shift', async ({ page }) => {
    await loginToTables(page)
    await openUserMenu(page)
    await page.getByTestId('user-menu-lock').click()
    await page.waitForURL(/#\/$/u)
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(1)
    expect(await ipcCalls(page, 'work_out')).toHaveLength(0)
  })

  test('"Fin de service" needs a second tap, then signs the clock-out without asking for the PIN', async ({
    page,
  }) => {
    await loginToTables(page, { list_open_shifts: () => ON_SHIFT })
    await openUserMenu(page)
    const end = page.getByTestId('user-menu-end-shift')
    await end.click()
    await expect(end).toHaveText('Confirmer la fin de service ?')
    // Nothing is signed by the first tap.
    expect(await ipcCalls(page, 'work_out')).toHaveLength(0)
    await end.click()

    await page.waitForURL(/#\/$/u)
    expect(await ipcCalls(page, 'work_out')).toEqual([{ employeeInsz: DEFAULT_USER.insz }])
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(1)
    // The PIN pad was not needed, and the login screen's own command was not used.
    expect(await ipcCalls(page, 'end_shift')).toHaveLength(0)
    await expect(page.locator('.toast-success')).toContainText('Service terminé · Bruno Moyse')
    await expect(page.locator('.toast-error')).toHaveCount(0)
  })

  test('the confirmation lasts 5 seconds, then the button is back to "Fin de service"', async ({
    page,
  }) => {
    await loginToTables(page)
    await openUserMenu(page)
    const end = page.getByTestId('user-menu-end-shift')
    await end.click()
    await expect(end).toHaveText('Confirmer la fin de service ?')
    await expect(end).toHaveText('Fin de service', { timeout: 8000 })
    // Tapping once more only arms it again.
    await end.click()
    await expect(end).toHaveText('Confirmer la fin de service ?')
    expect(await ipcCalls(page, 'work_out')).toHaveLength(0)
  })

  test('an unregistered clock-out is reported, and the till still locks', async ({ page }) => {
    await loginToTables(page, {
      work_out: () => ({ success: false, error: 'FDM error: offline' }),
    })
    await openUserMenu(page)
    const end = page.getByTestId('user-menu-end-shift')
    await end.click()
    await end.click()
    await page.waitForURL(/#\/$/u)
    await expect(page.locator('.toast-error')).toContainText('Pointage non enregistré')
    await expect(page.locator('.toast-success')).toHaveCount(0)
  })

  test('with an order open, it is registered before the shift ends', async ({ page }) => {
    await loginToTables(page)
    await page.locator('.table-card').first().click()
    await page.waitForURL(/order/u)
    await openUserMenu(page)
    const end = page.getByTestId('user-menu-end-shift')
    await end.click()
    await end.click()
    await page.waitForURL(/#\/$/u)
    expect((await ipcCalls(page, 'leave_order')).length).toBeGreaterThanOrEqual(1)
    expect(await ipcCalls(page, 'work_out')).toHaveLength(1)
  })
})

test.describe('Login tile of an employee on shift (L1)', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER, SERVER],
      list_open_shifts: () => ON_SHIFT,
    })
    await page.goto('/')
  })

  test('shows "Se connecter" and "Terminer le service", the pad title says which one runs', async ({
    page,
  }) => {
    await page.locator('.user-btn', { hasText: 'Bruno' }).click()
    const choice = page.getByTestId('shift-choice')
    await expect(choice.getByRole('button', { name: 'Se connecter' })).toBeVisible()
    await expect(choice.getByRole('button', { name: 'Terminer le service' })).toBeVisible()
    await expect(page.getByTestId('pin-title')).toHaveText('Se connecter')

    await choice.getByRole('button', { name: 'Terminer le service' }).click()
    await expect(page.getByTestId('pin-title')).toHaveText('Fin de service')
    await choice.getByRole('button', { name: 'Se connecter' }).click()
    await expect(page.getByTestId('pin-title')).toHaveText('Se connecter')
    for (const button of await choice.getByRole('button').all()) {
      expect((await button.boundingBox())?.height ?? 0).toBeGreaterThanOrEqual(48)
    }
  })

  test('the default is to log in: the PIN opens the session', async ({ page }) => {
    await page.locator('.user-btn', { hasText: 'Bruno' }).click()
    for (const d of TEST_PIN) await page.locator('.pin-overlay .nk', { hasText: d }).first().click()
    await expect(page).toHaveURL(/tables|order/u)
    expect(await ipcCalls(page, 'login')).toEqual([{ insz: DEFAULT_USER.insz, pin: TEST_PIN }])
    expect(await ipcCalls(page, 'end_shift')).toHaveLength(0)
  })

  test('"Terminer le service" then the PIN ends the shift through the engine', async ({ page }) => {
    await page.locator('.user-btn', { hasText: 'Bruno' }).click()
    await page.getByRole('button', { name: 'Terminer le service' }).click()
    for (const d of TEST_PIN) await page.locator('.pin-overlay .nk', { hasText: d }).first().click()
    await expect(page.locator('.toast-success')).toContainText('Service terminé')
    expect(await ipcCalls(page, 'end_shift')).toEqual([{ insz: DEFAULT_USER.insz, pin: TEST_PIN }])
    await expect(page).toHaveURL(/#\/$/u)
  })

  test('an employee off shift has no choice and the usual PIN title', async ({ page }) => {
    await page.locator('.user-btn', { hasText: 'Serveur' }).click()
    await expect(page.getByTestId('shift-choice')).toHaveCount(0)
    await expect(page.getByTestId('pin-title')).toHaveText('Code PIN')
  })
})

test.describe('Back to the table after an idle lock (T6)', () => {
  async function openTableThenIdleLock(page: Page, overrides: Record<string, unknown> = {}) {
    await loginToTables(page, overrides)
    await page.locator('.table-card').first().click()
    await page.waitForURL(/order/u)
    await emitTauriEvent(page, 'session-locked', {
      reason: 'idle',
      orderError: null,
      idleAfterMs: 300_000,
    })
    await page.waitForURL(/#\/$/u)
  }

  test('the same user logging back in finds the order again', async ({ page }) => {
    await openTableThenIdleLock(page)
    const activeBefore = (await ipcCalls(page, 'set_active_order')).length
    await tapUserTile(page, 'Bruno')
    await page.waitForURL(/order/u)
    // Reloaded by its id through the existing commands, and made the engine's active order again.
    const active = await ipcCalls(page, 'set_active_order')
    expect(active.length).toBeGreaterThan(activeBefore)
    expect(active.at(-1)).toEqual({ orderId: expect.any(Number) })
    expect((await ipcCalls(page, 'get_order')).length).toBeGreaterThan(0)
  })

  test('another user lands on Tables', async ({ page }) => {
    await openTableThenIdleLock(page)
    await tapUserTile(page, 'Serveur')
    await page.waitForURL(/tables/u)
  })

  test('an order closed meanwhile lands on Tables', async ({ page }) => {
    await openTableThenIdleLock(page)
    await overrideIPC(page, 'get_order', {
      id: 1,
      table_number: 1,
      order_type: 'dine_in',
      status: 'closed',
      user_insz: DEFAULT_USER.insz,
      created_at: '2026-09-26T09:00:00Z',
      closed_at: '2026-09-26T10:00:00Z',
      subtotal_cents: 0,
      total_cents: 0,
    })
    await tapUserTile(page, 'Bruno')
    await page.waitForURL(/tables/u)
  })

  test('a manual lock does not bring the order back', async ({ page }) => {
    await loginToTables(page)
    await page.locator('.table-card').first().click()
    await page.waitForURL(/order/u)
    await page.locator('.nav-logout').click()
    await page.waitForURL(/#\/$/u)
    await tapUserTile(page, 'Bruno')
    await page.waitForURL(/tables/u)
  })

  test('the order is brought back once: a later login lands on Tables', async ({ page }) => {
    await openTableThenIdleLock(page)
    await tapUserTile(page, 'Bruno')
    await page.waitForURL(/order/u)
    await page.locator('.nav-logout').click()
    await page.waitForURL(/#\/$/u)
    await tapUserTile(page, 'Bruno')
    await page.waitForURL(/tables/u)
  })
})

test.describe('Language per user (R5)', () => {
  const STORAGE_KEY = 'pili-pos-user-languages'

  test('the choice of the header menu applies at once and is kept for this user', async ({
    page,
  }) => {
    await loginToTables(page)
    await expect(page.locator('.nav-logout')).toHaveText('Verrouiller')
    await openUserMenu(page)
    await page.getByTestId('user-lang-zh').click()
    await expect(page.getByTestId('user-lang-zh')).toHaveAttribute('aria-pressed', 'true')
    await expect(page.locator('.nav-logout')).toHaveText('锁屏')
    const stored = await page.evaluate((k) => localStorage.getItem(k), STORAGE_KEY)
    expect(JSON.parse(stored ?? '{}')).toEqual({ [DEFAULT_USER.insz]: 'zh' })
    // The terminal language of Settings is untouched.
    expect(await page.evaluate(() => localStorage.getItem('pili-pos-language'))).toBeNull()
  })

  test('the login screen stays in the terminal language, the user gets theirs at login', async ({
    page,
  }) => {
    await loginToTables(page)
    await openUserMenu(page)
    await page.getByTestId('user-lang-en').click()
    await closeUserMenu(page)
    await expect(page.locator('.nav-logout')).toHaveText('Lock')
    await page.locator('.nav-logout').click()
    await page.waitForURL(/#\/$/u)
    // Back on the login screen: the terminal's French.
    await expect(page.locator('.title')).toHaveText('Qui êtes-vous ?')
    await tapUserTile(page, 'Bruno')
    await page.waitForURL(/tables/u)
    await expect(page.locator('.nav-logout')).toHaveText('Lock')
  })

  test('another user keeps the terminal language', async ({ page }) => {
    await loginToTables(page)
    await openUserMenu(page)
    await page.getByTestId('user-lang-zh').click()
    await closeUserMenu(page)
    await page.locator('.nav-logout').click()
    await page.waitForURL(/#\/$/u)
    await tapUserTile(page, 'Serveur')
    await page.waitForURL(/tables/u)
    await expect(page.locator('.nav-logout')).toHaveText('Verrouiller')
  })

  test('the selected login tile offers FR · 中文 · EN and applies the choice after the login', async ({
    page,
  }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER, SERVER], login: loginAsTapped })
    await page.goto('/')
    await expect(page.getByTestId('login-user-lang')).toHaveCount(0)
    await page.locator('.user-btn', { hasText: 'Serveur' }).click()
    const selector = page.getByTestId('login-user-lang')
    await expect(selector).toBeVisible()
    await expect(selector.getByRole('button')).toHaveText(['FR', '中文', 'EN'])
    // Without a choice it shows the terminal's language.
    await expect(page.getByTestId('login-lang-fr')).toHaveAttribute('aria-pressed', 'true')
    await page.getByTestId('login-lang-zh').click()
    await expect(page.getByTestId('login-lang-zh')).toHaveAttribute('aria-pressed', 'true')
    // The login screen itself does not change language.
    await expect(page.locator('.title')).toHaveText('Qui êtes-vous ?')
    for (const d of TEST_PIN) await page.locator('.pin-overlay .nk', { hasText: d }).first().click()
    await page.waitForURL(/tables/u)
    await expect(page.locator('.nav-logout')).toHaveText('锁屏')
  })

  test('Settings names its language "Langue par défaut du terminal"', async ({ page }) => {
    await loginToTables(page)
    await page.locator('.nav-admin', { hasText: 'Config' }).click()
    await page.waitForURL(/settings/u)
    await expect(page.locator('h2', { hasText: 'Langue par défaut du terminal' })).toBeVisible()
  })
})
