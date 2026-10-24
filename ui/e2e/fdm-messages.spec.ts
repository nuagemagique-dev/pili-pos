import { test, expect, type Page } from '@playwright/test'
import {
  mockTauriIPC,
  DEFAULT_USER,
  DEFAULT_CATEGORY,
  DEFAULT_MENU_ITEM,
  tapUserTile,
  fdmHealth,
  fdmMocks,
} from './fixtures/mock-ipc'

// Certification manual §V: every message code of tb_messages, as the
// engine's FDM monitor reports it (FdmHealth, classified in Rust:
// crates/pos-engine/src/commands/fdm_monitor.rs), against the header
// indicator.
// "M" and "O" codes show their French message (fdmMessages.ts) in the
// banner under the header; the four blocking errors turn the indicator
// red and veil the sales screens. "N" codes come with
// showPos NEVER and must not be shown. One test per code, tagged with
// its matrix id.

type Nature = 'Error' | 'Warning'

const BLOCKING = new Set(['BUFFER_FULL', 'FDM_LOCKED', 'FDM_NOT_OPERATIONAL', 'UNKNOWN_POS'])

/** The health the engine reports for one FDM message: errors first,
 * blocking for the four blocking codes, a showPos NEVER message not
 * shown at all. */
function healthFor(code: string, nature: Nature, showPos: string) {
  if (showPos === 'NEVER') return fdmHealth()
  const blocking = nature === 'Error' && BLOCKING.has(code)
  return fdmHealth({
    connected: !blocking,
    severity: blocking ? 'blocking' : 'warning',
    code,
    mandatory: showPos === 'MANDATORY',
  })
}

async function orderScreenWithStatus(page: Page, code: string, nature: Nature, showPos: string) {
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...fdmMocks(healthFor(code, nature, showPos)),
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/)
  // A blocking error veils the sales screens (FdmAlerts): the message is
  // read there, over the Tables screen. Otherwise, go on to the order.
  if (nature === 'Error' && BLOCKING.has(code) && showPos !== 'NEVER') return
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/)
}

/** A displayed code: its message, and the indicator's color. */
async function shows(
  page: Page,
  code: string,
  nature: Nature,
  showPos: 'MANDATORY' | 'OPTIONAL',
  message: string,
  state: 'warn' | 'err',
) {
  await orderScreenWithStatus(page, code, nature, showPos)
  await expect(page.locator('.fdm-alert .msg')).toHaveText(message)
  await expect(page.locator(`.fdm.${state}`)).toBeVisible()
}

/** A showPos NEVER code: nothing shown, the register stays usable. */
async function hidden(page: Page, code: string, nature: Nature) {
  await orderScreenWithStatus(page, code, nature, 'NEVER')
  await expect(page.locator('.fdm.ok')).toBeVisible()
  await expect(page.locator('.fdm-alert .msg')).toHaveCount(0)
}

test.describe('§V messages obligatoires (M)', () => {
  // POS description v2.2 tb_messages: mandatory, not in the manual's §V.
  test('numéro de TVA différent de celui de la boîte noire (UPDATE_POS_VATNO)', async ({
    page,
  }) => {
    await shows(
      page,
      'UPDATE_POS_VATNO',
      'Warning',
      'MANDATORY',
      'Le numéro de TVA de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien.',
      'warn',
    )
  })
  test("numéro d'unité d'établissement différent (UPDATE_POS_ESTNO)", async ({ page }) => {
    await shows(
      page,
      'UPDATE_POS_ESTNO',
      'Warning',
      'MANDATORY',
      "Le numéro d'unité d'établissement de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien.",
      'warn',
    )
  })
  test('[V_CLIENT_CERT_EXPIRED] certificat expiré', async ({ page }) => {
    await shows(
      page,
      'CLIENT_CERT_EXPIRED',
      'Warning',
      'MANDATORY',
      'Le certificat de la boîte noire a expiré. Contactez votre technicien.',
      'warn',
    )
  })
  test('[V_BUFFER_NEAR_FULL] buffer au-delà de 70%', async ({ page }) => {
    await shows(
      page,
      'BUFFER_NEAR_FULL',
      'Warning',
      'MANDATORY',
      'La boîte noire est presque pleine. Programmez une intervention.',
      'warn',
    )
  })
  test('[V_BUFFER_FULL] buffer plein, caisse bloquée', async ({ page }) => {
    await shows(
      page,
      'BUFFER_FULL',
      'Error',
      'MANDATORY',
      'La boîte noire est pleine. Contactez votre technicien.',
      'err',
    )
  })
  test('[V_FDM_LOCKED] FDM verrouillé, caisse bloquée', async ({ page }) => {
    await shows(
      page,
      'FDM_LOCKED',
      'Error',
      'MANDATORY',
      'La boîte noire est verrouillée. Contactez votre technicien.',
      'err',
    )
  })
  test('[V_FDM_NOT_OPERATIONAL] FDM non opérationnel, caisse bloquée', async ({ page }) => {
    await shows(
      page,
      'FDM_NOT_OPERATIONAL',
      'Error',
      'MANDATORY',
      "La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes.",
      'err',
    )
  })
  test('[V_UNKNOWN_POS] caisse absente de la liste du FDM, caisse bloquée', async ({ page }) => {
    await shows(
      page,
      'UNKNOWN_POS',
      'Error',
      'MANDATORY',
      "Cette caisse n'est pas reconnue par la boîte noire.",
      'err',
    )
  })
})

test.describe('§V messages facultatifs (O), affichés', () => {
  test("[V_CLIENT_CERT_NEAR_EXPIRATION] certificat proche de l'expiration", async ({ page }) => {
    await shows(
      page,
      'CLIENT_CERT_NEAR_EXPIRATION',
      'Warning',
      'OPTIONAL',
      'Le certificat de la boîte noire expire bientôt.',
      'warn',
    )
  })
  test('[V_RTC_SYNC_FAILED] horloge non synchronisée', async ({ page }) => {
    await shows(
      page,
      'RTC_SYNC_FAILED',
      'Warning',
      'OPTIONAL',
      "L'horloge de la boîte noire n'a pas pu se synchroniser.",
      'warn',
    )
  })
  test('[V_UPDATE_URLS_FAILED] mise à jour des URLs', async ({ page }) => {
    await shows(
      page,
      'UPDATE_URLS_FAILED',
      'Warning',
      'OPTIONAL',
      'Mise à jour des URLs de la boîte noire échouée.',
      'warn',
    )
  })
  test('[V_UPDATE_TRUST_CERT_FAILED] certificats de confiance', async ({ page }) => {
    await shows(
      page,
      'UPDATE_TRUST_CERT_FAILED',
      'Warning',
      'OPTIONAL',
      'Mise à jour des certificats de confiance échouée.',
      'warn',
    )
  })
  test('[V_UPDATE_TASK_LIST_FAILED] liste de tâches', async ({ page }) => {
    await shows(
      page,
      'UPDATE_TASK_LIST_FAILED',
      'Warning',
      'OPTIONAL',
      'Téléchargement des tâches FDM échoué.',
      'warn',
    )
  })
  test('[V_TASK_FEEDBACK_FAILED] résultat de tâche', async ({ page }) => {
    await shows(
      page,
      'TASK_FEEDBACK_FAILED',
      'Warning',
      'OPTIONAL',
      "La boîte noire n'a pas pu signaler le résultat d'une tâche.",
      'warn',
    )
  })
  test('[V_NOP_FAILED] message NOP', async ({ page }) => {
    await shows(
      page,
      'NOP_FAILED',
      'Warning',
      'OPTIONAL',
      'Communication intermédiaire avec la boîte noire échouée.',
      'warn',
    )
  })
  test('[V_SERVER_CERT_RESOLVE_FAILED] certificat serveur', async ({ page }) => {
    await shows(
      page,
      'SERVER_CERT_RESOLVE_FAILED',
      'Warning',
      'OPTIONAL',
      'Impossible de résoudre le certificat du serveur FDM.',
      'warn',
    )
  })
  test('[V_TRANSACTION_UPLOAD_FAILED] envoi des transactions', async ({ page }) => {
    await shows(
      page,
      'TRANSACTION_UPLOAD_FAILED',
      'Warning',
      'OPTIONAL',
      'La boîte noire échoue à transmettre les transactions au SPF Finances.',
      'warn',
    )
  })
  test('[V_INITIALIZATION_FAILED] initialisation', async ({ page }) => {
    await shows(
      page,
      'INITIALIZATION_FAILED',
      'Warning',
      'OPTIONAL',
      "L'initialisation de la boîte noire a échoué.",
      'warn',
    )
  })
  test('[V_UNAUTHORIZED] requête non autorisée', async ({ page }) => {
    await shows(
      page,
      'UNAUTHORIZED',
      'Error',
      'OPTIONAL',
      "La caisse n'est pas autorisée à parler à la boîte noire.",
      'warn',
    )
  })
  test('[V_INVALID_REQUEST] requête invalide', async ({ page }) => {
    await shows(
      page,
      'INVALID_REQUEST',
      'Error',
      'OPTIONAL',
      'Erreur de communication avec la boîte noire. Contactez votre technicien.',
      'warn',
    )
  })
  test('[V_INTERNAL_ERROR] problème technique', async ({ page }) => {
    await shows(
      page,
      'INTERNAL_ERROR',
      'Error',
      'OPTIONAL',
      'Erreur technique de la boîte noire. Contactez votre technicien.',
      'warn',
    )
  })
  test('[V_UNDEFINED_ERROR] erreur indéfinie', async ({ page }) => {
    await shows(
      page,
      'UNDEFINED_ERROR',
      'Error',
      'OPTIONAL',
      'Erreur indéfinie de la boîte noire. Contactez votre technicien.',
      'warn',
    )
  })
  test('[V_UNDEFINED_OTHER] autre message', async ({ page }) => {
    await shows(
      page,
      'UNDEFINED_OTHER',
      'Warning',
      'OPTIONAL',
      'Erreur de la boîte noire. Contactez votre technicien.',
      'warn',
    )
  })
})

test.describe('§V messages jamais affichés (N)', () => {
  test('[V_RTC_NOT_INITIALIZED] horloge non initialisée', async ({ page }) => {
    await hidden(page, 'RTC_NOT_INITIALIZED', 'Warning')
  })
  test('[V_CORRUPT_RECORD_ENCOUNTERED] transaction corrompue', async ({ page }) => {
    await hidden(page, 'CORRUPT_RECORD_ENCOUNTERED', 'Warning')
  })
  test('[V_UPDATE_PARAMS_FAILED] mise à jour des paramètres', async ({ page }) => {
    await hidden(page, 'UPDATE_PARAMS_FAILED', 'Warning')
  })
  test('[V_UPDATE_CLIENT_CERT_FAILED] mise à jour du certificat client', async ({ page }) => {
    await hidden(page, 'UPDATE_CLIENT_CERT_FAILED', 'Warning')
  })
  test('[V_UPDATE_VAT_RATES_FAILED] mise à jour des taux de TVA', async ({ page }) => {
    await hidden(page, 'UPDATE_VAT_RATES_FAILED', 'Warning')
  })
  test("[V_UPDATE_POS_ALLOWLIST_FAILED] mise à jour de la liste d'autorisation", async ({
    page,
  }) => {
    await hidden(page, 'UPDATE_POS_ALLOWLIST_FAILED', 'Warning')
  })
  test('[V_DUPLICATE_REQUEST] requête en doublon', async ({ page }) => {
    await hidden(page, 'DUPLICATE_REQUEST', 'Warning')
  })
  test('[V_TOO_MANY_MEMORY_ERRORS] trop de transactions corrompues', async ({ page }) => {
    await hidden(page, 'TOO_MANY_MEMORY_ERRORS', 'Error')
  })
})

test('an FDM that is not initialized says so under the header', async ({ page }) => {
  // module2's online simulator: `initialized: false`, no message, and
  // it still signs. Amber, with the reason.
  await mockTauriIPC(page, {
    login: () => DEFAULT_USER,
    list_users: () => [DEFAULT_USER],
    get_categories: () => [DEFAULT_CATEGORY],
    get_menu_items: () => [DEFAULT_MENU_ITEM],
    ...fdmMocks(fdmHealth({ connected: false, code: 'FDM_NOT_INITIALIZED' })),
  })
  await page.goto('/')
  await tapUserTile(page)
  await page.waitForURL(/tables/)
  await page.locator('.table-card').first().click()
  await page.waitForURL(/order/)
  await expect(page.locator('.fdm.warn')).toBeVisible()
  await expect(page.locator('.fdm-alert .msg')).toHaveText(
    "La boîte noire n'est pas encore initialisée.",
  )
})
