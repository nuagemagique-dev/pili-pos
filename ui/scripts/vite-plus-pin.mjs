// The `overrides.vite` alias must name the vite-plus-core release that vite-plus itself depends on. Dependabot bumps
// vite-plus as one group but leaves the override alone, so a bump fails `npm run check` until the pin moves with it.

/**
 * @typedef {object} Manifest
 * @property {string} [version]
 * @property {Record<string, string>} [dependencies]
 * @property {Record<string, string>} [devDependencies]
 * @property {Record<string, string>} [overrides]
 */

/**
 * What disagrees between ui/package.json and the installed vite-plus, empty when the pin holds.
 * @param {Manifest} pkg ui/package.json
 * @param {Manifest} installed node_modules/vite-plus/package.json
 * @returns {string[]}
 */
export function pinProblems(pkg, installed) {
  const declared = pkg.devDependencies?.['vite-plus'] ?? 'missing'
  const override = pkg.overrides?.vite ?? 'missing'
  const expected = `npm:@voidzero-dev/vite-plus-core@${declared}`
  const bundled = installed.dependencies?.vite ?? 'missing'
  const version = installed.version ?? 'missing'

  const problems = []
  if (override !== expected) {
    problems.push(`overrides.vite is ${override}, expected ${expected}`)
  }
  if (version !== declared) {
    problems.push(
      `node_modules has vite-plus ${version}, package.json declares ${declared}: run npm ci`,
    )
  }
  if (bundled !== expected) {
    problems.push(`vite-plus ${version} depends on vite ${bundled}, not ${expected}`)
  }
  return problems
}

/**
 * ui/package.json text with `overrides.vite` moved to the declared vite-plus release, the rest of the file untouched.
 * Returns the text unchanged when the pin already holds.
 * @param {string} text ui/package.json
 * @returns {string}
 */
export function syncedPin(text) {
  /** @type {unknown} */
  const value = JSON.parse(text)
  /** @type {Manifest} */
  const pkg = typeof value === 'object' && value !== null ? value : {}
  const declared = pkg.devDependencies?.['vite-plus']
  const override = pkg.overrides?.vite
  if (declared === undefined || override === undefined) {
    throw new Error('ui/package.json needs devDependencies.vite-plus and overrides.vite')
  }
  const expected = `npm:@voidzero-dev/vite-plus-core@${declared}`
  if (override === expected) {
    return text
  }
  const from = `"vite": ${JSON.stringify(override)}`
  if (text.split(from).length !== 2) {
    throw new Error(`expected one ${from} in ui/package.json`)
  }
  return text.replace(from, `"vite": ${JSON.stringify(expected)}`)
}
