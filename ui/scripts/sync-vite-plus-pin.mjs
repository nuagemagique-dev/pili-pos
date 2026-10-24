// Moves `overrides.vite` to the vite-plus release ui/package.json declares (run by the Dependabot Vite+ workflow,
// .github/workflows/dependabot-vite-plus.yml). Prints `changed` or `unchanged`.
import { readFileSync, writeFileSync } from 'node:fs'
import { syncedPin } from './vite-plus-pin.mjs'

const path = new URL('../package.json', import.meta.url)
const text = readFileSync(path, 'utf8')
const synced = syncedPin(text)
if (synced === text) {
  console.log('unchanged')
} else {
  writeFileSync(path, synced)
  console.log('changed')
}
