import { describe, expect, it } from 'vite-plus/test'
import { pinProblems } from './vite-plus-pin.mjs'

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

  it('flags node_modules that were not reinstalled after the bump', () => {
    expect(pinProblems(pkg('1.5.0', core('1.5.0')), installed('1.4.0', core('1.4.0')))).toEqual([
      'node_modules has vite-plus 1.4.0, package.json declares 1.5.0: run npm ci',
      `vite-plus 1.4.0 depends on vite ${core('1.4.0')}, not ${core('1.5.0')}`,
    ])
  })

  it('flags a missing override', () => {
    expect(
      pinProblems({ devDependencies: { 'vite-plus': '1.4.0' } }, installed('1.4.0', core('1.4.0'))),
    ).toEqual([`overrides.vite is missing, expected ${core('1.4.0')}`])
  })
})
