import { defineStore } from 'pinia'

// Minimal toast store used to surface non-blocking operator feedback
// (print success, printer-not-configured warning, delay applied, …).
// UX Guide §8.2: "erreurs non-bloquantes → bandeau, auto-dismiss".
//
// C5: info, success and warning toasts go away after their duration; an
// ERROR toast stays until it is tapped (a failure must not vanish while
// the operator looks elsewhere), whatever duration the caller gave.

export type ToastVariant = 'info' | 'success' | 'warning' | 'error'

export interface ToastEntry {
  id: number
  message: string
  variant: ToastVariant
  expires_at: number
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
    /** The toasts that are drawn: the newest {@link MAX_VISIBLE_TOASTS}, oldest first. */
    visible: (state): ToastEntry[] => state.toasts.slice(-MAX_VISIBLE_TOASTS),
  },

  actions: {
    show(message: string, variant: ToastVariant = 'info', ttl_ms = DEFAULT_TTL_MS) {
      const id = this.next_id++
      this.toasts.push({
        id,
        message,
        variant,
        expires_at: expiryOf(variant, Date.now(), ttl_ms),
      })
      this.ensureCleanupLoop()
      return id
    },

    dismiss(id: number) {
      this.toasts = this.toasts.filter((t) => t.id !== id)
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
