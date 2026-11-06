<script setup lang="ts">
// The takeaway chip of the header (UX audit T2): "À emporter E-61 · Dupont", and the button that
// adds the customer. "Ajouter un nom" opens the text keypad for the name, then the numeric pad for
// the phone ("Passer" leaves it). Both are optional and local: they print on the kitchen slip (the
// name only) and show on the takeaway cards, nothing else.
import { computed, ref } from 'vue'
import { useOrderStore } from '../stores/order'
import { useI18n } from '../i18n'
import { phoneValue, takeawayTitle, TAKEAWAY_NAME_MAX } from '../utils/takeaway'
import TextKeypad from './TextKeypad.vue'
import PhoneDialog from './PhoneDialog.vue'

const order = useOrderStore()
const { t } = useI18n()

const tag = computed(() => order.takeawayTag)
const title = computed(() => takeawayTitle(t('takeaway_badge'), tag.value))
const hasCustomer = computed(
  () => (tag.value?.customerName ?? '') !== '' || (tag.value?.customerPhone ?? '') !== '',
)
// Only an open order takes a name.
const canEdit = computed(() => tag.value !== null && order.currentOrder?.status === 'open')

const step = ref<'name' | 'phone' | null>(null)

// Accents of French names and the usual punctuation, that the letter rows lack (CP858 prints them).
const NAME_SYMBOLS = ['é', 'è', 'ë', 'ï', 'ô', 'ç', 'à', 'â', '-', "'", '.']

async function onName(name: string) {
  const ok = await order.setTakeawayCustomer(name, tag.value?.customerPhone ?? null)
  step.value = ok ? 'phone' : null
}

async function onPhone(digits: string) {
  step.value = null
  await order.setTakeawayCustomer(tag.value?.customerName ?? null, phoneValue(digits))
}
</script>

<template>
  <span class="header-takeaway" data-testid="header-takeaway">{{ title }}</span>
  <button
    v-if="canEdit"
    class="takeaway-customer-btn"
    data-testid="takeaway-customer-btn"
    @click="step = 'name'"
  >
    {{ hasCustomer ? t('takeaway_edit_customer') : t('takeaway_add_name') }}
  </button>

  <TextKeypad
    v-if="step === 'name'"
    :title="t('takeaway_name_title')"
    :placeholder="t('takeaway_name_placeholder')"
    :initial="tag?.customerName ?? ''"
    :max-len="TAKEAWAY_NAME_MAX"
    :symbols="NAME_SYMBOLS"
    :hint="t('takeaway_name_hint')"
    show-counter
    @confirm="onName"
    @cancel="step = null"
  />
  <PhoneDialog
    v-else-if="step === 'phone'"
    :initial="tag?.customerPhone"
    @confirm="onPhone"
    @skip="step = null"
  />
</template>

<style scoped>
.header-takeaway {
  height: 40px;
  padding: 0 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--accent-warning);
  color: var(--accent-warning);
  font-size: 17px;
  font-weight: var(--font-weight-heavy);
  display: flex;
  align-items: center;
  white-space: nowrap;
}
.takeaway-customer-btn {
  height: 48px;
  padding: 0 16px;
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 15px;
  font-weight: var(--font-weight-bold);
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.takeaway-customer-btn:active {
  transform: scale(var(--tap-scale));
}
</style>
