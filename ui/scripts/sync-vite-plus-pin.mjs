// Moves `overrides.vite` to the vite-plus-core release the vite-plus of ui/package.json depends on (run by the Dependabot
// Vite+ workflow, .github/workflows/dependabot-vite-plus.yml). Prints `changed` or `unchanged`.
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { declaredVitePlus, syncedPin } from './vite-plus-pin.mjs'

const path = new URL('../package.json', import.meta.url)
const text = readFileSync(path, 'utf8')
const declared = declaredVitePlus(text)
if (declared === undefined) {
  throw new Error('ui/package.json needs devDependencies.vite-plus')
}
// The vite-plus-core release that vite-plus depends on, as published (its version differs since vite-plus 1.7.0).
// Run outside the repo so no project setting of npm applies.
const coreSpec = execFileSync('npm', ['view', `vite-plus@${declared}`, 'dependencies.vite'], {
  cwd: tmpdir(),
  encoding: 'utf8',
}).trim()
const synced = syncedPin(text, coreSpec)
if (synced === text) {
  console.log('unchanged')
} else {
  writeFileSync(path, synced)
  console.log('changed')
}
