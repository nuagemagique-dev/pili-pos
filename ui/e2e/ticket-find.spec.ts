// R2: find a ticket of the archive by the number printed on it, for the copy and the refund;
// and "Plus ancien" to load the previous 20 tickets of the list.
import { test, expect, type Page } from '@playwright/test'
import { mockTauriIPC, loginAndGoto, ipcCalls, live } from './fixtures/mock-ipc'

// The answer handlers run inside the page, so they carry their own data.

const LATEST = [
  {
    fiscalEventId: 5,
    eventLabel: 'N',
    ticketNumber: 12,
    tableNumber: null,
    totalCents: 1800,
    createdAt: '2026-10-03T10:00:00Z',
  },
]

async function openTickets(page: Page, extra: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    list_copyable_tickets: () => LATEST,
    list_refundable_events: () => [],
    find_ticket_by_number: live(({ number }: any = {}) => {
      const sale = {
        fiscalEventId: 90,
        eventLabel: 'N',
        ticketNumber: 4821,
        tableNumber: null,
        totalCents: 5200,
        createdAt: '2026-10-02T12:10:00Z',
        isRefund: false,
        copyable: {
          fiscalEventId: 90,
          eventLabel: 'N',
          ticketNumber: 4821,
          tableNumber: null,
          totalCents: 5200,
          createdAt: '2026-10-02T12:10:00Z',
        },
        refundable: {
          fiscalEventId: 90,
          ticketNumber: 4821,
          orderId: 7,
          tableNumber: null,
          totalCents: 5200,
          createdAt: '2026-10-02T12:10:00Z',
        },
        refundBlocked: null,
      }
      if (number === 4821) {
        return {
          number,
          matches: [
            sale,
            {
              fiscalEventId: 92,
              eventLabel: 'P',
              ticketNumber: 4821,
              tableNumber: 6,
              totalCents: 3100,
              createdAt: '2026-10-02T12:00:00Z',
              isRefund: false,
              copyable: {
                fiscalEventId: 92,
                eventLabel: 'P',
                ticketNumber: 4821,
                tableNumber: 6,
                totalCents: 3100,
                createdAt: '2026-10-02T12:00:00Z',
              },
              refundable: null,
              refundBlocked: null,
            },
          ],
        }
      }
      if (number === 4000) {
        return {
          number,
          matches: [
            {
              ...sale,
              fiscalEventId: 91,
              ticketNumber: 4000,
              refundable: null,
              refundBlocked: 'refund_already_done',
              copyable: { ...sale.copyable, fiscalEventId: 91, ticketNumber: 4000 },
            },
          ],
        }
      }
      return { number, matches: [] }
    }),
    ...extra,
  })
  await loginAndGoto(page, '/reports')
  await page.getByRole('tab', { name: 'Tickets' }).click()
}

async function typeNumber(page: Page, digits: string) {
  await page.getByTestId('ticket-find').click()
  for (const key of digits) {
    await page.locator('.numpad button', { hasText: new RegExp(`^${key}$`, 'u') }).click()
  }
  await page.getByTestId('number-entry-ok').click()
}

test.describe('Find a ticket by its number [R2]', () => {
  test('the field is "N° de ticket" and the list keeps the latest sales by default', async ({
    page,
  }) => {
    await openTickets(page)
    await expect(page.getByTestId('ticket-find')).toHaveText('N° de ticket')
    await expect(page.locator('.ticket-row')).toHaveCount(1)
    await expect(page.getByTestId('ticket-find-clear')).toHaveCount(0)
    expect(await ipcCalls(page, 'find_ticket_by_number')).toHaveLength(0)
  })

  test('a number found offers the copy and the refund of that sale', async ({ page }) => {
    await openTickets(page)
    await typeNumber(page, '4821')

    const calls = await ipcCalls(page, 'find_ticket_by_number')
    expect(calls).toHaveLength(1)
    expect(calls[0]).toMatchObject({ number: 4821 })
    // The sale and the pre-bill with the same number both come back, the field shows the number.
    await expect(page.getByTestId('ticket-find')).toHaveText('4821')
    await expect(page.locator('.ticket-row')).toHaveCount(2)
    const sale = page.getByTestId('ticket-row-90')
    await expect(sale).toContainText('#4821')
    await expect(sale.getByRole('button', { name: 'Copie' })).toBeVisible()
    await expect(sale.getByRole('button', { name: 'Rembourser' })).toBeVisible()
    const preBill = page.getByTestId('ticket-row-92')
    await expect(preBill.getByRole('button', { name: 'Copie' })).toBeVisible()
    await expect(preBill.getByRole('button', { name: 'Rembourser' })).toHaveCount(0)

    // The refund opens on the lines of that very sale.
    await sale.getByRole('button', { name: 'Rembourser' }).click()
    await expect.poll(() => ipcCalls(page, 'list_order_lines')).toHaveLength(1)
    expect((await ipcCalls(page, 'list_order_lines'))[0]).toMatchObject({ orderId: 7 })
  })

  test('a sale that cannot be refunded shows why, and can still be copied', async ({ page }) => {
    await openTickets(page)
    await typeNumber(page, '4000')

    const row = page.getByTestId('ticket-row-91')
    await expect(row.getByRole('button', { name: 'Rembourser' })).toHaveCount(0)
    await expect(page.getByTestId('ticket-blocked-91')).toHaveText(
      'Cette vente est déjà remboursée en totalité.',
    )
    await expect(row.getByRole('button', { name: 'Copie' })).toBeVisible()
  })

  test('a number nobody holds says so', async ({ page }) => {
    await openTickets(page)
    await typeNumber(page, '999')
    await expect(page.getByTestId('ticket-find-none')).toHaveText('Aucun ticket n° 999')
    await expect(page.locator('.ticket-row')).toHaveCount(0)
  })

  test('the clear button returns to the latest tickets', async ({ page }) => {
    await openTickets(page)
    await typeNumber(page, '4821')
    await expect(page.locator('.ticket-row')).toHaveCount(2)
    await page.getByTestId('ticket-find-clear').click()
    await expect(page.getByTestId('ticket-find-clear')).toHaveCount(0)
    await expect(page.getByTestId('ticket-find')).toHaveText('N° de ticket')
    await expect(page.locator('.ticket-row')).toHaveCount(1)
    await expect(page.getByTestId('ticket-row-5')).toBeVisible()
  })

  test('the pad refuses 0 and a ticket number needs at least one digit', async ({ page }) => {
    await openTickets(page)
    await page.getByTestId('ticket-find').click()
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    await page.locator('.numpad button', { hasText: /^0$/u }).click()
    await expect(page.getByTestId('number-entry-ok')).toBeDisabled()
    expect(await ipcCalls(page, 'find_ticket_by_number')).toHaveLength(0)
  })

  test('the field and its buttons are touch sized', async ({ page }) => {
    await openTickets(page)
    const box = await page.getByTestId('ticket-find').boundingBox()
    expect(box?.height).toBeGreaterThanOrEqual(48)
    await typeNumber(page, '4821')
    const clear = await page.getByTestId('ticket-find-clear').boundingBox()
    expect(clear?.height).toBeGreaterThanOrEqual(48)
  })
})

test.describe('Older tickets [R2]', () => {
  /** `count` copyable tickets, newest first, the way the engine answers a limit. */
  const manyCopyable = live(({ limit }: any = {}) => {
    const total = 60
    return Array.from({ length: Math.min(limit ?? 30, total) }, (_, i) => ({
      fiscalEventId: 1000 - i,
      eventLabel: 'N',
      ticketNumber: 500 - i,
      tableNumber: null,
      totalCents: 1000 + i,
      createdAt: '2026-10-03T10:00:00Z',
    }))
  })

  test('the list shows the latest sales and Plus ancien loads 20 more, until the archive ends', async ({
    page,
  }) => {
    await openTickets(page, { list_copyable_tickets: manyCopyable })
    await expect(page.locator('.ticket-row')).toHaveCount(30)
    await expect(page.getByTestId('tickets-older')).toHaveText('Plus ancien')

    await page.getByTestId('tickets-older').click()
    await expect(page.locator('.ticket-row')).toHaveCount(50)
    await expect(page.getByTestId('tickets-older')).toBeVisible()
    const limits = (await ipcCalls(page, 'list_copyable_tickets')).map((c: any) => c.limit)
    expect(limits).toEqual([30, 50])
    const refundLimits = (await ipcCalls(page, 'list_refundable_events')).map((c: any) => c.limit)
    expect(refundLimits).toEqual([20, 40])

    await page.getByTestId('tickets-older').click()
    await expect(page.locator('.ticket-row')).toHaveCount(60)
    // 60 tickets for a limit of 70: nothing older is left.
    await expect(page.getByTestId('tickets-older')).toHaveCount(0)
  })

  test('there is no Plus ancien while the archive fits the first page', async ({ page }) => {
    await openTickets(page)
    await expect(page.locator('.ticket-row')).toHaveCount(1)
    await expect(page.getByTestId('tickets-older')).toHaveCount(0)
  })

  test('Plus ancien is not offered on a found ticket', async ({ page }) => {
    await openTickets(page, { list_copyable_tickets: manyCopyable })
    await expect(page.getByTestId('tickets-older')).toBeVisible()
    await typeNumber(page, '4821')
    await expect(page.getByTestId('tickets-older')).toHaveCount(0)
  })
})
