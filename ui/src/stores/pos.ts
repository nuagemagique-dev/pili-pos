import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '../api/ipc'

import {
  isAppLanguage,
  languageFor,
  parseUserLanguages,
  type AppLanguage,
} from '../utils/userLanguage'

export type { AppLanguage }

/** What leads a product button or cart line: its name, or its menu code. */
export type ProductLabel = 'name' | 'code'

const LANG_KEY = 'pili-pos-language'
const USER_LANGS_KEY = 'pili-pos-user-languages'
const PRODUCT_LABEL_KEY = 'pili-pos-product-label'
const ADD_BEEP_KEY = 'pili-pos-add-beep'

export const usePosStore = defineStore('pos', () => {
  const trainingMode = ref(false)
  const settings = ref<ipc.PosSettings | null>(null)
  const storedLanguage = localStorage.getItem(LANG_KEY)
  // The default language of this terminal (Settings), used by the login screen and by
  // every user who has not chosen their own (UX audit R5).
  const terminalLanguage = ref<AppLanguage>(isAppLanguage(storedLanguage) ? storedLanguage : 'fr')
  // What each user chose on this terminal, by user id.
  const userLanguages = ref<Record<string, AppLanguage>>(
    parseUserLanguages(localStorage.getItem(USER_LANGS_KEY)),
  )
  // Who the wording follows now: null on the login screen.
  const languageUser = ref<string | null>(null)
  // The language of the screens right now.
  const language = ref<AppLanguage>(terminalLanguage.value)
  const productLabel = ref<ProductLabel>(
    localStorage.getItem(PRODUCT_LABEL_KEY) === 'name' ? 'name' : 'code',
  )
  // The short beep on each add (C9): on unless this terminal switched it off.
  const addBeep = ref(localStorage.getItem(ADD_BEEP_KEY) !== '0')

  async function loadSettings() {
    settings.value = await ipc.getSettings()
    trainingMode.value = settings.value.trainingMode
  }

  /** Turning training mode on is confirmed by the manager's own PIN (checked by the engine). */
  async function toggleTrainingMode(operatorPin?: string) {
    const newValue = !trainingMode.value
    await ipc.setTrainingMode(newValue, operatorPin)
    trainingMode.value = newValue
  }

  function refreshLanguage() {
    language.value = languageFor(userLanguages.value, languageUser.value, terminalLanguage.value)
  }

  /** The terminal's default language (Settings): users with their own choice keep it. */
  function setLanguage(lang: AppLanguage) {
    terminalLanguage.value = lang
    localStorage.setItem(LANG_KEY, lang)
    refreshLanguage()
  }

  /** Who is logged in (null: nobody): the wording becomes theirs, or the terminal's. */
  function applyUser(userId: string | null) {
    languageUser.value = userId
    refreshLanguage()
  }

  /** The language `userId` chose, or null while they follow the terminal. */
  function userLanguage(userId: string): AppLanguage | null {
    return Object.hasOwn(userLanguages.value, userId) ? userLanguages.value[userId] : null
  }

  /** Remember the language of `userId` on this terminal; applies at once when they are logged in. */
  function setUserLanguage(userId: string, lang: AppLanguage) {
    userLanguages.value = { ...userLanguages.value, [userId]: lang }
    try {
      localStorage.setItem(USER_LANGS_KEY, JSON.stringify(userLanguages.value))
    } catch {
      /* the choice then lasts until the next start */
    }
    refreshLanguage()
  }

  function setProductLabel(label: ProductLabel) {
    productLabel.value = label
    localStorage.setItem(PRODUCT_LABEL_KEY, label)
  }

  function setAddBeep(on: boolean) {
    addBeep.value = on
    localStorage.setItem(ADD_BEEP_KEY, on ? '1' : '0')
  }

  return {
    trainingMode,
    settings,
    language,
    terminalLanguage,
    productLabel,
    addBeep,
    loadSettings,
    toggleTrainingMode,
    setLanguage,
    applyUser,
    userLanguage,
    setUserLanguage,
    setProductLabel,
    setAddBeep,
  }
})
