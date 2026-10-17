import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC,
  badgeInit,
  scanBadge,
  ipcCalls,
  loginAndGoto,
  shiftsInit,
  tapUserTile,
  overrideIPC,
  DEFAULT_USER,
  TEST_PIN,
  type BadgeTapStep,
} from './fixtures/mock-ipc';

// USB wedge badge reader (plan L5-D12): a burst of physical keys ending
// with Enter is a badge, anything slower or shorter is typing.

const TOKEN = '04A1B2C3';
const CASHIER = { insz: '84022899837', name: 'Serveur', role: 'cashier', active: true, has_pin: true };
const WEI = { insz: '99012345667', name: 'Wei', role: 'cashier', active: true, has_pin: true };
const COOK = { insz: '88010112345', name: 'Chen', role: 'waiter', active: true, has_pin: false, presence_only: true };
const USB = { reader: 'usb' as const };

async function open(
  page: Page,
  init: { reader?: boolean; settings?: Record<string, unknown>; taps?: BadgeTapStep[]; badges?: any[]; log?: any[]; linkReject?: string } = {},
  hash = '/',
  users: unknown[] = [DEFAULT_USER],
) {
  await badgeInit(page, {
    settings: init.reader === false ? {} : { ...USB, ...(init.settings ?? {}) },
    taps: init.taps,
    badges: init.badges,
    log: init.log,
    linkReject: init.linkReject,
    names: { [WEI.insz]: 'Wei', [COOK.insz]: 'Chen' },
  });
  await mockTauriIPC(page, { list_users: () => users });
  await page.goto(hash);
  await page.waitForFunction(() => (window as any).__ipcCalls.some((c: any) => c.cmd === 'get_badge_settings'));
  await page.waitForTimeout(150);
}

/** The badge panel is folded by default: unfold it. */
async function unfold(page: Page) {
  const toggle = page.getByTestId('badge-toggle');
  await toggle.waitFor();
  if ((await toggle.getAttribute('aria-expanded')) !== 'true') await toggle.click();
}

const nameField = (page: Page) => page.getByPlaceholder('Nom');
const taps = (page: Page) => ipcCalls(page, 'badge_tap');

test.describe('Reader off', () => {
  test('with the reader off a burst is typed into the field and nothing is sent', async ({ page }) => {
    await open(page, { reader: false }, '/#/settings');
    await nameField(page).focus();
    await page.keyboard.type('04123456', { delay: 5 });
    await page.keyboard.press('Enter');
    await expect(nameField(page)).toHaveValue('04123456');
    expect(await taps(page)).toEqual([]);
  });
});

test.describe('Burst detection', () => {
  test('a fast burst ending with Enter is a badge', async ({ page }) => {
    await open(page);
    await scanBadge(page, TOKEN);
    await expect.poll(() => taps(page)).toEqual([{ token: TOKEN, endShift: false }]);
  });

  test('the keys are read as physical keys, whatever the layout types', async ({ page }) => {
    await open(page);
    // An AZERTY digit row types & é " ' ( § but the keys are Digit1..6.
    await page.evaluate(() => {
      const keys: Array<[string, string]> = [['Digit1', '&'], ['Digit2', 'é'], ['Digit3', '"'], ['Digit4', "'"], ['Digit5', '('], ['Digit6', '§']];
      for (const [code, key] of keys) window.dispatchEvent(new KeyboardEvent('keydown', { code, key, bubbles: true, cancelable: true }));
      window.dispatchEvent(new KeyboardEvent('keydown', { code: 'NumpadEnter', key: 'Enter', bubbles: true, cancelable: true }));
    });
    await expect.poll(() => taps(page)).toEqual([{ token: '123456', endShift: false }]);
  });

  test('the numeric keypad keys count too', async ({ page }) => {
    await open(page);
    await page.evaluate(() => {
      for (const d of '987654') window.dispatchEvent(new KeyboardEvent('keydown', { code: `Numpad${d}`, key: d, bubbles: true, cancelable: true }));
      window.dispatchEvent(new KeyboardEvent('keydown', { code: 'Enter', key: 'Enter', bubbles: true, cancelable: true }));
    });
    await expect.poll(() => taps(page)).toEqual([{ token: '987654', endShift: false }]);
  });

  test('Shift held by the reader does not break the burst', async ({ page }) => {
    await open(page);
    await page.keyboard.down('Shift');
    await page.keyboard.type('ABCDEF12', { delay: 5 });
    await page.keyboard.up('Shift');
    await page.keyboard.press('Enter');
    await expect.poll(() => taps(page)).toEqual([{ token: 'ABCDEF12', endShift: false }]);
  });

  test('a scan is not user activity', async ({ page }) => {
    await open(page, { taps: [{ kind: 'ignored' }] });
    const before = (await ipcCalls(page, 'user_activity')).length;
    await scanBadge(page, TOKEN);
    await expect.poll(() => taps(page)).toHaveLength(1);
    await page.waitForTimeout(100);
    expect((await ipcCalls(page, 'user_activity')).length).toBe(before);
  });
});

test.describe('Typing is never a badge', () => {
  test('human typing reaches the field in order', async ({ page }) => {
    await open(page, { settings: { wedgeMaxGapMs: 80 } }, '/#/settings');
    await nameField(page).focus();
    await page.keyboard.type('123456789', { delay: 150 });
    await page.keyboard.press('Enter');
    await expect(nameField(page)).toHaveValue('123456789');
    expect(await taps(page)).toEqual([]);
  });

  test('a burst never reaches a focused field', async ({ page }) => {
    await open(page, {}, '/#/settings');
    await nameField(page).focus();
    await nameField(page).fill('Wei ');
    await scanBadge(page, TOKEN);
    await expect.poll(() => taps(page)).toHaveLength(1);
    await expect(nameField(page)).toHaveValue('Wei ');
  });

  test('a short burst is replayed into the field, Enter included', async ({ page }) => {
    await open(page, {}, '/#/settings');
    await nameField(page).focus();
    await page.keyboard.type('12345', { delay: 5 });
    await page.keyboard.press('Enter');
    await expect(nameField(page)).toHaveValue('12345');
    expect(await taps(page)).toEqual([]);
  });

  test('a long burst is replayed whole and its tail is not a badge', async ({ page }) => {
    await open(page, {}, '/#/settings');
    await nameField(page).focus();
    const long = '1234567890'.repeat(4);
    await page.keyboard.type(long, { delay: 5 });
    await page.keyboard.press('Enter');
    await expect(nameField(page)).toHaveValue(long);
    expect(await taps(page)).toEqual([]);
  });

  test('a key that is not a badge key breaks the burst and keeps the order', async ({ page }) => {
    await open(page, {}, '/#/settings');
    await nameField(page).focus();
    await page.keyboard.type('12', { delay: 5 });
    await page.keyboard.type('xy', { delay: 5 });
    await page.keyboard.type('34', { delay: 5 });
    await page.keyboard.press('Enter');
    await expect(nameField(page)).toHaveValue('12xy34');
    expect(await taps(page)).toEqual([]);
  });

  test('the configured gap is honoured: slower than the gap is typing', async ({ page }) => {
    await open(page, { settings: { wedgeMaxGapMs: 80 } }, '/#/settings');
    await nameField(page).focus();
    await page.keyboard.type('123456', { delay: 130 });
    await page.keyboard.press('Enter');
    await expect(nameField(page)).toHaveValue('123456');
    expect(await taps(page)).toEqual([]);
  });

  test('the configured gap is honoured: a wider gap makes a slow reader work', async ({ page }) => {
    await open(page, { settings: { wedgeMaxGapMs: 200 } });
    await page.keyboard.type('123456', { delay: 110 });
    await page.keyboard.press('Enter');
    await expect.poll(() => taps(page)).toEqual([{ token: '123456', endShift: false }]);
  });

  test('the configured length is honoured', async ({ page }) => {
    await open(page, { settings: { wedgeMinLen: 4 } });
    await scanBadge(page, '1234');
    await expect.poll(() => taps(page)).toEqual([{ token: '1234', endShift: false }]);
  });
});

test.describe('What a scan shows', () => {
  test('a badge on the order screen shows Arrivée with name and time', async ({ page }) => {
    await open(page, { taps: [{ kind: 'clock_in', insz: WEI.insz, name: 'Wei', at: '2026-10-17T10:02:00Z' }] });
    await tapUserTile(page);
    await page.waitForURL(/tables|order/);
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast')).toHaveText('Arrivée · Wei · 12:02');
    // The cashier stays where they are.
    await expect(page).toHaveURL(/tables|order/);
  });

  test('a departure shows Départ', async ({ page }) => {
    await open(page, { taps: [{ kind: 'clock_out', insz: WEI.insz, name: 'Wei', at: '2026-10-17T20:45:00Z' }] });
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast')).toHaveText('Départ · Wei · 22:45');
  });

  test('an FDM failure shows the FDM text', async ({ page }) => {
    await open(page, { taps: [{ kind: 'presence_failed', insz: WEI.insz, name: 'Wei', error: 'FDM injoignable' }] });
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast-error')).toContainText('Pointage refusé · Wei');
    await expect(page.locator('.toast-error')).toContainText('Erreur boîte noire');
  });

  test('an ignored repeat shows nothing', async ({ page }) => {
    await open(page, { taps: [{ kind: 'ignored' }] });
    await scanBadge(page, TOKEN);
    await expect.poll(() => taps(page)).toHaveLength(1);
    await page.waitForTimeout(150);
    await expect(page.locator('.toast')).toHaveCount(0);
  });

  const REFUSALS: Array<[string, string]> = [
    ['badge_unknown', 'Badge inconnu'],
    ['badge_revoked', 'Badge retiré : demandez un nouveau badge au Gérant'],
    ['badge_reader_off', 'Le lecteur de badges est désactivé'],
    ['badge_token_invalid', 'Badge illisible : présentez-le de nouveau'],
    ['badge_key_unavailable', 'Clé des badges inaccessible : contactez l\'assistance'],
    ['presence_self', 'Votre arrivée et votre départ se font à la connexion et par « Fin de service »'],
    ['presence_not_in', 'Cet employé n\'est pas en service'],
    ['presence_training', 'Mode formation'],
  ];
  for (const [code, text] of REFUSALS) {
    test(`the refusal ${code} is shown as its text`, async ({ page }) => {
      await open(page, { taps: [{ reject: code }] });
      await scanBadge(page, TOKEN);
      await expect(page.locator('.toast-error')).toContainText(text);
    });
  }

  test('English texts', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'en'));
    await open(page, { taps: [{ kind: 'clock_in', name: 'Wei', at: '2026-10-17T10:02:00Z' }, { reject: 'badge_unknown' }] });
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast')).toHaveText('Clock-in · Wei · 12:02');
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast-error')).toHaveText('Unknown badge');
  });

  test('Chinese texts', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'zh'));
    await open(page, { taps: [{ kind: 'clock_out', name: 'Wei', at: '2026-10-17T10:02:00Z' }, { reject: 'badge_unknown' }, { kind: 'presence_failed', name: 'Wei', error: 'x' }] });
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast')).toHaveText('下班 · Wei · 12:02');
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast-error')).toHaveText('未知工牌');
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast-error', { hasText: '打卡失败 · Wei' })).toBeVisible();
  });
});

test.describe('Login screen', () => {
  test('the hint shows when badge login is on, and not otherwise', async ({ page }) => {
    await open(page, { settings: { loginEnabled: true } });
    await expect(page.getByTestId('badge-login-hint')).toHaveText('Ou présentez votre badge');
    await open(page, { settings: { loginEnabled: false } });
    await expect(page.getByTestId('badge-login-hint')).toHaveCount(0);
    await open(page, { reader: false });
    await expect(page.getByTestId('badge-login-hint')).toHaveCount(0);
  });

  test('a badge on the login screen logs a cashier in', async ({ page }) => {
    await open(page, { settings: { loginEnabled: true }, taps: [{ kind: 'logged_in', insz: CASHIER.insz, name: CASHIER.name, user: CASHIER }] }, '/', [CASHIER]);
    await scanBadge(page, TOKEN);
    await expect(page).toHaveURL(/tables|order/);
    expect(await ipcCalls(page, 'login')).toEqual([]);
  });

  test('a manager badge opens the PIN pad for that manager', async ({ page }) => {
    await open(page, { settings: { loginEnabled: true }, taps: [{ kind: 'pin_required', insz: DEFAULT_USER.insz, name: DEFAULT_USER.name }] });
    await scanBadge(page, TOKEN);
    const overlay = page.locator('.pin-overlay');
    await expect(overlay).toBeVisible();
    await expect(overlay.locator('.pin-user')).toHaveText('Bruno Moyse');
    await expect(page).toHaveURL(/#\/$/);
    for (const d of TEST_PIN) await overlay.locator('.nk', { hasText: d }).first().click();
    await expect(page).toHaveURL(/tables|order/);
    expect(await ipcCalls(page, 'login')).toEqual([{ insz: DEFAULT_USER.insz, pin: TEST_PIN }]);
  });

  test('in Fin de service mode the badge is sent with end_shift and clocks its holder out', async ({ page }) => {
    await shiftsInit(page, [{ userInsz: DEFAULT_USER.insz, startedAt: '2026-10-17T08:00:00Z' }]);
    await open(page, { taps: [{ kind: 'clock_out', insz: WEI.insz, name: 'Wei', at: '2026-10-17T10:02:00Z' }, { kind: 'ignored' }] });
    await page.locator('.user-btn').first().click();
    await expect(page.getByTestId('end-shift-badge-hint')).toHaveCount(0);
    await page.locator('.end-shift-btn').click();
    await expect(page.getByTestId('end-shift-badge-hint')).toHaveText('Présentez votre badge pour terminer le service');
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast')).toHaveText('Départ · Wei · 12:02');
    await expect.poll(() => taps(page)).toEqual([{ token: TOKEN, endShift: true }]);
    // The pad has done its job; the next scan is a plain one again.
    await expect(page.locator('.pin-overlay')).toHaveCount(0);
    await scanBadge(page, TOKEN);
    await expect.poll(() => taps(page)).toEqual([{ token: TOKEN, endShift: true }, { token: TOKEN, endShift: false }]);
  });
});

test.describe('Settings: the reader', () => {
  test('a manager sees the panel and changes the reader', async ({ page }) => {
    await open(page, { reader: false }, '/');
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    const panel = page.getByTestId('badge-settings');
    // Folded by default, with the reader it is set to.
    await expect(page.getByTestId('badge-toggle')).toHaveAttribute('aria-expanded', 'false');
    await expect(page.getByTestId('badge-toggle')).toContainText('Désactivé');
    await expect(panel.locator('.reader-btn')).toHaveCount(0);
    await unfold(page);
    await expect(panel.locator('h2')).toHaveText('Lecteur de badges');
    await expect(panel.locator('.reader-btn')).toHaveText(['Désactivé', 'Lecteur USB']);
    await expect(panel.locator('.reader-btn.active')).toHaveText('Désactivé');
    await expect(panel.getByTestId('badge-login-switch')).toHaveCount(0);
    await panel.locator('.reader-btn', { hasText: 'Lecteur USB' }).click();
    await expect.poll(() => ipcCalls(page, 'set_badge_settings')).toEqual([
      { input: { reader: 'usb', loginEnabled: false, wedgeMaxGapMs: 150, wedgeMinLen: 6, wedgeMaxLen: 32 } },
    ]);
    await expect(panel.locator('.reader-btn.active')).toHaveText('Lecteur USB');
    await expect(panel.getByTestId('badge-log')).toContainText('Réglages modifiés');
    // The reader is on now: a scan reaches the engine without a reload.
    await page.locator('body').click();
    await scanBadge(page, TOKEN);
    await expect.poll(() => taps(page)).toHaveLength(1);
  });

  test('the badge login switch and its note', async ({ page }) => {
    await open(page, {}, '/');
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    const panel = page.getByTestId('badge-settings');
    await unfold(page);
    await expect(panel).toContainText('Les Gérants saisissent toujours leur PIN après le badge.');
    await panel.getByTestId('badge-login-switch').click();
    await expect.poll(() => ipcCalls(page, 'set_badge_settings')).toEqual([
      { input: { reader: 'usb', loginEnabled: true, wedgeMaxGapMs: 150, wedgeMinLen: 6, wedgeMaxLen: 32 } },
    ]);
    await expect(panel.getByTestId('badge-login-switch')).toHaveAttribute('aria-checked', 'true');
  });

  test('the USB timing is typed on the pad and saved', async ({ page }) => {
    await open(page, {}, '/');
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    const panel = page.getByTestId('badge-settings');
    await unfold(page);
    await expect(panel.getByTestId('badge-timing-save')).toHaveCount(0);
    await panel.getByTestId('badge-wedgeMaxGapMs').click();
    await panel.getByRole('button', { name: 'C', exact: true }).click();
    await panel.getByRole('button', { name: '1', exact: true }).click();
    await panel.getByRole('button', { name: '2', exact: true }).click();
    await panel.getByRole('button', { name: '0', exact: true }).click();
    await expect(panel.getByTestId('badge-wedgeMaxGapMs')).toHaveText('120');
    await panel.getByTestId('badge-timing-save').click();
    await expect.poll(() => ipcCalls(page, 'set_badge_settings')).toEqual([
      { input: { reader: 'usb', loginEnabled: false, wedgeMaxGapMs: 120, wedgeMinLen: 6, wedgeMaxLen: 32 } },
    ]);
  });

  test('a refused setting shows its text and keeps the stored value', async ({ page }) => {
    await open(page, {}, '/');
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    const panel = page.getByTestId('badge-settings');
    await unfold(page);
    await overrideIPC(page, 'set_badge_settings', { __reject: 'badge_settings_invalid' });
    await panel.getByTestId('badge-wedgeMaxGapMs').click();
    await panel.getByRole('button', { name: 'C', exact: true }).click();
    await panel.getByRole('button', { name: '5', exact: true }).click();
    await panel.getByTestId('badge-timing-save').click();
    await expect(panel.getByTestId('badge-error')).toHaveText('Réglages du lecteur de badges invalides');
    await expect(panel.getByTestId('badge-wedgeMaxGapMs')).toHaveText('150');
  });

  test('a stored NFC choice gets the unavailable note, and the relink banner shows', async ({ page }) => {
    await open(page, { settings: { reader: 'nfc', relinkNeeded: true } }, '/');
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    const panel = page.getByTestId('badge-settings');
    await unfold(page);
    await expect(panel.getByTestId('badge-nfc-unavailable')).toHaveText('Pas de NFC sur cet appareil : choisissez un lecteur USB');
    await expect(panel.locator('.reader-btn.active')).toHaveCount(0);
    await expect(panel.getByTestId('badge-relink')).toHaveText('La clé des badges a été renouvelée : liez de nouveau les badges');
    await expect(panel).toContainText('Seule une empreinte chiffrée du badge est conservée');
  });

  test('the badge log lists links and removals', async ({ page }) => {
    await open(page, {
      log: [
        { action: 'revoke', insz: WEI.insz, name: 'Wei', by: DEFAULT_USER.insz, byName: 'Bruno Moyse', at: '2026-10-17 09:00:00', detail: 'manager' },
        { action: 'link', insz: WEI.insz, name: 'Wei', by: DEFAULT_USER.insz, byName: 'Bruno Moyse', at: '2026-10-16 09:00:00', detail: null },
      ],
    }, '/');
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    await unfold(page);
    const log = page.getByTestId('badge-log');
    await expect(log.locator('li')).toHaveCount(2);
    await expect(log.locator('li').first()).toContainText('Badge retiré · Wei');
    await expect(log.locator('li').first()).toContainText('Bruno Moyse 17/10 11:00');
    await expect(log.locator('li').nth(1)).toContainText('Badge lié · Wei');
  });

  test('the badge panel is for managers only', async ({ page }) => {
    await badgeInit(page, { settings: USB });
    await mockTauriIPC(page, { list_users: () => [CASHIER], login: () => CASHIER });
    await page.goto('/');
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    await expect(page.locator('.workers-section')).toBeVisible();
    await expect(page.getByTestId('badge-settings')).toHaveCount(0);
    await expect(page.getByTestId('badge-state')).toHaveCount(0);
    await expect(page.getByTestId('badge-btn')).toHaveCount(0);
  });
});

test.describe('Settings: employees and badges', () => {
  const USERS = [DEFAULT_USER, WEI, COOK];
  const WEI_BADGE = { insz: WEI.insz, name: 'Wei', linkedAt: '2026-10-17 08:30:00', linkedBy: DEFAULT_USER.insz, linkedByName: 'Bruno Moyse' };
  const row = (page: Page, name: string) => page.locator('.worker-row', { hasText: name });

  async function settings(page: Page, init: Parameters<typeof open>[1] = {}) {
    await open(page, init, '/', USERS);
    await tapUserTile(page);
    await page.evaluate(() => { window.location.hash = '/settings'; });
    await expect(row(page, 'Wei')).toBeVisible();
  }

  test('each row says if it has a badge and offers the right button', async ({ page }) => {
    await settings(page, { badges: [WEI_BADGE] });
    await expect(row(page, 'Wei').getByTestId('badge-state')).toHaveText('Badge lié le 17/10/2026');
    await expect(row(page, 'Wei').getByTestId('badge-btn')).toHaveText('Retirer le badge');
    await expect(row(page, 'Chen').getByTestId('badge-state')).toHaveText('Aucun badge');
    await expect(row(page, 'Chen').getByTestId('badge-btn')).toHaveText('Lier un badge');
  });

  test('with the reader off the rows have no badge state and no button', async ({ page }) => {
    await settings(page, { reader: false, badges: [WEI_BADGE] });
    await expect(page.getByTestId('badge-state')).toHaveCount(0);
    await expect(page.getByTestId('badge-btn')).toHaveCount(0);
  });

  test('linking a badge asks for it then for the manager PIN', async ({ page }) => {
    await settings(page);
    await row(page, 'Chen').getByTestId('badge-btn').click();
    await expect(page.getByTestId('badge-present')).toHaveText('Présentez le badge de Chen');
    await scanBadge(page, TOKEN);
    // The scan was captured, never sent as a tap.
    const dialog = page.locator('.pin-dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText('Confirmez avec votre PIN de gérant');
    expect(await taps(page)).toEqual([]);
    for (const d of TEST_PIN) await dialog.getByRole('button', { name: d, exact: true }).click();
    await expect.poll(() => ipcCalls(page, 'link_badge')).toEqual([
      { employeeInsz: COOK.insz, token: TOKEN, operatorPin: TEST_PIN },
    ]);
    await expect(page.locator('.toast')).toHaveText('Badge lié · Chen');
    await expect(row(page, 'Chen').getByTestId('badge-btn')).toHaveText('Retirer le badge');
    await unfold(page);
    await expect(page.getByTestId('badge-log')).toContainText('Badge lié · Chen');
  });

  test('Annuler leaves nothing linked', async ({ page }) => {
    await settings(page);
    await row(page, 'Chen').getByTestId('badge-btn').click();
    await page.getByTestId('badge-enrol').getByRole('button', { name: 'Retour' }).click();
    await expect(page.getByTestId('badge-enrol')).toHaveCount(0);
    await scanBadge(page, TOKEN);
    // Back to a plain scan.
    await expect.poll(() => taps(page)).toHaveLength(1);
    expect(await ipcCalls(page, 'link_badge')).toEqual([]);
  });

  test('no badge within 30 seconds ends the dialog', async ({ page }) => {
    await page.clock.install();
    await settings(page);
    await row(page, 'Chen').getByTestId('badge-btn').click();
    await expect(page.getByTestId('badge-enrol')).toBeVisible();
    await page.clock.fastForward(31_000);
    await expect(page.getByTestId('badge-enrol')).toHaveCount(0);
    await expect(page.locator('.toast')).toContainText('Aucun badge présenté');
  });

  test('a badge already linked to someone else is refused with its text', async ({ page }) => {
    await settings(page, { linkReject: 'badge_in_use' });
    await row(page, 'Chen').getByTestId('badge-btn').click();
    await scanBadge(page, TOKEN);
    const dialog = page.locator('.pin-dialog');
    for (const d of TEST_PIN) await dialog.getByRole('button', { name: d, exact: true }).click();
    await expect(page.locator('.toast-error')).toHaveText('Ce badge est déjà lié à un autre employé');
    await expect(page.locator('.pin-dialog')).toHaveCount(0);
    await expect(row(page, 'Chen').getByTestId('badge-btn')).toHaveText('Lier un badge');
  });

  test('a wrong manager PIN keeps the dialog for another try', async ({ page }) => {
    await settings(page, { linkReject: 'pin_wrong' });
    await row(page, 'Chen').getByTestId('badge-btn').click();
    await scanBadge(page, TOKEN);
    const dialog = page.locator('.pin-dialog');
    for (const d of TEST_PIN) await dialog.getByRole('button', { name: d, exact: true }).click();
    await expect(dialog.locator('.error-msg')).toBeVisible();
    await expect(dialog).toBeVisible();
  });

  test('removing a badge asks for a confirmation first', async ({ page }) => {
    await settings(page, { badges: [WEI_BADGE] });
    await row(page, 'Wei').getByTestId('badge-btn').click();
    const dialog = page.getByTestId('badge-revoke-dialog');
    await expect(dialog).toContainText('Retirer le badge de Wei ?');
    await dialog.getByRole('button', { name: 'Retour' }).click();
    await expect(dialog).toHaveCount(0);
    expect(await ipcCalls(page, 'revoke_badge')).toEqual([]);
    await row(page, 'Wei').getByTestId('badge-btn').click();
    await page.getByTestId('badge-revoke-confirm').click();
    await expect.poll(() => ipcCalls(page, 'revoke_badge')).toEqual([{ employeeInsz: WEI.insz }]);
    await expect(page.locator('.toast')).toHaveText('Badge retiré · Wei');
    await expect(row(page, 'Wei').getByTestId('badge-state')).toHaveText('Aucun badge');
    await unfold(page);
    await expect(page.getByTestId('badge-log')).toContainText('Badge retiré');
  });

  test('a badge registering someone refreshes who is on shift', async ({ page }) => {
    await settings(page, { taps: [{ kind: 'clock_in', insz: WEI.insz, name: 'Wei', at: '2026-10-17T10:02:00Z' }] });
    const before = (await ipcCalls(page, 'list_open_shifts')).length;
    await page.locator('body').click();
    await scanBadge(page, TOKEN);
    await expect(page.locator('.toast')).toHaveText('Arrivée · Wei · 12:02');
    await expect.poll(async () => (await ipcCalls(page, 'list_open_shifts')).length).toBeGreaterThan(before);
  });

  test('the texts follow the language (Chinese)', async ({ page }) => {
    await page.addInitScript(() => localStorage.setItem('pili-pos-language', 'zh'));
    await settings(page);
    await expect(row(page, 'Chen').getByTestId('badge-state')).toHaveText('无工牌');
    await expect(row(page, 'Chen').getByTestId('badge-btn')).toHaveText('绑定工牌');
    await expect(page.getByTestId('badge-settings').locator('h2')).toHaveText('工牌读卡器');
  });
});
