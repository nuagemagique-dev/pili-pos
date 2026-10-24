import { defineStore } from "pinia";
import { computed, ref } from "vue";
import * as ipc from "../api/ipc";

/** The system product's department: a voucher line is told apart by it. */
export const GIFT_VOUCHER_DEPARTMENT = "GV";

/** The gift voucher setting, read once per screen that needs it. A till
 * that does not answer simply has vouchers off. */
export const useGiftVouchersStore = defineStore("giftVouchers", () => {
  const settings = ref<ipc.GiftVoucherSettings>({
    enabled: false,
    validity_months: 12,
    max_amount_cents: 50000,
  });
  const enabled = computed(() => settings.value.enabled);

  async function load() {
    try {
      settings.value = await ipc.getGiftVoucherSettings();
    } catch {
      settings.value = { ...settings.value, enabled: false };
    }
  }

  return { settings, enabled, load };
});
