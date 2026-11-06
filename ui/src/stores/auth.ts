import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { User } from '../types'
import * as ipc from '../api/ipc'
import { useToastStore } from './toast'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'

export const useAuthStore = defineStore('auth', () => {
  const currentUser = ref<User | null>(null)
  const error = ref<string | null>(null)

  /** When the engine will lock the idle session (ms since epoch), while
   * its warning is showing; null otherwise (UX audit L4). */
  const idleLockAt = ref<number | null>(null)

  /** Minutes of inactivity after which the engine last locked the till
   * by itself; shown on the login screen until someone logs in. */
  const idleLockedAfterMin = ref<number | null>(null)

  /** The order that was open when the engine locked the till for inactivity, and whose it was
   * (UX audit T6): the same user logging back in lands on it again. */
  const lockedReturn = ref<{ insz: string; orderId: number } | null>(null)

  // A login ends the "locked after N min" line; so does any new session.
  watch(currentUser, (user) => {
    if (user) {
      idleLockedAfterMin.value = null
      idleLockAt.value = null
    }
  })

  /** Hide the countdown at once: the operator touched the screen, which
   * the engine hears too. */
  function dismissIdleWarning() {
    idleLockAt.value = null
  }

  async function loginWithInsz(insz: string, pin?: string) {
    error.value = null
    try {
      currentUser.value = await ipc.login(insz, pin)
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  /** First login with a PIN chosen now (a manager must have one). */
  async function loginWithNewPin(insz: string, pin: string) {
    error.value = null
    try {
      currentUser.value = await ipc.loginWithNewPin(insz, pin)
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  // True while lock() runs: its own `session-locked` event is handled by
  // the caller, not by the listener.
  let locking = false

  /** Lock the screen through the engine: it registers the open order
   * (it needs the open session to sign the P), ends the session and
   * keeps the operator on shift. A failed order registration comes back
   * in the outcome and never blocks the lock. */
  async function lock(): Promise<ipc.LockOutcome> {
    locking = true
    try {
      const outcome = await ipc.lockSession()
      currentUser.value = null
      return outcome
    } catch {
      currentUser.value = null
      try {
        await ipc.logout()
      } catch {
        /* the next login replaces the session anyway */
      }
      return { orderError: null }
    } finally {
      locking = false
    }
  }

  let sessionLockedListened = false

  /** The engine locked the session by itself (idle): drop the operator
   * and let the caller leave the screen. Call once at start-up. */
  async function listenSessionLocked(
    onLocked: (e: ipc.SessionLocked, previous: User | null) => void,
  ) {
    if (sessionLockedListened) return
    sessionLockedListened = true
    await listen<ipc.SessionLocked>('session-locked', (event) => {
      idleLockAt.value = null
      if (locking || !currentUser.value) return
      const previous = currentUser.value
      currentUser.value = null
      const { reason, idleAfterMs } = event.payload
      idleLockedAfterMin.value =
        reason === 'idle' && typeof idleAfterMs === 'number' && idleAfterMs > 0
          ? Math.max(1, Math.round(idleAfterMs / 60_000))
          : null
      onLocked(event.payload, previous)
    })
  }

  let idleWarningListened = false

  /** The engine warns shortly before the idle lock, and says when the
   * operator was active again. Call once at start-up. */
  async function listenIdleLockWarning() {
    if (idleWarningListened) return
    idleWarningListened = true
    await listen<ipc.IdleLockWarning>('idle-lock-warning', (event) => {
      const { lockInMs } = event.payload
      idleLockAt.value = lockInMs === null || !currentUser.value ? null : Date.now() + lockInMs
    })
  }

  let presenceFailedListened = false

  /** The engine clocks the employee in itself after a login; when that
   * fails it says so here. Never blocks anything. Call once at start-up. */
  async function listenPresenceFailed() {
    if (presenceFailedListened) return
    presenceFailedListened = true
    await listen<ipc.PresenceFailed>('presence-failed', (event) => {
      const { t } = useI18n()
      useToastStore().show(
        `${t('presence_failed')} : ${describeError(event.payload.error)}`,
        'error',
        6000,
      )
    })
  }

  /** End the employee's shift from the login screen, through the engine:
   * it checks the PIN (a refused PIN is thrown for the PIN pad to show),
   * signs the clock-out (S) and always ends the session. Returns whether
   * the clock-out was registered. */
  async function endShift(insz: string, pin?: string): Promise<boolean> {
    error.value = null
    const r = await ipc.endShift(insz, pin)
    currentUser.value = null
    if (!r.success) {
      const { t } = useI18n()
      useToastStore().show(
        `${t('presence_failed')} : ${describeError(r.error ?? 'FDM error')}`,
        'error',
        6000,
      )
    }
    return r.success
  }

  /** Remember the order open at an idle lock for the user who was working on it. */
  function rememberLockedOrder(insz: string, orderId: number | null) {
    lockedReturn.value = orderId === null ? null : { insz, orderId }
  }

  /** The order to reopen for `insz` at login, once: another user, or none, gets null. Any login
   * forgets it, so a later lock never brings back an old order. */
  function takeLockedOrder(insz: string): number | null {
    const memory = lockedReturn.value
    lockedReturn.value = null
    return memory !== null && memory.insz === insz ? memory.orderId : null
  }

  /** End the logged-in employee's own shift from the header: the engine signs their
   * clock-out (the same S event as the end of shift of the login screen; the session
   * is the proof of the PIN, so none is asked again), then the session is locked.
   * Returns whether the clock-out was registered. */
  async function endOwnShift(): Promise<boolean> {
    const user = currentUser.value
    if (!user) return false
    let result: ipc.SocialResult
    try {
      result = await ipc.workOut(user.insz)
    } catch (e) {
      result = { success: false, error: String(e) }
    }
    await lock()
    if (!result.success) {
      const { t } = useI18n()
      useToastStore().show(
        `${t('presence_failed')} : ${describeError(result.error ?? 'FDM error')}`,
        'error',
        6000,
      )
    }
    return result.success
  }

  // What the role may do (PROFILS_UTILISATEURS_FR.md §2). The backend
  // enforces it; the UI only hides what would be refused.
  const role = computed(() => currentUser.value?.role)
  const canTakePayment = computed(() => role.value === 'admin' || role.value === 'cashier')
  const isManager = computed(() => role.value === 'admin')

  return {
    currentUser,
    error,
    idleLockAt,
    idleLockedAfterMin,
    dismissIdleWarning,
    listenIdleLockWarning,
    loginWithInsz,
    loginWithNewPin,
    lock,
    listenSessionLocked,
    listenPresenceFailed,
    endShift,
    endOwnShift,
    rememberLockedOrder,
    takeLockedOrder,
    canTakePayment,
    isManager,
  }
})
