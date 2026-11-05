import { usePosStore } from '../stores/pos'

/** Something with a name in the UI language and, when known, its
 * French and Chinese names (categories, products, cart lines). */
export interface Named {
  name: string
  name_fr?: string | null
  name_zh?: string | null
  /** Menu code ("A2"): products and cart lines from a catalog that has one. */
  code?: string | null
}

export interface BilingualName {
  primary: string
  /** The other language, smaller underneath; null when it adds nothing. */
  secondary: string | null
  /** `primary` is the menu code (Settings: products shown by code). */
  isCode: boolean
}

/**
 * Product names are shown in the UI language only (user decision
 * 2026-10-10: no second language underneath).
 *
 * With products shown by code, a product that has one leads with it
 * and keeps its name in the UI language underneath.
 */
export function useBilingualName() {
  const pos = usePosStore()

  return function bilingual(item: Named): BilingualName {
    const zh = pos.language === 'zh'
    const primary = zh ? (item.name_zh ?? item.name) : item.name
    const code = item.code?.trim()
    if (pos.productLabel === 'code' && code !== undefined && code !== '') {
      return { primary: code, secondary: primary, isCode: true }
    }
    return { primary, secondary: null, isCode: false }
  }
}

/** The category of a cart line's product in the UI language, the other
 * one when the catalog lacks it; null when the catalog names neither. */
export function lineCategory(
  line: { category_fr?: string | null; category_zh?: string | null },
  language: string,
): string | null {
  const [first, second] =
    language === 'zh' ? [line.category_zh, line.category_fr] : [line.category_fr, line.category_zh]
  const name = (first ?? second)?.trim()
  return name === undefined || name === '' ? null : name
}
