import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  loginAndGoto,
  ipcCalls,
  overrideIPC,
  live,
  DEFAULT_USER,
} from './fixtures/mock-ipc'

// Non-fiscal sales report (V11-D20), fourth tab of Reports: presets, a typed range, the two
// tables, the concordance lines and the print. The engine rules are tested in
// crates/pos-engine/tests/sales_report.rs; here the screen is checked against a canned report.

// 2026-10-17 is a Saturday.
const REPORT = {
  from: '2026-10-16',
  to: '2026-10-17',
  generatedAt: '2026-10-17T18:30:00+02:00',
  businessDate: '2026-10-17',
  days: [
    {
      bookingDate: '2026-10-16',
      zNumber: 12,
      totalCents: 12550,
      zTotalCents: 12550,
      open: false,
      matchesZ: true,
      gapCents: 0,
    },
    {
      bookingDate: '2026-10-16',
      zNumber: 13,
      totalCents: 4000,
      zTotalCents: 3800,
      open: false,
      matchesZ: false,
      gapCents: 200,
    },
    {
      bookingDate: '2026-10-17',
      zNumber: null,
      totalCents: 1500,
      zTotalCents: null,
      open: true,
      matchesZ: false,
      gapCents: null,
    },
  ],
  openPeriod: true,
  byProduct: [
    {
      productId: '1',
      name: 'Maki saumon',
      department: 'Plats',
      quantityMilli: 3000,
      amountCents: 1950,
    },
    {
      productId: '2',
      name: 'Sashimi',
      department: 'Plats',
      quantityMilli: 12000,
      amountCents: 900,
    },
    {
      productId: '3',
      name: 'Soupe',
      department: 'Entrees',
      quantityMilli: -1000,
      amountCents: -400,
    },
  ],
  byHour: [
    { hour: 19, tickets: 4, amountCents: 9000, barPercent: 100 },
    { hour: 20, tickets: 2, amountCents: 4500, barPercent: 50 },
    { hour: 1, tickets: 1, amountCents: 500, barPercent: 6 },
  ],
  priceChangesCents: -50,
  roundingCents: 2,
  refundsCents: -400,
  ticketCount: 7,
  totalCents: 18050,
}

async function salesTab(page: Page, report: unknown = REPORT) {
  await page.addInitScript((r) => {
    ;(globalThis as any).__sales = r
  }, report)
  await mockTauriIPC(page, {
    sales_report: live(() => (globalThis as any).__sales),
    print_sales_report: () => undefined,
  })
  await loginAndGoto(page, '/reports')
  await page.getByRole('tab', { name: 'Ventes' }).click()
  await expect(page.getByTestId('sales-report')).toBeVisible()
}

async function lastRange(page: Page) {
  const calls = await ipcCalls(page, 'sales_report')
  const last = calls[calls.length - 1]
  return [last.fromDate, last.toDate]
}

test.describe('Sales report', () => {
  test('is the fourth tab and opens on today, as the engine says it is', async ({ page }) => {
    await salesTab(page)
    await expect(page.getByRole('tab')).toHaveText(['Journée', 'Caisse', 'Tickets', 'Ventes'])
    // The business date of the engine wins over the calendar date of the device.
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-17', '2026-10-17'])
  })

  test('says it is not a fiscal document', async ({ page }) => {
    await salesTab(page)
    await expect(page.getByTestId('sales-notice')).toContainText('Document non fiscal')
  })

  test('presets ask for yesterday, the week from Monday and the month from the 1st', async ({
    page,
  }) => {
    await salesTab(page)
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-17', '2026-10-17'])
    await page.getByTestId('sales-preset-yesterday').click()
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-16', '2026-10-16'])
    await page.getByTestId('sales-preset-week').click()
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-12', '2026-10-17'])
    await page.getByTestId('sales-preset-month').click()
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-01', '2026-10-17'])
    await page.getByTestId('sales-preset-today').click()
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-17', '2026-10-17'])
  })

  test('a typed range is asked for once both dates are real', async ({ page }) => {
    await salesTab(page)
    await page.getByTestId('sales-preset-custom').click()
    const custom = page.getByTestId('sales-custom')
    const type = async (digits: string) => {
      for (const d of digits) await custom.getByRole('button', { name: d, exact: true }).click()
    }
    await expect(page.getByTestId('sales-apply')).toBeDisabled()
    await type('01102026')
    // The "to" field takes over once the first date is complete.
    await expect(page.getByTestId('sales-field-to')).toHaveClass(/active/u)
    await type('1510')
    await expect(page.getByTestId('sales-apply')).toBeDisabled()
    await type('2026')
    await expect(page.getByTestId('sales-field-to')).toContainText('15/10/2026')
    await page.getByTestId('sales-apply').click()
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-01', '2026-10-15'])
    await expect(page.getByTestId('sales-range')).toHaveText('01/10/2026 - 15/10/2026')
  })

  test('an impossible date keeps Show disabled', async ({ page }) => {
    await salesTab(page)
    await page.getByTestId('sales-preset-custom').click()
    const custom = page.getByTestId('sales-custom')
    for (const d of '31022026') await custom.getByRole('button', { name: d, exact: true }).click()
    for (const d of '01032026') await custom.getByRole('button', { name: d, exact: true }).click()
    await expect(page.getByTestId('sales-apply')).toBeDisabled()
  })

  test('the engine refusing the range is told in French', async ({ page }) => {
    await salesTab(page)
    await page.getByTestId('sales-preset-custom').click()
    const custom = page.getByTestId('sales-custom')
    for (const d of '01012026') await custom.getByRole('button', { name: d, exact: true }).click()
    for (const d of '31122026') await custom.getByRole('button', { name: d, exact: true }).click()
    await overrideIPC(page, 'sales_report', { __reject: 'sales_report_range_invalid' })
    await page.getByTestId('sales-apply').click()
    await expect(page.getByTestId('sales-error')).toContainText('92 jours')
  })

  test('shows the products, biggest amount first, then by quantity', async ({ page }) => {
    await salesTab(page)
    const names = page.getByTestId('sales-product').locator('.pname')
    await expect(names).toHaveText(['Maki saumon', 'Sashimi', 'Soupe'])
    await expect(page.getByTestId('sales-product').first()).toContainText('19,50 €')
    await page.getByTestId('sales-sort-quantity').click()
    await expect(names).toHaveText(['Sashimi', 'Maki saumon', 'Soupe'])
    // A refund shows negative.
    await expect(page.getByTestId('sales-product').last()).toContainText('-1')
    await expect(page.getByTestId('sales-product').last()).toContainText('-4,00 €')
  })

  test('draws the hours as bars in proportion to their amount', async ({ page }) => {
    await salesTab(page)
    const bars = page.getByTestId('sales-hour-bar')
    await expect(bars).toHaveCount(3)
    await expect(page.getByTestId('sales-hour').first()).toContainText('19h')
    await expect(page.getByTestId('sales-hour').first()).toContainText('4 tickets')
    const widths = await bars.evaluateAll((els) => els.map((e) => (e as HTMLElement).style.width))
    expect(widths).toEqual(['100%', '50%', '6%'])
  })

  test('shows the concordance of each period: equal, gap and open', async ({ page }) => {
    await salesTab(page)
    const days = page.getByTestId('sales-day')
    await expect(days).toHaveCount(3)
    await expect(days.nth(0)).toContainText('16/10/2026')
    await expect(days.nth(0)).toContainText('Z 12 : identique')
    await expect(days.nth(1)).toContainText('Z 13 : écart de 2,00 € (Z : 38,00 €)')
    await expect(days.nth(1)).toHaveClass(/gap/u)
    await expect(days.nth(2)).toContainText('Période en cours')
    await expect(days.nth(2)).toHaveClass(/open/u)
  })

  test('shows the totals with the rounding apart', async ({ page }) => {
    await salesTab(page)
    await expect(page.getByTestId('sales-total')).toHaveText('180,50 €')
    await expect(page.getByTestId('sales-tickets')).toHaveText('7')
    await expect(page.getByTestId('sales-refunds')).toHaveText('-4,00 €')
    await expect(page.getByTestId('sales-rounding')).toHaveText('0,02 €')
  })

  test('an empty period says so and prints nothing wrong', async ({ page }) => {
    await salesTab(page, {
      ...REPORT,
      days: [],
      byProduct: [],
      byHour: [],
      openPeriod: false,
      ticketCount: 0,
      totalCents: 0,
      refundsCents: 0,
      priceChangesCents: 0,
      roundingCents: 0,
    })
    await expect(page.getByText('Aucune vente sur cette période.')).toBeVisible()
    await expect(page.getByTestId('sales-total')).toHaveText('0,00 €')
    // No price change: the line is not drawn at all.
    await expect(page.getByTestId('sales-price-changes')).toHaveCount(0)
  })

  test('the price changes line shows the amount only when there is one', async ({ page }) => {
    await salesTab(page)
    const line = page.getByTestId('sales-price-changes')
    await expect(line).toHaveText('-0,50 €')
    await expect(line.locator('xpath=..')).toContainText('Modifications de prix (tickets)')
  })

  test('a closed period without a Z says so, neither identical nor a gap', async ({ page }) => {
    const missing = {
      bookingDate: '2026-10-15',
      zNumber: null,
      totalCents: 800,
      zTotalCents: null,
      open: false,
      matchesZ: false,
      gapCents: null,
    }
    await salesTab(page, { ...REPORT, days: [...REPORT.days, missing] })
    const days = page.getByTestId('sales-day')
    await expect(days).toHaveCount(4)
    const row = days.nth(3)
    await expect(row).toContainText('15/10/2026')
    await expect(row).toContainText('Z introuvable pour cette période')
    await expect(row).not.toContainText('identique')
    await expect(row).not.toContainText('écart')
  })

  test('Imprimer sends the shown range to the printer', async ({ page }) => {
    await salesTab(page)
    await page.getByTestId('sales-preset-week').click()
    await expect.poll(() => lastRange(page)).toEqual(['2026-10-12', '2026-10-17'])
    await page.getByTestId('sales-print').click()
    await expect
      .poll(() => ipcCalls(page, 'print_sales_report'))
      .toEqual([{ fromDate: '2026-10-12', toDate: '2026-10-17' }])
    await expect(
      page.locator('.toast', { hasText: "Rapport de ventes envoyé à l'imprimante" }),
    ).toBeVisible()
  })

  test('a printer problem is shown as an error toast, never as a success', async ({ page }) => {
    await salesTab(page)
    await overrideIPC(page, 'print_sales_report', { __reject: 'printer_not_configured' })
    await page.getByTestId('sales-print').click()
    await expect(page.locator('.toast-error')).toContainText('Aucune imprimante configurée.')
    await expect(page.locator('.toast-success')).toHaveCount(0)
    await expect(page.locator('.toast', { hasText: "envoyé à l'imprimante" })).toHaveCount(0)
  })

  // V11-D8: the sales report is for managers (the engine says forbidden). The tab is shown to
  // everybody, like on Android (SalesViewModel), and the refusal is told in French.
  test('a cashier who reaches the tab gets the engine refusal and cannot print', async ({
    page,
  }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' }
    await mockTauriIPC(page, {
      login: () => cashier,
      list_users: () => [cashier],
      print_sales_report: () => undefined,
    })
    await loginAndGoto(page, '/reports')
    await overrideIPC(page, 'sales_report', { __reject: 'forbidden' })
    await page.getByRole('tab', { name: 'Ventes' }).click()
    await expect(page.getByTestId('sales-error')).toHaveText('Action réservée à un autre profil.')
    await expect(page.getByTestId('sales-total')).toHaveCount(0)
    await expect(page.getByTestId('sales-print')).toBeDisabled()
    await page.getByTestId('sales-print').click({ force: true })
    expect(await ipcCalls(page, 'print_sales_report')).toHaveLength(0)
  })
})
