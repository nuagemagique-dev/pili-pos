import { defineStore } from "pinia";
import { ref } from "vue";
import type { MailStatus, TicketDeliveryRequest, TicketMediumRequest } from "../types";
import * as ipc from "../api/ipc";

const PAPER: TicketDeliveryRequest = { medium: "paper" };

/**
 * The operator's ticket-delivery choice (paper / email / both), shared
 * across checkout, refund and copy so the picker and keypad never hold
 * business state themselves — only the transient typing buffer does.
 */
export const useDeliveryStore = defineStore("delivery", () => {
  const mailStatus = ref<MailStatus | null>(null);
  const request = ref<TicketDeliveryRequest>({ ...PAPER });

  /** Refreshed whenever the picker is about to be shown (probe is
   * cached 60s backend-side, so calling this often is cheap). */
  async function refreshMailStatus() {
    try {
      mailStatus.value = await ipc.checkMailStatus();
    } catch {
      mailStatus.value = null;
    }
  }

  function setPaper() {
    request.value = { ...PAPER };
  }

  function setEmail(medium: Exclude<TicketMediumRequest, "paper">, email: string) {
    request.value = { medium, email };
  }

  /** Back to Papier — call after every checkout/refund/copy so the
   * next customer starts from the default. */
  function reset() {
    request.value = { ...PAPER };
  }

  return { mailStatus, request, refreshMailStatus, setPaper, setEmail, reset };
});
