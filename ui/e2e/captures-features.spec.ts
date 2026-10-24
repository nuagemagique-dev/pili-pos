import { test, expect, type Page } from '@playwright/test';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  mockTauriIPC,
  loginAndGoto,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CART_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc';

// Certification dossier screenshots of the v1.0 features added after the
// first tour (table transfer, payment correction, till discount, kitchen
// notes, invoices, sold-out, menu editor, sales report, owner alerts,
// allergens, courses, gift vouchers). Run with `npm run captures`, like
// captures.spec.ts; the names (60 to 99) continue its numbering and are
// listed in docs/internal/CAPTURES_ECRAN_RUNBOOK_FR.md. Every test mocks
// the Tauri IPC layer like the rest of the e2e suite and asserts the
// expected screen before the picture is taken. SPF test data only.

const SCREENSHOTS_DIR = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../docs/SUBMISSION_ARTIFACTS/screenshots',
);

async function capture(page: Page, name: string) {
  // PILI_CAPTURES_DRY=1 runs the scenarios without writing a PNG.
  if (process.env.PILI_CAPTURES_DRY) return;
  await page.screenshot({ path: path.join(SCREENSHOTS_DIR, `${name}.png`), animations: 'disabled' });
}

const GERANT = { ...DEFAULT_USER, insz: '94060242576', name: 'Gérant Test', role: 'admin' };
const asManager = {
  login: () => GERANT,
  list_users: () => [GERANT],
};

async function typePin(page: Page, pin: string) {
  const dialog = page.locator('.pin-dialog');
  for (const d of pin) await dialog.getByRole('button', { name: d, exact: true }).click();
}

async function toOrder(page: Page, extra: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    ...asManager,
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2400 }),
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...extra,
  } as any);
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

/** A settings panel, brought into view in its scrolling column. */
async function settingsPanel(page: Page, extra: Record<string, unknown>, testId: string) {
  await mockTauriIPC(page, { ...asManager, ...extra } as any);
  await loginAndGoto(page, '/settings');
  const panel = page.getByTestId(testId);
  await expect(panel).toBeVisible();
  await panel.scrollIntoViewIfNeeded();
  return panel;
}

// ── Table transfer (M122) ───────────────────────────────────────────
const tablesWith = (occupied: Record<number, number>) => () =>
  Array.from({ length: 20 }, (_, i) => {
    const n = i + 1;
    const orderId = occupied[n] ?? null;
    return {
      tableNumber: n, orderId, totalCents: orderId ? 2400 : 0, lineCount: orderId ? 2 : 0,
      status: orderId ? 'occupied' : 'free', openedAt: null, preBillPrinted: false,
    };
  });

test.describe('Lot 2 - transfert de table, correction de paiement, remise, note de cuisine', () => {
  test('60_table_transfer_picker - changer de table depuis la commande', async ({ page }) => {
    await toOrder(page, { list_tables: tablesWith({ 4: 44 }) });
    await page.getByTestId('transfer-table').click();
    const picker = page.getByRole('dialog', { name: 'Changer de table' });
    await expect(picker).toBeVisible();
    await picker.getByTestId('pick-table-5').click();
    await expect(picker.getByTestId('transfer-confirm')).toBeEnabled();
    await capture(page, '60_table_transfer_picker');
  });

  test('61_table_transfer_move_mode - déplacer une table depuis le plan', async ({ page }) => {
    await mockTauriIPC(page, { ...asManager, list_tables: tablesWith({ 3: 42, 4: 44 }) } as any);
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await page.getByTestId('move-toggle').click();
    await page.locator('.table-card.occupied').first().click();
    await page.locator('.table-card .table-num', { hasText: /^7$/ }).click();
    await expect(page.getByTestId('move-confirm')).toContainText('Déplacer la table 3 vers la table 7 ?');
    await capture(page, '61_table_transfer_move_mode');
  });

  test('62_payment_correction - corriger le mode de paiement d\'une vente', async ({ page }) => {
    await mockTauriIPC(page, {
      ...asManager,
      list_copyable_tickets: () => [
        { fiscalEventId: 41, eventLabel: 'N', ticketNumber: 9, tableNumber: null, totalCents: -1200, createdAt: '2026-09-25 13:00:00' },
        { fiscalEventId: 38, eventLabel: 'N', ticketNumber: 6, tableNumber: 4, totalCents: 5200, createdAt: '2026-09-25 12:00:00' },
      ],
      list_refundable_events: () => [],
      list_sale_payments: () => ({
        fiscalEventId: 38, ticketNumber: 6, totalCents: 5200,
        lines: [{ index: 0, paymentType: 'CASH', name: 'Cash', amountCents: 5200, inputMethod: 'MANUAL', amountType: 'PAYMENT', brand: null }],
        correctable: true, refusal: null, corrected: null,
      }),
    } as any);
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    await page.getByTestId('correct-payment-38').click();
    const dialog = page.getByTestId('pc-dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByTestId('pc-method-card').click();
    await expect(dialog.getByTestId('pc-summary')).toHaveText('Espèces 52,00 € → Carte 52,00 €');
    await capture(page, '62_payment_correction');
  });

  const PLAIN = { ...DEFAULT_CART_LINE, id: 1, quantity: 2, unsent_quantity: 0 };
  const HALF_OFF = {
    ...PLAIN,
    discount_cents: -1200,
    price_changes: [{ id: 'TILL-DISCOUNT', name: 'Remise 50 %', scope: 'EVENT', amount_cents: -1200 }],
  };

  test('63_discount_dialog - remise du Gérant, aperçu avant confirmation', async ({ page }) => {
    await toOrder(page, { list_cart_lines: () => [PLAIN] });
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByTestId('discount-quick-50').click();
    await dialog.getByTestId('discount-reason-staff_meal').click();
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 12,00 €');
    await capture(page, '63_discount_dialog');
  });

  test('64_cart_discount - panier avec une remise et son total net', async ({ page }) => {
    await toOrder(page, {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
      list_cart_lines: () => [HALF_OFF],
    });
    await expect(page.getByTestId('price-change-row')).toBeVisible();
    await capture(page, '64_cart_discount');
  });

  test('65_kitchen_note - note de cuisine sur une ligne', async ({ page }) => {
    await toOrder(page, {
      list_cart_lines: () => [{ ...DEFAULT_CART_LINE, id: 1, quantity: 2, unsent_quantity: 2, kitchen_note: 'sans wasabi' }],
    });
    await expect(page.getByTestId('line-kitchen-note')).toHaveText('sans wasabi');
    await page.getByTestId('line-note').click();
    await expect(page.locator('.keypad-modal')).toContainText('Note pour la cuisine');
    await capture(page, '65_kitchen_note');
  });
});

// ── Invoices, sold-out, menu editor, sales report ───────────────────
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
const LOOKUP = {
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
const INVOICES = [
  { id: 3, invoiceNo: '20260003', fiscalEventId: 43, issueDate: '2026-10-17', customerName: 'ACME SPRL', customerVatNumber: 'BE0308357159', totalCents: 3900, saleEventIds: [11, 12], deliveryStatus: 'queued', deliveryId: 5 },
  { id: 2, invoiceNo: '20260002', fiscalEventId: 41, issueDate: '2026-10-16', customerName: 'BETA SA', customerVatNumber: 'BE0403019261', totalCents: 1200, saleEventIds: [8], deliveryStatus: 'accepted', deliveryId: 4 },
  { id: 1, invoiceNo: '20260001', fiscalEventId: 38, issueDate: '2026-10-15', customerName: 'GAMMA GMBH', customerVatNumber: 'DE123456789', totalCents: 900, saleEventIds: [10], deliveryStatus: 'failed', deliveryId: 3 },
  { id: 0, invoiceNo: '20250999', fiscalEventId: 30, issueDate: '2026-10-10', customerName: 'DELTA', customerVatNumber: 'FR12345678901', totalCents: 500, saleEventIds: [2], deliveryStatus: null, deliveryId: null },
];

function invoiceMocks(over: Record<string, unknown> = {}) {
  return {
    ...asManager,
    list_invoiceable_sales: () => SALES,
    list_copyable_tickets: () => COPYABLE,
    lookup_b2b_customer: () => LOOKUP,
    save_b2b_customer: () => CUSTOMER,
    invoice_preview: () => PREVIEW,
    sign_invoice_fiscal: () => ({ success: true, fiscalEventId: 40, invoiceNo: '20260002', printed: true, deliveryStatus: 'queued', error: null }),
    list_invoices: () => INVOICES,
    ...over,
  };
}

async function typeKeys(page: Page, text: string) {
  const kbd = page.getByTestId('text-keyboard');
  for (const ch of text) {
    if (ch === ' ') await kbd.getByRole('button', { name: 'Espace' }).click();
    else await kbd.getByRole('button', { name: ch, exact: true }).click();
  }
}

async function fillCustomer(page: Page) {
  await page.getByTestId('invoice-vat-field').click();
  await typeKeys(page, 'BE0308357159');
  await page.getByTestId('invoice-vat-check').click();
  await expect(page.getByTestId('invoice-chip')).toHaveText('Inscrit sur Peppol');
  for (const [id, text] of [['name', 'ACME SPRL'], ['street', 'RUE DU PONT 1'], ['postalCode', '4000'], ['city', 'LIEGE']]) {
    await page.getByTestId(`invoice-field-${id}`).click();
    await typeKeys(page, text);
  }
}

async function openInvoiceDialog(page: Page, over: Record<string, unknown> = {}) {
  await mockTauriIPC(page, invoiceMocks(over) as any);
  await loginAndGoto(page, '/reports');
  await page.getByRole('tab', { name: 'Tickets' }).click();
  await page.getByTestId('ticket-check-11').click();
  await page.getByTestId('ticket-check-12').click();
}

test.describe('Lot 3 - factures, produits épuisés, éditeur de carte, rapport de ventes', () => {
  test('66_reports_tickets_invoice - ventes cochées et bouton Facture', async ({ page }) => {
    await openInvoiceDialog(page);
    await expect(page.getByTestId('invoice-action')).toHaveText('Facture (2)');
    await expect(page.getByRole('tab')).toHaveText(['Journée', 'Caisse', 'Tickets', 'Ventes']);
    await capture(page, '66_reports_tickets_invoice');
  });

  test('67_invoice_customer - client, TVA vérifiée, puce Peppol', async ({ page }) => {
    await openInvoiceDialog(page);
    await page.getByTestId('invoice-action').click();
    await expect(page.getByTestId('invoice-dialog')).toBeVisible();
    await fillCustomer(page);
    await expect(page.getByTestId('invoice-continue')).toBeEnabled();
    await capture(page, '67_invoice_customer');
  });

  test('68_invoice_preview - lignes, TVA et déjà payé', async ({ page }) => {
    await openInvoiceDialog(page);
    await page.getByTestId('invoice-action').click();
    await fillCustomer(page);
    await page.getByTestId('invoice-continue').click();
    await expect(page.getByTestId('invoice-preview')).toBeVisible();
    await expect(page.getByTestId('invoice-already-paid')).toContainText('Déjà payé : 39,00 €');
    await capture(page, '68_invoice_preview');
  });

  test('69_invoice_result - facture établie, envoi Peppol en file', async ({ page }) => {
    await openInvoiceDialog(page);
    await page.getByTestId('invoice-action').click();
    await fillCustomer(page);
    await page.getByTestId('invoice-continue').click();
    await page.getByTestId('invoice-confirm').click();
    await expect(page.getByTestId('invoice-result')).toContainText('Facture 20260002 établie');
    await capture(page, '69_invoice_result');
  });

  test('70_invoice_peppol_required - refus avant signature, client belge sans Peppol', async ({ page }) => {
    await openInvoiceDialog(page, {
      sign_invoice_fiscal: () => ({ success: false, fiscalEventId: null, invoiceNo: null, printed: false, deliveryStatus: null, error: 'invoice_peppol_required' }),
    });
    await page.getByTestId('invoice-action').click();
    await fillCustomer(page);
    await page.getByTestId('invoice-continue').click();
    await page.getByTestId('invoice-confirm').click();
    await expect(page.getByTestId('invoice-error')).toContainText('doit passer par Peppol');
    await capture(page, '70_invoice_peppol_required');
  });

  test('71_invoice_list - liste des factures et état Peppol de chacune', async ({ page }) => {
    await mockTauriIPC(page, invoiceMocks() as any);
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Tickets' }).click();
    const list = page.getByTestId('invoice-list');
    await expect(list.locator('li')).toHaveCount(4);
    await list.scrollIntoViewIfNeeded();
    await capture(page, '71_invoice_list');
  });

  test('72_settings_peppol - envoi bloqué et bouton Réessayer', async ({ page }) => {
    const delivery = {
      id: 9, fiscalEventId: 41, invoiceNo: '20260002', customerEndpointId: '0208:0308357159', apVendor: 'storecove',
      apMessageId: null, status: 'failed', errorMsg: 'recipient not on Peppol', attempts: 3, nextAttemptAt: null,
      lastAttemptAt: '2026-10-17 09:00:00', parkedAt: '2026-10-17 09:00:00', acceptedAt: null,
      createdAt: '2026-10-17 08:00:00', lookupStatus: 'not_registered',
    };
    const panel = await settingsPanel(page, {
      list_peppol_deliveries: () => ({
        configured: true, provider: 'storecove', senderId: '0208:0123456789', queued: 2, accepted: 14, failed: 1,
        deliveries: [delivery, { ...delivery, id: 10, status: 'queued', errorMsg: null, parkedAt: null }],
      }),
    }, 'peppol-panel');
    await expect(panel.getByTestId('peppol-parked-9')).toContainText('Motif : recipient not on Peppol');
    await capture(page, '72_settings_peppol');
  });

  const SOLD_ITEMS = [
    { ...DEFAULT_MENU_ITEM, id: 1, name: 'Maki saumon x8', sold_out: true, sold_out_until: '2026-10-18T04:00:00+02:00' },
    { ...DEFAULT_MENU_ITEM, id: 2, name: 'Edamame', price_cents: 500, sold_out: true, sold_out_until: null },
    { ...DEFAULT_MENU_ITEM, id: 3, name: 'Gyoza', price_cents: 700 },
  ];
  const SOLD_LIST = [
    { menu_item_id: 1, name: 'Maki saumon x8', until: '2026-10-18T04:00:00+02:00', set_by: '94060242576', set_at: '2026-10-17 10:00:00' },
    { menu_item_id: 2, name: 'Edamame', until: null, set_by: '94060242576', set_at: '2026-10-17 10:00:00' },
  ];
  const soldOutOrder = (page: Page) =>
    toOrder(page, {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
      list_cart_lines: () => [],
      get_menu_items: () => SOLD_ITEMS,
      list_sold_out: () => SOLD_LIST,
    });

  test('73_soldout_mode - mode Épuisé, produits grisés avec leur pastille', async ({ page }) => {
    await soldOutOrder(page);
    await page.getByTestId('soldout-mode').click();
    await expect(page.getByTestId('soldout-hint')).toBeVisible();
    await expect(page.locator('.item-btn', { hasText: 'Maki saumon' }).getByTestId('soldout-badge')).toHaveText('Épuisé');
    await capture(page, '73_soldout_mode');
  });

  test('74_soldout_dialog - disponibilité d\'un produit épuisé pour la journée', async ({ page }) => {
    await soldOutOrder(page);
    await page.getByTestId('soldout-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon' }).click();
    await expect(page.getByTestId('soldout-dialog')).toBeVisible();
    await expect(page.getByTestId('soldout-status')).toContainText("Jusqu'à la fin de la journée");
    await capture(page, '74_soldout_dialog');
  });

  test('75_soldout_list - liste Produits épuisés (2)', async ({ page }) => {
    await soldOutOrder(page);
    await expect(page.getByTestId('soldout-list-button')).toHaveText('Produits épuisés (2)');
    await page.getByTestId('soldout-list-button').click();
    await expect(page.getByTestId('soldout-list')).toBeVisible();
    await capture(page, '75_soldout_list');
  });

  const MENU = {
    managed_by: null,
    languages: ['fr', 'nl', 'en', 'zh'],
    categories: [
      { id: 1, names: { fr: 'Sushi', zh: '寿司' }, sort_order: 1, color: '#E94560', active: true },
      { id: 2, names: { fr: 'Boissons' }, sort_order: 2, color: null, active: false },
    ],
    products: [
      { id: 10, category_id: 1, names: { fr: 'Maki saumon', zh: '三文鱼卷' }, price_cents: 650, vat_code: 'B', vat_code_takeaway: 'C', kitchen_printer: true, code: 'A1', open_price: false, sort_order: 1, active: true },
      { id: 11, category_id: 1, names: { fr: 'Edamame' }, price_cents: 450, vat_code: 'B', vat_code_takeaway: null, kitchen_printer: false, code: null, open_price: false, sort_order: 2, active: false },
    ],
    groups: [{ id: 20, menu_item_id: 10, names: { fr: 'Sauce' }, min_select: 0, max_select: 2, sort_order: 1, active: true }],
    options: [{ id: 30, group_id: 20, names: { fr: 'Soja' }, price_cents: 50, sort_order: 1, active: true }],
    allergens: [{ entity_type: 'menu_item', entity_id: 10, contains: ['fish', 'soybeans'], may_contain: ['sesame'] }],
  };
  const CHANGES = [
    { id: 2, entity_type: 'product', entity_id: 10, action: 'update', label: 'Maki saumon', changed_fields: ['price_cents', 'vat_code'], before_json: '{}', after_json: '{}', changed_by: '94060242576', changed_by_name: 'Gérant Test', changed_at: '2026-10-17 09:30:00' },
    { id: 1, entity_type: 'category', entity_id: 1, action: 'create', label: 'Sushi', changed_fields: ['names', 'sort_order'], before_json: null, after_json: '{}', changed_by: '94060242576', changed_by_name: 'Gérant Test', changed_at: '2026-10-17 09:00:00' },
  ];
  const menuEditor = async (page: Page, over: Record<string, unknown> = {}) => {
    await mockTauriIPC(page, {
      ...asManager,
      menu_admin_get: () => MENU,
      list_menu_changes: () => CHANGES,
      get_course_settings: () => ({ enabled: true, category_courses: [{ category_id: 1, course: 1 }] }),
      ...over,
    } as any);
    await loginAndGoto(page, '/menu-editor');
    await expect(page.getByTestId('menu-editor-view')).toBeVisible();
  };

  test('76_settings_carte - panneau Carte', async ({ page }) => {
    const panel = await settingsPanel(page, { menu_admin_get: () => MENU }, 'menu-editor-panel');
    await expect(panel.getByTestId('menu-editor-open')).toBeVisible();
    await capture(page, '76_settings_carte');
  });

  test('77_menu_editor_product - catégories, produits et fiche produit avec allergènes', async ({ page }) => {
    await menuEditor(page);
    await page.getByTestId('menu-ed-product-row').first().click();
    await expect(page.getByTestId('menu-ed-form-title')).toHaveText('Maki saumon');
    await expect(page.getByTestId('menu-ed-allergen-fish')).toHaveAttribute('data-level', 'contains');
    await page.getByTestId('menu-ed-allergen-fish').scrollIntoViewIfNeeded();
    await capture(page, '77_menu_editor_product');
  });

  test('78_menu_editor_category - catégorie avec son temps de service', async ({ page }) => {
    await menuEditor(page);
    await page.getByTestId('menu-ed-category-row').first().click();
    await expect(page.getByTestId('menu-ed-course-1')).toHaveClass(/on/);
    await capture(page, '78_menu_editor_category');
  });

  test('79_menu_editor_history - historique des modifications de la carte', async ({ page }) => {
    await menuEditor(page);
    await page.getByTestId('menu-ed-history-button').click();
    await expect(page.getByTestId('menu-ed-history-row')).toHaveCount(2);
    await capture(page, '79_menu_editor_history');
  });

  const REPORT = {
    from: '2026-10-16', to: '2026-10-17', generatedAt: '2026-10-17T18:30:00+02:00', businessDate: '2026-10-17',
    days: [
      { bookingDate: '2026-10-16', zNumber: 12, totalCents: 12550, zTotalCents: 12550, open: false, matchesZ: true, gapCents: 0 },
      { bookingDate: '2026-10-16', zNumber: 13, totalCents: 4000, zTotalCents: 3800, open: false, matchesZ: false, gapCents: 200 },
      { bookingDate: '2026-10-17', zNumber: null, totalCents: 1500, zTotalCents: null, open: true, matchesZ: false, gapCents: null },
    ],
    openPeriod: true,
    byProduct: [
      { productId: '1', name: 'Maki saumon', department: 'Plats', quantityMilli: 3000, amountCents: 1950 },
      { productId: '2', name: 'Sashimi', department: 'Plats', quantityMilli: 12000, amountCents: 900 },
      { productId: '3', name: 'Soupe', department: 'Entrees', quantityMilli: -1000, amountCents: -400 },
    ],
    byHour: [
      { hour: 19, tickets: 4, amountCents: 9000, barPercent: 100 },
      { hour: 20, tickets: 2, amountCents: 4500, barPercent: 50 },
      { hour: 1, tickets: 1, amountCents: 500, barPercent: 6 },
    ],
    priceChangesCents: -50, roundingCents: 2, refundsCents: -400, ticketCount: 7, totalCents: 18050,
  };

  test('80_reports_sales - onglet Ventes, concordance avec les Z', async ({ page }) => {
    await mockTauriIPC(page, { ...asManager, sales_report: () => REPORT, print_sales_report: () => undefined } as any);
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Ventes' }).click();
    await expect(page.getByTestId('sales-report')).toBeVisible();
    await expect(page.getByTestId('sales-notice')).toContainText('Document non fiscal');
    await capture(page, '80_reports_sales');
  });
});

// ── Owner alerts, allergens, courses, gift vouchers ─────────────────
test.describe('Lot 4 - alertes, allergènes, service en plusieurs temps, bons cadeaux', () => {
  test('81_settings_owner_alerts - panneau Alertes sur téléphone', async ({ page }) => {
    const panel = await settingsPanel(page, {
      get_owner_alerts: () => ({
        enabled: true, chat_id: '-10012345', label: 'Salle 1', language: 'fr',
        kinds: { fdm: true, printer: true, terminal: true, z_missing: true, online_waiting: false },
        z_deadline_hour: 5, online_wait_minutes: 5, reminder_minutes: 60, token_configured: true,
      }),
    }, 'alerts-panel');
    await expect(panel.locator('h2')).toHaveText('Alertes sur téléphone');
    await expect(panel.getByTestId('alerts-token')).toContainText('Robot Telegram configuré');
    await capture(page, '81_settings_owner_alerts');
  });

  const MAKI = {
    ...DEFAULT_MENU_ITEM, has_modifiers: true,
    allergens: ['sesame', 'fish', 'gluten', 'milk', 'eggs', 'soybeans', 'nuts', 'celery'],
    traces: ['molluscs'], allergens_declared: true,
  };
  const COCA = { ...DEFAULT_MENU_ITEM, id: 2, name: 'Coca-Cola', price_cents: 300, allergens: [], traces: [], allergens_declared: true };
  const DESSERT = { ...DEFAULT_MENU_ITEM, id: 3, name: 'Dessert du jour', price_cents: 500 };
  const SHEET = {
    lang: 'fr',
    products: [{
      id: 1, name: 'Maki saumon x8', name_fr: 'Maki saumon x8', declared: true,
      contains: [
        { code: 'gluten', number: 1, name: 'Céréales contenant du gluten' },
        { code: 'fish', number: 4, name: 'Poissons' },
      ],
      traces: [{ code: 'molluscs', number: 14, name: 'Mollusques' }],
      options: [
        { group: 'Sauce', name: 'Soja', declared: true, contains: [{ code: 'soybeans', number: 6, name: 'Soja' }], traces: [] },
        { group: 'Sauce', name: 'Nature', declared: false, contains: [], traces: [] },
      ],
    }],
  };
  const allergenOrder = (page: Page) =>
    toOrder(page, {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 0 }),
      list_cart_lines: () => [],
      get_menu_items: () => [MAKI, COCA, DESSERT],
      get_allergen_sheet: () => SHEET,
    });

  test('82_order_allergens - pastilles numérotées sur les produits et légende', async ({ page }) => {
    await allergenOrder(page);
    await page.getByTestId('allergen-mode').click();
    await expect(page.getByTestId('allergen-legend').locator('li')).toHaveCount(14);
    await expect(page.locator('.item-btn', { hasText: 'Maki saumon x8' }).getByTestId('allergen-badge')).toHaveCount(6);
    await capture(page, '82_order_allergens');
  });

  test('83_allergen_sheet - fiche allergènes d\'un produit', async ({ page }) => {
    await allergenOrder(page);
    await page.getByTestId('allergen-mode').click();
    await page.locator('.item-btn', { hasText: 'Maki saumon x8' }).click();
    const sheet = page.getByTestId('allergen-sheet');
    await expect(sheet.getByTestId('allergen-contains')).toContainText('4 Poissons');
    await expect(sheet.getByTestId('allergen-options')).toContainText('Information non disponible');
    await capture(page, '83_allergen_sheet');
  });

  test('84_settings_courses - panneau Service en plusieurs temps', async ({ page }) => {
    const panel = await settingsPanel(page, {
      get_categories: () => [DEFAULT_CATEGORY, { ...DEFAULT_CATEGORY, id: 2, name: 'Desserts maison' }],
      get_course_settings: () => ({ enabled: true, category_courses: [{ category_id: 2, course: 3 }] }),
    }, 'courses-panel');
    await expect(panel.locator('h2')).toHaveText('Service en plusieurs temps');
    await capture(page, '84_settings_courses');
  });

  const LINE = (id: number, name: string, course: number, state: string, unsent = 0) => ({
    ...DEFAULT_CART_LINE, id, menu_item_id: id, name, name_fr: name, name_zh: null,
    unit_price_cents: 500, course, kitchen_state: state, unsent_quantity: unsent,
  });
  const CART = [
    LINE(1, 'Sake', 0, 'printed'), LINE(2, 'Edamame', 1, 'printed'),
    LINE(3, 'Maki saumon', 2, 'held'), LINE(4, 'Gyoza', 2, 'held', 1), LINE(5, 'Mochi', 3, 'held'),
  ];
  const courseOrder = (page: Page, over: Record<string, unknown> = {}) =>
    toOrder(page, {
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2500 }),
      get_course_settings: () => ({ enabled: true, category_courses: [] }),
      list_cart_lines: () => CART,
      list_held_courses: () => [{ course: 2, lines: 2, quantity: 2 }, { course: 3, lines: 1, quantity: 1 }],
      ...over,
    });

  test('85_cart_courses - panier groupé par temps et bouton Envoyer la suite', async ({ page }) => {
    await courseOrder(page);
    await expect(page.locator('.course-label')).toHaveText(['Direct', 'Entrées', 'Plats', 'Desserts']);
    await expect(page.getByTestId('send-next-course')).toHaveText('Envoyer la suite (Plats)');
    await capture(page, '85_cart_courses');
  });

  test('86_course_picker - choix du temps d\'une ligne', async ({ page }) => {
    await courseOrder(page);
    await page.getByTestId('course-chip-5').click();
    await expect(page.getByTestId('course-picker')).toBeVisible();
    await capture(page, '86_course_picker');
  });

  test('87_tables_held_course - plan de salle avec Suite : Plats', async ({ page }) => {
    await mockTauriIPC(page, {
      ...asManager,
      list_tables: () => Array.from({ length: 20 }, (_, i) => (i === 1
        ? { tableNumber: 2, orderId: 9, totalCents: 4200, lineCount: 3, status: 'occupied', openedAt: null, preBillPrinted: false, heldCourse: 2 }
        : { tableNumber: i + 1, orderId: null, totalCents: 0, lineCount: 0, status: 'free', openedAt: null, preBillPrinted: false, heldCourse: null })),
    } as any);
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await expect(page.getByTestId('held-course-2')).toHaveText('Suite : Plats');
    await capture(page, '87_tables_held_course');
  });

  test('88_checkout_held_course - avertissement d\'une suite non envoyée', async ({ page }) => {
    await courseOrder(page);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await expect(page.getByTestId('held-warning')).toContainText('Suite non envoyée : Plats');
    await capture(page, '88_checkout_held_course');
  });

  const ON = { enabled: true, validity_months: 12, max_amount_cents: 50000 };
  const VOUCHER = {
    id: 7, code_masked: '**** **** 9095', face_value_cents: 5000, balance_cents: 3000,
    issued_on: '2026-10-01', expires_on: '2027-10-01', status: 'active', training: false,
    movements: [
      { kind: 'issue', amount_cents: 5000, date: '2026-10-01', ticket: 'N 12' },
      { kind: 'redeem', amount_cents: 2000, date: '2026-10-05', ticket: 'N 40' },
    ],
  };
  const VOUCHER_LINE = {
    ...DEFAULT_ORDER_LINE, id: 5, menu_item_id: 990001, unit_price_cents: 2500, vat_code: 'X',
    name: 'Bon cadeau', department_id: 'GV', department_name: 'Bon cadeau',
    name_fr: null, name_zh: null, unsent_quantity: 0, course: 0, kitchen_state: 'printed',
  };

  test('89_order_gift_voucher - bouton Bon cadeau, montant et ligne de bon', async ({ page }) => {
    await toOrder(page, {
      get_gift_voucher_settings: () => ON,
      list_cart_lines: () => [VOUCHER_LINE],
    });
    await expect(page.getByTestId('voucher-badge')).toHaveText('Bon');
    await page.getByTestId('gift-voucher-sell').click();
    await expect(page.getByRole('dialog', { name: 'Vendre un bon cadeau' })).toBeVisible();
    for (const d of '2500') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await expect(page.getByTestId('open-price-amount')).toContainText('25,00');
    await capture(page, '89_order_gift_voucher');
  });

  test('90_checkout_gift_voucher - paiement par bon, code trouvé', async ({ page }) => {
    await toOrder(page, {
      get_gift_voucher_settings: () => ON,
      get_order: () => ({ ...DEFAULT_ORDER, total_cents: 4500 }),
      list_cart_lines: () => [{ ...DEFAULT_ORDER_LINE, id: 1, unit_price_cents: 4500 }],
      lookup_gift_voucher: () => VOUCHER,
    });
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await page.getByTestId('method-gift_voucher').click();
    await expect(page.getByTestId('gv-dialog')).toBeVisible();
    for (const d of '123456789095') await page.locator('.numpad .key', { hasText: new RegExp(`^${d}$`) }).click();
    await page.getByTestId('gv-lookup').click();
    await expect(page.getByTestId('gv-status')).toHaveText('Valide');
    await expect(page.getByTestId('gv-balance')).toContainText('30,00');
    await capture(page, '90_checkout_gift_voucher');
  });

  test('91_reports_gift_vouchers - onglet Bons cadeaux, liste et mouvements', async ({ page }) => {
    await mockTauriIPC(page, {
      ...asManager,
      get_gift_voucher_settings: () => ON,
      list_gift_vouchers: () => [VOUCHER],
    } as any);
    await loginAndGoto(page, '/reports');
    await page.getByRole('tab', { name: 'Bons cadeaux' }).click();
    await page.getByTestId('gv-row-7').locator('.voucher-head').click();
    await expect(page.getByTestId('gv-row-7')).toContainText('N 40');
    await capture(page, '91_reports_gift_vouchers');
  });

  test('92_settings_gift_vouchers - panneau Bons cadeaux', async ({ page }) => {
    const panel = await settingsPanel(page, { get_gift_voucher_settings: () => ON }, 'gv-settings');
    await expect(panel.locator('h2')).toHaveText('Bons cadeaux');
    await capture(page, '92_settings_gift_vouchers');
  });
});
