import { describe, expect, it } from 'vite-plus/test'
import { pinProblems, syncedPin } from './vite-plus-pin.mjs'

const pkg = (vitePlus: string, override: string) => ({
  devDependencies: { 'vite-plus': vitePlus },
  overrides: { vite: override },
})
const installed = (version: string, vite: string) => ({ version, dependencies: { vite } })
const core = (version: string) => `npm:@voidzero-dev/vite-plus-core@${version}`

describe('pinProblems', () => {
  it('accepts a pin, override and install that agree', () => {
    expect(pinProblems(pkg('1.4.0', core('1.4.0')), installed('1.4.0', core('1.4.0')))).toEqual([])
  })

  it('flags a Dependabot bump that left the override behind', () => {
    expect(pinProblems(pkg('1.5.0', core('1.4.0')), installed('1.5.0', core('1.5.0')))).toEqual([
      `overrides.vite is ${core('1.4.0')}, expected ${core('1.5.0')}`,
    ])
  })

  it('expects the core release vite-plus depends on, even when its version differs', () => {
    expect(
      pinProblems(pkg('1.7.0', core('8.3.8000')), installed('1.7.0', core('8.3.8000'))),
    ).toEqual([])
    expect(pinProblems(pkg('1.7.0', core('1.7.0')), installed('1.7.0', core('8.3.8000')))).toEqual([
      `overrides.vite is ${core('1.7.0')}, expected ${core('8.3.8000')}`,
    ])
  })

  it('flags node_modules that were not reinstalled after the bump', () => {
    expect(pinProblems(pkg('1.5.0', core('1.5.0')), installed('1.4.0', core('1.4.0')))).toEqual([
      'node_modules has vite-plus 1.4.0, package.json declares 1.5.0: run npm ci',
    ])
  })

  it('flags an installed vite-plus whose vite is not a vite-plus-core release', () => {
    expect(pinProblems(pkg('1.5.0', core('1.5.0')), installed('1.5.0', '^8.0.0'))).toEqual([
      'vite-plus 1.5.0 depends on vite ^8.0.0, not a vite-plus-core release',
    ])
  })

  it('flags a missing override', () => {
    expect(
      pinProblems({ devDependencies: { 'vite-plus': '1.4.0' } }, installed('1.4.0', core('1.4.0'))),
    ).toEqual([`overrides.vite is missing, expected ${core('1.4.0')}`])
  })
})

describe('syncedPin', () => {
  const text = (vitePlus: string, override: string) =>
    `${JSON.stringify(pkg(vitePlus, override), null, 2)}\n`

  it('moves the override to the vite-plus release a Dependabot bump declared', () => {
    expect(syncedPin(text('1.5.0', core('1.4.0')), core('1.5.0'))).toBe(
      text('1.5.0', core('1.5.0')),
    )
  })

  it('leaves a pin that holds untouched', () => {
    const holding = text('1.4.0', core('1.4.0'))
    expect(syncedPin(holding, core('1.4.0'))).toBe(holding)
  })

  it('pins the core release vite-plus depends on, even when its version differs', () => {
    expect(syncedPin(text('1.7.0', core('1.5.0')), core('8.3.8000'))).toBe(
      text('1.7.0', core('8.3.8000')),
    )
  })

  it('refuses a vite dependency that is not a vite-plus-core release', () => {
    for (const spec of [
      '',
      '^8.0.0',
      'npm:vite@8.0.0',
      'npm:@voidzero-dev/vite-plus-core@8 && x',
    ]) {
      expect(() => syncedPin(text('1.7.0', core('1.5.0')), spec)).toThrow(
        'not a vite-plus-core release',
      )
    }
  })

  it('refuses a package.json without the override', () => {
    expect(() =>
      syncedPin(JSON.stringify({ devDependencies: { 'vite-plus': '1.4.0' } }), core('1.4.0')),
    ).toThrow('needs devDependencies.vite-plus and overrides.vite')
  })
})
