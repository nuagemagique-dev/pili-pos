import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM, tapUserTile } from './fixtures/mock-ipc';

// An FDM refusal reaches the operator in plain French, as in the header
// indicator (certification manual §V); other refusals are backend codes
// with their own text (errors.spec.ts).

async function payByCard(page: Page, checkoutResult: object) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 1200 }),
    list_order_lines: () => [DEFAULT_ORDER_LINE],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    checkout: () => checkoutResult,
  });
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
  await page.locator('.item-btn').first().click();
  await page.locator('.checkout-btn').click();
  await page.waitForURL(/checkout/);
  await page.locator('.card-btn').click();
  await page.locator('.card-confirm').click();
}

const refused = (error: string) => ({
  success: false, ticket: null, error, fdm_connected: false, fiscal_event_id: null, delivery: null,
});

test('an unreachable FDM is explained, not shown as a raw message', async ({ page }) => {
  await payByCard(page, refused('FDM not connected - cannot complete transaction'));
  const msg = page.locator('.checkout .error-msg');
  await expect(msg).toHaveText('Connexion perdue avec la boîte noire. Vérifiez le câble réseau.');
  // Still on the payment step: nothing was sold.
  await expect(page.locator('.card-btn')).toBeVisible();
});

test('an FDM error code gets its §V message', async ({ page }) => {
  // As a spec-shaped FDM error reaches the UI: the code from
  // `extensions`, then the FDM's own text.
  await payByCard(page, refused('FDM error: GraphQL error: FDM_LOCKED: FDM is locked.'));
  await expect(page.locator('.checkout .error-msg')).toHaveText(
    'La boîte noire est verrouillée. Contactez votre technicien.',
  );
});

test('an FDM answer that does not match the sale is explained', async ({ page }) => {
  await payByCard(
    page,
    refused('FDM error: FDM_RESPONSE_MISMATCH: posFiscalTicketNo attendu 42, reçu 41'),
  );
  await expect(page.locator('.checkout .error-msg')).toHaveText(
    'Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien.',
  );
  await expect(page.locator('.card-btn')).toBeVisible();
});

test('an error that is not the FDM’s gets its own message', async ({ page }) => {
  await payByCard(page, refused('forbidden'));
  await expect(page.locator('.checkout .error-msg')).toHaveText('Action réservée à un autre profil.');
});

test('a refused token is explained, though the FDM sends no code', async ({ page }) => {
  // module2 answers a missing or wrong bearer token with a bare message.
  await payByCard(page, refused('FDM error: GraphQL error: Not a valid Token'));
  await expect(page.locator('.checkout .error-msg')).toHaveText(
    "La caisse n'est pas autorisée à parler à la boîte noire.",
  );
});

test('a refused FDM certificate is not reported as a cable problem', async ({ page }) => {
  await payByCard(
    page,
    refused('FDM error: FDM_CERTIFICATE_REFUSED: invalid peer certificate: UnknownIssuer'),
  );
  await expect(page.locator('.checkout .error-msg')).toHaveText(
    'Connexion sécurisée à la boîte noire refusée (certificat). Contactez votre technicien.',
  );
});
