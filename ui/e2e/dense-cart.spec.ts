import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  tapUserTile,
  live,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_CART_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// UX audit C2: the dense cart. Every line is a 56 px row (kitchen mark, quantity, code, name, chips,
// price); the actions (minus, plus, Note, Offrir, course, Supprimer) show only under the selected line,
// which is the newest one by default. Every action and its rule is the one of the earlier cart.

const CASHIER = { ...DEFAULT_USER, insz: '84022899837', name: 'Elisa Serveuse', role: 'cashier' }

const line = (id: number, extra: Record<string, unknown> = {}) => ({
  ...DEFAULT_CART_LINE,
  id,
  name: `Plat ${id}`,
  name_fr: `Plat ${id}`,
  code: `A${id}`,
  quantity: 1,
  unsent_quantity: 0,
  ...extra,
})

async function toOrder(
  page: Page,
  lines: unknown[],
  opts: { user?: typeof DEFAULT_USER; orderType?: 'dine_in' | 'takeaway'; extra?: object } = {},
) {
  const user = opts.user ?? DEFAULT_USER
  await mockTauriIPC(page, {
    login: () => user,
    list_users: () => [user],
    create_order: () => ({
      ...DEFAULT_ORDER,
      order_type: opts.orderType ?? 'dine_in',
      table_number: opts.orderType === 'takeaway' ? null : DEFAULT_ORDER.table_number,
    }),
    get_order: () => ({
      ...DEFAULT_ORDER,
      order_type: opts.orderType ?? 'dine_in',
      total_cents: 2400,
    }),
    list_cart_lines: () => lines,
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...opts.extra,
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

const row = (page: Page, id: number) => page.getByTestId(`line-row-${id}`)

test.describe('The row', () => {
  test('a line is 56 px high and shows mark, quantity, code, name and price on one row', async ({
    page,
  }) => {
    await toOrder(page, [line(1, { quantity: 2 }), line(2)])
    const box = await row(page, 1).boundingBox()
    expect(box?.height).toBe(56)
    await expect(row(page, 1).locator('.line-qty')).toHaveText('2×')
    await expect(row(page, 1).locator('.line-code')).toHaveText('A1')
    await expect(row(page, 1).locator('.line-name')).toHaveText('Plat 1')
    await expect(row(page, 1).locator('.line-price')).toHaveText('24,00')
    // Everything is on that one row: the name sits inside its box.
    const name = await row(page, 1).locator('.line-name').boundingBox()
    expect(name && box && name.y >= box.y && name.y + name.height <= box.y + box.height).toBe(true)
  })

  test('about twelve lines fit the cart at 1080 p', async ({ page }) => {
    const lines = Array.from({ length: 16 }, (_, i) => line(i + 1))
    await toOrder(page, lines)
    // Fold the newest line: only rows are left.
    await row(page, 16).click()
    await expect(page.getByTestId('line-detail')).toHaveCount(0)
    const area = await page.locator('.cart-lines').boundingBox()
    let visible = 0
    for (let id = 1; id <= 16; id++) {
      const box = await row(page, id).boundingBox()
      if (area && box && box.y >= area.y && box.y + box.height <= area.y + area.height) visible++
    }
    expect(visible).toBeGreaterThanOrEqual(11)
  })

  test('the code is the leading text when products are shown by code', async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('pili-pos-product-label', 'code')
    })
    await toOrder(page, [line(1)])
    await expect(row(page, 1).locator('.line-code')).toHaveClass(/lead/u)
    await expect(row(page, 1).locator('.line-name')).toHaveText('Plat 1')
  })

  test('a dine-in line shows a check once sent and an amber dot while not; a takeaway none', async ({
    page,
  }) => {
    await toOrder(page, [line(1), line(2, { unsent_quantity: 1 })])
    await expect(row(page, 1).getByTestId('line-state-sent')).toHaveText('✓')
    await expect(row(page, 2).getByTestId('line-state-pending')).toBeVisible()
    await expect(page.locator('.order-line.pending')).toHaveCount(1)
  })

  test('a takeaway line has no kitchen mark', async ({ page }) => {
    await toOrder(page, [line(1, { unsent_quantity: 1 })], { orderType: 'takeaway' })
    await expect(page.getByTestId('line-state-sent')).toHaveCount(0)
    await expect(page.getByTestId('line-state-pending')).toHaveCount(0)
  })

  test('the chips (note, discount, options, course) are inline on the same row, truncated', async ({
    page,
  }) => {
    const noted = line(1, {
      kitchen_note: 'sans wasabi, bien cuit, sauce à part, sans sésame, sans oignon',
      modifiers_json: '["Piquant moyen"]',
      price_changes: [{ id: 'TILL-FREE-LINE', name: 'Offert', scope: 'LINE', amount_cents: -1200 }],
      course: 2,
    })
    await toOrder(page, [noted, line(2)], {
      extra: { get_course_settings: () => ({ enabled: true, category_courses: [] }) },
    })
    await row(page, 2).click() // fold nothing: line 2 was the selected one, now folded
    const rowBox = await row(page, 1).boundingBox()
    for (const id of ['line-kitchen-note', 'line-change-chip', 'course-tag-1']) {
      const box = await page.getByTestId(id).boundingBox()
      expect(
        box && rowBox && box.y >= rowBox.y && box.y + box.height <= rowBox.y + rowBox.height,
        id,
      ).toBe(true)
    }
    await expect(page.getByTestId('line-change-chip')).toHaveText('Offert')
    await expect(page.getByTestId('course-tag-1')).toHaveText('Plats')
    // The long note is cut with an ellipsis instead of growing the row.
    const overflow = await page
      .getByTestId('line-kitchen-note')
      .evaluate((el) => el.scrollWidth > el.clientWidth)
    expect(overflow).toBe(true)
    expect((await row(page, 1).boundingBox())?.height).toBe(56)
  })
})

test.describe('The selected line', () => {
  test('the newest line is selected by default; the others show no action', async ({ page }) => {
    await toOrder(page, [line(1), line(2), line(3)])
    await expect(page.getByTestId('line-detail')).toHaveCount(1)
    await expect(page.locator('.order-line.selected')).toHaveCount(1)
    await expect(page.locator('.order-line.selected .line-name')).toHaveText('Plat 3')
    await expect(page.getByTestId('line-note')).toHaveCount(1)
    await expect(page.locator('.qty-btn')).toHaveCount(2)
  })

  test('a tap selects another line, a tap on the selected line folds it', async ({ page }) => {
    await toOrder(page, [line(1), line(2), line(3)])
    await row(page, 1).click()
    await expect(page.locator('.order-line.selected .line-name')).toHaveText('Plat 1')
    await expect(page.getByTestId('line-detail')).toHaveCount(1)
    await expect(row(page, 1)).toHaveAttribute('aria-expanded', 'true')
    await row(page, 1).click()
    await expect(page.getByTestId('line-detail')).toHaveCount(0)
    await expect(row(page, 1)).toHaveAttribute('aria-expanded', 'false')
    // Folded stays folded: the default does not come back by itself.
    await expect(page.locator('.order-line.selected')).toHaveCount(0)
  })

  test('a product added from the grid selects its line', async ({ page }) => {
    await toOrder(page, [line(1, { menu_item_id: 91 }), line(2, { menu_item_id: 92 })], {
      extra: {
        add_order_line: live(({ input }: any) => ({
          ...(window as any).__base,
          id: 11,
          order_id: input.order_id,
          menu_item_id: input.menu_item_id,
          quantity: input.quantity,
          name: 'Maki saumon x8',
        })),
      },
    })
    await page.evaluate((base) => {
      ;(window as any).__base = base
    }, DEFAULT_CART_LINE)
    await row(page, 1).click()
    await page.locator('.item-btn').first().click()
    await expect(page.locator('.order-line.selected')).toHaveCount(1)
    await expect(page.locator('.order-line.selected .line-name')).toHaveText('Maki saumon x8')
  })

  test('the actions of the selected line are the earlier ones: minus, plus, Note, Supprimer', async ({
    page,
  }) => {
    await toOrder(page, [line(1, { quantity: 2 }), line(2)], {
      extra: { update_order_line_quantity: () => null, remove_order_line: () => null },
    })
    await row(page, 1).click()
    const detail = page.getByTestId('line-detail')
    await expect(detail.locator('.qty')).toHaveText('2')
    await detail.getByRole('button', { name: 'Un de plus' }).click()
    await detail.getByRole('button', { name: 'Un de moins' }).click()
    expect(await ipcCalls(page, 'update_order_line_quantity')).toEqual([
      { lineId: 1, quantity: 3 },
      { lineId: 1, quantity: 2 },
    ])
    await expect(page.getByTestId('line-note')).toBeVisible()
    await expect(page.getByTestId('line-remove')).toHaveText(/Supprimer/u)
  })

  test('the minus stops at 1', async ({ page }) => {
    await toOrder(page, [line(1)])
    await expect(page.getByRole('button', { name: 'Un de moins' })).toBeDisabled()
  })

  test('Supprimer removes the line and the selection goes back to the newest one', async ({
    page,
  }) => {
    await toOrder(page, [line(1), line(2), line(3)], {
      extra: { remove_order_line: () => null },
    })
    await row(page, 2).click()
    await page.getByTestId('line-remove').click()
    expect(await ipcCalls(page, 'remove_order_line')).toEqual([{ lineId: 2 }])
    await expect(row(page, 2)).toHaveCount(0)
    await expect(page.locator('.order-line.selected .line-name')).toHaveText('Plat 3')
  })

  test('Offrir is for a manager only, and a cashier keeps Note and Supprimer', async ({ page }) => {
    await toOrder(page, [line(1)], { user: CASHIER })
    await expect(page.getByTestId('line-note')).toBeVisible()
    await expect(page.getByTestId('line-remove')).toBeVisible()
    await expect(page.getByTestId('line-offer')).toHaveCount(0)
  })

  test('a manager sees Offrir under the selected line', async ({ page }) => {
    await toOrder(page, [line(1), line(2)])
    await expect(page.getByTestId('line-offer')).toHaveCount(1)
    await row(page, 1).click()
    await expect(page.getByTestId('line-offer')).toHaveCount(1)
    await row(page, 1).click()
    await expect(page.getByTestId('line-offer')).toHaveCount(0)
  })

  test('with courses on the course picker is an action of the selected line', async ({ page }) => {
    await toOrder(page, [line(1), line(2)], {
      extra: { get_course_settings: () => ({ enabled: true, category_courses: [] }) },
    })
    await expect(page.getByTestId('course-chip-1')).toHaveCount(0)
    await row(page, 1).click()
    await expect(page.getByTestId('course-chip-1')).toBeVisible()
  })

  test('a gift voucher has no stepper, note or course; a weighing no stepper', async ({ page }) => {
    const voucher = line(1, { department_id: 'GV', name: 'Bon cadeau', kitchen_state: 'printed' })
    const weighed = line(2, { weight_grams: 278, unit_price_cents: 4500 })
    await toOrder(page, [voucher, weighed], {
      extra: { get_course_settings: () => ({ enabled: true, category_courses: [] }) },
    })
    await row(page, 1).click()
    await expect(page.locator('.qty-btn')).toHaveCount(0)
    await expect(page.getByTestId('line-note')).toHaveCount(0)
    await expect(page.getByTestId('course-chip-1')).toHaveCount(0)
    await expect(page.getByTestId('line-remove')).toBeVisible()
    await row(page, 2).click()
    await expect(page.locator('.qty-btn')).toHaveCount(0)
    await expect(page.getByTestId('line-per-kilo')).toHaveText('45,00 €/kg')
  })

  test('every action button is at least 48 px high', async ({ page }) => {
    await toOrder(page, [line(1)])
    for (const b of await page.getByTestId('line-detail').getByRole('button').all()) {
      const box = await b.boundingBox()
      expect(box?.height).toBeGreaterThanOrEqual(48)
    }
  })

  test('the detail repeats what the row cuts: the category and the whole note', async ({
    page,
  }) => {
    await toOrder(page, [
      line(1, { category_fr: 'Accompagnements', kitchen_note: 'sans wasabi, bien cuit' }),
    ])
    await expect(page.getByTestId('line-sub')).toHaveText('Accompagnements')
    await expect(page.getByTestId('line-kitchen-note-full')).toHaveText('sans wasabi, bien cuit')
  })
})
