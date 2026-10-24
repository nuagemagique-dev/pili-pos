<script setup lang="ts">
/**
 * Blocking overlay of a card payment on the linked terminal
 * (stores/eft.ts). It covers the checkout and the online orders board
 * while the terminal waits for the card, then shows the answer:
 *
 * - waiting: amount, spinner, "Annuler" (a request to the terminal)
 * - checking: spinner while `eft_check` runs
 * - declined / cancelled: the reason, OK (the card part stays editable)
 * - unknown: the sale waits for the terminal's answer, "Vérifier"
 * - refused before anything was charged: the error, OK
 *
 * An approved payment closes the overlay by itself: the card part shows
 * locked "Accepté" behind it.
 */
import { computed } from 'vue'
import { useEftStore } from '../stores/eft'
import { useI18n } from '../i18n'
import { formatPrice } from '../utils/format'

const eft = useEftStore()
const { t } = useI18n()

const visible = computed(() => eft.phase !== 'idle')
const busy = computed(() => eft.phase === 'waiting' || eft.phase === 'checking')
/** The payment is still waiting for its answer. */
const unknown = computed(
  () =>
    eft.phase === 'result' && (eft.payment?.status === 'unknown' || eft.payment?.status === 'sent'),
)

const title = computed(() => {
  if (eft.phase === 'result') {
    if (unknown.value) return t('eft_unknown_title')
    if (eft.payment?.status === 'declined') return t('eft_declined')
    if (eft.payment?.status === 'cancelled') return t('eft_cancelled')
    return t('eft_refused')
  }
  return t('eft_overlay_title')
})
</script>

<template>
  <div
    v-if="visible"
    class="eft-backdrop"
    role="alertdialog"
    aria-modal="true"
    data-testid="eft-overlay"
  >
    <div class="eft-dialog">
      <h2 class="eft-title" data-testid="eft-title">{{ title }}</h2>
      <div class="eft-amount">{{ formatPrice(eft.amountCents) }} €</div>

      <template v-if="busy">
        <div class="eft-spinner" />
        <p class="eft-text">
          {{ eft.phase === 'waiting' ? t('eft_present_card') : t('eft_checking') }}
        </p>
        <button
          v-if="eft.phase === 'waiting'"
          class="eft-btn eft-secondary"
          data-testid="eft-cancel"
          :disabled="eft.abortRequested"
          @click="eft.abort()"
        >
          {{ eft.abortRequested ? t('eft_cancel_requested') : t('eft_cancel') }}
        </button>
      </template>

      <template v-else>
        <p v-if="eft.failure" class="eft-text eft-danger" data-testid="eft-failure">
          {{ eft.failure }}
        </p>
        <p v-if="unknown" class="eft-text">{{ t('eft_unknown_explain') }}</p>
        <p v-if="eft.payment?.detail && !unknown" class="eft-text" data-testid="eft-detail">
          {{ eft.payment.detail }}
        </p>
        <p v-if="eft.payment && !unknown && !eft.failure" class="eft-text eft-muted">
          {{ t('eft_not_charged') }}
        </p>
        <div class="eft-actions">
          <button
            v-if="unknown"
            class="eft-btn eft-primary"
            data-testid="eft-check"
            @click="eft.check()"
          >
            {{ t('eft_check') }}
          </button>
          <button class="eft-btn eft-secondary" data-testid="eft-ok" @click="eft.dismiss()">
            {{ unknown ? t('eft_close') : t('eft_ok') }}
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.eft-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.78);
}
.eft-dialog {
  width: min(640px, 92vw);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
  padding: 40px 32px;
  border-radius: var(--radius-lg);
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  text-align: center;
}
.eft-title {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.eft-amount {
  font-family: var(--font-mono);
  font-size: 56px;
  font-weight: 700;
}
.eft-text {
  font-size: 20px;
  color: var(--text-secondary);
  text-wrap: pretty;
}
.eft-muted {
  font-size: 17px;
}
.eft-danger {
  color: var(--accent-danger);
  font-weight: 700;
}
.eft-spinner {
  width: 56px;
  height: 56px;
  border: 4px solid var(--bg-surface);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: eft-spin 0.8s linear infinite;
}
@keyframes eft-spin {
  to {
    transform: rotate(360deg);
  }
}
.eft-actions {
  display: flex;
  gap: 12px;
  width: 100%;
}
.eft-btn {
  flex: 1;
  height: 72px;
  padding: 0 24px;
  border-radius: var(--radius-md);
  font-size: 22px;
  font-weight: 800;
  transition: transform var(--tap-duration) ease;
}
.eft-btn:active:not(:disabled) {
  transform: scale(var(--tap-scale));
}
.eft-btn:disabled {
  opacity: 0.5;
}
.eft-primary {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.eft-secondary {
  background: var(--bg-surface);
  color: var(--text-primary);
  min-width: 220px;
  flex: 0 1 auto;
}
.eft-actions .eft-secondary {
  flex: 1;
}
</style>
