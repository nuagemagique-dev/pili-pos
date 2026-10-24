<script setup lang="ts">
import { useI18n } from "../i18n";
import TextKeypad from "./TextKeypad.vue";

// On-screen keyboard for typing a customer's email address - the
// kiosk has no physical keyboard. Two steps: type (with live
// validation) then confirm (large type, per cert requirement that the
// address is confirmed on screen before it is used). The keys and the
// layout are the shared TextKeypad.

defineEmits<{
  confirm: [email: string];
  cancel: [];
}>();

const { t } = useI18n();

const QUICK_KEYS = [".be", ".com", "@gmail.com"];
const SYMBOLS = ["@", ".", "-", "_", "+"];

// Basic local@domain.tld shape, no whitespace. The backend re-validates
// authoritatively before signing - this is just to save the operator a
// round trip on an obvious typo.
function isEmail(v: string): boolean {
  if (!v || /\s/.test(v)) return false;
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v);
}
</script>

<template>
  <TextKeypad
    :title="t('delivery_enter_email_title')"
    :placeholder="t('delivery_email_placeholder')"
    :quick-keys="QUICK_KEYS"
    :symbols="SYMBOLS"
    :valid="isEmail"
    :invalid-text="t('delivery_invalid_email')"
    confirm-step
    :confirm-title="t('delivery_confirm_title')"
    :edit-label="t('delivery_edit')"
    :confirm-label="t('delivery_confirm_btn')"
    @confirm="$emit('confirm', $event)"
    @cancel="$emit('cancel')"
  />
</template>
