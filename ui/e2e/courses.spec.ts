import { test, expect, type Page } from '@playwright/test';
import {
  mockTauriIPC, ipcCalls, live, DEFAULT_USER, DEFAULT_ORDER, DEFAULT_CATEGORY, DEFAULT_MENU_ITEM,
  DEFAULT_CART_LINE, tapUserTile, loginAndGoto,
} from './fixtures/mock-ipc';

// Courses and "Envoyer la suite" (kitchen printing only): the Settings
// panel, the cart grouped by course with a chip and a picker per line,
// the "Envoyer la suite (Plats)" button, the badge on the table plan and
// the warning at checkout. The engine state lives in a page global the
// live handlers read and write.
const LINE = (id: number, name: string, course: number, state: string, unsent = 0) => ({
  ...DEFAULT_CART_LINE,
  id,
  menu_item_id: id,
  name,
  name_fr: name,
  name_zh: null,
  unit_price_cents: 500,
  course,
  kitchen_state: state,
  unsent_quantity: unsent,
});

const CART = [
  LINE(1, 'Sake', 0, 'printed'),
  LINE(2, 'Edamame', 1, 'printed'),
  LINE(3, 'Maki saumon', 2, 'held'),
  LINE(4, 'Gyoza', 2, 'held', 1),
  LINE(5, 'Mochi', 3, 'held'),
];

async function orderScreen(page: Page, cart = CART, extra: Record<string, unknown> = {}) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => ({ ...DEFAULT_ORDER, total_cents: 2500 }),
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    get_course_settings: () => ({ enabled: true, category_courses: [] }),
    list_cart_lines: live(() => JSON.parse(JSON.stringify((window as any).__lines))),
    list_held_courses: live(() => {
      const held: Record<number, { course: number; lines: number; quantity: number }> = {};
      for (const l of (window as any).__lines) {
        if (l.kitchen_state !== 'held') continue;
        const h = held[l.course] ?? { course: l.course, lines: 0, quantity: 0 };
        h.lines += 1;
        h.quantity += l.quantity;
        held[l.course] = h;
      }
      return Object.values(held).sort((a, b) => a.course - b.course);
    }),
    set_line_course: live((args: any) => {
      const line = (window as any).__lines.find((l: any) => l.id === args.lineId);
      if (!line || line.kitchen_state === 'printed') throw 'course_already_sent';
      line.course = args.course;
      line.kitchen_state = args.course > 1 ? 'held' : 'unsent';
    }),
    send_next_course: live(() => {
      const lines = (window as any).__lines;
      const next = Math.min(...lines.filter((l: any) => l.kitchen_state === 'held').map((l: any) => l.course));
      for (const l of lines) {
        if (l.kitchen_state === 'held' && l.course === next) {
          l.kitchen_state = 'printed';
          l.unsent_quantity = 0;
        }
      }
      return { course: next, signed: null, slip: 'printed' };
    }),
    ...extra,
  } as any);
  await page.addInitScript((lines) => { (window as any).__lines = lines; }, cart);
  await page.goto('/');
  await tapUserTile(page);
  await page.waitForURL(/tables/);
  await page.locator('.table-card').first().click();
  await page.waitForURL(/order/);
}

test.describe('Courses in the cart', () => {
  test('the cart is grouped by course, lowest first, with a chip per line', async ({ page }) => {
    await orderScreen(page);
    const headers = page.locator('.course-label');
    await expect(headers).toHaveText(['Direct', 'Entrées', 'Plats', 'Desserts']);
    // A printed line is locked, a held one says it waits.
    await expect(page.getByTestId('course-chip-1')).toBeDisabled();
    await expect(page.getByTestId('course-chip-2')).toBeDisabled();
    await expect(page.getByTestId('course-chip-3')).toBeEnabled();
    await expect(page.getByTestId('course-chip-3')).toContainText('En attente');
    await expect(page.getByTestId('course-chip-3')).toContainText('Plats');
    await expect(page.getByTestId('course-chip-2')).not.toContainText('En attente');
  });

  test('with courses off the cart is the plain one', async ({ page }) => {
    await orderScreen(page, [LINE(1, 'Sake', 0, 'unsent', 1)], {
      get_course_settings: () => ({ enabled: false, category_courses: [] }),
    });
    await expect(page.locator('.order-line')).toHaveCount(1);
    await expect(page.locator('.course-label')).toHaveCount(0);
    await expect(page.getByTestId('course-chip-1')).toHaveCount(0);
    await expect(page.getByTestId('send-next-course')).toHaveCount(0);
  });

  test('the chip opens a four-button picker and the pick changes the course', async ({ page }) => {
    await orderScreen(page);
    await page.getByTestId('course-chip-5').click();
    const picker = page.getByTestId('course-picker');
    await expect(picker).toBeVisible();
    await expect(picker.locator('.choice')).toHaveText(['Direct', 'Entrées', 'Plats', 'Desserts']);
    await expect(picker.getByTestId('course-pick-3')).toHaveAttribute('aria-pressed', 'true');
    await picker.getByTestId('course-pick-1').click();
    await expect(picker).toHaveCount(0);

    const calls = await ipcCalls(page, 'set_line_course');
    expect(calls).toEqual([{ lineId: 5, course: 1 }]);
    // Moved to the starters: no longer held, grouped under Entrées.
    await expect(page.getByTestId('course-chip-5')).not.toContainText('En attente');
    await expect(page.locator('.course-label')).toHaveText(['Direct', 'Entrées', 'Plats']);
  });

  test('a refusal from the engine is shown', async ({ page }) => {
    await orderScreen(page, CART, {
      set_line_course: live(() => { throw 'course_already_sent'; }),
    });
    await page.getByTestId('course-chip-3').click();
    await page.getByTestId('course-pick-1').click();
    await expect(page.getByText('déjà partie en cuisine')).toBeVisible();
  });

  test('"Envoyer la suite" releases the courses one after the other', async ({ page }) => {
    await orderScreen(page);
    const button = page.getByTestId('send-next-course');
    await expect(button).toHaveText('Envoyer la suite (Plats)');
    await button.click();
    await expect(page.getByText('Suite envoyée : Plats')).toBeVisible();
    expect(await ipcCalls(page, 'send_next_course')).toHaveLength(1);
    // Plats are printed now; desserts are next.
    await expect(page.getByTestId('course-chip-3')).toBeDisabled();
    await expect(button).toHaveText('Envoyer la suite (Desserts)');
    await button.click();
    await expect(button).toHaveCount(0);
    expect(await ipcCalls(page, 'send_next_course')).toHaveLength(2);
  });

  test('a refused release (FDM down) is shown and nothing changes', async ({ page }) => {
    await orderScreen(page, CART, {
      send_next_course: live(() => { throw 'fdm_unreachable'; }),
    });
    await page.getByTestId('send-next-course').click();
    await expect(page.getByTestId('send-next-course')).toHaveText('Envoyer la suite (Plats)');
    await expect(page.getByTestId('course-chip-3')).toContainText('En attente');
  });
});

test.describe('Courses on the table plan and at checkout', () => {
  test('a table with a held course says which one', async ({ page }) => {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      list_tables: () => Array.from({ length: 20 }, (_, i) => (i === 1
        ? { tableNumber: 2, orderId: 9, totalCents: 4200, lineCount: 3, status: 'occupied', openedAt: null, preBillPrinted: false, heldCourse: 2 }
        : { tableNumber: i + 1, orderId: null, totalCents: 0, lineCount: 0, status: 'free', openedAt: null, preBillPrinted: false, heldCourse: null })),
    } as any);
    await page.goto('/');
    await tapUserTile(page);
    await page.waitForURL(/tables/);
    await expect(page.getByTestId('held-course-2')).toHaveText('Suite : Plats');
    await expect(page.locator('[data-testid^="held-course-"]')).toHaveCount(1);
  });

  test('checkout warns about a held course and sends it now', async ({ page }) => {
    await orderScreen(page);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    const warning = page.getByTestId('held-warning');
    await expect(warning).toContainText('Suite non envoyée : Plats');
    await page.getByTestId('held-send-now').click();
    await expect(warning).toContainText('Suite non envoyée : Desserts');
    expect(await ipcCalls(page, 'send_next_course')).toHaveLength(1);
    await page.getByTestId('held-send-now').click();
    await expect(warning).toHaveCount(0);
  });

  test('without a held course the checkout shows no warning', async ({ page }) => {
    await orderScreen(page, [LINE(1, 'Sake', 0, 'printed')]);
    await page.locator('.checkout-btn').click();
    await page.waitForURL(/checkout/);
    await expect(page.getByTestId('held-warning')).toHaveCount(0);
  });
});

test.describe('Courses settings panel', () => {
  async function panel(page: Page, extra: Record<string, unknown> = {}) {
    await mockTauriIPC(page, {
      login: () => DEFAULT_USER,
      list_users: () => [DEFAULT_USER],
      get_categories: () => [DEFAULT_CATEGORY, { ...DEFAULT_CATEGORY, id: 2, name: 'Desserts maison' }],
      ...extra,
    } as any);
    await loginAndGoto(page, '/settings');
    return page.getByTestId('courses-panel');
  }

  test('is off by default, with a row of four buttons per category', async ({ page }) => {
    const p = await panel(page);
    await expect(p.locator('h2')).toHaveText('Service en plusieurs temps');
    await expect(p.getByTestId('courses-enabled')).toContainText('Inactif');
    await expect(p.getByTestId('courses-save')).toBeDisabled();
    await expect(p.getByTestId('courses-cat-1').locator('.seg-btn')).toHaveText(['Direct', 'Entrées', 'Plats', 'Desserts']);
    await expect(p.getByTestId('courses-cat-1-0')).toHaveClass(/active/);
    await expect(p.getByTestId('courses-cat-2')).toContainText('Desserts maison');
  });

  test('a manager switches it on, maps categories and saves', async ({ page }) => {
    const p = await panel(page);
    await p.getByTestId('courses-enabled').click();
    await p.getByTestId('courses-cat-1-2').click();
    await p.getByTestId('courses-cat-2-3').click();
    await p.getByTestId('courses-save').click();
    await expect(p.getByTestId('courses-saved')).toBeVisible();
    const calls = await ipcCalls(page, 'set_course_settings');
    expect(calls).toHaveLength(1);
    expect(calls[0].input).toEqual({
      enabled: true,
      category_courses: [{ category_id: 1, course: 2 }, { category_id: 2, course: 3 }],
    });
  });

  test('a refusal of the engine is shown', async ({ page }) => {
    const p = await panel(page, { set_course_settings: live(() => { throw 'course_invalid|category=2'; }) });
    await p.getByTestId('courses-enabled').click();
    await p.getByTestId('courses-save').click();
    await expect(p.getByTestId('courses-error')).toBeVisible();
    await expect(p.getByTestId('courses-saved')).toHaveCount(0);
  });
});
