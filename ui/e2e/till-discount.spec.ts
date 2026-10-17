import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  ipcCalls,
  overrideIPC,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CART_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc';

// A manager's discount at the till (commands apply_discount and
// remove_discount): an order discount (percent, amount, Offert) or one
// line for free, confirmed by the PIN. The engine applies every rule;
// the screen collects the input and shows the net amounts.

const CASHIER = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa Serveuse', role: 'cashier' };

// Two makis at 12,00 (VAT C): 24,00 before any discount.
const PLAIN = { ...DEFAULT_CART_LINE, id: 1, quantity: 2, unsent_quantity: 0 };
const HALF_OFF = {
  ...PLAIN,
  discount_cents: -1200,
  price_changes: [{ id: 'TILL-DISCOUNT', name: 'Remise 50 %', scope: 'EVENT', amount_cents: -1200 }],
};
const OFFERED = {
  ...PLAIN,
  discount_cents: -2400,
  price_changes: [{ id: 'TILL-FREE-LINE', name: 'Offert', scope: 'LINE', amount_cents: -2400 }],
};

async function toOrder(page: Page, lines: unknown[] = [PLAIN], total = 2400, user = DEFAULT_USER) {
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: total }),
    list_cart_lines: () => lines,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

async function typePin(page: Page, pin: string) {
  const dialog = page.locator('.pin-dialog');
  for (const d of pin) await dialog.getByRole('button', { name: d, exact: true }).click();
}

/** What the engine would answer once the discount is recorded. */
async function afterDiscount(page: Page, lines: unknown[], total: number) {
  await overrideIPC(page, 'list_cart_lines', lines);
  await overrideIPC(page, 'get_order', { ...DEFAULT_ORDER, total_cents: total });
}

test.describe('Remise button', () => {
  test('a manager sees it on an order with lines', async ({ page }) => {
    await toOrder(page);
    await expect(page.getByTestId('discount-open')).toHaveText('Remise');
  });

  test('a cashier does not', async ({ page }) => {
    await toOrder(page, [PLAIN], 2400, CASHIER);
    await expect(page.locator('.cart-lines')).toBeVisible();
    await expect(page.getByTestId('discount-open')).toHaveCount(0);
    await expect(page.getByTestId('line-offer')).toHaveCount(0);
  });

  test('is absent on an empty order', async ({ page }) => {
    await toOrder(page, [], 0);
    await expect(page.getByTestId('discount-open')).toHaveCount(0);
  });

  test('is absent on an online order, whatever the role', async ({ page }) => {
    await toOrder(page, [{ ...PLAIN, online: true }]);
    await expect(page.locator('.cart-lines')).toBeVisible();
    await expect(page.getByTestId('discount-open')).toHaveCount(0);
    await expect(page.getByTestId('discount-remove')).toHaveCount(0);
    await expect(page.getByTestId('line-offer')).toHaveCount(0);
  });

  test('no removal action on an online order carrying EVENT and LINE changes', async ({ page }) => {
    // HubRise discounts arrive as price changes; the till must not offer to remove them.
    const hubrise = {
      ...PLAIN,
      online: true,
      discount_cents: -1800,
      price_changes: [
        { id: 'HUBRISE-1', name: 'Promo web', scope: 'EVENT', amount_cents: -1200 },
        { id: 'HUBRISE-2', name: 'Offert web', scope: 'LINE', amount_cents: -600 },
      ],
    };
    await toOrder(page, [hubrise], 600);
    await expect(page.locator('.cart-lines')).toBeVisible();
    // The changes are shown (so the rows are rendered) but nothing can be removed.
    await expect(page.getByTestId('price-change-row')).toHaveCount(2);
    await expect(page.getByTestId('discount-remove')).toHaveCount(0);
    await expect(page.getByTestId('line-remove-discount')).toHaveCount(0);
    await expect(page.getByTestId('discount-open')).toHaveCount(0);
    await expect(page.getByTestId('line-offer')).toHaveCount(0);
  });
});

test.describe('Order discount', () => {
  test('a quick percent with a reason, previewed then confirmed by the PIN', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByTestId('discount-confirm')).toBeDisabled();

    await dialog.getByTestId('discount-quick-50').click();
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 12,00 €');
    // A reason is required.
    await expect(dialog.getByTestId('discount-confirm')).toBeDisabled();
    await dialog.getByTestId('discount-reason-staff_meal').click();
    await dialog.getByTestId('discount-confirm').click();

    await expect(page.locator('.pin-dialog .hint')).toContainText('ticket fiscal');
    await afterDiscount(page, [HALF_OFF], 1200);
    await typePin(page, '4321');

    await expect(page.locator('.toast', { hasText: 'Remise appliquée' })).toBeVisible();
    expect(await ipcCalls(page, 'apply_discount')).toEqual([
      {
        orderId: expect.any(Number),
        input: { target: { type: 'order' }, kind: 'percent', value: 50, reason_code: 'staff_meal', reason_text: null },
        insz: DEFAULT_USER.insz,
        pin: '4321',
      },
    ]);
    // Net amounts: the line shows its change under it, the total follows.
    await expect(page.getByTestId('price-change-row')).toContainText('Remise 50 %');
    await expect(page.getByTestId('price-change-row')).toContainText('-12,00');
    await expect(page.locator('.total-amount')).toHaveText('12,00 €');
    await expect(page.locator('.checkout-amount')).toHaveText('12,00 €');
    // The order discount is removed, not stacked.
    await expect(page.getByTestId('discount-open')).toHaveCount(0);
    await expect(page.getByTestId('discount-remove')).toBeVisible();
  });

  test('a custom percent on the keypad', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByRole('button', { name: '7', exact: true }).click();
    await dialog.getByRole('button', { name: '5', exact: true }).click();
    await expect(dialog.getByTestId('discount-typed')).toContainText('75 %');
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 6,00 €');
    // 0 and 100 are out of range.
    await dialog.getByRole('button', { name: '⌫' }).click();
    await dialog.getByRole('button', { name: '⌫' }).click();
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 24,00 €');
    await dialog.getByTestId('discount-reason-other').click();
    await expect(dialog.getByTestId('discount-confirm')).toBeDisabled();
  });

  test('an amount typed in cents, capped at the total', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-kind-amount').click();
    for (const d of '500') await dialog.getByRole('button', { name: d, exact: true }).click();
    await expect(dialog.getByTestId('discount-typed')).toContainText('5,00 €');
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 19,00 €');
    await dialog.getByTestId('discount-reason-service_issue').click();
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    const calls = await ipcCalls(page, 'apply_discount');
    expect(calls[0].input).toMatchObject({ kind: 'amount', value: 500, reason_code: 'service_issue' });

    // More than the total is not accepted.
    await page.getByTestId('discount-open').click();
    await dialog.getByTestId('discount-kind-amount').click();
    for (const d of '9999') await dialog.getByRole('button', { name: d, exact: true }).click();
    await dialog.getByTestId('discount-reason-other').click();
    await expect(dialog.getByTestId('discount-confirm')).toBeDisabled();
  });

  test('Offert gives the whole order, with no value', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-kind-free').click();
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 0,00 €');
    await dialog.getByTestId('discount-reason-commercial_gesture').click();
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    const calls = await ipcCalls(page, 'apply_discount');
    expect(calls[0].input).toEqual({
      target: { type: 'order' }, kind: 'free', reason_code: 'commercial_gesture', reason_text: null,
    });
  });

  test('the reason text is typed on the on-screen keypad', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-10').click();
    await dialog.getByTestId('discount-reason-service_issue').click();
    await dialog.getByTestId('discount-reason-text').click();
    for (const ch of 'froid') await page.locator('.keypad-modal .key', { hasText: new RegExp(`^${ch}$`) }).click();
    await expect(page.locator('.keypad-modal [data-testid="keypad-display"]')).toContainText('froid');
    await page.locator('.keypad-modal .act-btn.primary').click();
    await expect(dialog.getByTestId('discount-reason-text')).toContainText('froid');
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    const calls = await ipcCalls(page, 'apply_discount');
    expect(calls[0].input).toMatchObject({ reason_code: 'service_issue', reason_text: 'froid' });
  });

  test('the preview rounds a percent half up, like the engine', async ({ page }) => {
    // 10 % of 1,25 is 12,5 cents: 13, so 1,12.
    await toOrder(page, [{ ...PLAIN, quantity: 1, unit_price_cents: 125 }], 125);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-10').click();
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 1,25 € → 1,12 €');
  });

  test('the preview rounds 15 % of 23,90 up to 3,59', async ({ page }) => {
    // 358,5 cents: 359, so 20,31.
    await toOrder(page, [{ ...PLAIN, quantity: 1, unit_price_cents: 2390 }], 2390);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-15').click();
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 23,90 € → 20,31 €');
  });

  test('switching the kind resets the typed value', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-20').click();
    await expect(dialog.getByTestId('discount-typed')).toContainText('20 %');
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 19,20 €');

    await dialog.getByTestId('discount-kind-amount').click();
    await expect(dialog.getByTestId('discount-typed')).toContainText('0,00 €');
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 24,00 €');

    await dialog.getByTestId('discount-kind-percent').click();
    await expect(dialog.getByTestId('discount-typed')).toContainText('0 %');
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 24,00 €');
  });

  test('a kitchen slip that did not print is warned, with the discount toast', async ({ page }) => {
    await toOrder(page, [{ ...PLAIN, unsent_quantity: 2 }]);
    await overrideIPC(page, 'apply_discount', {
      order: DEFAULT_ORDER, kitchenSlip: 'failed', registrationError: null,
    });
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-10').click();
    await dialog.getByTestId('discount-reason-other').click();
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    await expect(page.locator('.toast', { hasText: 'Bon de commande non imprimé' })).toBeVisible();
    await expect(page.locator('.toast', { hasText: 'Remise appliquée' })).toBeVisible();
  });

  test('is offered on a takeaway too', async ({ page }) => {
    const TAKEAWAY = {
      ...DEFAULT_ORDER, id: 77, table_number: null, order_type: 'takeaway', total_cents: 2400,
    };
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      create_order: () => TAKEAWAY,
      get_order: () => TAKEAWAY,
      list_cart_lines: () => [PLAIN],
      get_categories: () => [DEFAULT_CATEGORY],
      get_menu_items: () => [DEFAULT_MENU_ITEM],
    });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await page.getByRole('button', { name: 'Nouvelle vente à emporter' }).click();
    await page.waitForURL(/order/);
    await expect(page.getByTestId('transfer-table')).toHaveCount(0);

    // A takeaway starts empty: the first product makes the discount possible.
    await expect(page.getByTestId('discount-open')).toHaveCount(0);
    await page.locator('.item-btn').first().click();
    await expect(page.getByTestId('discount-open')).toBeVisible();
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-10').click();
    await dialog.getByTestId('discount-reason-commercial_gesture').click();
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    await expect(page.locator('.toast', { hasText: 'Remise appliquée' })).toBeVisible();
    const calls = await ipcCalls(page, 'apply_discount');
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ orderId: 77, input: { target: { type: 'order' }, kind: 'percent', value: 10 } });
    expect((await ipcCalls(page, 'create_order'))[0].input).toMatchObject({ order_type: 'takeaway' });
  });

  test('a wrong PIN or a refusal is shown on the pad and changes nothing', async ({ page }) => {
    await toOrder(page);
    await overrideIPC(page, 'apply_discount', { __reject: 'pin_wrong' });
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-20').click();
    await dialog.getByTestId('discount-reason-other').click();
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '0000');
    await expect(page.locator('.pin-dialog .error-msg')).toHaveText('PIN incorrect');
    await expect(page.locator('.toast', { hasText: 'Remise appliquée' })).toHaveCount(0);

    await overrideIPC(page, 'apply_discount', { __reject: 'discount_already_applied' });
    await typePin(page, '4321');
    await expect(page.locator('.pin-dialog .error-msg')).toContainText('déjà');
  });

  test('warns when the P that registers the change could not be signed', async ({ page }) => {
    await toOrder(page);
    await overrideIPC(page, 'apply_discount', {
      order: DEFAULT_ORDER, kitchenSlip: 'nothing_to_print', registrationError: 'fdm_unreachable',
    });
    await page.getByTestId('discount-open').click();
    const dialog = page.getByTestId('discount-dialog');
    await dialog.getByTestId('discount-quick-15').click();
    await dialog.getByTestId('discount-reason-other').click();
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    await expect(page.locator('.toast', { hasText: 'Remise enregistrée' })).toBeVisible();
  });

  test('back leaves the dialog without a call', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('discount-open').click();
    await page.getByTestId('discount-dialog').getByRole('button', { name: 'Retour' }).click();
    await expect(page.getByTestId('discount-dialog')).toHaveCount(0);
    expect(await ipcCalls(page, 'apply_discount')).toHaveLength(0);
  });

  test('removing it asks for the PIN', async ({ page }) => {
    await toOrder(page, [HALF_OFF], 1200);
    await page.getByTestId('discount-remove').click();
    await expect(page.locator('.pin-dialog .hint')).toContainText('prix d\'origine');
    await afterDiscount(page, [PLAIN], 2400);
    await typePin(page, '4321');
    await expect(page.locator('.toast', { hasText: 'Remise retirée' })).toBeVisible();
    expect(await ipcCalls(page, 'remove_discount')).toEqual([
      { orderId: expect.any(Number), lineId: null, insz: DEFAULT_USER.insz, pin: '4321' },
    ]);
    await expect(page.locator('.total-amount')).toHaveText('24,00 €');
    await expect(page.getByTestId('discount-open')).toBeVisible();
  });
});

test.describe('Offered line', () => {
  test('Offrir on a line without change, with a reason and the PIN', async ({ page }) => {
    await toOrder(page);
    await page.getByTestId('line-offer').click();
    const dialog = page.getByTestId('discount-dialog');
    // Only the reason: a free line has no percent or amount.
    await expect(dialog.getByTestId('discount-kind-percent')).toHaveCount(0);
    await expect(dialog.getByTestId('discount-preview')).toHaveText('Total 24,00 € → 0,00 €');
    await dialog.getByTestId('discount-reason-commercial_gesture').click();
    await dialog.getByTestId('discount-confirm').click();
    await afterDiscount(page, [OFFERED], 0);
    await typePin(page, '4321');

    const calls = await ipcCalls(page, 'apply_discount');
    expect(calls[0].input).toEqual({
      target: { type: 'line', line_id: 1 }, kind: 'free', reason_code: 'commercial_gesture', reason_text: null,
    });
    await expect(page.getByTestId('price-change-row')).toContainText('Offert');
    await expect(page.locator('.line-price')).toHaveText('0,00');
    await expect(page.locator('.total-amount')).toHaveText('0,00 €');
    await expect(page.getByTestId('line-offer')).toHaveCount(0);
    await expect(page.getByTestId('line-remove-discount')).toBeVisible();
  });

  test('Retirer la remise on the offered line', async ({ page }) => {
    await toOrder(page, [OFFERED], 0);
    await page.getByTestId('line-remove-discount').click();
    await afterDiscount(page, [PLAIN], 2400);
    await typePin(page, '4321');
    expect(await ipcCalls(page, 'remove_discount')).toEqual([
      { orderId: expect.any(Number), lineId: 1, insz: DEFAULT_USER.insz, pin: '4321' },
    ]);
    await expect(page.getByTestId('line-offer')).toBeVisible();
  });

  test('Offrir on a line not yet sent to the kitchen', async ({ page }) => {
    await toOrder(page, [{ ...PLAIN, unsent_quantity: 2 }]);
    await expect(page.locator('.order-line.pending')).toHaveCount(1);
    await page.getByTestId('line-offer').click();
    const dialog = page.getByTestId('discount-dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByTestId('discount-reason-staff_meal').click();
    await dialog.getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    const calls = await ipcCalls(page, 'apply_discount');
    expect(calls).toHaveLength(1);
    expect(calls[0].input).toMatchObject({ target: { type: 'line', line_id: 1 }, kind: 'free', reason_code: 'staff_meal' });
  });

  test('Retirer la remise on an offered line not yet sent', async ({ page }) => {
    await toOrder(page, [{ ...OFFERED, unsent_quantity: 2 }], 0);
    await expect(page.locator('.order-line.pending')).toHaveCount(1);
    await page.getByTestId('line-remove-discount').click();
    await typePin(page, '4321');
    expect(await ipcCalls(page, 'remove_discount')).toEqual([
      { orderId: expect.any(Number), lineId: 1, insz: DEFAULT_USER.insz, pin: '4321' },
    ]);
  });

  test('the base of a line is its own amount, not the order total', async ({ page }) => {
    const second = { ...PLAIN, id: 2, quantity: 1, unit_price_cents: 500, name: 'Edamame', name_fr: 'Edamame' };
    // 2 x 12,00 + 1 x 5,00.
    await toOrder(page, [{ ...PLAIN, unit_price_cents: 1200 }, second], 2900);
    await page.getByTestId('line-offer').nth(1).click();
    await expect(page.getByTestId('discount-preview')).toHaveText('Total 5,00 € → 0,00 €');
    await page.getByTestId('discount-dialog').getByTestId('discount-reason-other').click();
    await page.getByTestId('discount-dialog').getByTestId('discount-confirm').click();
    await typePin(page, '4321');
    const calls = await ipcCalls(page, 'apply_discount');
    expect(calls[0].input.target).toEqual({ type: 'line', line_id: 2 });
  });

  test('a quantity change on a discounted line is refused and shown', async ({ page }) => {
    await toOrder(page, [HALF_OFF], 1200);
    await overrideIPC(page, 'update_order_line_quantity', { __reject: 'line_has_price_changes' });
    await expect(page.locator('.qty')).toHaveText('2');
    await page.getByRole('button', { name: 'Un de plus' }).click();
    await expect(
      page.locator('.toast-error', { hasText: 'Cette ligne a une remise : retirez-la avant de changer la quantité.' }),
    ).toBeVisible();
    await expect(page.locator('.qty')).toHaveText('2');
  });

  test('no Offrir while an order discount applies', async ({ page }) => {
    await toOrder(page, [HALF_OFF], 1200);
    await expect(page.getByTestId('line-offer')).toHaveCount(0);
    await expect(page.getByTestId('line-remove-discount')).toHaveCount(0);
  });
});

test.describe('Net amounts', () => {
  test('the VAT summary follows the net amounts', async ({ page }) => {
    await toOrder(page, [HALF_OFF], 1200);
    // 12,00 at 6 %: 0,68 of VAT (24,00 would give 1,36).
    await expect(page.locator('.vat-summary')).toContainText('6% 0,68');
  });

  test('the same product again makes a new line when the line has a change', async ({ page }) => {
    await toOrder(page, [HALF_OFF], 1200);
    await page.locator('.item-btn').first().click();
    await expect.poll(async () => (await ipcCalls(page, 'add_order_line')).length).toBe(1);
    expect(await ipcCalls(page, 'update_order_line_quantity')).toHaveLength(0);
  });

  test('the same product again adds to a plain line', async ({ page }) => {
    await toOrder(page, [PLAIN], 2400);
    await page.locator('.item-btn').first().click();
    await expect.poll(async () => (await ipcCalls(page, 'update_order_line_quantity')).length).toBe(1);
    expect(await ipcCalls(page, 'add_order_line')).toHaveLength(0);
  });

  test('checkout owes the net total and shows the discount rows', async ({ page }) => {
    await toOrder(page, [HALF_OFF], 1200);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await expect(page.getByTestId('summary-subtotal')).toContainText('24,00');
    await expect(page.getByTestId('summary-discount')).toContainText('-12,00');
    await expect(page.getByTestId('summary-change')).toContainText('Remise 50 %');
    await expect(page.locator('.rest-amount')).toContainText('12,00');
  });

  test('checkout of an offered order owes nothing', async ({ page }) => {
    await toOrder(page, [OFFERED], 0);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await expect(page.getByTestId('summary-discount')).toContainText('-24,00');
    await expect(page.locator('.rest-amount')).toContainText('0,00');
  });
});
