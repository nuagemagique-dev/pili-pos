import { defineStore } from 'pinia'
import { ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import * as ipc from '../api/ipc'
import { ERROR_MESSAGES } from '../utils/fdmMessages'
import { describeError } from '../utils/errorMessages'
import { useToastStore } from './toast'

// Shape of an FDM message item as returned by the SCE 2.0 spec
// (Status.warnings[i] and Status.errors[i]).
interface FdmMessage {
  message?: string
  extensions?: {
    category?: string
    code?: string
    showPos?: string
  }
}

// Messages the FDM attached to a signed event (Rust
// `pos_fiscal::fdm_client::SignMessages`, event "fdm-sign-messages"),
// showPos NEVER already removed.
export interface SignMessages {
  mutation: string
  bookingDate?: string | null
  warnings: FdmMessage[]
  informations: FdmMessage[]
}

const SIGN_MESSAGE_TTL_MS = 8000

// Severity tier - what the UI does about a code.
// "blocking": red pill + checkout-blocking modal.
// "warning":  yellow pill + tooltip, no modal.
// "info":     informational, no UI change.
type Severity = 'blocking' | 'warning' | 'info'

export const useFdmStore = defineStore('fdm', () => {
  const connected = ref(false)
  const checking = ref(false)
  const lastError = ref<string | null>(null) // FR message or raw code
  const severity = ref<Severity>('info')
  // The FDM asks the POS to keep showing this message (showPos
  // MANDATORY, certification manual §V): it must not auto-dismiss.
  const mandatory = ref(false)

  /** When the engine checks next (ms since epoch), for the countdown on
   * the blocking veil; null when it did not say. */
  const nextCheckAt = ref<number | null>(null)

  /** Take the engine's reading of the FDM; it classified it already. */
  function apply(h: ipc.FdmHealth) {
    connected.value = h.connected
    severity.value = h.severity
    lastError.value = h.code
    mandatory.value = h.mandatory
    nextCheckAt.value = h.nextCheckAt ? Date.parse(h.nextCheckAt) : null
  }

  let statusListened = false

  /** Follow the engine's FDM monitor ("fdm-status"); call once at
   * start-up. The last known reading is applied at once. */
  async function listenStatus() {
    if (statusListened) return
    statusListened = true
    await listen<ipc.FdmHealth>('fdm-status', (event) => apply(event.payload))
    try {
      const last = await ipc.fdmHealth()
      if (last) apply(last)
    } catch {
      /* no engine to tell: the next event will */
    }
  }

  /** Ask the engine to check the FDM now. Returns false when the FDM
   * did not answer (transport failure), true when it answered, even
   * with an error. */
  async function checkNow(): Promise<boolean> {
    checking.value = true
    try {
      const h = await ipc.fdmCheckNow()
      apply(h)
      return h.answered
    } catch (e) {
      // The command itself failed (an internal error, not an FDM
      // state): say so and keep the state as it is.
      useToastStore().show(describeError(e), 'error')
      return false
    } finally {
      checking.value = false
    }
  }

  // POS description v2.2, Message: a warning received with a
  // mutation's response is shown at least once per bookingDate. Each
  // code is shown once per bookingDate, whichever event carried it.
  const shownSignMessages = new Set<string>()

  function showSignMessages(signed: SignMessages) {
    const toast = useToastStore()
    const groups: Array<[FdmMessage[], 'warning' | 'info']> = [
      [signed.warnings ?? [], 'warning'],
      [signed.informations ?? [], 'info'],
    ]
    for (const [items, variant] of groups) {
      for (const item of items) {
        const code = item.extensions?.code
        if (!code) continue
        const key = `${signed.bookingDate ?? ''}|${code}`
        if (shownSignMessages.has(key)) continue
        shownSignMessages.add(key)
        const text = ERROR_MESSAGES[code] ?? item.message ?? code
        toast.show(text, variant, SIGN_MESSAGE_TTL_MS)
        // The header indicator shows it too until the next status. A
        // MANDATORY one keeps its banner open, as the monitor's reading
        // of the same code would (a sale can bring it before the next
        // status check).
        if (variant === 'warning') {
          const keep = item.extensions?.showPos === 'MANDATORY'
          if (severity.value === 'info') {
            severity.value = 'warning'
            lastError.value = code
            mandatory.value = keep
          } else if (severity.value === 'warning' && keep && !mandatory.value) {
            lastError.value = code
            mandatory.value = true
          }
        }
      }
    }
  }

  let signMessagesListened = false

  /** Show the messages of every signed event; call once at start-up. */
  async function listenSignMessages() {
    if (signMessagesListened) return
    signMessagesListened = true
    await listen<SignMessages>('fdm-sign-messages', (event) => showSignMessages(event.payload))
  }

  return {
    connected,
    checking,
    lastError,
    severity,
    mandatory,
    nextCheckAt,
    apply,
    listenStatus,
    checkNow,
    showSignMessages,
    listenSignMessages,
  }
})
