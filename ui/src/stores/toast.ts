import { defineStore } from 'pinia'

// Minimal toast store used to surface non-blocking operator feedback
// (print success, printer-not-configured warning, delay applied, …).
// UX Guide §8.2: "erreurs non-bloquantes → bandeau, auto-dismiss".

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

export const useToastStore = defineStore('toast', {
  state: (): ToastState => ({
    toasts: [],
    next_id: 1,
    cleanup_handle: null,
  }),

  actions: {
    show(message: string, variant: ToastVariant = 'info', ttl_ms = DEFAULT_TTL_MS) {
      const id = this.next_id++
      this.toasts.push({
        id,
        message,
        variant,
        expires_at: Date.now() + ttl_ms,
      })
      this.ensureCleanupLoop()
      return id
    },

    dismiss(id: number) {
      this.toasts = this.toasts.filter((t) => t.id !== id)
    },

    /**
     * Start a shared setInterval that prunes expired toasts. Runs
     * once per store instance. Stops when the queue is empty.
     */
    ensureCleanupLoop() {
      if (this.cleanup_handle !== null) return
      this.cleanup_handle = window.setInterval(() => {
        const now = Date.now()
        this.toasts = this.toasts.filter((t) => t.expires_at > now)
        if (this.toasts.length === 0 && this.cleanup_handle !== null) {
          window.clearInterval(this.cleanup_handle)
          this.cleanup_handle = null
        }
      }, 250)
    },
  },
})
