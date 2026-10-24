<script setup lang="ts">
import { computed, ref } from 'vue'
import { useDeliveryStore } from '../stores/delivery'
import { useI18n } from '../i18n'
import EmailKeypad from './EmailKeypad.vue'
import type { TicketMediumRequest } from '../types'

// Three touch options for how the customer gets their ticket. Hidden
// entirely when the terminal has no working email setup - paper stays
// the silent default (see the checkout guard in the caller).

const delivery = useDeliveryStore()
const { t } = useI18n()

const showKeypad = ref(false)
const pendingMedium = ref<Exclude<TicketMediumRequest, 'paper'>>('paper_digital')

// DIGITAL (email only, no paper) is only ever offered when the
// terminal allows it AND the mail service answered a probe recently -
// otherwise the backend would downgrade it to PAPER_DIGITAL anyway.
const showDigitalOnly = computed(
  () => Boolean(delivery.mailStatus?.allowDigitalOnly) && Boolean(delivery.mailStatus?.reachable),
)

const showUnreachableNote = computed(
  () => Boolean(delivery.mailStatus?.configured) && !delivery.mailStatus?.reachable,
)

function selectPaper() {
  delivery.setPaper()
}

function openKeypad(medium: Exclude<TicketMediumRequest, 'paper'>) {
  pendingMedium.value = medium
  showKeypad.value = true
}

function onEmailConfirmed(email: string) {
  delivery.setEmail(pendingMedium.value, email)
  showKeypad.value = false
}

function onEmailCancelled() {
  // Cert UX: cancelling the address entry returns to Papier, never
  // leaves a half-picked email medium behind.
  delivery.setPaper()
  showKeypad.value = false
}
</script>

<template>
  <div v-if="delivery.mailStatus?.configured" class="medium-picker">
    <div class="medium-options">
      <button
        class="medium-btn"
        :class="{ active: delivery.request.medium === 'paper' }"
        @click="selectPaper"
      >
        {{ t('delivery_paper') }}
      </button>
      <button
        v-if="showDigitalOnly"
        class="medium-btn"
        :class="{ active: delivery.request.medium === 'digital' }"
        @click="openKeypad('digital')"
      >
        {{ t('delivery_email') }}
      </button>
      <button
        class="medium-btn"
        :class="{ active: delivery.request.medium === 'paper_digital' }"
        @click="openKeypad('paper_digital')"
      >
        {{ t('delivery_both') }}
      </button>
    </div>
    <p v-if="showUnreachableNote" class="medium-note">{{ t('delivery_unreachable_note') }}</p>

    <Teleport to="body">
      <EmailKeypad v-if="showKeypad" @confirm="onEmailConfirmed" @cancel="onEmailCancelled" />
    </Teleport>
  </div>
</template>

<style scoped>
.medium-picker {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  width: 100%;
}

.medium-options {
  display: flex;
  gap: var(--space-sm);
  width: 100%;
}

.medium-btn {
  flex: 1;
  height: 64px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-secondary);
  transition:
    transform var(--tap-duration) ease,
    background var(--tap-duration) ease,
    color var(--tap-duration) ease;
}
.medium-btn:active {
  transform: scale(var(--tap-scale));
}
.medium-btn.active {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}

.medium-note {
  font-size: var(--font-size-xs);
  color: var(--accent-warning);
  text-align: center;
}
</style>
