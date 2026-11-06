<script setup lang="ts">
/**
 * The idle lock is near (UX audit L4): a banner under the header with a
 * live countdown. The ENGINE owns the lock and says when to warn
 * (`idle-lock-warning`, `stores/auth.ts`); this component only counts
 * down what it was told. Any touch is forwarded to the engine as
 * activity (App.vue), which cancels the warning and hides the banner.
 * Amber, like every "waiting" state; it takes its own row and no tap.
 */
import { computed, onUnmounted, ref, watch } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useI18n } from '../i18n'

const auth = useAuthStore()
const { t } = useI18n()

const now = ref(Date.now())
let tick: ReturnType<typeof setInterval> | undefined

watch(
  () => auth.idleLockAt,
  (at) => {
    clearInterval(tick)
    if (at === null) return
    now.value = Date.now()
    tick = setInterval(() => (now.value = Date.now()), 250)
  },
  { immediate: true },
)

onUnmounted(() => clearInterval(tick))

const secondsLeft = computed(() =>
  auth.idleLockAt === null ? 0 : Math.max(0, Math.ceil((auth.idleLockAt - now.value) / 1000)),
)
</script>

<template>
  <div v-if="auth.idleLockAt !== null" class="idle-banner" role="status" data-testid="idle-banner">
    <span class="bang">!</span>
    <span class="msg">{{ t('idle_lock_warning', { s: secondsLeft }) }}</span>
  </div>
</template>

<style scoped>
.idle-banner {
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
  font-variant-numeric: tabular-nums;
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
</style>

<style>
/* Above the blocking FDM veil (900) so the countdown stays readable. */
.fdm-blocking .idle-banner {
  position: relative;
  z-index: 901;
}
</style>
