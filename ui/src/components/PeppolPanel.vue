<script setup lang="ts">
// Peppol delivery of the invoices (Settings, managers only). Drawn only
// when the installation has a `peppol` section: provider, sender id,
// the queue counts, and the parked deliveries (refused by the access
// point, recipient not on Peppol, authentication) with their reason and
// a retry. Polls every 30 s while mounted, like the nightly export.
import { computed, onMounted, onUnmounted, ref } from "vue";
import * as ipc from "../api/ipc";
import { useToastStore } from "../stores/toast";
import { formatDayTime, formatTimestamp } from "../utils/datetime";
import { describeError } from "../utils/errorMessages";
import { useI18n } from "../i18n";

const POLL_MS = 30_000;
/** How many deliveries the panel asks for. */
const LIMIT = 100;

const { t } = useI18n();
const toast = useToastStore();

const panel = ref<ipc.PeppolDeliveries | null>(null);
const callError = ref<string | null>(null);
const retrying = ref<number | null>(null);

let timer: ReturnType<typeof setInterval> | undefined;
let gone = false;

async function load() {
  try {
    const next = await ipc.listPeppolDeliveries(LIMIT);
    if (gone) return;
    panel.value = next;
    callError.value = null;
  } catch (e) {
    // Not configured is not an error: the panel stays hidden.
    if (!gone) callError.value = describeError(e);
  }
}

const configured = computed(() => panel.value?.configured === true);
const parked = computed(() => (panel.value?.deliveries ?? []).filter((d) => d.status === "failed"));

async function retry(id: number) {
  if (retrying.value !== null) return;
  retrying.value = id;
  try {
    await ipc.retryPeppolDelivery(id);
    toast.show(t("peppol_retried"), "success", 3000);
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  } finally {
    retrying.value = null;
  }
  await load();
}

const time = (iso: string | null) => (iso ? formatTimestamp(iso, formatDayTime) : "");

onMounted(() => {
  void load();
  timer = setInterval(() => void load(), POLL_MS);
});

onUnmounted(() => {
  gone = true;
  if (timer !== undefined) clearInterval(timer);
  timer = undefined;
});
</script>

<template>
  <section v-if="configured && panel" class="peppol-panel" data-testid="peppol-panel">
    <h2>{{ t('peppol_title') }}</h2>
    <p v-if="callError" class="call-error" role="alert">{{ callError }}</p>

    <div class="row">
      <span>{{ t('peppol_provider') }}</span><span class="value" data-testid="peppol-provider">{{ panel.provider }}</span>
    </div>
    <div class="row">
      <span>{{ t('peppol_sender') }}</span><span class="value" data-testid="peppol-sender">{{ panel.senderId }}</span>
    </div>

    <div class="counts">
      <div class="count" data-testid="peppol-queued">
        <span class="n">{{ panel.queued }}</span><span class="l">{{ t('peppol_queued') }}</span>
      </div>
      <div class="count ok" data-testid="peppol-accepted">
        <span class="n">{{ panel.accepted }}</span><span class="l">{{ t('peppol_accepted') }}</span>
      </div>
      <div class="count" :class="{ bad: panel.failed > 0 }" data-testid="peppol-failed">
        <span class="n">{{ panel.failed }}</span><span class="l">{{ t('peppol_failed') }}</span>
      </div>
    </div>

    <h3>{{ t('peppol_parked_title') }}</h3>
    <p v-if="parked.length === 0" class="none" data-testid="peppol-none-parked">{{ t('peppol_none_parked') }}</p>
    <ul v-else class="parked">
      <li v-for="d in parked" :key="d.id" class="parked-row" :data-testid="`peppol-parked-${d.id}`">
        <div class="info">
          <span class="no">{{ d.invoiceNo }} <span class="ep">{{ d.customerEndpointId }}</span></span>
          <span class="reason">{{ t('peppol_reason', { reason: d.errorMsg ?? '-' }) }}</span>
          <span class="meta">{{ t('peppol_attempts', { n: d.attempts }) }}<template v-if="d.parkedAt"> · {{ time(d.parkedAt) }}</template></span>
        </div>
        <button
          class="retry-btn"
          :data-testid="`peppol-retry-${d.id}`"
          :disabled="retrying !== null"
          @click="retry(d.id)"
        >{{ t('peppol_retry') }}</button>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.peppol-panel { flex-shrink: 0; display: flex; flex-direction: column; gap: 12px; }
.peppol-panel h2 { font-size: 28px; font-weight: var(--font-weight-heavy); line-height: 1.2; }
.peppol-panel h3 { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); margin-top: 4px; }
.call-error { font-size: 15px; color: var(--accent-danger); overflow-wrap: anywhere; }

.row {
  display: flex; justify-content: space-between; align-items: center; gap: 16px;
  min-height: 40px; border-bottom: 1px solid var(--border); font-size: 16px;
}
.row span:first-child { color: var(--text-secondary); }
.row .value { font-family: var(--font-mono); text-align: right; overflow-wrap: anywhere; }

.counts { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; }
.count {
  display: flex; flex-direction: column; align-items: center; gap: 2px;
  padding: 12px 8px; border-radius: var(--radius-md); background: var(--bg-surface);
}
.count .n { font-family: var(--font-mono); font-size: 28px; font-weight: var(--font-weight-heavy); }
.count .l { font-size: 14px; color: var(--text-secondary); }
.count.ok .n { color: var(--accent-success); }
.count.bad .n { color: var(--accent-danger); }

.none { font-size: 16px; color: var(--text-secondary); }
.parked { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
.parked-row {
  display: flex; align-items: center; justify-content: space-between; gap: 16px;
  padding: 12px 16px; border-radius: var(--radius-md); background: var(--bg-surface);
}
.info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.no { font-family: var(--font-mono); font-size: 18px; font-weight: var(--font-weight-bold); }
.ep { color: var(--text-secondary); font-size: 14px; }
.reason { font-size: 15px; color: var(--accent-danger); overflow-wrap: anywhere; }
.meta { font-size: 14px; color: var(--text-secondary); }
.retry-btn {
  height: 56px; min-width: 128px; padding: 0 20px; border-radius: var(--radius-md);
  background: var(--bg-primary); color: var(--text-primary); border: 2px solid var(--accent-info);
  font-size: 17px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.retry-btn:active { transform: scale(var(--tap-scale)); }
.retry-btn:disabled { opacity: 0.4; }
</style>
