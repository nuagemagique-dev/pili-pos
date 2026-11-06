import { onMounted, onUnmounted } from 'vue'
import { useBadgeStore } from '../stores/badge'
import { WedgeDetector } from '../utils/badgeWedge'
import { claimScans } from '../utils/scanClaim'

/**
 * A USB barcode reader in keyboard wedge mode, for the screen that calls
 * this (UX audit P7): one window `keydown` capture listener while the
 * component is mounted. A burst of keys ending with Enter (same detector and
 * timing as the badge reader, the gap is the one of the badge settings) is
 * handed to `onScan` as its token; the keys of the burst never reach the
 * page. Anything that is not a burst is left alone.
 *
 * The scans are claimed meanwhile: the badge reader stands aside, so the
 * reader works with the badge setting off, and never logs anybody in with a
 * voucher barcode.
 */
export function useVoucherScanner(onScan: (token: string) => void) {
  const badge = useBadgeStore()
  const detector = new WedgeDetector({ maxGapMs: 50, minLen: 6, maxLen: 32 })
  let timer: ReturnType<typeof setTimeout> | undefined
  let release: (() => void) | undefined

  function onKeyDown(e: KeyboardEvent) {
    if (e.isComposing) return
    const gap = badge.settings?.wedgeMaxGapMs ?? 50
    detector.configure({ maxGapMs: gap, minLen: 6, maxLen: 32 })
    // Keys that were held and turned out to be typing are dropped: the
    // screen has no field to type them into.
    const r = detector.onKey({
      code: e.code,
      key: e.key,
      at: e.timeStamp,
      repeat: e.repeat,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      meta: e.metaKey,
    })
    if (r.action === 'neutral') return
    clearTimeout(timer)
    timer = undefined
    if (r.action === 'pass') return
    // Held or recognised: the page never sees the key.
    e.preventDefault()
    e.stopImmediatePropagation()
    if (r.action === 'badge') {
      onScan(r.token)
      return
    }
    timer = setTimeout(() => detector.flush(), gap)
  }

  onMounted(() => {
    release = claimScans()
    window.addEventListener('keydown', onKeyDown, true)
  })
  onUnmounted(() => {
    window.removeEventListener('keydown', onKeyDown, true)
    clearTimeout(timer)
    detector.flush()
    release?.()
    release = undefined
  })
}
