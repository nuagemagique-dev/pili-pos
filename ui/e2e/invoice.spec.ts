import { test, expect, type Page } from '@playwright/test';
import { mockTauriIPC, loginAndGoto, ipcCalls, overrideIPC, DEFAULT_USER } from './fixtures/mock-ipc';

// B2B invoices (event I, M150): multi-select on the tickets tab, the
// dialog (customer, preview, signing, result) and the list of invoices.

const SALES = [
  { fiscalEventId: 12, ticketNumber: 102, posDateTime: '2026-10-17T12:30:00+02:00', totalCents: 1500, fdmId: 'FDM0000001', eventCounter: 6, totalCounter: 22, isRefund: false, invoicedBy: null },
  { fiscalEventId: 11, ticketNumber: 101, posDateTime: '2026-10-17T12:10:00+02:00', totalCents: 2400, fdmId: 'FDM0000001', eventCounter: 5, totalCounter: 21, isRefund: false, invoicedBy: null },
  { fiscalEventId: 10, ticketNumber: 100, posDateTime: '2026-10-17T11:50:00+02:00', totalCents: 900, fdmId: 'FDM0000001', eventCounter: 4, totalCounter: 20, isRefund: false, invoicedBy: '20260001' },
];

const COPYABLE = SALES.map((s) => ({
  fiscalEventId: s.fiscalEventId, eventLabel: 'N', ticketNumber: s.ticketNumber, tableNumber: 3,
  totalCents: s.totalCents, createdAt: '2026-10-17T10:00:00Z',
}));

const CUSTOMER = {
  id: 7, vatNumber: 'BE0308357159', name: 'ACME SPRL', street: 'RUE DU PONT 1', postalCode: '4000',
  city: 'LIEGE', countryCode: 'BE', peppolStatus: 'registered', peppolCheckedAt: '2026-10-17 10:00:00',
};

const LOOKUP_REGISTERED = {
  vatNumber: 'BE0308357159', countryCode: 'BE', customer: null,
  peppolStatus: 'registered', peppolChecked: true, deliverable: true,
};

const PREVIEW = {
  sales: [SALES[1], SALES[0]],
  lines: [
    { description: 'Maki saumon x8', quantityMilli: 2000, totalCents: 2400, vatLabel: 'B' },
    { description: 'Coca-Cola', quantityMilli: 1000, totalCents: 1500, vatLabel: 'A' },
  ],
  vat: [
    { label: 'A', rateHundredths: 2100, baseCents: 1240, vatCents: 260, totalCents: 1500 },
    { label: 'B', rateHundredths: 1200, baseCents: 2143, vatCents: 257, totalCents: 2400 },
  ],
  totalCents: 3900,
  problems: [],
};

const RESULT_OK = {
  success: true, fiscalEventId: 40, invoiceNo: '20260002', printed: true, deliveryStatus: 'queued', error: null,
};

function base(over: Record<string, unknown> = {}) {
  return {
    list_invoiceable_sales: () => SALES,
    list_copyable_tickets: () => COPYABLE,
    lookup_b2b_customer: () => LOOKUP_REGISTERED,
    save_b2b_customer: () => CUSTOMER,
    invoice_preview: () => PREVIEW,
    sign_invoice_fiscal: () => RESULT_OK,
    ...over,
  };
}

/** Types on the on-screen keyboard, key by key. */
async function typeKeys(page: Page, text: string) {
  const kbd = page.getByTestId('text-keyboard');
  for (const ch of text) {
    if (ch === ' ') await kbd.getByRole('button', { name: 'Espace' }).click();
    else await kbd.getByRole('button', { name: ch, exact: true }).click();
  }
}

async function field(page: Page, id: string, text: string) {
  await page.getByTestId(`invoice-field-${id}`).click();
  await typeKeys(page, text);
}

async function fillAddress(page: Page) {
  await field(page, 'name', 'ACME SPRL');
  await field(page, 'street', 'RUE DU PONT 1');
  await field(page, 'postalCode', '4000');
  await field(page, 'city', 'LIEGE');
}

async function openDialog(page: Page) {
  await loginAndGoto(page, '/reports');
  await page.getByRole('tab', { name: 'Tickets' }).click();
  await page.getByTestId('ticket-check-11').click();
  await page.getByTestId('ticket-check-12').click();
  await page.getByTestId('invoice-action').click();
  await expect(page.getByTestId('invoice-dialog')).toBeVisible();
}

test.describe('Invoices: selection on the tickets tab', () => {
  test('the Facture action needs a ticked sale, an invoiced sale cannot be ticked', async ({ page }) => {
    await mockTauriIPC(page, base());
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    const action = page.getByTestId('invoice-action');
    await expect(action).toBeDisabled();
    await expect(action).toHaveText('Facture');
    await page.getByTestId('ticket-check-11').click();
    await expect(action).toHaveText('Facture (1)');
    await page.getByTestId('ticket-check-12').click();
    await expect(action).toHaveText('Facture (2)');
    await page.getByTestId('ticket-check-11').click();
    await expect(action).toHaveText('Facture (1)');
    // Ticket 100 is on invoice 20260001: no box, the number instead.
    await expect(page.getByTestId('ticket-check-10')).toHaveCount(0);
    await expect(page.getByTestId('ticket-invoiced-10')).toHaveText('Facture 20260001');
  });

  test('the tabs stay three', async ({ page }) => {
    await mockTauriIPC(page, base());
    await loginAndGoto(page, '/reports');
    await expect(page.getByRole('tab')).toHaveText(['Journée', 'Caisse', 'Tickets']);
  });
});

test.describe('Invoices: the dialog', () => {
  test('a new customer, a Peppol lookup, the preview and the signed invoice', async ({ page }) => {
    await mockTauriIPC(page, base());
    await openDialog(page);
    await expect(page.getByTestId('invoice-continue')).toBeDisabled();

    // VAT number on the keyboard, checked by the engine.
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'BE0308357159');
    await page.getByTestId('invoice-vat-check').click();
    await expect(page.getByTestId('invoice-chip')).toHaveText('Inscrit sur Peppol');
    expect(await ipcCalls(page, 'lookup_b2b_customer')).toEqual([{ vatNumber: 'BE0308357159' }]);

    await fillAddress(page);
    await expect(page.getByTestId('invoice-field-name')).toHaveText('ACME SPRL');
    await page.getByTestId('invoice-continue').click();

    // The customer is saved, then the engine previews the invoice of the ticked sales.
    await expect(page.getByTestId('invoice-preview')).toBeVisible();
    const saved = await ipcCalls(page, 'save_b2b_customer');
    expect(saved).toHaveLength(1);
    expect(saved[0].customer).toMatchObject({
      id: null, vatNumber: 'BE0308357159', name: 'ACME SPRL', street: 'RUE DU PONT 1', postalCode: '4000', city: 'LIEGE',
    });
    const previewCalls = await ipcCalls(page, 'invoice_preview');
    expect([...previewCalls[0].saleEventIds].sort()).toEqual([11, 12]);
    await expect(page.getByTestId('invoice-lines').locator('li')).toHaveCount(2);
    await expect(page.getByTestId('invoice-vat').locator('tbody tr')).toHaveCount(2);
    await expect(page.getByTestId('invoice-total')).toContainText('39,00');
    await expect(page.getByTestId('invoice-already-paid')).toContainText('Déjà payé : 39,00 €');

    await page.getByTestId('invoice-confirm').click();
    await expect(page.getByTestId('invoice-result')).toContainText('Facture 20260002 établie');
    await expect(page.getByTestId('invoice-result')).toContainText('Facture imprimée.');
    await expect(page.getByTestId('invoice-result')).toContainText("Envoi Peppol en file d'attente.");
    const signed = await ipcCalls(page, 'sign_invoice_fiscal');
    expect(signed).toHaveLength(1);
    expect([...signed[0].saleEventIds].sort()).toEqual([11, 12]);
    expect(signed[0].customerId).toBe(7);
    expect(signed[0].employeeInsz).toBe(DEFAULT_USER.insz);

    await page.getByTestId('invoice-finish').click();
    await expect(page.getByTestId('invoice-dialog')).toHaveCount(0);
    // The ticks are cleared once the invoice is signed.
    await expect(page.getByTestId('invoice-action')).toBeDisabled();
  });

  test('offline: the lookup is unknown and the invoice is still possible', async ({ page }) => {
    await mockTauriIPC(page, base({
      lookup_b2b_customer: () => ({ ...LOOKUP_REGISTERED, peppolStatus: 'unknown', peppolChecked: false }),
    }));
    await openDialog(page);
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'BE0308357159');
    await page.getByTestId('invoice-vat-check').click();
    await expect(page.getByTestId('invoice-chip')).toHaveText('Peppol inconnu (hors ligne)');
    await expect(page.getByTestId('invoice-chip-note')).toContainText("l'envoi sera retenté");
    await fillAddress(page);
    await expect(page.getByTestId('invoice-continue')).toBeEnabled();
    await page.getByTestId('invoice-continue').click();
    await expect(page.getByTestId('invoice-preview')).toBeVisible();
    await expect(page.getByTestId('invoice-chip')).toHaveText('Peppol inconnu (hors ligne)');
  });

  test('a customer not on Peppol is told before signing', async ({ page }) => {
    await mockTauriIPC(page, base({
      lookup_b2b_customer: () => ({ ...LOOKUP_REGISTERED, peppolStatus: 'not_registered' }),
    }));
    await openDialog(page);
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'BE0308357159');
    await page.getByTestId('invoice-vat-check').click();
    await expect(page.getByTestId('invoice-chip')).toHaveText('Non inscrit sur Peppol');
    await expect(page.getByTestId('invoice-chip-note')).toContainText('Ce client ne peut pas recevoir la facture par Peppol');
  });

  test('a foreign customer gets a printed invoice only', async ({ page }) => {
    await mockTauriIPC(page, base({
      lookup_b2b_customer: () => ({
        vatNumber: 'FR12345678901', countryCode: 'FR', customer: null,
        peppolStatus: 'unknown', peppolChecked: false, deliverable: false,
      }),
    }));
    await openDialog(page);
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'FR12345678901');
    await page.getByTestId('invoice-vat-check').click();
    await expect(page.getByTestId('invoice-chip-note')).toContainText('Client hors Belgique');
  });

  test('a wrong VAT number is refused by the engine check and the dialog stays on the customer', async ({ page }) => {
    await mockTauriIPC(page, base());
    await openDialog(page);
    await overrideIPC(page, 'lookup_b2b_customer', { __reject: 'customer_vat_invalid' });
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'BE0123456789');
    await page.getByTestId('invoice-vat-check').click();
    await expect(page.getByTestId('invoice-error')).toContainText('Numéro de TVA invalide');
    await expect(page.getByTestId('invoice-chip')).toHaveCount(0);
    await fillAddress(page);
    await page.getByTestId('invoice-continue').click();
    await expect(page.getByTestId('invoice-error')).toContainText('Numéro de TVA invalide');
    await expect(page.getByTestId('invoice-preview')).toHaveCount(0);
    expect(await ipcCalls(page, 'save_b2b_customer')).toHaveLength(0);
  });

  test('a known customer is found by name and fills the form', async ({ page }) => {
    await mockTauriIPC(page, base({ search_b2b_customers: () => [CUSTOMER] }));
    await openDialog(page);
    await page.getByTestId('invoice-search-field').click();
    await typeKeys(page, 'AC');
    await expect(page.getByTestId('invoice-search-results')).toContainText('ACME SPRL');
    expect((await ipcCalls(page, 'search_b2b_customers')).at(-1)).toEqual({ query: 'AC' });
    await page.getByTestId('invoice-search-results').getByRole('button').first().click();
    await expect(page.getByTestId('invoice-vat-field')).toHaveText('BE0308357159');
    await expect(page.getByTestId('invoice-field-name')).toHaveText('ACME SPRL');
    await expect(page.getByTestId('invoice-field-city')).toHaveText('LIEGE');
    await expect(page.getByTestId('invoice-continue')).toBeEnabled();
    await page.getByTestId('invoice-continue').click();
    await expect(page.getByTestId('invoice-preview')).toBeVisible();
    const saved = await ipcCalls(page, 'save_b2b_customer');
    expect(saved[0].customer.id).toBe(7);
  });

  test('no known customer says so', async ({ page }) => {
    await mockTauriIPC(page, base());
    await openDialog(page);
    await page.getByTestId('invoice-search-field').click();
    await typeKeys(page, 'ZZ');
    await expect(page.getByTestId('invoice-no-match')).toHaveText('Aucun client connu.');
  });

  test('a problem of the preview blocks the confirmation', async ({ page }) => {
    await mockTauriIPC(page, base({
      invoice_preview: () => ({ ...PREVIEW, problems: ['invoice_sale_already_invoiced|event=11|invoice=20260001'] }),
    }));
    await openDialog(page);
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'BE0308357159');
    await fillAddress(page);
    await page.getByTestId('invoice-continue').click();
    await expect(page.getByTestId('invoice-problem')).toContainText('La vente 11 figure déjà sur la facture 20260001.');
    await expect(page.getByTestId('invoice-confirm')).toBeDisabled();
    await page.getByTestId('invoice-back').click();
    await expect(page.getByTestId('invoice-continue')).toBeVisible();
  });

  test('a refused signature shows the engine error', async ({ page }) => {
    await mockTauriIPC(page, base({
      sign_invoice_fiscal: () => ({
        success: false, fiscalEventId: null, invoiceNo: null, printed: false, deliveryStatus: null,
        error: 'invoice_training_mode',
      }),
    }));
    await openDialog(page);
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'BE0308357159');
    await fillAddress(page);
    await page.getByTestId('invoice-continue').click();
    await page.getByTestId('invoice-confirm').click();
    await expect(page.getByTestId('invoice-result')).toContainText("La facture n'a pas été établie.");
    await expect(page.getByTestId('invoice-error')).toContainText('Pas de facture en mode formation.');
  });

  test('an invoice printed on paper says so when it was not printed', async ({ page }) => {
    await mockTauriIPC(page, base({ sign_invoice_fiscal: () => ({ ...RESULT_OK, printed: false, deliveryStatus: null }) }));
    await openDialog(page);
    await page.getByTestId('invoice-vat-field').click();
    await typeKeys(page, 'BE0308357159');
    await fillAddress(page);
    await page.getByTestId('invoice-continue').click();
    await page.getByTestId('invoice-confirm').click();
    await expect(page.getByTestId('invoice-result')).toContainText('pas imprimée');
    await expect(page.getByTestId('invoice-result')).toContainText('Pas d\'envoi Peppol pour cette facture.');
  });

  test('the keyboard types accents and punctuation and deletes', async ({ page }) => {
    await mockTauriIPC(page, base());
    await openDialog(page);
    await page.getByTestId('invoice-field-street').click();
    await typeKeys(page, "RUE DE L'ÉGLISE");
    await expect(page.getByTestId('invoice-field-street')).toHaveText("RUE DE L'ÉGLISE");
    await page.getByTestId('text-keyboard').getByRole('button', { name: 'Effacer une lettre' }).click();
    await expect(page.getByTestId('invoice-field-street')).toHaveText("RUE DE L'ÉGLIS");
    await page.getByTestId('text-keyboard').getByRole('button', { name: 'Effacer', exact: true }).click();
    await expect(page.getByTestId('invoice-field-street')).toHaveText('');
  });

  test('the next key walks the form', async ({ page }) => {
    await mockTauriIPC(page, base());
    await openDialog(page);
    await page.getByTestId('invoice-vat-field').click();
    await page.getByTestId('text-keyboard').getByRole('button', { name: 'Suivant' }).click();
    await typeKeys(page, 'X');
    await expect(page.getByTestId('invoice-field-name')).toHaveText('X');
  });
});

test.describe('Invoices: the list', () => {
  const INVOICES = [
    { id: 3, invoiceNo: '20260003', fiscalEventId: 43, issueDate: '2026-10-17', customerName: 'ACME SPRL', customerVatNumber: 'BE0308357159', totalCents: 3900, saleEventIds: [11, 12], deliveryStatus: 'queued', deliveryId: 5 },
    { id: 2, invoiceNo: '20260002', fiscalEventId: 41, issueDate: '2026-10-16', customerName: 'BETA SA', customerVatNumber: 'BE0403019261', totalCents: 1200, saleEventIds: [8], deliveryStatus: 'accepted', deliveryId: 4 },
    { id: 1, invoiceNo: '20260001', fiscalEventId: 38, issueDate: '2026-10-15', customerName: 'GAMMA GMBH', customerVatNumber: 'DE123456789', totalCents: 900, saleEventIds: [10], deliveryStatus: 'failed', deliveryId: 3 },
    { id: 0, invoiceNo: '20250999', fiscalEventId: 30, issueDate: '2026-10-10', customerName: 'DELTA', customerVatNumber: 'FR12345678901', totalCents: 500, saleEventIds: [2], deliveryStatus: null, deliveryId: null },
  ];

  test('lists the invoices with the Peppol state of each', async ({ page }) => {
    await mockTauriIPC(page, base({ list_invoices: () => INVOICES }));
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    const rows = page.getByTestId('invoice-list').locator('li');
    await expect(rows).toHaveCount(4);
    await expect(rows.nth(0)).toContainText('20260003');
    await expect(rows.nth(0)).toContainText('ACME SPRL');
    await expect(rows.nth(0)).toContainText('17/10/2026');
    await expect(rows.nth(0)).toContainText('39,00 €');
    await expect(rows.nth(0).getByTestId('invoice-status')).toHaveText('Peppol : en attente');
    await expect(rows.nth(1).getByTestId('invoice-status')).toHaveText('Peppol : remise');
    await expect(rows.nth(2).getByTestId('invoice-status')).toHaveText('Peppol : bloquée');
    await expect(rows.nth(3).getByTestId('invoice-status')).toHaveText('Papier seulement');
  });

  test('no list while there is no invoice', async ({ page }) => {
    await mockTauriIPC(page, base());
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    await expect(page.getByTestId('invoice-list')).toHaveCount(0);
  });

  test('Copie signs a copy of the invoice event', async ({ page }) => {
    await mockTauriIPC(page, base({ list_invoices: () => INVOICES }));
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    await page.getByTestId('invoice-copy-3').click();
    await expect.poll(() => ipcCalls(page, 'copy_ticket')).toHaveLength(1);
    const call = (await ipcCalls(page, 'copy_ticket'))[0];
    expect(call.fiscalEventId).toBe(43);
    expect(call.employeeInsz).toBe(DEFAULT_USER.insz);
    await expect(page.locator('.toast', { hasText: 'Copie' })).toBeVisible();
  });

  test('a cashier gets no Copie button', async ({ page }) => {
    const cashier = { ...DEFAULT_USER, role: 'cashier' };
    await mockTauriIPC(page, base({ list_invoices: () => INVOICES, login: () => cashier, list_users: () => [cashier] }));
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    await expect(page.getByTestId('invoice-list').locator('li')).toHaveCount(4);
    await expect(page.getByTestId('invoice-copy-3')).toHaveCount(0);
  });
});
