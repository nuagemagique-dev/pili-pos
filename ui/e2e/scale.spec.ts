import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  ipcCalls,
  settingsWith,
  tapUserTile,
  live,
  DEFAULT_USER,
  DEFAULT_ORDER,
  DEFAULT_ORDER_LINE,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
} from './fixtures/mock-ipc'

// A product sold by weight: the tap opens the weighing, the engine answers
// with the line (crates/pos-engine/src/commands/scale.rs), and the cart shows
// the weight, the price per kilo and the scale's amount with no +/-.

const SALMON = {
  ...DEFAULT_MENU_ITEM,
  id: 7,
  name: 'Saumon au poids',
  name_fr: 'Saumon au poids',
  price_cents: 4500,
  sold_by_weight: true,
}

const WEIGHED_LINE = {
  ...DEFAULT_ORDER_LINE,
  id: 70,
  menu_item_id: 7,
  name: 'Saumon au poids',
  quantity: 1,
  unit_price_cents: 4500,
  weight_grams: 278,
  weighed_amount_cents: 1251,
}

async function openOrder(page: Page, overrides: Record<string, unknown>) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_order: () => DEFAULT_ORDER,
    list_cart_lines: () => [],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [SALMON],
    ...(overrides as Record<string, () => unknown>),
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/u)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/u)
}

test.describe('Sale by weight', () => {
  test('the button shows a price per kilo', async ({ page }) => {
    await openOrder(page, { get_settings: settingsWith({ scaleConfigured: true }) })
    await expect(page.getByTestId('item-price-per-kilo')).toHaveText('45,00 €/kg')
  })

  test('a tap weighs, and the line carries the weight with no quantity controls', async ({
    page,
  }) => {
    await openOrder(page, {
      get_settings: settingsWith({ scaleConfigured: true }),
      scale_start_weighing: () => WEIGHED_LINE,
    })
    await page.getByRole('button', { name: /Saumon au poids/u }).click()

    await expect(page.getByTestId('line-weight')).toHaveText('0,278 kg')
    await expect(page.getByTestId('line-per-kilo')).toHaveText('45,00 €/kg')
    await expect(page.locator('.line-price')).toHaveText('12,51')
    await expect(page.locator('.qty-btn')).toHaveCount(0)
    await expect(page.getByTestId('weigh-dialog')).toHaveCount(0)

    const calls = await ipcCalls(page, 'scale_start_weighing')
    expect(calls).toHaveLength(1)
    expect(calls[0].input.menu_item_id).toBe(7)
  })

  test('the dialog stays up during the weighing and cancels it', async ({ page }) => {
    await openOrder(page, {
      get_settings: settingsWith({ scaleConfigured: true }),
      scale_start_weighing: live(
        () =>
          new Promise((_, reject) => {
            ;(window as any).__cancelWeighing = () => {
              reject('scale_cancelled')
            }
          }),
      ),
      scale_cancel_weighing: live(() => {
        ;(window as any).__cancelWeighing?.()
        return null
      }),
    })
    await page.getByRole('button', { name: /Saumon au poids/u }).click()

    await expect(page.getByTestId('weigh-stage')).toHaveText('Posez le produit sur la balance')
    await page.getByTestId('weigh-cancel').click()

    await expect(page.getByTestId('weigh-dialog')).toHaveCount(0)
    await expect(page.getByText('Pesée annulée.')).toBeVisible()
    await expect(page.getByTestId('line-weight')).toHaveCount(0)
  })

  test('a refusal of the scale adds nothing', async ({ page }) => {
    await openOrder(page, {
      get_settings: settingsWith({ scaleConfigured: true }),
      scale_start_weighing: live(() => Promise.reject('scale_overload')),
    })
    await page.getByRole('button', { name: /Saumon au poids/u }).click()

    await expect(page.getByText('La balance indique une surcharge', { exact: false })).toBeVisible()
    await expect(page.getByTestId('line-weight')).toHaveCount(0)
  })

  test('without a scale the product stays off sale', async ({ page }) => {
    await openOrder(page, {})
    await page.getByRole('button', { name: /Saumon au poids/u }).click()

    await expect(page.getByText("Aucune balance n'est configurée", { exact: false })).toBeVisible()
    expect(await ipcCalls(page, 'scale_start_weighing')).toEqual([])
  })

  test('the header light follows the scale', async ({ page }) => {
    await openOrder(page, {
      get_scale_status: () => ({ configured: true, connected: true, busy: false, check: 'ok' }),
    })
    await expect(page.getByTestId('light-scale')).toHaveClass(/ok/u)
  })

  test('no light without a scale', async ({ page }) => {
    await openOrder(page, {})
    await expect(page.getByTestId('light-terminal')).toHaveCount(0)
    await expect(page.getByTestId('light-scale')).toHaveCount(0)
  })

  // A malatang bowl: the options (its broth) are picked first, then the bowl
  // is weighed with them; the paid broth is a line of its own under the bowl.
  const BOWL = {
    ...SALMON,
    id: 8,
    name: 'Bol malatang',
    name_fr: 'Bol malatang',
    price_cents: 1990,
    has_modifiers: true,
  }
  const BROTHS = [
    {
      id: 70,
      name: 'Bouillon',
      min: 1,
      max: 1,
      options: [
        { id: 701, name: 'Bouillon tomate', price_cents: 150 },
        { id: 702, name: 'Bouillon clair', price_cents: 0 },
      ],
    },
  ]

  test('a bowl asks for its broth, then weighs with it', async ({ page }) => {
    await openOrder(page, {
      get_settings: settingsWith({ scaleConfigured: true }),
      get_menu_items: () => [BOWL],
      get_item_modifiers: () => BROTHS,
      scale_start_weighing: () => ({ ...WEIGHED_LINE, id: 80, menu_item_id: 8 }),
    })
    await page.getByRole('button', { name: /Bol malatang/u }).click()

    const confirm = page.getByTestId('modifier-confirm')
    await expect(confirm).toBeDisabled()
    expect(await ipcCalls(page, 'scale_start_weighing')).toEqual([])
    await page.getByRole('button', { name: /Bouillon tomate/u }).click()
    await expect(confirm).toHaveText('Peser · 19,90 € / kg + 1,50 €')
    await confirm.click()

    await expect(page.getByTestId('modifier-dialog')).toHaveCount(0)
    const calls = await ipcCalls(page, 'scale_start_weighing')
    expect(calls).toHaveLength(1)
    expect(calls[0].input.menu_item_id).toBe(8)
    expect(calls[0].input.modifier_ids).toEqual([701])
    expect(await ipcCalls(page, 'add_order_line_with_modifiers')).toEqual([])
  })

  test('the broth sits under its bowl with no control of its own', async ({ page }) => {
    const bowl = {
      ...WEIGHED_LINE,
      id: 80,
      menu_item_id: 8,
      name: 'Bol malatang',
      unit_price_cents: 1990,
      weight_grams: 612,
      weighed_amount_cents: 1218,
      modifiers_json: '[{"id":701,"name":"Bouillon tomate","price_cents":150}]',
      bowl_options: ['Bouillon tomate', 'Piquant moyen'],
    }
    const broth = {
      ...DEFAULT_ORDER_LINE,
      id: 81,
      menu_item_id: 8,
      name: 'Bouillon tomate',
      name_fr: null,
      quantity: 1,
      unit_price_cents: 150,
      bowl_line_id: 80,
    }
    await openOrder(page, {
      get_settings: settingsWith({ scaleConfigured: true }),
      get_menu_items: () => [BOWL],
      list_cart_lines: () => [bowl, broth],
    })

    await expect(page.getByTestId('line-bowl-options')).toHaveText(
      'Bouillon tomate · Piquant moyen',
    )
    const brothRow = page.locator('.order-line.bowl-option')
    await expect(brothRow).toHaveCount(1)
    await expect(brothRow.locator('.line-price')).toHaveText('1,50')
    await expect(brothRow.locator('.remove-btn')).toHaveCount(0)
    await expect(brothRow.locator('.qty-btn')).toHaveCount(0)
    await expect(page.locator('.order-line .remove-btn')).toHaveCount(1)
  })
})
