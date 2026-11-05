import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '../api/ipc'

export type AppLanguage = 'fr' | 'en' | 'zh'

/** What leads a product button or cart line: its name, or its menu code. */
export type ProductLabel = 'name' | 'code'

const LANG_KEY = 'pili-pos-language'
const LANGUAGES: readonly AppLanguage[] = ['fr', 'en', 'zh']

function isAppLanguage(value: string | null): value is AppLanguage {
  return LANGUAGES.some((language) => language === value)
}
const PRODUCT_LABEL_KEY = 'pili-pos-product-label'
const ADD_BEEP_KEY = 'pili-pos-add-beep'

export const usePosStore = defineStore('pos', () => {
  const trainingMode = ref(false)
  const settings = ref<ipc.PosSettings | null>(null)
  const storedLanguage = localStorage.getItem(LANG_KEY)
  const language = ref<AppLanguage>(isAppLanguage(storedLanguage) ? storedLanguage : 'fr')
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

  function setLanguage(lang: AppLanguage) {
    language.value = lang
    localStorage.setItem(LANG_KEY, lang)
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
    productLabel,
    addBeep,
    loadSettings,
    toggleTrainingMode,
    setLanguage,
    setProductLabel,
    setAddBeep,
  }
})
