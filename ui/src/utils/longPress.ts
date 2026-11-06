// A long press on a touch target (UX audit T7): the finger stays down for 500 ms without
// moving. A finger that moves further than the slop is a scroll, a lift or a cancel before
// the delay is a plain tap. After a long press fires, the click the browser sends when the
// finger lifts must be ignored: `consumeClick` says so once.
// Android: the 500 ms `LocalViewConfiguration` of `feature/tables/TablesScreen.kt`.

/** How long the finger must stay down, in milliseconds. */
export const LONG_PRESS_MS = 500

/** How far the finger may drift before the press becomes a scroll, in CSS pixels. */
export const LONG_PRESS_SLOP_PX = 12

export interface LongPressTimers {
  /** Runs `run` after `ms`; returns the function that cancels it. */
  set: (run: () => void, ms: number) => () => void
}

export interface LongPressOptions {
  delayMs?: number
  slopPx?: number
  /** Replaceable in tests. */
  timers?: LongPressTimers
}

export interface LongPress {
  /** The finger went down at (x, y). */
  start: (x: number, y: number) => void
  /** The finger moved to (x, y): too far cancels the press. */
  move: (x: number, y: number) => void
  /** The finger lifted or the browser took the gesture: the press is over. */
  cancel: () => void
  /** Forgets the press and any long press waiting for its click (another gesture began). */
  reset: () => void
  /** True once after a long press fired: the click that follows must do nothing. */
  consumeClick: () => boolean
}

const realTimers: LongPressTimers = {
  set: (run, ms) => {
    const id = setTimeout(run, ms)
    return () => {
      clearTimeout(id)
    }
  },
}

export function createLongPress(
  onLongPress: () => void,
  options: LongPressOptions = {},
): LongPress {
  const delayMs = options.delayMs ?? LONG_PRESS_MS
  const slopPx = options.slopPx ?? LONG_PRESS_SLOP_PX
  const timers = options.timers ?? realTimers
  let cancelTimer: (() => void) | null = null
  let origin: { x: number; y: number } | null = null
  let fired = false

  function stop() {
    cancelTimer?.()
    cancelTimer = null
    origin = null
  }

  return {
    start(x, y) {
      stop()
      fired = false
      origin = { x, y }
      cancelTimer = timers.set(() => {
        cancelTimer = null
        origin = null
        fired = true
        onLongPress()
      }, delayMs)
    },
    move(x, y) {
      if (origin === null) return
      if (Math.hypot(x - origin.x, y - origin.y) > slopPx) stop()
    },
    cancel: stop,
    reset() {
      stop()
      fired = false
    },
    consumeClick() {
      const was = fired
      fired = false
      return was
    },
  }
}
