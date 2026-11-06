import { describe, expect, it } from 'vite-plus/test'
import type { Category } from '../types'
import { defaultTab, FAVOURITES_TAB, keepTab } from './favourites'

const cat = (id: number): Category => ({
  id,
  name: `C${id}`,
  sort_order: id,
  color: null,
  active: true,
})
const categories = [cat(4), cat(7)]

describe('the favourites tab id', () => {
  it('is not the id of any category', () => {
    expect(FAVOURITES_TAB).toBeLessThan(0)
  })
})

describe('defaultTab', () => {
  it('opens the favourites when there are some', () => {
    expect(defaultTab(categories, 3)).toBe(FAVOURITES_TAB)
  })
  it('opens the first category when there are none', () => {
    expect(defaultTab(categories, 0)).toBe(4)
  })
  it('opens nothing when there is no menu at all', () => {
    expect(defaultTab([], 0)).toBeNull()
  })
})

describe('keepTab', () => {
  it('takes the default tab when nothing was selected', () => {
    expect(keepTab(null, categories, 2)).toBe(FAVOURITES_TAB)
    expect(keepTab(null, categories, 0)).toBe(4)
  })
  it('keeps a category', () => {
    expect(keepTab(7, categories, 5)).toBe(7)
  })
  it('keeps the favourites while they have products', () => {
    expect(keepTab(FAVOURITES_TAB, categories, 1)).toBe(FAVOURITES_TAB)
  })
  it('leaves empty favourites for the first category', () => {
    expect(keepTab(FAVOURITES_TAB, categories, 0)).toBe(4)
    expect(keepTab(FAVOURITES_TAB, [], 0)).toBeNull()
  })
})
