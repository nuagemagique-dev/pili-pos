// Products with options (HubRise builds only). Imported by the option
// picker alone, so the certification bundle carries none of it.
import { invoke } from '@tauri-apps/api/core'
import type { OrderLine } from '../types'

export interface ModifierOption {
  id: number
  name: string
  price_cents: number
  /** Allergen codes the option contains. */
  allergens?: string[]
  /** Allergen codes it may contain as traces. */
  traces?: string[]
  /** The option's list is complete (possibly empty); false: no information. */
  allergens_declared?: boolean
}

export interface ModifierGroup {
  id: number
  name: string
  min: number
  /** `null`: no maximum. */
  max: number | null
  options: ModifierOption[]
}

export function getItemModifiers(menuItemId: number, lang?: string): Promise<ModifierGroup[]> {
  return invoke('get_item_modifiers', { menuItemId, lang })
}

/** The backend prices the options and names the line; only ids go up. */
export function addOrderLineWithModifiers(
  orderId: number,
  menuItemId: number,
  quantity: number,
  modifierIds: number[],
): Promise<OrderLine> {
  return invoke('add_order_line_with_modifiers', {
    input: { order_id: orderId, menu_item_id: menuItemId, quantity, modifier_ids: modifierIds },
  })
}
