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
