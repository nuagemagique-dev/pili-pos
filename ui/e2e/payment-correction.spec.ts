import { test, expect, type Page } from '@playwright/test';
import { mockTauriIPC, loginAndGoto, ipcCalls, overrideIPC, DEFAULT_USER } from './fixtures/mock-ipc';

// Correcting the payment method of a sale signed today (command
// correct_payment, event F): a manager picks the payment line and the
// real method in the Tickets tab, then confirms with the PIN.

const CASHIER = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa Serveuse', role: 'cashier' };

const TICKETS = [
  { fiscalEventId: 41, eventLabel: 'N', ticketNumber: 9, tableNumber: null, totalCents: -1200, createdAt: '2026-09-25 13:00:00' },
  { fiscalEventId: 40, eventLabel: 'P', ticketNumber: 8, tableNumber: 4, totalCents: 2000, createdAt: '2026-09-25 12:30:00' },
  { fiscalEventId: 38, eventLabel: 'N', ticketNumber: 6, tableNumber: 4, totalCents: 5200, createdAt: '2026-09-25 12:00:00' },
];

const CASH_SALE = {
  fiscalEventId: 38, ticketNumber: 6, totalCents: 5200,
  lines: [{ index: 0, paymentType: 'CASH', name: 'Cash', amountCents: 5200, inputMethod: 'MANUAL', amountType: 'PAYMENT', brand: null }],
  correctable: true, refusal: null, corrected: null,
};

const SPLIT_WITH_TERMINAL = {
  ...CASH_SALE,
  lines: [
    { index: 0, paymentType: 'CARD_DEBIT', name: 'Card', amountCents: 2000, inputMethod: 'AUTOMATIC', amountType: 'PAYMENT', brand: 'Bancontact' },
    { index: 1, paymentType: 'CASH', name: 'Cash', amountCents: 3200, inputMethod: 'MANUAL', amountType: 'PAYMENT', brand: null },
  ],
};

async function openTickets(page: Page, extra: Record<string, unknown> = {}, user = DEFAULT_USER) {
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    list_copyable_tickets: () => TICKETS,
    list_refundable_events: () => [],
    list_sale_payments: () => CASH_SALE,
    ...extra,
  });
  await loginAndGoto(page, '/reports');
  await page.getByRole('tab', { name: 'Tickets' }).click();
}

async function typePin(page: Page, pin: string) {
  const dialog = page.locator('.pin-dialog');
  for (const d of pin) await dialog.getByRole('button', { name: d, exact: true }).click();
}

test.describe('Payment correction', () => {
  test('is offered on sales only, to a manager', async ({ page }) => {
    await openTickets(page);
    await expect(page.getByTestId('correct-payment-38')).toBeVisible();
    // A refund (negative total) and a pre-bill are not sales.
    await expect(page.getByTestId('correct-payment-41')).toHaveCount(0);
    await expect(page.getByTestId('correct-payment-40')).toHaveCount(0);
  });

  test('a cashier does not see the action', async ({ page }) => {
    await openTickets(page, {}, CASHIER);
    await expect(page.locator('.ticket-row')).toHaveCount(3);
    await expect(page.locator('.correct-btn')).toHaveCount(0);
  });

  test('corrects cash to card with the PIN', async ({ page }) => {
    await openTickets(page);
    await page.getByTestId('correct-payment-38').click();

    const dialog = page.getByTestId('pc-dialog');
    await expect(dialog).toBeVisible();
    // The only payment line is picked; the method it has is not offered.
    await expect(dialog.getByTestId('pc-line-0')).toContainText('Espèces · 52,00');
    await expect(dialog.getByTestId('pc-method-cash')).toBeDisabled();
    await expect(dialog.getByTestId('pc-confirm')).toBeDisabled();

    await dialog.getByTestId('pc-method-card').click();
    await expect(dialog.getByTestId('pc-summary')).toHaveText('Espèces 52,00 € → Carte 52,00 €');
    await dialog.getByTestId('pc-confirm').click();
    await expect(page.locator('.pin-dialog .hint')).toContainText('code PIN');
    // Once signed, the engine reports the correction the sale now has.
    await overrideIPC(page, 'list_sale_payments', {
      ...CASH_SALE,
      correctable: false,
      refusal: 'payment_correction_already_done',
      corrected: { fromType: 'CASH', toType: 'CARD_DEBIT', amountCents: 5200, at: '2026-09-25 12:10:00' },
    });
    await typePin(page, '4321');

    await expect(page.locator('.toast', { hasText: 'Paiement corrigé' })).toBeVisible();
    await expect(dialog).toHaveCount(0);
    expect(await ipcCalls(page, 'correct_payment')).toEqual([
      { fiscalEventId: 38, lineIndex: 0, method: 'card', insz: DEFAULT_USER.insz, pin: '4321' },
    ]);
    // The tickets are read again: the row shows the mark, not the action.
    await expect(page.getByTestId('corrected-38')).toHaveText('Paiement corrigé : Espèces → Carte');
    await expect(page.getByTestId('correct-payment-38')).toHaveCount(0);
  });

  test('a card taken on the terminal cannot be picked', async ({ page }) => {
    await openTickets(page, { list_sale_payments: () => SPLIT_WITH_TERMINAL });
    await page.getByTestId('correct-payment-38').click();
    const dialog = page.getByTestId('pc-dialog');
    await expect(dialog.getByTestId('pc-line-0')).toBeDisabled();
    await expect(dialog.getByTestId('pc-line-0')).toContainText('Terminal');
    await expect(dialog.getByTestId('pc-line-0')).toContainText('Bancontact');
    await expect(dialog.getByTestId('pc-line-1')).toBeEnabled();
    // The only line left is picked on its own: cash is what it has.
    await expect(dialog.getByTestId('pc-method-cash')).toBeDisabled();
    await expect(dialog.getByTestId('pc-method-card')).toBeEnabled();
    await dialog.getByTestId('pc-method-meal_voucher').click();
    await expect(dialog.getByTestId('pc-summary')).toHaveText('Espèces 32,00 € → Titres-repas 32,00 €');
  });

  test('says why a sale cannot be corrected', async ({ page }) => {
    await openTickets(page, {
      list_sale_payments: () => ({ ...CASH_SALE, correctable: false, refusal: 'payment_correction_period_closed' }),
    });
    await page.getByTestId('correct-payment-38').click();
    await expect(page.getByTestId('pc-refusal')).toContainText('journée déjà clôturée');
    await expect(page.getByTestId('pc-confirm')).toHaveCount(0);
  });

  test('a wrong PIN corrects nothing and says so', async ({ page }) => {
    await openTickets(page, { correct_payment: () => ({ success: false, error: 'pin_wrong' }) });
    await page.getByTestId('correct-payment-38').click();
    await page.getByTestId('pc-method-card').click();
    await page.getByTestId('pc-confirm').click();
    await typePin(page, '0000');
    await expect(page.locator('.pin-dialog .error-msg')).toHaveText('PIN incorrect');
    await expect(page.locator('.toast', { hasText: 'Paiement corrigé' })).toHaveCount(0);
  });

  test("shows the engine's refusal after the PIN", async ({ page }) => {
    await openTickets(page);
    await overrideIPC(page, 'correct_payment', { success: false, error: 'payment_correction_rounding' });
    await page.getByTestId('correct-payment-38').click();
    await page.getByTestId('pc-method-card').click();
    await page.getByTestId('pc-confirm').click();
    await typePin(page, '4321');
    await expect(page.locator('.pin-dialog .error-msg')).toContainText('arrondi');
  });

  test('marks a corrected sale and offers no second correction', async ({ page }) => {
    await openTickets(page, {
      list_sale_payments: () => ({
        ...CASH_SALE,
        correctable: false,
        refusal: 'payment_correction_already_done',
        corrected: { fromType: 'CASH', toType: 'CARD_DEBIT', amountCents: 5200, at: '2026-09-25 12:10:00' },
      }),
    });
    await expect(page.getByTestId('corrected-38')).toHaveText('Paiement corrigé : Espèces → Carte');
    await expect(page.getByTestId('correct-payment-38')).toHaveCount(0);
  });

  test('several manual lines: none preselected, a line enables the methods, rounding is not listed', async ({ page }) => {
    await openTickets(page, {
      list_sale_payments: () => ({
        ...CASH_SALE,
        totalCents: 5200,
        lines: [
          { index: 0, paymentType: 'CASH', name: 'Cash', amountCents: 3000, inputMethod: 'MANUAL', amountType: 'PAYMENT', brand: null },
          { index: 1, paymentType: 'CHEQUE_MEAL', name: 'Meal', amountCents: 2200, inputMethod: 'MANUAL', amountType: 'PAYMENT', brand: null },
          { index: 2, paymentType: 'CASH', name: 'Rounding', amountCents: -2, inputMethod: 'MANUAL', amountType: 'ROUNDING', brand: null },
        ],
      }),
    });
    await page.getByTestId('correct-payment-38').click();
    const dialog = page.getByTestId('pc-dialog');

    // The rounding is not a payment.
    await expect(dialog.getByTestId('pc-line-0')).toBeVisible();
    await expect(dialog.getByTestId('pc-line-1')).toBeVisible();
    await expect(dialog.getByTestId('pc-line-2')).toHaveCount(0);
    // Nothing is picked: no method, no confirmation.
    await expect(dialog.getByTestId('pc-confirm')).toBeDisabled();
    for (const m of ['cash', 'card', 'meal_voucher']) {
      await expect(dialog.getByTestId(`pc-method-${m}`)).toBeDisabled();
    }

    // Picking the cash line offers the other two.
    await dialog.getByTestId('pc-line-0').click();
    await expect(dialog.getByTestId('pc-method-cash')).toBeDisabled();
    await expect(dialog.getByTestId('pc-method-card')).toBeEnabled();
    await dialog.getByTestId('pc-method-meal_voucher').click();
    await expect(dialog.getByTestId('pc-summary')).toHaveText('Espèces 30,00 € → Titres-repas 30,00 €');
    await expect(dialog.getByTestId('pc-confirm')).toBeEnabled();

    // The meal voucher line already is a meal voucher: the choice is cleared.
    await dialog.getByTestId('pc-line-1').click();
    await expect(dialog.getByTestId('pc-summary')).toHaveCount(0);
    await expect(dialog.getByTestId('pc-confirm')).toBeDisabled();
    await expect(dialog.getByTestId('pc-method-meal_voucher')).toBeDisabled();

    await dialog.getByTestId('pc-method-cash').click();
    await dialog.getByTestId('pc-confirm').click();
    await typePin(page, '4321');
    expect(await ipcCalls(page, 'correct_payment')).toEqual([
      { fiscalEventId: 38, lineIndex: 1, method: 'cash', insz: DEFAULT_USER.insz, pin: '4321' },
    ]);
  });

  test('a lookup that fails is shown in the dialog', async ({ page }) => {
    await openTickets(page);
    // The row loaded; the lookup fails when the dialog opens.
    await overrideIPC(page, 'list_sale_payments', { __reject: 'payment_correction_not_a_sale' });
    await page.getByTestId('correct-payment-38').click();
    await expect(page.getByTestId('pc-refusal')).toHaveText('Seul un ticket de vente peut être corrigé.');
    await expect(page.getByTestId('pc-confirm')).toHaveCount(0);
  });

  test('says "cannot be corrected" when the engine gives no reason, and keeps the correction note', async ({ page }) => {
    await openTickets(page);
    await overrideIPC(page, 'list_sale_payments', {
      ...CASH_SALE,
      correctable: false,
      refusal: null,
      corrected: { fromType: 'CASH', toType: 'CHEQUE_MEAL', amountCents: 5200, at: '2026-09-25 12:10:00' },
    });
    await page.getByTestId('correct-payment-38').click();
    await expect(page.getByTestId('pc-refusal')).toHaveText('Ce paiement ne peut pas être corrigé');
    await expect(page.getByTestId('pc-corrected')).toHaveText('Paiement corrigé : Espèces → Titres-repas');
    await expect(page.getByTestId('pc-confirm')).toHaveCount(0);
  });

  test('back closes the dialog without a call', async ({ page }) => {
    await openTickets(page);
    await page.getByTestId('correct-payment-38').click();
    await page.getByTestId('pc-close').click();
    await expect(page.getByTestId('pc-dialog')).toHaveCount(0);
    expect(await ipcCalls(page, 'correct_payment')).toHaveLength(0);
  });
});
