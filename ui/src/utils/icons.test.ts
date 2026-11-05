import { describe, expect, it } from 'vite-plus/test'
import { ICONS, pathOf } from './icons'

describe('icons', () => {
  it('reads the path data of an icon file', () => {
    expect(pathOf('<svg viewBox="0 0 24 24"><path d="M1 2h3z"/></svg>')).toBe('M1 2h3z')
  })

  it('refuses a file without path data', () => {
    expect(() => pathOf('<svg viewBox="0 0 24 24"></svg>')).toThrow('icon without path data')
  })

  it('has real path data for every icon of the toolbar', () => {
    for (const d of Object.values(ICONS)) {
      expect(d.length).toBeGreaterThan(40)
      expect(d.startsWith('M')).toBe(true)
    }
  })
})
