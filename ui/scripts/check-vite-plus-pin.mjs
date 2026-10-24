// The `overrides.vite` alias must name the vite-plus-core release that vite-plus itself depends on. Dependabot bumps
// vite-plus as one group but leaves the override alone, so a bump fails here until the pin is moved with it.
import { readFileSync } from 'node:fs'

const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'))

const pkg = read('../package.json')
const installed = read('../node_modules/vite-plus/package.json')

const declared = pkg.devDependencies['vite-plus']
const override = pkg.overrides?.vite
const expected = `npm:@voidzero-dev/vite-plus-core@${declared}`
const bundled = installed.dependencies.vite

const problems = []
if (override !== expected) {
  problems.push(`overrides.vite is ${override ?? 'missing'}, expected ${expected}`)
}
if (installed.version !== declared) {
  problems.push(
    `node_modules has vite-plus ${installed.version}, package.json declares ${declared}: run npm ci`,
  )
}
if (bundled !== expected) {
  problems.push(
    `vite-plus ${installed.version} depends on vite ${bundled}, the override says ${override}`,
  )
}

if (problems.length > 0) {
  for (const problem of problems) {
    console.error(`vite-plus pin: ${problem}`)
  }
  process.exit(1)
}
console.log(`vite-plus pin: ${declared}, override and bundled vite agree`)
