import { defineStore } from "pinia";
import { ref } from "vue";
import * as ipc from "../api/ipc";

export type AppLanguage = "fr" | "en" | "zh";

/** What leads a product button or cart line: its name, or its menu code. */
export type ProductLabel = "name" | "code";

const LANG_KEY = "pili-pos-language";
const PRODUCT_LABEL_KEY = "pili-pos-product-label";

export const usePosStore = defineStore("pos", () => {
  const trainingMode = ref(false);
  const settings = ref<ipc.PosSettings | null>(null);
  const language = ref<AppLanguage>(
    (localStorage.getItem(LANG_KEY) as AppLanguage) || "fr"
  );
  const productLabel = ref<ProductLabel>(
    localStorage.getItem(PRODUCT_LABEL_KEY) === "name" ? "name" : "code"
  );

  async function loadSettings() {
    settings.value = await ipc.getSettings();
    trainingMode.value = settings.value.trainingMode;
  }

  async function toggleTrainingMode() {
    const newValue = !trainingMode.value;
    await ipc.setTrainingMode(newValue);
    trainingMode.value = newValue;
  }

  function setLanguage(lang: AppLanguage) {
    language.value = lang;
    localStorage.setItem(LANG_KEY, lang);
  }

  function setProductLabel(label: ProductLabel) {
    productLabel.value = label;
    localStorage.setItem(PRODUCT_LABEL_KEY, label);
  }

  return {
    trainingMode,
    settings,
    language,
    productLabel,
    loadSettings,
    toggleTrainingMode,
    setLanguage,
    setProductLabel,
  };
});
