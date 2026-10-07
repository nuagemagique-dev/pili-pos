import { test, expect } from '@playwright/test';
import {
  mockTauriIPC,
  overrideIPC,
  ipcCalls,
  emitTauriEvent,
  tapUserTile,
  DEFAULT_USER,
  DEFAULT_USER_WITH_PIN,
} from './fixtures/mock-ipc';

// A manager without a PIN yet (the first login after installation).
const NEW_MANAGER = { ...DEFAULT_USER, has_pin: false };
// A server without a PIN: allowed, with a warning.
const SERVER_NO_PIN = { ...DEFAULT_USER, insz: '84022899837', name: 'Serveur', role: 'cashier', has_pin: false };

async function typeOnDialog(page: import('@playwright/test').Page, pin: string) {
  const dialog = page.locator('.pin-dialog');
  for (const d of pin) await dialog.getByRole('button', { name: d, exact: true }).click();
}

test.describe('Login View', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
    });
    await page.goto('/');
  });

  test('no configuration banner when pos.json is usable', async ({ page }) => {
    await expect(page.locator('.user-btn')).toBeVisible();
    await expect(page.locator('.config-problems')).toHaveCount(0);
  });

  test('a configuration the FDM would refuse shows a banner', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
      get_settings: () => ({
        posId: 'CPOS0031234567',
        terminalId: 'POS-1-BAR',
        restaurantName: 'The FPS Finances Experience',
        fdmUrl: 'http://localhost:8081/graphql',
        printerAddress: null,
        trainingMode: false,
        language: 'fr',
        posSwVersion: 'pili-pos/1.0.0',
        configProblems: ['restaurant_vat: BE followed by 10 digits, got "BE123"'],
      }),
    });
    await page.goto('/');
    const banner = page.locator('.config-problems');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText('Configuration invalide');
    await expect(banner.locator('li')).toHaveText(['restaurant_vat: BE followed by 10 digits, got "BE123"']);
  });

  test('shows the software version declared to the FDM', async ({ page }) => {
    await expect(page.locator('.sw-version')).toHaveText('Version pili-pos/1.0.0');
    await expect(page.locator('.sw-version')).toBeInViewport();
  });

  test('shows user tile with name and role', async ({ page }) => {
    await expect(page.locator('.user-btn')).toBeVisible();
    await expect(page.locator('.user-name')).toContainText('Bruno Moyse');
    await expect(page.locator('.user-role')).toHaveText('Gérant');
  });

  test('a manager logs in with their PIN', async ({ page }) => {
    await tapUserTile(page);
    await expect(page).toHaveURL(/tables/);
    expect(await ipcCalls(page, 'login')).toEqual([{ insz: DEFAULT_USER.insz, pin: '1234' }]);
  });

  test('a server without PIN logs in directly, with a warning', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [SERVER_NO_PIN], login: () => SERVER_NO_PIN });
    await page.goto('/');
    await page.click('.user-btn');
    await expect(page).toHaveURL(/tables/);
    await expect(page.locator('.toast', { hasText: 'touchez votre nom' })).toBeVisible();
    expect(await ipcCalls(page, 'login')).toEqual([{ insz: SERVER_NO_PIN.insz, pin: null }]);
  });

  test('shows PIN overlay when user has PIN', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER_WITH_PIN],
      login: () => DEFAULT_USER_WITH_PIN,
    });
    await page.goto('/');
    await page.click('.user-btn');
    await expect(page.locator('.pin-overlay')).toBeVisible();
    await expect(page.locator('.pin-title')).toBeVisible();
    // 4 empty dots
    await expect(page.locator('.pin-dot')).toHaveCount(4);
  });

  test('correct PIN logs in', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER_WITH_PIN],
      login: () => DEFAULT_USER_WITH_PIN,
    });
    await page.goto('/');
    await page.click('.user-btn');
    // Enter 1111
    for (const d of ['1', '1', '1', '1']) {
      await page.click(`.nk:has-text("${d}")`);
    }
    // The 4th digit auto-submits.
    await expect(page).toHaveURL(/tables/);
  });

  test('wrong PIN shows error and clears dots', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER_WITH_PIN],
    });
    await page.goto('/');
    // Override login to reject AFTER page load (so the mock is
    // installed in the live page context, not pre-serialized).
    await overrideIPC(page, 'login', '__THROW__');
    await page.click('.user-btn');
    for (const d of ['9', '9', '9', '9']) {
      await page.click(`.nk:has-text("${d}")`);
    }
    // The 4th digit auto-submits.
    await expect(page.locator('.error-msg')).toBeVisible();
    // Dots should be cleared (no filled class)
    await expect(page.locator('.pin-dot.filled')).toHaveCount(0);
  });

  test('a locked-out user is told how long to wait', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER_WITH_PIN],
    });
    await page.goto('/');
    await overrideIPC(page, 'login', { __reject: 'pin_locked|seconds=30' });
    await page.click('.user-btn');
    for (const d of ['9', '9', '9', '9']) {
      await page.click(`.nk:has-text("${d}")`);
    }
    await expect(page.locator('.error-msg')).toContainText('30 s');
  });

  test('INSZ numpad shows on "Autre" tap', async ({ page }) => {
    await page.click('.other-btn');
    await expect(page.locator('.numpad-section')).toBeVisible();
    await expect(page.locator('.insz-display')).toBeVisible();
  });

  test('empty user list shows numpad directly', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [],
    });
    await page.goto('/');
    await expect(page.locator('.numpad-section')).toBeVisible();
  });
});

test.describe('First login of a manager without PIN', () => {
  test.beforeEach(async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [NEW_MANAGER],
      login: () => NEW_MANAGER,
      login_with_new_pin: () => ({ ...NEW_MANAGER, has_pin: true }),
    });
    await page.goto('/');
  });

  test('the tile asks to choose a PIN, typed twice, then logs in', async ({ page }) => {
    await page.click('.user-btn');
    const dialog = page.locator('.pin-dialog');
    await expect(dialog.locator('.title')).toHaveText('Choisissez votre PIN');
    await expect(dialog.locator('.who')).toHaveText(NEW_MANAGER.name);
    await expect(dialog.locator('.hint')).toContainText('Première connexion d\'un gérant');
    expect(await ipcCalls(page, 'login')).toHaveLength(0);

    await typeOnDialog(page, '4321');
    await expect(dialog.locator('.title')).toHaveText('Confirmer le PIN');
    await typeOnDialog(page, '4321');

    await expect(page).toHaveURL(/tables/);
    expect(await ipcCalls(page, 'login_with_new_pin')).toEqual([
      { insz: NEW_MANAGER.insz, newPin: '4321' },
    ]);
    expect(await ipcCalls(page, 'set_pin')).toHaveLength(0);
    await expect(page.locator('.toast', { hasText: 'PIN enregistré' })).toBeVisible();
  });

  test('two different PINs start over and log nobody in', async ({ page }) => {
    await page.click('.user-btn');
    await typeOnDialog(page, '4321');
    await typeOnDialog(page, '1234');
    await expect(page.locator('.pin-dialog .error-msg')).toHaveText('Les PIN ne correspondent pas');
    expect(await ipcCalls(page, 'login_with_new_pin')).toHaveLength(0);
    await expect(page).not.toHaveURL(/tables/);
  });

  test('back closes the dialog and stays on the login screen', async ({ page }) => {
    await page.click('.user-btn');
    await page.locator('.pin-dialog').getByRole('button', { name: 'Retour' }).click();
    await expect(page.locator('.pin-dialog')).toHaveCount(0);
    await expect(page.locator('.user-btn')).toBeVisible();
    expect(await ipcCalls(page, 'login_with_new_pin')).toHaveLength(0);
  });

  test('a manager typed by INSZ is asked to choose a PIN too', async ({ page }) => {
    await overrideIPC(page, 'login', { __reject: 'pin_setup_required' });
    await page.click('.other-btn');
    for (const d of NEW_MANAGER.insz) await page.click(`.numpad-section .nk:has-text("${d}")`);
    await page.locator('.submit-btn').click();
    await expect(page.locator('.pin-dialog .title')).toHaveText('Choisissez votre PIN');
    await expect(page.locator('.pin-dialog .who')).toHaveText(NEW_MANAGER.name);
  });
});

test('a PIN-protected user typed by INSZ gets the PIN overlay', async ({ page }) => {
  await mockTauriIPC(page, { list_users: () => [DEFAULT_USER_WITH_PIN] });
  await page.goto('/');
  await overrideIPC(page, 'login', { __reject: 'pin_required' });
  await page.click('.other-btn');
  for (const d of DEFAULT_USER_WITH_PIN.insz) await page.click(`.numpad-section .nk:has-text("${d}")`);
  await page.locator('.submit-btn').click();
  await expect(page.locator('.pin-overlay')).toBeVisible();
  await expect(page.locator('.pin-user')).toHaveText(DEFAULT_USER_WITH_PIN.name);
});

test.describe('Shifts (clock-in / clock-out, event S)', () => {
  const ON_SHIFT = [{ userInsz: DEFAULT_USER.insz, startedAt: '2026-09-26T09:02:00Z' }];

  test('a tile shows who is on shift, and since when', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER], list_open_shifts: () => ON_SHIFT });
    await page.goto('/');
    await expect(page.locator('.user-shift.on')).toContainText('en service · 11:02');
  });

  test('an employee on shift is not clocked in again', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER], list_open_shifts: () => ON_SHIFT });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables|order/);
    await page.waitForTimeout(300);
    expect(await ipcCalls(page, 'work_in')).toHaveLength(0);
  });

  test('the UI no longer clocks in at login, the engine does', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER], list_open_shifts: () => [] });
    await page.goto('/');
    await expect(page.locator('.user-shift')).toContainText('hors service');
    const listed = (await ipcCalls(page, 'list_open_shifts')).length;
    await tapUserTile(page);
    await page.waitForURL(/tables|order/);
    await page.waitForTimeout(300);
    expect(await ipcCalls(page, 'work_in')).toHaveLength(0);
    // Only the login screen lists the open shifts, for its display.
    expect(await ipcCalls(page, 'list_open_shifts')).toHaveLength(listed);
  });

  test('a presence-failed event shows the presence_failed toast', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER] });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables|order/);
    await emitTauriEvent(page, 'presence-failed', {
      insz: DEFAULT_USER.insz,
      clockIn: true,
      error: 'FDM error: offline',
    });
    await expect(page.locator('.toast-error')).toContainText('Pointage non enregistré');
    await expect(page.locator('.toast-error')).toContainText('Erreur boîte noire. Contactez votre technicien.');
    // The login is not blocked.
    await expect(page).toHaveURL(/tables|order/);
  });

  test('"Fin de service" then the PIN ends the shift through the engine and stays on the login screen', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER], list_open_shifts: () => ON_SHIFT });
    await page.goto('/');
    await page.locator('.user-btn').first().click();
    await page.locator('.end-shift-btn').click();
    await expect(page.locator('.pin-title')).toHaveText('Fin de service');
    for (const d of '1234') await page.locator('.pin-overlay .nk', { hasText: d }).first().click();

    await expect(page.locator('.toast-success')).toContainText('Service terminé');
    expect(await ipcCalls(page, 'end_shift')).toEqual([{ insz: DEFAULT_USER.insz, pin: '1234' }]);
    for (const cmd of ['login', 'work_out', 'work_in', 'lock_session', 'logout']) {
      expect(await ipcCalls(page, cmd)).toHaveLength(0);
    }
    await expect(page).toHaveURL(/#\/$/);
    await expect(page.locator('.toast-error')).toHaveCount(0);
  });

  test('a refused PIN on the end of shift is shown by the PIN pad', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
      list_open_shifts: () => ON_SHIFT,
      end_shift: { __reject: 'pin_locked|seconds=30' },
    });
    await page.goto('/');
    await page.locator('.user-btn').first().click();
    await page.locator('.end-shift-btn').click();
    for (const d of '0000') await page.locator('.pin-overlay .nk', { hasText: d }).first().click();

    await expect(page.locator('.pin-overlay')).toBeVisible();
    await expect(page.locator('.pin-overlay .error-msg')).toContainText('30 s');
    await expect(page.locator('.toast-success')).toHaveCount(0);
    expect(await ipcCalls(page, 'end_shift')).toHaveLength(1);
  });

  test('an unregistered clock-out is reported by a toast', async ({ page }) => {
    await mockTauriIPC(page, {
      list_users: () => [DEFAULT_USER],
      list_open_shifts: () => ON_SHIFT,
      end_shift: () => ({ success: false, error: 'FDM error: offline' }),
    });
    await page.goto('/');
    await page.locator('.user-btn').first().click();
    await page.locator('.end-shift-btn').click();
    for (const d of '1234') await page.locator('.pin-overlay .nk', { hasText: d }).first().click();

    await expect(page.locator('.toast-error')).toContainText('Pointage non enregistré');
    await expect(page.locator('.toast-error')).toContainText('Erreur boîte noire. Contactez votre technicien.');
    await expect(page.locator('.toast-success')).toHaveCount(0);
    await expect(page).toHaveURL(/#\/$/);
  });

  test('without being on shift, there is no "Fin de service"', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER], list_open_shifts: () => [] });
    await page.goto('/');
    await page.locator('.user-btn').first().click();
    await expect(page.locator('.pin-overlay')).toBeVisible();
    await expect(page.locator('.end-shift-btn')).toHaveCount(0);
  });
});

test.describe('Idle lock (owned by the engine)', () => {
  test('a tap is forwarded to the engine as user_activity, at most once per second', async ({ page }) => {
    await page.clock.install();
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER] });
    await page.goto('/');
    await expect(page.locator('.user-btn')).toBeVisible();
    const before = (await ipcCalls(page, 'user_activity')).length;

    await page.mouse.click(5, 5);
    await expect.poll(async () => (await ipcCalls(page, 'user_activity')).length).toBe(before + 1);

    // Within the same second: not forwarded again.
    await page.clock.runFor(400);
    await page.mouse.click(5, 5);
    await page.keyboard.press('Shift');
    await page.clock.runFor(400);
    await page.mouse.click(5, 5);
    expect((await ipcCalls(page, 'user_activity')).length).toBe(before + 1);

    // A second later: forwarded again.
    await page.clock.runFor(1_000);
    await page.mouse.click(5, 5);
    await expect.poll(async () => (await ipcCalls(page, 'user_activity')).length).toBe(before + 2);
  });

  test('the UI has no idle timer of its own', async ({ page }) => {
    await page.clock.install();
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER] });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables|order/);
    await page.clock.runFor(6 * 60 * 1000);
    await expect(page).toHaveURL(/tables|order/);
    expect(await ipcCalls(page, 'lock_session')).toHaveLength(0);
  });

  test('an idle session-locked event returns to the login screen', async ({ page }) => {
    await mockTauriIPC(page, { list_users: () => [DEFAULT_USER] });
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables|order/);
    await emitTauriEvent(page, 'session-locked', { reason: 'idle', orderError: null });
    await page.waitForURL(/#\/$/);
    await expect(page.locator('.user-btn').first()).toBeVisible();
  });
});
