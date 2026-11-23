// The `overrides.vite` alias must name the vite-plus-core release that vite-plus itself depends on. Dependabot bumps
// vite-plus as one group but leaves the override alone, so a bump fails `npm run check` until the pin moves with it.

const CORE_SPEC = /^npm:@voidzero-dev\/vite-plus-core@[\w.+-]+$/u

/**
 * @typedef {object} Manifest
 * @property {string} [version]
 * @property {Record<string, string>} [dependencies]
 * @property {Record<string, string>} [devDependencies]
 * @property {Record<string, string>} [overrides]
 */

/**
 * The vite-plus version ui/package.json declares in devDependencies.
 * @param {string} text ui/package.json
 * @returns {string | undefined}
 */
export function declaredVitePlus(text) {
  /** @type {unknown} */
  const value = JSON.parse(text)
  /** @type {Manifest} */
  const pkg = typeof value === 'object' && value !== null ? value : {}
  return pkg.devDependencies?.['vite-plus']
}

/**
 * What disagrees between ui/package.json and the installed vite-plus, empty when the pin holds.
 * @param {Manifest} pkg ui/package.json
 * @param {Manifest} installed node_modules/vite-plus/package.json
 * @returns {string[]}
 */
export function pinProblems(pkg, installed) {
  const declared = pkg.devDependencies?.['vite-plus'] ?? 'missing'
  const override = pkg.overrides?.vite ?? 'missing'
  const version = installed.version ?? 'missing'
  if (version !== declared) {
    // The installed vite-plus is not the declared one, so its `vite` dependency says nothing yet.
    return [`node_modules has vite-plus ${version}, package.json declares ${declared}: run npm ci`]
  }
  // Since vite-plus 1.7.0 (core 8.3.8000) the core version no longer follows vite-plus: the truth is the `vite`
  // dependency the installed vite-plus publishes.
  const expected = installed.dependencies?.vite ?? 'missing'
  if (!CORE_SPEC.test(expected)) {
    return [`vite-plus ${version} depends on vite ${expected}, not a vite-plus-core release`]
  }
  return override === expected ? [] : [`overrides.vite is ${override}, expected ${expected}`]
}

/**
 * ui/package.json text with `overrides.vite` moved to the vite-plus-core release the declared vite-plus depends on, the
 * rest of the file untouched. Returns the text unchanged when the pin already holds.
 * @param {string} text ui/package.json
 * @param {string} coreSpec the `vite` dependency the declared vite-plus release publishes
 *   (`npm view vite-plus@<version> dependencies.vite`), e.g. `npm:@voidzero-dev/vite-plus-core@8.3.8000`
 * @returns {string}
 */
export function syncedPin(text, coreSpec) {
  /** @type {unknown} */
  const value = JSON.parse(text)
  /** @type {Manifest} */
  const pkg = typeof value === 'object' && value !== null ? value : {}
  const declared = pkg.devDependencies?.['vite-plus']
  const override = pkg.overrides?.vite
  if (declared === undefined || override === undefined) {
    throw new Error('ui/package.json needs devDependencies.vite-plus and overrides.vite')
  }
  if (!CORE_SPEC.test(coreSpec)) {
    throw new Error(
      `vite-plus@${declared} declares vite as ${JSON.stringify(coreSpec)}, not a vite-plus-core release`,
    )
  }
  const expected = coreSpec
  if (override === expected) {
    return text
  }
  const from = `"vite": ${JSON.stringify(override)}`
  if (text.split(from).length !== 2) {
    throw new Error(`expected one ${from} in ui/package.json`)
  }
  return text.replace(from, `"vite": ${JSON.stringify(expected)}`)
}
