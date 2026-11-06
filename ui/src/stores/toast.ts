import { defineStore } from 'pinia'

// Minimal toast store used to surface non-blocking operator feedback
// (print success, printer-not-configured warning, delay applied, …).
// UX Guide §8.2: "erreurs non-bloquantes → bandeau, auto-dismiss".
//
// C5: info, success and warning toasts go away after their duration; an
// ERROR toast stays until it is tapped (a failure must not vanish while
// the operator looks elsewhere), whatever duration the caller gave.

export type ToastVariant = 'info' | 'success' | 'warning' | 'error'

/** A button on a toast (UX audit C8: "Annuler" after a table move). */
export interface ToastAction {
  label: string
  run: () => void | Promise<void>
}

export interface ToastEntry {
  id: number
  message: string
  variant: ToastVariant
  expires_at: number
  action?: ToastAction
}

interface ToastState {
  toasts: ToastEntry[]
  next_id: number
  cleanup_handle: number | null
}

const DEFAULT_TTL_MS = 3000

/** At most this many toasts are drawn at once, the newest ones, so a
 * burst of messages never grows into a wall under the header. */
export const MAX_VISIBLE_TOASTS = 3

/**
 * The toasts to draw, oldest first: error toasts come first in the
 * choice (an error waits for a tap, so it must not be pushed out of view
 * by newer messages and come back unprompted later), then the newest of
 * the others fill the remaining places. At most `max` in total.
 */
export function pickVisible(toasts: ToastEntry[], max: number = MAX_VISIBLE_TOASTS): ToastEntry[] {
  const errors = toasts.filter((t) => t.variant === 'error').slice(-max)
  const others = toasts.filter((t) => t.variant !== 'error')
  const room = max - errors.length
  const chosen = new Set<number>([
    ...errors.map((t) => t.id),
    ...(room > 0 ? others.slice(-room) : []).map((t) => t.id),
  ])
  return toasts.filter((t) => chosen.has(t.id))
}

/** An error toast never expires by itself. */
export function expiryOf(variant: ToastVariant, now: number, ttl_ms: number): number {
  return variant === 'error' ? Number.POSITIVE_INFINITY : now + ttl_ms
}

export const useToastStore = defineStore('toast', {
  state: (): ToastState => ({
    toasts: [],
    next_id: 1,
    cleanup_handle: null,
  }),

  getters: {
    /** The toasts that are drawn: errors first, then the newest others, {@link MAX_VISIBLE_TOASTS} at most, oldest first. */
    visible: (state): ToastEntry[] => pickVisible(state.toasts),
  },

  actions: {
    show(message: string, variant: ToastVariant = 'info', ttl_ms = DEFAULT_TTL_MS) {
      return this.push({ message, variant, ttl_ms })
    },

    /** A success toast with one button (C8: "Annuler" after a table move). */
    showWithAction(message: string, action: ToastAction, ttl_ms = DEFAULT_TTL_MS) {
      return this.push({ message, variant: 'success', ttl_ms, action })
    },

    push(spec: { message: string; variant: ToastVariant; ttl_ms: number; action?: ToastAction }) {
      const id = this.next_id++
      this.toasts.push({
        id,
        message: spec.message,
        variant: spec.variant,
        expires_at: expiryOf(spec.variant, Date.now(), spec.ttl_ms),
        ...(spec.action ? { action: spec.action } : {}),
      })
      this.ensureCleanupLoop()
      return id
    },

    dismiss(id: number) {
      this.toasts = this.toasts.filter((t) => t.id !== id)
    },

    /** The tap on the button of a toast: the toast goes first, then its action runs once. */
    runAction(id: number) {
      const action = this.toasts.find((t) => t.id === id)?.action
      this.dismiss(id)
      if (action) void action.run()
    },

    /**
     * Start a shared setInterval that prunes expired toasts. Runs
     * once per store instance. Stops when no toast can expire any more
     * (the queue is empty, or only errors waiting for a tap are left).
     */
    ensureCleanupLoop() {
      if (this.cleanup_handle !== null) return
      this.cleanup_handle = window.setInterval(() => {
        const now = Date.now()
        this.toasts = this.toasts.filter((t) => t.expires_at > now)
        const expiring = this.toasts.some((t) => Number.isFinite(t.expires_at))
        if (!expiring && this.cleanup_handle !== null) {
          window.clearInterval(this.cleanup_handle)
          this.cleanup_handle = null
        }
      }, 250)
    },
  },
})
