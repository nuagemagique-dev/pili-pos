// `npm run check` step: fails when the vite-plus pin and its `overrides.vite` alias disagree (see vite-plus-pin.mjs).
import { readFileSync } from 'node:fs'
import process from 'node:process'
import { pinProblems } from './vite-plus-pin.mjs'

/**
 * @param {string} path relative to this script
 * @returns {import('./vite-plus-pin.mjs').Manifest}
 */
function read(path) {
  /** @type {unknown} */
  const value = JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'))
  return typeof value === 'object' && value !== null ? value : {}
}

const pkg = read('../package.json')
const problems = pinProblems(pkg, read('../node_modules/vite-plus/package.json'))
if (problems.length > 0) {
  for (const problem of problems) {
    console.error(`vite-plus pin: ${problem}`)
  }
  process.exit(1)
}
console.log(
  `vite-plus pin: ${pkg.devDependencies?.['vite-plus'] ?? 'missing'}, override and bundled vite agree`,
)
