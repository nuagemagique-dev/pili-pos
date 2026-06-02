import { defineStore } from "pinia";
import { ref } from "vue";
import * as ipc from "../api/ipc";

export type AppLanguage = "fr" | "en" | "zh";

const LANG_KEY = "moyun-pos-language";

export const usePosStore = defineStore("pos", () => {
  const trainingMode = ref(false);
  const settings = ref<ipc.PosSettings | null>(null);
  const language = ref<AppLanguage>(
    (localStorage.getItem(LANG_KEY) as AppLanguage) || "fr"
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

  return { trainingMode, settings, language, loadSettings, toggleTrainingMode, setLanguage };
});
