import { onUnmounted, watch } from 'vue'
import { useBadgeStore } from '../stores/badge'
import { WedgeDetector } from '../utils/badgeWedge'

/** Type `text` into the focused text field, at the caret, as the keys
 * would have: a held key that turned out to be typing is replayed here.
 * Nothing focused: dropped, the keys would have gone nowhere. */
export function replayInto(target: Element | null, text: string) {
  if (!text) return
  if (!(target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement)) return
  if (target.readOnly || target.disabled) return
  let insert = text
  if (target.maxLength >= 0) {
    const room = target.maxLength - target.value.length
    insert = text.slice(0, Math.max(0, room))
    if (!insert) return
  }
  try {
    target.setRangeText(
      insert,
      target.selectionStart ?? target.value.length,
      target.selectionEnd ?? target.value.length,
      'end',
    )
  } catch {
    // Input types without a selection (number, email): append.
    target.value += insert
  }
  target.dispatchEvent(new Event('input', { bubbles: true }))
}

/**
 * The USB wedge badge reader (plan L5-D12): one window `keydown` capture
 * listener, installed only while the reader is `usb`, registered before any
 * other keyboard listener of the app. It holds the keys of a possible badge
 * (`preventDefault` + `stopImmediatePropagation`, so the scan is neither
 * typed nor user activity), hands a recognised badge to the badge store and
 * replays everything else in order.
 *
 * `onReplay` tells the caller that held keys turned out to be typing, which
 * is user activity after all (they were swallowed before the activity listener).
 */
export function useBadgeReader(onReplay?: () => void) {
  const badge = useBadgeStore()
  const detector = new WedgeDetector({ maxGapMs: 50, minLen: 6, maxLen: 32 })
  let timer: ReturnType<typeof setTimeout> | undefined
  let focusAtHold: Element | null = null

  function release() {
    clearTimeout(timer)
    timer = undefined
    const text = detector.flush()
    if (text) {
      replayInto(focusAtHold ?? document.activeElement, text)
      onReplay?.()
    }
    focusAtHold = null
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.isComposing) return
    const r = detector.onKey({
      code: e.code,
      key: e.key,
      at: e.timeStamp,
      repeat: e.repeat,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      meta: e.metaKey,
    })
    if (r.replay) {
      replayInto(focusAtHold ?? document.activeElement, r.replay)
      focusAtHold = null
      onReplay?.()
    }
    // Modifiers and repeats change nothing, the gap timer keeps running.
    if (r.action === 'neutral') return
    if (r.action === 'pass') {
      clearTimeout(timer)
      timer = undefined
      focusAtHold = null
      return
    }
    // Held or recognised: the app never sees the key.
    e.preventDefault()
    e.stopImmediatePropagation()
    clearTimeout(timer)
    timer = undefined
    if (r.action === 'badge') {
      focusAtHold = null
      void badge.onScan(r.token)
      return
    }
    if (!focusAtHold) focusAtHold = document.activeElement
    timer = setTimeout(release, badge.settings?.wedgeMaxGapMs ?? 50)
  }

  let installed = false
  function install() {
    if (installed) return
    installed = true
    window.addEventListener('keydown', onKeyDown, true)
  }
  function uninstall() {
    if (!installed) return
    installed = false
    window.removeEventListener('keydown', onKeyDown, true)
    release()
  }

  watch(
    () => badge.settings,
    (s) => {
      if (s)
        detector.configure({
          maxGapMs: s.wedgeMaxGapMs,
          minLen: s.wedgeMinLen,
          maxLen: s.wedgeMaxLen,
        })
      if (s?.reader === 'usb') install()
      else uninstall()
    },
    { immediate: true, deep: true },
  )

  onUnmounted(uninstall)
}
