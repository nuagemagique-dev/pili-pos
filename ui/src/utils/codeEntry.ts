import type { MenuItem } from '../types'

/**
 * Order entry by menu code: the operator types the code printed on the
 * menu ("A12", "105"), the habit of the tills most Asian restaurants used
 * before. The grammar:
 *
 *   A12      one A12
 *   3*A12    three A12 (the number before × is the quantity)
 *   A12*3    three A12
 *   3*       three of the next product, typed or tapped in the grid
 *
 * While a code is typed the grid shows the products whose code starts
 * with it ([searchByCode]); removing a product is the cart's job.
 *
 * Pure: the till's rules (sold out, options, weighing) stay with the
 * screen. The Compose UI has the same grammar (`CodeEntry.kt`).
 */
export type CodeEntry =
  | { kind: 'empty' }
  | { kind: 'multiplier'; quantity: number }
  | { kind: 'add'; code: string; quantity: number }
  | { kind: 'invalid' }

/** How the line under the field reads: a hint, the product OK adds, a refusal, what was done. */
export type CodePadTone = 'hint' | 'match' | 'error' | 'done'

/** Longest text the pad takes. */
export const CODE_ENTRY_MAX = 12

/** Largest quantity a multiplier gives (two digits). */
export const CODE_QUANTITY_MAX = 99

/** The multiply sign as it is kept in the typed text. */
export const TIMES = '*'

const CODE = '[A-Z0-9]+'
const QTY = '\\d{1,2}'
const MULTIPLIER = new RegExp(`^(${QTY})\\*$`, 'u')
const QTY_FIRST = new RegExp(`^(${QTY})\\*(${CODE})$`, 'u')
const CODE_FIRST = new RegExp(`^(${CODE})\\*(${QTY})$`, 'u')
const PLAIN = new RegExp(`^(${CODE})$`, 'u')

/** A code as it is compared: no blanks, upper case. */
export function normalizeCode(code: string): string {
  return code.replace(/\s+/gu, '').toUpperCase()
}

function quantity(digits: string): number | null {
  const n = Number(digits)
  return n >= 1 && n <= CODE_QUANTITY_MAX ? n : null
}

/** What the typed text asks for. */
export function parseCodeEntry(text: string): CodeEntry {
  const s = normalizeCode(text)
  if (s === '') return { kind: 'empty' }
  let m = MULTIPLIER.exec(s)
  if (m) {
    const q = quantity(m[1])
    return q === null ? { kind: 'invalid' } : { kind: 'multiplier', quantity: q }
  }
  m = QTY_FIRST.exec(s)
  if (m) {
    const q = quantity(m[1])
    return q === null ? { kind: 'invalid' } : { kind: 'add', code: m[2], quantity: q }
  }
  m = CODE_FIRST.exec(s)
  if (m) {
    const q = quantity(m[2])
    return q === null ? { kind: 'invalid' } : { kind: 'add', code: m[1], quantity: q }
  }
  m = PLAIN.exec(s)
  if (m) return { kind: 'add', code: m[1], quantity: 1 }
  return { kind: 'invalid' }
}

/** The code being typed, else null. */
export function typedCode(entry: CodeEntry): string | null {
  return entry.kind === 'add' ? entry.code : null
}

/**
 * The text after one key of the pad: a digit, a letter or `*`. A key that
 * cannot make a valid entry is ignored: a second multiply sign, one on an
 * empty text, or more than [CODE_ENTRY_MAX] characters.
 */
export function typeKey(text: string, key: string): string {
  const k = key.toUpperCase()
  if (text.length >= CODE_ENTRY_MAX) return text
  if (k === TIMES) return text === '' || text.includes(TIMES) ? text : text + TIMES
  return /^[A-Z0-9]$/u.test(k) ? text + k : text
}

/** The product a code names. */
export type CodeMatch =
  | { kind: 'found'; item: MenuItem }
  | { kind: 'unknown' }
  | { kind: 'ambiguous'; items: MenuItem[] }

/** Looks [code] up among the products that have one (exact, any case). */
export function findByCode(items: readonly MenuItem[], code: string): CodeMatch {
  const wanted = normalizeCode(code)
  const found = items.filter(
    (i) => i.code !== null && i.code !== undefined && normalizeCode(i.code) === wanted,
  )
  if (found.length === 0) return { kind: 'unknown' }
  if (found.length > 1) return { kind: 'ambiguous', items: found }
  return { kind: 'found', item: found[0] }
}

const naturalOrder = new Intl.Collator('fr', { numeric: true, sensitivity: 'base' })

/**
 * The products whose code starts with [code], for the grid while a code is
 * typed: the shorter codes first (the exact one leads), then in natural
 * order (A2 before A10).
 */
export function searchByCode(items: readonly MenuItem[], code: string): MenuItem[] {
  const wanted = normalizeCode(code)
  if (wanted === '') return []
  return items
    .filter((i) => normalizeCode(i.code ?? '').startsWith(wanted))
    .sort((a, b) => {
      const ca = normalizeCode(a.code ?? '')
      const cb = normalizeCode(b.code ?? '')
      return ca.length - cb.length || naturalOrder.compare(ca, cb)
    })
}

/**
 * The product OK takes for [code]: the exact code, else the only product
 * whose code starts with it. Several products under the exact code stay
 * ambiguous; nothing found, or several starting with it, is unknown.
 */
export function pickByCode(items: readonly MenuItem[], code: string): CodeMatch {
  const exact = findByCode(items, code)
  if (exact.kind !== 'unknown') return exact
  const results = searchByCode(items, code)
  return results.length === 1 ? { kind: 'found', item: results[0] } : { kind: 'unknown' }
}

/** The letters the menu codes use, sorted: the letter keys of the pad. */
export function codeLetters(items: readonly MenuItem[]): string[] {
  const letters = new Set<string>()
  for (const i of items) {
    for (const ch of normalizeCode(i.code ?? '')) {
      if (ch >= 'A' && ch <= 'Z') letters.add(ch)
    }
  }
  return [...letters].sort()
}

/** A product added through a multiplier must take a plain quantity: not weighed, no options. */
export function takesQuantity(item: MenuItem): boolean {
  return item.sold_by_weight !== true && item.has_modifiers !== true
}
