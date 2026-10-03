<script setup lang="ts">
// HubRise (webshop) connection panel for Settings. Only rendered when
// the build has __HUBRISE_ENABLED__ (see env.d.ts); the certification
// build leaves this component out of the bundle.
import { computed, onMounted, ref } from "vue";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import * as ipc from "../api/ipc";
import type {
  CatalogAdoption,
  HubriseChannel,
  HubriseConnectionInfo,
  HubriseHealth,
} from "../api/ipc";
import { useAuthStore } from "../stores/auth";
import { useToastStore } from "../stores/toast";
import { formatDayTime, formatTimestamp } from "../utils/datetime";
import { formatPrice } from "../utils/format";
import { useI18n } from "../i18n";
import { useWebshopOrdersStore } from "../stores/webshopOrders";
import { describeError } from "../utils/errorMessages";
const { t } = useI18n();
// Pending count kept live by the header's "En ligne" button.
const webshop = useWebshopOrdersStore();
const auth = useAuthStore();
const toast = useToastStore();

// ── HubRise connection ─────────────────────────────────────────────
const hubriseStatus = ref<HubriseConnectionInfo | null>(null);
const hubriseCode = ref("");
const hubriseError = ref<string | null>(null);
const hubriseLoading = ref(false);

async function loadHubriseStatus() {
  try {
    hubriseStatus.value = await ipc.hubriseGetConnectionStatus();
  } catch (e) {
    hubriseError.value = describeError(e);
  }
}

async function connectHubrise() {
  hubriseError.value = null;
  hubriseLoading.value = true;
  try {
    const url = await ipc.hubriseStartOauthFlow();
    await openUrl(url);
  } catch (e) {
    hubriseError.value = describeError(e);
  } finally {
    hubriseLoading.value = false;
  }
}

async function completeHubrise() {
  if (!hubriseCode.value.trim()) {
    hubriseError.value = t("hubrise_err_code_required");
    return;
  }
  hubriseLoading.value = true;
  hubriseError.value = null;
  try {
    hubriseStatus.value = await ipc.hubriseCompleteOauth(hubriseCode.value.trim());
    hubriseCode.value = "";
  } catch (e) {
    hubriseError.value = describeError(e);
  } finally {
    hubriseLoading.value = false;
  }
}

async function disconnectHubrise() {
  hubriseLoading.value = true;
  try {
    await ipc.hubriseDisconnect();
    await loadHubriseStatus();
  } catch (e) {
    hubriseError.value = describeError(e);
  } finally {
    hubriseLoading.value = false;
  }
}

// ── Health, catalog, adoption, channels ────────────────────────────
const health = ref<HubriseHealth | null>(null);
const catalogBusy = ref(false);
const adoptions = ref<CatalogAdoption[]>([]);
// Ticked by default: every match keeps its id unless the manager says no.
const adopted = ref<Set<number>>(new Set());
const channels = ref<HubriseChannel[]>([]);
const newChannel = ref("");

const catalogRefused = computed(() => {
  const c = health.value?.catalog;
  return !!c?.rejected_at && (!c.applied_at || c.rejected_at >= c.applied_at);
});

const catalogSynced = computed(() => {
  const at = health.value?.last_catalog_sync_at ?? health.value?.catalog?.applied_at;
  return at
    ? t("hubrise_catalog_synced", { time: formatTimestamp(at, formatDayTime) })
    : t("hubrise_catalog_never");
});

async function loadHealth() {
  try {
    health.value = await ipc.hubriseStatus();
  } catch (e) {
    hubriseError.value = describeError(e);
    return;
  }
  if (health.value.catalog_awaiting_adoption && auth.isManager) await loadAdoptions();
  if (health.value.connected && auth.isManager) await loadChannels();
}

async function syncCatalog() {
  catalogBusy.value = true;
  hubriseError.value = null;
  try {
    await ipc.hubriseSyncCatalog();
    toast.show(t("hubrise_catalog_sync_ok"), "success");
  } catch (e) {
    hubriseError.value = describeError(e);
  } finally {
    catalogBusy.value = false;
    await loadHealth();
  }
}

async function loadAdoptions() {
  try {
    adoptions.value = await ipc.hubriseAdoptionPreview();
    adopted.value = new Set(adoptions.value.map((a) => a.menu_item_id));
  } catch (e) {
    hubriseError.value = describeError(e);
  }
}

function toggleAdoption(id: number) {
  const next = new Set(adopted.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  adopted.value = next;
}

async function adoptCatalog() {
  catalogBusy.value = true;
  hubriseError.value = null;
  try {
    await ipc.hubriseAdoptCatalog(adoptions.value.filter((a) => adopted.value.has(a.menu_item_id)));
    adoptions.value = [];
    toast.show(t("hubrise_catalog_sync_ok"), "success");
  } catch (e) {
    hubriseError.value = describeError(e);
  } finally {
    catalogBusy.value = false;
    await loadHealth();
  }
}

async function loadChannels() {
  try {
    channels.value = await ipc.hubriseListChannels();
  } catch (e) {
    hubriseError.value = describeError(e);
  }
}

async function saveChannel(channel: HubriseChannel) {
  hubriseError.value = null;
  try {
    const released = await ipc.hubriseSetChannel(channel);
    toast.show(t("hubrise_channel_saved", { count: released }), "success");
    await webshop.fetchOrders();
  } catch (e) {
    hubriseError.value = describeError(e);
  }
  await loadChannels();
}

async function addChannel() {
  const name = newChannel.value.trim();
  if (!name) return;
  await saveChannel({ channel: name, kind: "platform", payment_type: "APP", charge_policy: "exclude" });
  newChannel.value = "";
}

/** The same channel with one setting changed; a platform pays APP, a webshop ONLINE. */
function withChange(c: HubriseChannel, field: keyof HubriseChannel, value: string): HubriseChannel {
  const next = { ...c, [field]: value } as HubriseChannel;
  if (field === "kind") next.payment_type = value === "webshop" ? "ONLINE" : "APP";
  return next;
}

onMounted(async () => {
  await loadHubriseStatus();
  await loadHealth();
});
</script>

<template>
  <!-- HubRise connection panel. The parent passes the panel classes
       (background, border, padding, grow); the layout is set here. -->
  <section class="hubrise-section">
    <h2>HubRise</h2>
    <div class="pills">
      <span class="pill" :class="hubriseStatus?.connected ? 'status-ok' : 'status-off'">
        <span class="dot"></span>{{ hubriseStatus?.connected ? t('hubrise_connected') : t('hubrise_disconnected') }}
      </span>
      <span v-if="hubriseStatus?.connected" class="pill" :class="{ 'status-warn': webshop.pendingCount > 0 }">
        <span class="dot"></span>{{ t('settings_hubrise_pending', { count: webshop.pendingCount }) }}
      </span>
    </div>

    <template v-if="hubriseStatus?.connected">
      <div class="info-rows">
        <div class="info-row"><span>{{ t('hubrise_account_label') }}</span><span class="info-value">{{ hubriseStatus.account_name }}</span></div>
        <div class="info-row"><span>{{ t('hubrise_location_label') }}</span><span class="info-value">{{ hubriseStatus.location_name }}</span></div>
        <div v-if="hubriseStatus.catalog_name" class="info-row">
          <span>{{ t('hubrise_catalog_label') }}</span><span class="info-value">{{ hubriseStatus.catalog_name }}</span>
        </div>
      </div>
      <p v-if="health?.catalog?.manages_menu" class="banner" data-testid="menu-managed">
        {{ t('hubrise_menu_managed_banner') }}
      </p>
      <p v-if="health?.auth_error_at" class="banner banner-err">{{ t('hubrise_health_auth') }}</p>
      <p v-if="health && health.parked_events + health.outbox.parked > 0" class="banner banner-err">
        {{ t('hubrise_health_parked', { count: health.parked_events + health.outbox.parked }) }}
      </p>
      <p v-if="health && health.outbox.pending > 0 && health.outbox_error" class="banner">
        {{ t('hubrise_health_outbox', { count: health.outbox.pending }) }}
      </p>

      <!-- Catalog: last sync, the issues of a refused one -->
      <div class="section-label">{{ t('hubrise_catalog_title') }}</div>
      <div class="catalog-row">
        <span data-testid="catalog-synced">{{ catalogSynced }}</span>
        <button class="action-btn" :disabled="catalogBusy" @click="syncCatalog">
          {{ t('webshop_action_resync') }}
        </button>
      </div>
      <div v-if="catalogRefused" class="banner banner-err" data-testid="catalog-refused">
        {{ t('hubrise_catalog_rejected') }}
        <ul class="issues">
          <li v-for="(issue, i) in health!.catalog!.issues" :key="i">
            {{ t(`catalog_issue_${issue.code}`) }}<span v-if="issue.key" class="mono"> · {{ issue.key }}</span>
          </li>
        </ul>
      </div>

      <!-- First connect: the manager picks the products that keep their id -->
      <template v-if="health?.catalog_awaiting_adoption && auth.isManager">
        <div class="section-label">{{ t('hubrise_adoption_title') }}</div>
        <p class="hubrise-help">{{ t('hubrise_adoption_help') }}</p>
        <p v-if="adoptions.length === 0" class="hubrise-help">{{ t('hubrise_adoption_none') }}</p>
        <ul v-else class="adoptions" data-testid="adoptions">
          <li v-for="a in adoptions" :key="a.menu_item_id">
            <label class="adoption">
              <input
                type="checkbox"
                :checked="adopted.has(a.menu_item_id)"
                @change="toggleAdoption(a.menu_item_id)"
              />
              <span>{{ a.name }}</span>
              <span class="mono">{{ formatPrice(a.price_cents) }} €</span>
            </label>
          </li>
        </ul>
        <button class="action-btn" :disabled="catalogBusy" @click="adoptCatalog">
          {{ t('hubrise_adoption_action') }}
        </button>
      </template>

      <!-- Channels: how each one is signed -->
      <template v-if="auth.isManager">
        <div class="section-label">{{ t('hubrise_channels_title') }}</div>
        <ul class="channels" data-testid="channels">
          <li v-for="c in channels" :key="c.channel" class="channel">
            <span class="mono channel-name">{{ c.channel }}</span>
            <select
              class="field select"
              :value="c.kind"
              @change="saveChannel(withChange(c, 'kind', ($event.target as HTMLSelectElement).value))"
            >
              <option value="platform">{{ t('hubrise_channel_platform') }} · APP</option>
              <option value="webshop">{{ t('hubrise_channel_webshop') }} · ONLINE</option>
            </select>
            <select
              class="field select"
              :value="c.charge_policy"
              @change="saveChannel(withChange(c, 'charge_policy', ($event.target as HTMLSelectElement).value))"
            >
              <option value="include">{{ t('hubrise_channel_charges_include') }}</option>
              <option value="exclude">{{ t('hubrise_channel_charges_exclude') }}</option>
            </select>
          </li>
        </ul>
        <div class="code-row">
          <input v-model="newChannel" class="field" :placeholder="t('hubrise_channel_name')" />
          <button class="action-btn" :disabled="!newChannel.trim()" @click="addChannel">
            {{ t('hubrise_channel_add') }}
          </button>
        </div>
      </template>

      <button class="action-btn danger" :disabled="hubriseLoading" @click="disconnectHubrise">
        {{ t('hubrise_disconnect_action') }}
      </button>
    </template>

    <template v-else>
      <p class="hubrise-help">{{ t('hubrise_section_help') }}</p>
      <button class="action-btn" :disabled="hubriseLoading" @click="connectHubrise">
        {{ t('hubrise_open_authorize_url') }}
      </button>
      <div class="section-label">{{ t('hubrise_paste_code_title') }}</div>
      <div class="code-row">
        <input v-model="hubriseCode" class="field" :placeholder="t('hubrise_code_placeholder')" />
        <button class="action-btn" :disabled="hubriseLoading" @click="completeHubrise">
          {{ t('hubrise_complete_connection') }}
        </button>
      </div>
    </template>

    <p v-if="hubriseError" class="add-error">{{ hubriseError }}</p>
  </section>
</template>

<style scoped>
.hubrise-section { display: flex; flex-direction: column; gap: 16px; }
.hubrise-section h2 { font-size: 28px; font-weight: var(--font-weight-heavy); line-height: 1.2; }
.hubrise-help { font-size: 16px; color: var(--text-secondary); text-wrap: pretty; }
.section-label {
  font-family: var(--font-mono); font-size: 14px; font-weight: var(--font-weight-bold);
  color: var(--text-secondary); letter-spacing: 0.08em; text-transform: uppercase;
}

.pills { display: flex; gap: 8px; flex-wrap: wrap; }
.pill {
  display: inline-flex; align-items: center; gap: 8px;
  height: 36px; padding: 0 14px; border-radius: 18px; border: 1px solid var(--border);
  font-family: var(--font-mono); font-size: 14px; font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.pill .dot { width: 8px; height: 8px; border-radius: 50%; background: currentColor; }
.status-ok { color: var(--accent-success); }
.status-off { color: var(--text-secondary); }
.status-warn { color: var(--accent-warning); }

.info-row {
  display: flex; justify-content: space-between; align-items: center; gap: 16px;
  min-height: 48px; border-bottom: 1px solid var(--border); font-size: 16px;
}
.info-row span:first-child { color: var(--text-secondary); flex-shrink: 0; }
.info-value { font-family: var(--font-mono); text-align: right; overflow-wrap: anywhere; }

.code-row { display: flex; gap: 8px; }
.field {
  flex: 1; min-width: 0; height: 56px; padding: 0 16px;
  border-radius: var(--radius-md); border: 1px solid var(--border);
  background: transparent; color: var(--text-primary); font-family: var(--font-mono); font-size: 16px;
}
.field::placeholder { color: var(--text-secondary); font-family: var(--font-family); }
.action-btn {
  min-height: 56px; padding: 0 20px; border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary);
  font-size: 16px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.action-btn.danger { background: transparent; border: 2px solid var(--accent-danger); color: var(--accent-danger); }
.action-btn:active { transform: scale(var(--tap-scale)); }
.action-btn:disabled { opacity: 0.5; }
.add-error { color: var(--accent-danger); font-size: 15px; }
.mono { font-family: var(--font-mono); }
.banner {
  padding: 12px 14px; border-radius: var(--radius-sm); font-size: 15px;
  background: color-mix(in srgb, var(--accent-warning) 18%, transparent);
}
.banner-err { background: color-mix(in srgb, var(--accent-danger) 15%, transparent); }
.issues { margin: 6px 0 0; padding-left: 18px; }
.catalog-row { display: flex; justify-content: space-between; align-items: center; gap: 16px; font-size: 16px; }
.adoptions, .channels { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
.adoption {
  display: grid; grid-template-columns: 32px 1fr auto; align-items: center; gap: 12px;
  min-height: 48px; font-size: 16px;
}
.adoption input { width: 28px; height: 28px; }
.channel { display: grid; grid-template-columns: 1fr auto auto; gap: 8px; align-items: center; }
.channel-name { overflow-wrap: anywhere; }
.select { flex: none; min-width: 180px; }
</style>
