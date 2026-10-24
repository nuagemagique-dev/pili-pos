import { defineStore } from "pinia";
import { ref } from "vue";
import * as ipc from "../api/ipc";
import type { MenuItem, SlipOutcome } from "../types";
import { describeError } from "../utils/errorMessages";
import { usePosStore } from "./pos";

/**
 * The allergen mode of the order screen and the sheets (commands/
 * allergens.rs). In the mode a tap on a product opens its allergen sheet
 * instead of adding it. Non-fiscal display and print data.
 */
export const useAllergensStore = defineStore("allergens", () => {
  const mode = ref(false);
  const legend = ref<ipc.AllergenEntry[]>([]);
  const sheetItem = ref<MenuItem | null>(null);
  /** The product's sheet in the UI language, and in French when that differs. */
  const sheet = ref<ipc.AllergenSheetProduct | null>(null);
  const sheetFr = ref<ipc.AllergenSheetProduct | null>(null);
  const error = ref<string | null>(null);
  const printResult = ref<SlipOutcome | null>(null);
  const busy = ref(false);

  async function loadLegend() {
    if (legend.value.length > 0) return;
    try {
      legend.value = await ipc.listAllergens();
    } catch (e) {
      error.value = describeError(e);
    }
  }

  async function toggleMode() {
    mode.value = !mode.value;
    if (mode.value) await loadLegend();
  }

  async function openSheet(item: MenuItem) {
    const pos = usePosStore();
    error.value = null;
    printResult.value = null;
    sheetItem.value = item;
    sheet.value = null;
    sheetFr.value = null;
    try {
      const lang = pos.language;
      const [own, fr] = await Promise.all([
        ipc.getAllergenSheet(item.id, lang),
        lang === "fr" ? Promise.resolve(null) : ipc.getAllergenSheet(item.id, "fr"),
      ]);
      sheet.value = own.products[0] ?? null;
      sheetFr.value = fr ? (fr.products[0] ?? null) : null;
    } catch (e) {
      error.value = describeError(e);
    }
  }

  function closeSheet() {
    sheetItem.value = null;
    sheet.value = null;
    sheetFr.value = null;
    error.value = null;
    printResult.value = null;
  }

  /** Prints the open product's sheet, or the whole menu (`all`). The
   *  printer speaks fr, nl, en and de: Chinese prints in French. */
  async function print(all: boolean) {
    const pos = usePosStore();
    error.value = null;
    printResult.value = null;
    busy.value = true;
    try {
      const lang = pos.language === "zh" ? "fr" : pos.language;
      printResult.value = await ipc.printAllergenSheet(all ? null : (sheetItem.value?.id ?? null), lang);
    } catch (e) {
      error.value = describeError(e);
    } finally {
      busy.value = false;
    }
  }

  return { mode, legend, sheetItem, sheet, sheetFr, error, printResult, busy, loadLegend, toggleMode, openSheet, closeSheet, print };
});
