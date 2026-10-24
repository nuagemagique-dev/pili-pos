// USB "keyboard wedge" badge reader detection (plan L5-D12).
//
// A wedge reader types the badge number and Enter like a very fast
// keyboard. The detector tells such a burst from a person typing:
//
// - It maps PHYSICAL keys (`KeyboardEvent.code`: Digit0..9, Numpad0..9,
//   KeyA..KeyF), never characters, so an AZERTY layout (where the digit
//   row types `&é"'(`) reads the same badge.
// - Shift is neutral (a reader may shift letters); Ctrl, Alt and Meta make a
//   key a shortcut, not a badge key. Modifier key-downs and key repeats
//   change nothing.
// - Every candidate key is HELD (the caller swallows the event). The burst
//   is a badge when it ends with Enter, every gap between consecutive keys
//   is at most `maxGapMs` and its length is within [minLen, maxLen].
//   Anything else is replayed, in order, by the caller (`replay`).
//
// The detector is pure: no DOM, no timer. The caller times the gap
// (`flush()` when `maxGapMs` passed with no key) and applies the results.

export interface WedgeConfig {
  /** Largest gap between two keys of a burst, in ms. */
  maxGapMs: number
  minLen: number
  maxLen: number
}

export interface WedgeKey {
  /** `KeyboardEvent.code` */
  code: string
  /** `KeyboardEvent.key`: what the key typed, used to replay it. */
  key: string
  /** Milliseconds, any monotonic origin (`event.timeStamp`). */
  at: number
  repeat?: boolean
  ctrl?: boolean
  alt?: boolean
  meta?: boolean
}

export type WedgeResult =
  /** Not for the detector: let the event through untouched. */
  | { action: 'neutral'; replay: string }
  /** Swallow the event (it is held, or part of a held burst). */
  | { action: 'hold'; replay: string }
  /** Swallow the event and hand over the badge number. */
  | { action: 'badge'; replay: string; token: string }
  /** Let the event through, after typing `replay` into the focused field. */
  | { action: 'pass'; replay: string }

const MODIFIERS = new Set([
  'ShiftLeft',
  'ShiftRight',
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'MetaLeft',
  'MetaRight',
  'OSLeft',
  'OSRight',
  'CapsLock',
  'NumLock',
  'AltGraph',
])

/** The badge character of a physical key, or null for any other key. */
export function badgeChar(code: string): string | null {
  const m = /^(?:(?:Digit|Numpad)(?<digit>\d)|Key(?<hex>[A-F]))$/u.exec(code)
  return m?.groups?.digit ?? m?.groups?.hex ?? null
}

function isEnter(code: string): boolean {
  return code === 'Enter' || code === 'NumpadEnter'
}

interface Held {
  char: string
  text: string
}

export class WedgeDetector {
  private held: Held[] = []
  private lastAt = Number.NEGATIVE_INFINITY
  // A burst longer than maxLen was replayed: the rest of it, as long as it
  // keeps the pace, is typing too, never the tail of a short badge.
  private overlong = false

  private cfg: WedgeConfig

  constructor(cfg: WedgeConfig) {
    this.cfg = cfg
  }

  /** New timing settings; anything held is let go first (use `flush`). */
  configure(cfg: WedgeConfig) {
    this.cfg = cfg
  }

  get pending(): boolean {
    return this.held.length > 0
  }

  /** Release what is held (the gap timer fired): the text to replay. */
  flush(): string {
    const text = this.held.map((h) => h.text).join('')
    this.held = []
    return text
  }

  onKey(k: WedgeKey): WedgeResult {
    if (MODIFIERS.has(k.code)) return { action: 'neutral', replay: '' }
    if (k.repeat === true) {
      // A held key repeating inside a burst is swallowed, not counted.
      return this.held.length > 0
        ? { action: 'hold', replay: '' }
        : { action: 'neutral', replay: '' }
    }

    // A gap longer than the limit ends the burst: it was typing.
    let replay = ''
    const gapped = this.held.length > 0 && k.at - this.lastAt > this.cfg.maxGapMs
    if (gapped) replay = this.flush()
    if (k.at - this.lastAt > this.cfg.maxGapMs) this.overlong = false
    this.lastAt = k.at

    const chord = k.ctrl === true || k.alt === true || k.meta === true
    const ch = chord ? null : badgeChar(k.code)

    if (isEnter(k.code) && !chord) {
      if (this.held.length === 0) {
        this.overlong = false
        return { action: 'pass', replay }
      }
      const len = this.held.length
      if (len >= this.cfg.minLen && len <= this.cfg.maxLen) {
        const token = this.held.map((h) => h.char).join('')
        this.held = []
        return { action: 'badge', replay, token }
      }
      return { action: 'pass', replay: replay + this.flush() }
    }

    if (ch === null) {
      // Any other key breaks the burst; it goes through after the replay.
      this.overlong = false
      return { action: 'pass', replay: replay + this.flush() }
    }

    if (this.overlong) return { action: 'pass', replay }

    if (this.held.length + 1 > this.cfg.maxLen) {
      // The (maxLen + 1)th key: not a badge. Replay it all, in order.
      this.overlong = true
      return { action: 'pass', replay: replay + this.flush() }
    }
    this.held.push({ char: ch, text: k.key.length === 1 ? k.key : ch })
    return { action: 'hold', replay }
  }
}
