import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '../api/ipc'
import { describeError } from '../utils/errorMessages'

/** How many log rows the panel shows. */
export const ALERT_LOG_ROWS = 20

/**
 * The owner alert settings (commands/alerts.rs): the settings the
 * owner edits, the result of the last test message and the latest log
 * rows. Managers only; the engine refuses the commands to the others.
 */
export const useOwnerAlertsStore = defineStore('ownerAlerts', () => {
  const settings = ref<ipc.OwnerAlerts | null>(null)
  const log = ref<ipc.AlertLogRow[]>([])
  const error = ref<string | null>(null)
  const testResult = ref<ipc.AlertTestResult | null>(null)
  const busy = ref(false)

  async function loadLog() {
    try {
      log.value = await ipc.listOwnerAlertLog(ALERT_LOG_ROWS)
    } catch (e) {
      error.value = describeError(e)
    }
  }

  async function load() {
    error.value = null
    try {
      settings.value = await ipc.getOwnerAlerts()
    } catch (e) {
      error.value = describeError(e)
      return
    }
    await loadLog()
  }

  /** Returns true when the engine took the settings. */
  async function save(input: ipc.OwnerAlertsInput): Promise<boolean> {
    error.value = null
    busy.value = true
    try {
      settings.value = await ipc.setOwnerAlerts(input)
      return true
    } catch (e) {
      error.value = describeError(e)
      return false
    } finally {
      busy.value = false
    }
  }

  /** Sends one message with the stored settings; the log is read again after. */
  async function sendTest() {
    error.value = null
    testResult.value = null
    busy.value = true
    try {
      testResult.value = await ipc.sendTestAlert()
    } catch (e) {
      error.value = describeError(e)
    } finally {
      busy.value = false
    }
    await loadLog()
  }

  return { settings, log, error, testResult, busy, load, save, sendTest }
})
