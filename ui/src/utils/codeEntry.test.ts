import { describe, expect, it } from 'vite-plus/test'
import type { MenuItem } from '../types'
import {
  CODE_ENTRY_MAX,
  codeLetters,
  findByCode,
  parseCodeEntry,
  pickByCode,
  searchByCode,
  takesQuantity,
  typeKey,
} from './codeEntry'

function item(id: number, code: string | null, extra: Partial<MenuItem> = {}): MenuItem {
  return {
    id,
    category_id: 1,
    name: `P${id}`,
    code,
    price_cents: 450,
    vat_code: 'B',
    vat_code_takeaway: null,
    open_price: false,
    kitchen_printer: true,
    sort_order: id,
    active: true,
    ...extra,
  }
}

describe('parseCodeEntry', () => {
  it('reads a plain code as one unit, any case and blanks', () => {
    expect(parseCodeEntry('A12')).toEqual({ kind: 'add', code: 'A12', quantity: 1 })
    expect(parseCodeEntry(' a 12 ')).toEqual({ kind: 'add', code: 'A12', quantity: 1 })
    expect(parseCodeEntry('105')).toEqual({ kind: 'add', code: '105', quantity: 1 })
  })

  it('takes the number before the multiply sign as the quantity', () => {
    expect(parseCodeEntry('3*A12')).toEqual({ kind: 'add', code: 'A12', quantity: 3 })
    expect(parseCodeEntry('12*3')).toEqual({ kind: 'add', code: '3', quantity: 12 })
  })

  it('takes a quantity after a code that is not only digits', () => {
    expect(parseCodeEntry('A12*3')).toEqual({ kind: 'add', code: 'A12', quantity: 3 })
  })

  it('reads a lone multiplier as waiting for a product', () => {
    expect(parseCodeEntry('4*')).toEqual({ kind: 'multiplier', quantity: 4 })
  })

  it('refuses a zero or three-digit quantity and stray signs', () => {
    expect(parseCodeEntry('0*A1')).toEqual({ kind: 'invalid' })
    expect(parseCodeEntry('100*A1')).toEqual({ kind: 'invalid' })
    expect(parseCodeEntry('A1*0')).toEqual({ kind: 'invalid' })
    expect(parseCodeEntry('-')).toEqual({ kind: 'invalid' })
    expect(parseCodeEntry('*A1')).toEqual({ kind: 'invalid' })
    expect(parseCodeEntry('A1*B2')).toEqual({ kind: 'invalid' })
    expect(parseCodeEntry('-A1')).toEqual({ kind: 'invalid' })
  })

  it('reads an empty text as empty', () => {
    expect(parseCodeEntry('')).toEqual({ kind: 'empty' })
    expect(parseCodeEntry('  ')).toEqual({ kind: 'empty' })
  })
})

describe('typeKey', () => {
  it('appends digits and letters in upper case', () => {
    expect(typeKey('', 'a')).toBe('A')
    expect(typeKey('A', '1')).toBe('A1')
  })

  it('takes one multiply sign, never first', () => {
    expect(typeKey('', '*')).toBe('')
    expect(typeKey('3', '*')).toBe('3*')
    expect(typeKey('3*', '*')).toBe('3*')
  })

  it('ignores other keys and stops at the longest text', () => {
    expect(typeKey('A', '.')).toBe('A')
    expect(typeKey('A', '-')).toBe('A')
    const full = 'A'.repeat(CODE_ENTRY_MAX)
    expect(typeKey(full, '1')).toBe(full)
  })
})

describe('findByCode', () => {
  const items = [item(1, 'A12'), item(2, 'b3'), item(3, null), item(4, 'C1'), item(5, 'c1')]

  it('finds a product by its code in any case', () => {
    expect(findByCode(items, 'a12')).toEqual({ kind: 'found', item: items[0] })
    expect(findByCode(items, 'B3')).toEqual({ kind: 'found', item: items[1] })
  })

  it('tells an unknown code and a code two products share', () => {
    expect(findByCode(items, 'Z9')).toEqual({ kind: 'unknown' })
    expect(findByCode(items, 'C1')).toEqual({ kind: 'ambiguous', items: [items[3], items[4]] })
  })

  it('never matches a prefix', () => {
    expect(findByCode(items, 'A1')).toEqual({ kind: 'unknown' })
  })
})

describe('codeLetters', () => {
  it('lists the letters of the codes once, sorted', () => {
    expect(
      codeLetters([item(1, 'B2'), item(2, 'a1'), item(3, '105'), item(4, null), item(5, 'AB')]),
    ).toEqual(['A', 'B'])
  })
})

describe('takesQuantity', () => {
  it('refuses weighed products and products with options', () => {
    expect(takesQuantity(item(1, 'A1'))).toBe(true)
    expect(takesQuantity(item(1, 'A1', { sold_by_weight: true }))).toBe(false)
    expect(takesQuantity(item(1, 'A1', { has_modifiers: true }))).toBe(false)
  })
})

describe('searchByCode and pickByCode', () => {
  const items = [
    item(1, 'A10'),
    item(2, 'A2'),
    item(3, 'A1'),
    item(4, 'B1'),
    item(5, null),
    item(6, 'A12'),
  ]

  it('lists the codes that start with the text, exact first then natural order', () => {
    expect(searchByCode(items, 'a1').map((i) => i.code)).toEqual(['A1', 'A10', 'A12'])
    expect(searchByCode(items, 'A').map((i) => i.code)).toEqual(['A1', 'A2', 'A10', 'A12'])
    expect(searchByCode(items, '')).toEqual([])
  })

  it('takes the exact code, else the only product starting with the text', () => {
    expect(pickByCode(items, 'A1')).toEqual({ kind: 'found', item: items[2] })
    expect(pickByCode(items, 'B')).toEqual({ kind: 'found', item: items[3] })
    expect(pickByCode(items, 'A')).toEqual({ kind: 'unknown' })
    expect(pickByCode(items, 'Z')).toEqual({ kind: 'unknown' })
  })
})
