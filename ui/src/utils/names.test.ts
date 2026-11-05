import { describe, expect, it } from 'vite-plus/test'
import { lineCategory } from './names'

describe('lineCategory', () => {
  const line = { category_fr: 'California roll', category_zh: '加州卷' }

  it('names the category in the UI language', () => {
    expect(lineCategory(line, 'fr')).toBe('California roll')
    expect(lineCategory(line, 'en')).toBe('California roll')
    expect(lineCategory(line, 'zh')).toBe('加州卷')
  })

  it('falls back to the other language, else to nothing', () => {
    expect(lineCategory({ category_fr: 'Sushi', category_zh: null }, 'zh')).toBe('Sushi')
    expect(lineCategory({ category_fr: null, category_zh: '寿司' }, 'fr')).toBe('寿司')
    expect(lineCategory({}, 'fr')).toBeNull()
    expect(lineCategory({ category_fr: '  ' }, 'fr')).toBeNull()
  })
})
