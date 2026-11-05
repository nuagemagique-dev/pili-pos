import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  overrideIPC,
  tapUserTile,
  live,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CART_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  DEFAULT_QUICK_NOTES,
} from './fixtures/mock-ipc'

// UX audit C11: the quick notes. The note dialog shows the chips of the installation above the
// keyboard; a tap appends the chip to the note, comma separated, and the 60 character limit still
// applies. A manager edits the list in Settings (add, remove, reorder). The list is local data: the
// engine stores it in the owner settings, never in a signed request.

const CASHIER = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa Serveuse', role: 'cashier' }
const UNSENT = { ...DEFAULT_CART_LINE, id: 1, quantity: 2, unsent_quantity: 2 }

async function toOrder(page: Page, extra: Record<string, unknown> = {}, user = DEFAULT_USER) {
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2400 }),
    list_cart_lines: () => [UNSENT],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

const chip = (page: Page, text: string) =>
  page.getByTestId('keypad-chip').filter({ hasText: new RegExp(`^${text}$`, 'u') })

async function openNote(page: Page) {
  await page.getByTestId('line-note').click()
  await expect(page.locator('.keypad-modal')).toBeVisible()
}

test.describe('The chips of the note dialog', () => {
  test('show the nine usual notes of a new installation, above the keys', async ({ page }) => {
    await toOrder(page)
    await openNote(page)
    await expect(page.getByTestId('keypad-chip')).toHaveText(DEFAULT_QUICK_NOTES)
    const chips = await page.getByTestId('keypad-chips').boundingBox()
    const keys = await page.locator('.key-rows').boundingBox()
    expect(chips && keys && chips.y + chips.height <= keys.y).toBe(true)
    for (const c of await page.getByTestId('keypad-chip').all()) {
      expect((await c.boundingBox())?.height).toBeGreaterThanOrEqual(48)
    }
  })

  test('a tap appends the chip, comma separated, and the note is saved as typed', async ({
    page,
  }) => {
    await toOrder(page)
    await openNote(page)
    await chip(page, 'Sans wasabi').click()
    await expect(page.getByTestId('keypad-display')).toHaveText('Sans wasabi')
    await chip(page, 'Bien cuit').click()
    await expect(page.getByTestId('keypad-display')).toHaveText('Sans wasabi, Bien cuit')
    await expect(page.getByTestId('keypad-counter')).toHaveText('22/60')
    await page.locator('.keypad-modal .act-btn.primary').click()

    expect(await ipcCalls(page, 'set_line_note')).toEqual([
      { lineId: 1, note: 'Sans wasabi, Bien cuit' },
    ])
    await expect(page.getByTestId('line-kitchen-note')).toHaveText('Sans wasabi, Bien cuit')
  })

  test('a chip can follow typed text', async ({ page }) => {
    await toOrder(page)
    await openNote(page)
    for (const ch of 'ab') await page.locator('.keypad-modal .key', { hasText: ch }).first().click()
    await chip(page, 'Épicé').click()
    await expect(page.getByTestId('keypad-display')).toHaveText('ab, Épicé')
  })

  test('a chip already in the note is faded and does nothing', async ({ page }) => {
    await toOrder(page)
    await openNote(page)
    await chip(page, 'Bien cuit').click()
    await expect(chip(page, 'Bien cuit')).toBeDisabled()
    await expect(chip(page, 'Épicé')).toBeEnabled()
  })

  test('the 60 character limit still applies: a chip that does not fit is disabled', async ({
    page,
  }) => {
    await toOrder(page, {
      get_quick_notes: () => ({ notes: ['x'.repeat(40), 'y'.repeat(30), 'z'.repeat(19)] }),
    })
    await openNote(page)
    await chip(page, 'x'.repeat(40)).click()
    await expect(chip(page, 'y'.repeat(30))).toBeDisabled()
    await expect(chip(page, 'z'.repeat(19))).toBeDisabled()
    await expect(page.getByTestId('keypad-counter')).toHaveText('40/60')
    await page.getByRole('button', { name: 'Effacer' }).click()
    await expect(chip(page, 'y'.repeat(30))).toBeEnabled()
  })

  test('a note saved with chips goes through the same command and validation as a typed one', async ({
    page,
  }) => {
    await toOrder(page)
    await openNote(page)
    await overrideIPC(page, 'set_line_note', { __reject: 'note_unprintable' })
    await chip(page, 'Épicé').click()
    await page.locator('.keypad-modal .act-btn.primary').click()
    await expect(page.locator('.toast', { hasText: 'imprimante de cuisine' })).toBeVisible()
  })

  test("the manager's list shows, in its order; an empty list shows no chip", async ({ page }) => {
    await toOrder(page, { get_quick_notes: () => ({ notes: ['Sans glaçon', 'Avec citron'] }) })
    await openNote(page)
    await expect(page.getByTestId('keypad-chip')).toHaveText(['Sans glaçon', 'Avec citron'])
    await page.locator('.keypad-modal .act-btn.secondary').first().click()
    await overrideIPC(page, 'get_quick_notes', { notes: [] })
    await openNote(page)
    await expect(page.getByTestId('keypad-chips')).toHaveCount(0)
  })

  test('the list is read each time the dialog opens, and a failing read leaves a plain keypad', async ({
    page,
  }) => {
    await toOrder(page)
    await openNote(page)
    await page.locator('.keypad-modal .act-btn.secondary').first().click()
    await overrideIPC(page, 'get_quick_notes', { __reject: 'internal' })
    await openNote(page)
    await expect(page.getByTestId('keypad-chips')).toHaveCount(0)
    await expect(page.getByTestId('keypad-display')).toBeVisible()
    expect((await ipcCalls(page, 'get_quick_notes')).length).toBeGreaterThanOrEqual(1)
  })

  test('a cashier sees the chips too', async ({ page }) => {
    await toOrder(page, {}, CASHIER)
    await openNote(page)
    await expect(page.getByTestId('keypad-chip')).toHaveCount(9)
  })
})

async function toSettings(page: Page, extra: Record<string, unknown> = {}, user = DEFAULT_USER) {
  await mockTauriIPC(page, { login: () => user, list_users: () => [user], ...extra })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.evaluate(() => {
    window.location.hash = '/settings'
  })
  await page.waitForURL(/settings/u)
}

const notes = (page: Page) => page.getByTestId('quick-note-text')

test.describe('The Notes rapides panel of Settings', () => {
  test('is for a manager only', async ({ page }) => {
    await toSettings(page, {}, CASHIER)
    await expect(page.locator('.workers-section')).toBeVisible()
    await expect(page.getByTestId('quick-notes-panel')).toHaveCount(0)
    expect(await ipcCalls(page, 'get_quick_notes')).toHaveLength(0)
  })

  test('lists the notes in order', async ({ page }) => {
    await toSettings(page)
    await expect(page.getByTestId('quick-notes-panel')).toBeVisible()
    await expect(notes(page)).toHaveText(DEFAULT_QUICK_NOTES)
    await expect(page.getByTestId('quick-notes-save')).toBeDisabled()
  })

  test('removing, moving and saving sends the list in the new order', async ({ page }) => {
    await toSettings(page)
    await page.getByTestId('quick-note-0').getByTestId('quick-note-remove').click()
    await page.getByTestId('quick-note-1').getByTestId('quick-note-up').click()
    await expect(notes(page).first()).toHaveText('Sans sauce')
    await expect(page.getByTestId('quick-notes-save')).toBeEnabled()
    await page.getByTestId('quick-notes-save').click()

    const [call] = await ipcCalls(page, 'set_quick_notes')
    expect(call.input.notes).toEqual([
      'Sans sauce',
      'Sans sésame',
      'Sauce à part',
      'Bien cuit',
      'Épicé',
      'Pas épicé',
      'Sans oignon',
      'À emporter',
    ])
    await expect(page.getByTestId('quick-notes-saved')).toBeVisible()
    await expect(page.getByTestId('quick-notes-save')).toBeDisabled()
  })

  test('the first cannot go up and the last cannot go down', async ({ page }) => {
    await toSettings(page)
    await expect(page.getByTestId('quick-note-0').getByTestId('quick-note-up')).toBeDisabled()
    await expect(page.getByTestId('quick-note-8').getByTestId('quick-note-down')).toBeDisabled()
  })

  test('adding asks the text on the keypad and puts it last', async ({ page }) => {
    await toSettings(page)
    await page.getByTestId('quick-notes-add').click()
    const pad = page.locator('.keypad-modal')
    await expect(pad).toContainText('Nouvelle note rapide')
    for (const ch of 'miso')
      await pad.locator('.key', { hasText: new RegExp(`^${ch}$`, 'u') }).click()
    await pad.locator('.act-btn.primary').click()
    await expect(notes(page).last()).toHaveText('miso')
    await page.getByTestId('quick-notes-save').click()
    const [call] = await ipcCalls(page, 'set_quick_notes')
    expect(call.input.notes.at(-1)).toBe('miso')
    expect(call.input.notes).toHaveLength(10)
  })

  test('a repeated note cannot be validated', async ({ page }) => {
    await toSettings(page)
    await page.getByTestId('quick-notes-add').click()
    const pad = page.locator('.keypad-modal')
    for (const ch of 'bien cuit') {
      if (ch === ' ') await pad.locator('.key-space').click()
      else await pad.locator('.key', { hasText: new RegExp(`^${ch}$`, 'u') }).click()
    }
    await expect(pad.locator('.act-btn.primary')).toBeDisabled()
  })

  test('cancelling the add changes nothing', async ({ page }) => {
    await toSettings(page)
    await page.getByTestId('quick-notes-add').click()
    await page.locator('.keypad-modal .act-btn.secondary').first().click()
    await expect(notes(page)).toHaveCount(9)
    await expect(page.getByTestId('quick-notes-save')).toBeDisabled()
  })

  test('24 notes fill the list: the add button is disabled', async ({ page }) => {
    const many = Array.from({ length: 24 }, (_, i) => `Note ${i + 1}`)
    await toSettings(page, { get_quick_notes: () => ({ notes: many }) })
    await expect(page.getByTestId('quick-notes-full')).toBeVisible()
    await expect(page.getByTestId('quick-notes-add')).toBeDisabled()
  })

  test('an empty list can be saved', async ({ page }) => {
    await toSettings(page, { get_quick_notes: () => ({ notes: ['Seule'] }) })
    await page.getByTestId('quick-note-remove').click()
    await expect(page.getByTestId('quick-notes-empty')).toBeVisible()
    await page.getByTestId('quick-notes-save').click()
    const [call] = await ipcCalls(page, 'set_quick_notes')
    expect(call.input.notes).toEqual([])
  })

  test('a refusal of the engine is shown and the draft kept', async ({ page }) => {
    await toSettings(page, {
      set_quick_notes: live(() => {
        throw 'quick_notes_too_many|max=24'
      }),
    })
    await page.getByTestId('quick-note-0').getByTestId('quick-note-remove').click()
    await page.getByTestId('quick-notes-save').click()
    await expect(page.getByTestId('quick-notes-error')).toContainText('24 au plus')
    await expect(notes(page)).toHaveCount(8)
    await expect(page.getByTestId('quick-notes-save')).toBeEnabled()
  })
})
