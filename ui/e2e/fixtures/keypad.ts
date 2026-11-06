import { expect, type Locator, type Page } from '@playwright/test'

// Helpers for the on-screen keypads (UX audit C13 / R6): no field of the till takes the OS keyboard,
// so a test types a value the way the staff do, one key at a time.

/** The open text keypad (TextKeypad.vue). */
export const keypad = (page: Page): Locator => page.locator('.keypad-modal')

/** Types `text` on the open text keypad: letters, digits, symbols, ' ' on the space bar. */
export async function typeOnKeypad(page: Page, text: string): Promise<void> {
  const pad = keypad(page)
  const shift = pad.getByTestId('keypad-case')
  for (const ch of text) {
    if (ch === ' ') {
      await pad.getByRole('button', { name: /espace|space|空格/iu }).click()
      continue
    }
    // A capital needs the shift key (sticky, like the Aa key of TextKeyboard).
    const isUpper = ch !== ch.toLowerCase()
    if ((await shift.count()) > 0) {
      const on = (await shift.getAttribute('aria-pressed')) === 'true'
      if (on !== isUpper) await shift.click()
    }
    await pad.locator('.key', { hasText: new RegExp(`^${escapeRe(ch)}$`, 'u') }).click()
  }
}

/** Validates the open text keypad. */
export async function confirmKeypad(page: Page): Promise<void> {
  await keypad(page).locator('.act-btn.primary').click()
  await expect(keypad(page)).toHaveCount(0)
}

/** Opens `field`, types `text` on the text keypad and validates it. */
export async function fillWithKeypad(page: Page, field: Locator, text: string): Promise<void> {
  await field.click()
  await keypad(page)
    .locator('.act-btn', { hasText: /effacer|clear|清除/iu })
    .click()
  await typeOnKeypad(page, text)
  await confirmKeypad(page)
}

/** Opens `field` (numeric), types `digits` on the NumPad of the number dialog and validates it. */
export async function fillWithNumPad(page: Page, field: Locator, digits: string): Promise<void> {
  await field.click()
  const dialog = page.locator('[role="dialog"]', { has: page.getByTestId('number-entry-value') })
  await dialog.locator('.key-clear').click()
  for (const d of digits)
    await dialog.locator('.key', { hasText: new RegExp(`^${d}$`, 'u') }).click()
  await dialog.getByTestId('number-entry-ok').click()
  await expect(dialog).toHaveCount(0)
}

function escapeRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')
}
