import { test, expect } from '@playwright/test'
import { mockTauriIPC, loginAndGoto, ipcCalls, overrideIPC } from './fixtures/mock-ipc'

const SAMPLE_Z_REPORTS = [
  {
    id: 2,
    reportDate: '2026-04-12',
    sequenceNumber: 2,
    createdAt: '2026-04-12T22:30:00Z',
    fiscalEventId: 10,
  },
  {
    id: 1,
    reportDate: '2026-04-11',
    sequenceNumber: 1,
    createdAt: '2026-04-11T22:15:00Z',
    fiscalEventId: 5,
  },
]

const USER_X_OK = { success: true, reportType: 'X', reportNo: null, error: null }

test.describe('Reports View', () => {
  test('opens on the day tab, with X, user X and the Z close', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await expect(page.getByRole('tab', { name: 'Journée' })).toHaveAttribute(
      'aria-selected',
      'true',
    )
    await expect(page.getByRole('tab')).toHaveText(['Journée', 'Caisse', 'Tickets', 'Ventes'])
    await expect(page.getByRole('button', { name: /^Rapport X/ })).toBeVisible()
    await expect(page.getByRole('button', { name: /^Rapport utilisateurs X/ })).toBeVisible()
    await expect(page.locator('.z-btn')).toHaveText('Clôturer la journée (Z)')
  })

  test('X and user X are generated from their cards', async ({ page }) => {
    await mockTauriIPC(page, { generate_user_x_report: () => USER_X_OK })
    await loginAndGoto(page, '/reports')
    await page.getByRole('button', { name: /^Rapport X/ }).click()
    await expect(page.locator('.toast', { hasText: 'Rapport généré (X)' })).toBeVisible()
    expect(await ipcCalls(page, 'generate_x_report')).toHaveLength(1)
    await page.getByRole('button', { name: /^Rapport utilisateurs X/ }).click()
    await expect.poll(() => ipcCalls(page, 'generate_user_x_report')).toHaveLength(1)
  })

  test('warns while the Z of the day is not done', async ({ page }) => {
    await mockTauriIPC(page, { needs_z_report: () => true })
    await loginAndGoto(page, '/reports')
    await expect(page.locator('.z-notice')).toHaveText("Rapport Z pas encore fait aujourd'hui")
  })

  test('no warning once the Z is done', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await expect(page.locator('.z-btn')).toBeVisible()
    await expect(page.locator('.z-notice')).toHaveCount(0)
  })

  test('the Z clears the pending notice', async ({ page }) => {
    await mockTauriIPC(page, { needs_z_report: () => true })
    await loginAndGoto(page, '/reports')
    await expect(page.locator('.z-notice')).toBeVisible()
    await overrideIPC(page, 'needs_z_report', false)
    await page.locator('.z-btn').click()
    await page.locator('.z-btn').click()
    await expect.poll(() => ipcCalls(page, 'generate_z_report')).toHaveLength(1)
    await expect(page.locator('.z-notice')).toHaveCount(0)
  })

  test('the reprint card names the last Z', async ({ page }) => {
    await mockTauriIPC(page, { list_z_reports: () => SAMPLE_Z_REPORTS })
    await loginAndGoto(page, '/reports')
    await expect(page.locator('.z-history-btn')).toContainText('Dernier : Z #2 · 13/04/2026')
  })

  test('shows Z history with entries', async ({ page }) => {
    await mockTauriIPC(page, { list_z_reports: () => SAMPLE_Z_REPORTS })
    await loginAndGoto(page, '/reports')
    await page.locator('.z-history-btn').click()
    await expect(page.locator('.history-row')).toHaveCount(2)
    await expect(page.locator('.history-row').first()).toContainText('Z #2')
  })

  test('history rows have reprint buttons', async ({ page }) => {
    await mockTauriIPC(page, { list_z_reports: () => SAMPLE_Z_REPORTS })
    await loginAndGoto(page, '/reports')
    await page.locator('.z-history-btn').click()
    await expect(page.locator('.reprint-btn')).toHaveCount(2)
  })

  test('empty history shows placeholder', async ({ page }) => {
    await mockTauriIPC(page, { list_z_reports: () => [] })
    await loginAndGoto(page, '/reports')
    await expect(page.locator('.z-history-btn')).toContainText('Aucun rapport Z')
    await page.locator('.z-history-btn').click()
    await expect(page.locator('.empty')).toBeVisible()
  })

  test('Z generate button is enabled', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await expect(page.locator('.z-btn')).toBeEnabled()
  })

  test('a Z needs a second tap: the first one only asks to confirm', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    const z = page.locator('.z-btn')
    await z.click()
    await expect(z).toHaveText('Confirmer la clôture ?')
    await expect(z).toHaveClass(/armed/)
    expect(await ipcCalls(page, 'generate_z_report')).toHaveLength(0)

    await z.click()
    await expect.poll(() => ipcCalls(page, 'generate_z_report')).toHaveLength(1)
    await expect(z).toHaveText('Clôturer la journée (Z)')
  })

  test('an unconfirmed Z disarms by itself', async ({ page }) => {
    await page.clock.install()
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    const z = page.locator('.z-btn')
    await z.click()
    await expect(z).toHaveText('Confirmer la clôture ?')
    await page.clock.runFor(5_000)
    await expect(z).toHaveText('Clôturer la journée (Z)')
    expect(await ipcCalls(page, 'generate_z_report')).toHaveLength(0)
  })

  test('leaving the day tab disarms the Z', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await page.locator('.z-btn').click()
    await expect(page.locator('.z-btn')).toHaveClass(/armed/)
    await page.getByRole('tab', { name: 'Caisse' }).click()
    await page.getByRole('tab', { name: 'Journée' }).click()
    await expect(page.locator('.z-btn')).not.toHaveClass(/armed/)
    await page.locator('.z-btn').click()
    await expect(page.locator('.z-btn')).toHaveClass(/armed/)
    expect(await ipcCalls(page, 'generate_z_report')).toHaveLength(0)
  })
})

test.describe('Reports - cash tab', () => {
  test('offers only the reasons of the chosen direction', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await page.getByRole('tab', { name: 'Caisse' }).click()
    await expect(page.locator('.cash-reason')).toHaveText(['Fond de caisse', 'Pourboire'])
    await expect(page.locator('.cash-reason.active')).toHaveText('Fond de caisse')
    await page.locator('.cash-dir-out').click()
    await expect(page.locator('.cash-reason')).toHaveText(['Retrait', 'Pourboire'])
    await expect(page.locator('.cash-reason.active')).toHaveText('Retrait')
    await expect(page.locator('.cash-submit')).toHaveText('Enregistrer la sortie')
  })

  test('the summary follows the choices', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await page.getByRole('tab', { name: 'Caisse' }).click()
    await page.locator('.cash-reason', { hasText: 'Pourboire' }).click()
    await page.locator('.cash-preset', { hasText: '100 €' }).click()
    await expect(page.locator('.summary-line')).toHaveText('Entrée · Pourboire')
    await expect(page.locator('.summary-amount')).toContainText('100,00')
    await expect(page.locator('.cash-submit')).toHaveText("Enregistrer l'entrée")
  })

  test('"Autre" types any amount on the keypad', async ({ page }) => {
    await mockTauriIPC(page)
    await loginAndGoto(page, '/reports')
    await page.getByRole('tab', { name: 'Caisse' }).click()
    await page.locator('.cash-other').click()
    for (const key of ['3', '5', '5', '0']) {
      await page.locator('.numpad button', { hasText: new RegExp(`^${key}$`) }).click()
    }
    await page.locator('.btn-confirm').click()
    await expect(page.locator('.summary-amount')).toContainText('35,50')
    await expect(page.locator('.cash-other')).toHaveClass(/active/)
    await page.locator('.cash-submit').click()
    await expect(page.locator('.toast-success')).toContainText('Mouvement enregistré')
    const calls = await ipcCalls(page, 'money_in_out')
    expect(calls).toHaveLength(1)
    expect(calls[0]).toMatchObject({ amountCents: 3550, direction: 'in', reason: 'fond_de_caisse' })
  })
})
