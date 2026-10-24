<script setup lang="ts">
/**
 * What the FDM (the "boîte noire") says, under the header (UX guide
 * §8.2, certification manual §V):
 *
 * - a warning shows an amber banner. The sale continues. An optional
 *   message closes itself after 10 s; a MANDATORY one stays while the
 *   FDM reports it, as §V requires.
 * - a blocking error covers the sales screens with a veil that cannot
 *   be dismissed: only "Réessayer", and the engine's monitor tries again
 *   on its own (backing off while the FDM does not answer). Reports and
 *   Config stay reachable so the manager can act.
 */
import { computed, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useFdmStore } from '../stores/fdm'
import { usePosStore } from '../stores/pos'
import { useI18n } from '../i18n'
import { fdmCode, translateFdmError, translateFdmErrorZh } from '../utils/fdmMessages'

const AUTO_CLOSE_S = 10
const SALES_ROUTES = new Set(['tables', 'order', 'checkout'])

const fdm = useFdmStore()
const pos = usePosStore()
const route = useRoute()
const { t } = useI18n()

const message = computed(() => translateFdmError(fdm.lastError))
const messageZh = computed(() =>
  pos.language === 'zh' ? translateFdmErrorZh(fdm.lastError) : null,
)
const code = computed(
  () => fdmCode(fdm.lastError) ?? (fdm.lastError ? 'FDM_NOT_OPERATIONAL' : null),
)

// ── Warning banner ────────────────────────────────────────────────
const bannerOpen = ref(false)
const secondsLeft = ref(AUTO_CLOSE_S)
let bannerTimer: ReturnType<typeof setInterval> | undefined

function openBanner() {
  clearInterval(bannerTimer)
  bannerOpen.value = true
  secondsLeft.value = AUTO_CLOSE_S
  if (fdm.mandatory) return
  bannerTimer = setInterval(() => {
    secondsLeft.value -= 1
    if (secondsLeft.value <= 0) {
      bannerOpen.value = false
      clearInterval(bannerTimer)
    }
  }, 1000)
}

watch(
  () => [fdm.severity, fdm.lastError, fdm.mandatory, fdm.connected] as const,
  ([severity, , , connected]) => {
    // A warning, or an FDM that answers but is not ready (e.g. not
    // initialized) without blocking the sale: say why.
    const notReady = !connected && severity !== 'blocking'
    if ((severity === 'warning' || notReady) && message.value) openBanner()
    else {
      bannerOpen.value = false
      clearInterval(bannerTimer)
    }
  },
  { immediate: true },
)

// ── Blocking veil ─────────────────────────────────────────────────
const blocking = computed(() => fdm.severity === 'blocking' && SALES_ROUTES.has(String(route.name)))
// Seconds until the engine's next try, refreshed every second.
const now = ref(Date.now())
let tick: ReturnType<typeof setInterval> | undefined
const retryIn = computed(() =>
  fdm.nextCheckAt == null ? null : Math.max(0, Math.ceil((fdm.nextCheckAt - now.value) / 1000)),
)

watch(
  blocking,
  (on) => {
    clearInterval(tick)
    if (on) tick = setInterval(() => (now.value = Date.now()), 1000)
  },
  { immediate: true },
)

/** An explicit try, on top of the engine monitor's own. */
async function retry() {
  await fdm.checkNow()
}

onUnmounted(() => {
  clearInterval(bannerTimer)
  clearInterval(tick)
})
</script>

<template>
  <div v-if="bannerOpen && message && !blocking" class="fdm-alert fdm-banner" role="status">
    <span class="bang">!</span>
    <span class="msg">{{ message }}</span>
    <span v-if="messageZh" class="msg-zh">{{ messageZh }}</span>
    <span v-if="!fdm.mandatory" class="countdown">{{
      t('fdm_closes_in', { s: secondsLeft })
    }}</span>
  </div>

  <div v-if="blocking" class="fdm-alert fdm-veil" role="alertdialog" aria-modal="true">
    <div class="veil-card">
      <span class="veil-bang">!</span>
      <p class="msg">{{ message }}</p>
      <p v-if="messageZh" class="msg-zh">{{ messageZh }}</p>
      <span class="code"
        >{{ code
        }}<template v-if="retryIn != null">
          · {{ t('fdm_retry_in', { s: retryIn }) }}</template
        ></span
      >
      <button class="retry-btn" @click="retry">{{ t('fdm_retry_now') }}</button>
    </div>
  </div>
</template>

<style scoped>
.fdm-banner {
  height: 56px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 20px;
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
}
.bang {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--text-on-accent);
  color: var(--accent-warning);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 900;
  flex-shrink: 0;
}
.fdm-banner .msg-zh {
  font-family: var(--font-zh);
  font-weight: 500;
}
.countdown {
  margin-left: auto;
  font-family: var(--font-mono);
  font-size: 14px;
  white-space: nowrap;
}

/* §8.2 blocking: full screen, cannot be dismissed. */
.fdm-veil {
  position: fixed;
  inset: 0;
  z-index: 900;
  background: rgba(11, 13, 14, 0.86);
  display: flex;
  align-items: center;
  justify-content: center;
}
.veil-card {
  width: 880px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  padding: 56px;
  border-radius: 20px;
  background: var(--bg-secondary);
  border: 2px solid var(--accent-danger);
  text-align: center;
}
.veil-bang {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  background: var(--accent-danger);
  color: var(--text-on-accent);
  font-size: 56px;
  font-weight: 900;
  display: flex;
  align-items: center;
  justify-content: center;
}
.veil-card .msg {
  font-size: 34px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.25;
  text-wrap: pretty;
}
.veil-card .msg-zh {
  font-family: var(--font-zh);
  font-size: 26px;
  color: var(--text-secondary);
}
.code {
  font-family: var(--font-mono);
  font-size: 15px;
  color: var(--text-secondary);
}
.retry-btn {
  height: 72px;
  min-width: 360px;
  padding: 0 20px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 20px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.retry-btn:active {
  transform: scale(var(--tap-scale));
}
</style>
