<script setup lang="ts">
// HubRise (webshop) connection panel for Settings. Only rendered when
// the build has HUBRISE_ENABLED (see features.ts); the certification
// build leaves this component out of the bundle.
import { onMounted, ref } from "vue";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import * as ipc from "../api/ipc";
import type { HubriseConnectionInfo } from "../api/ipc";
import { useI18n } from "../i18n";
import { useWebshopOrdersStore } from "../stores/webshopOrders";
import { describeError } from "../utils/errorMessages";
const { t } = useI18n();
// Pending count kept live by the header's "En ligne" button.
const webshop = useWebshopOrdersStore();

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

onMounted(() => {
  loadHubriseStatus();
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
</style>
