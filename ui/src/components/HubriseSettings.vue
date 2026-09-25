<script setup lang="ts">
// HubRise (webshop) connection panel for Settings. Only rendered when
// the build has HUBRISE_ENABLED (see features.ts); the certification
// build leaves this component out of the bundle.
import { onMounted, ref } from "vue";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import * as ipc from "../api/ipc";
import type { HubriseConnectionInfo } from "../api/ipc";
import { useI18n } from "../i18n";
const { t } = useI18n();

// ── HubRise connection ─────────────────────────────────────────────
const hubriseStatus = ref<HubriseConnectionInfo | null>(null);
const hubriseCode = ref("");
const hubriseError = ref<string | null>(null);
const hubriseLoading = ref(false);

async function loadHubriseStatus() {
  try {
    hubriseStatus.value = await ipc.hubriseGetConnectionStatus();
  } catch (e) {
    hubriseError.value = String(e);
  }
}

async function connectHubrise() {
  hubriseError.value = null;
  hubriseLoading.value = true;
  try {
    const url = await ipc.hubriseStartOauthFlow();
    await openUrl(url);
  } catch (e) {
    hubriseError.value = String(e);
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
    hubriseError.value = String(e);
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
    hubriseError.value = String(e);
  } finally {
    hubriseLoading.value = false;
  }
}

onMounted(() => {
  loadHubriseStatus();
});
</script>

<template>
  <!-- HubRise connection -->
  <div class="hubrise-section">
    <h3>HubRise</h3>
    <p class="hubrise-help">{{ t('hubrise_section_help') }}</p>

    <div class="info-row">
      <span>{{ t('hubrise_status_label') }}</span>
      <span :class="hubriseStatus?.connected ? 'status-ok' : 'status-off'">
        {{ hubriseStatus?.connected ? t('hubrise_connected') : t('hubrise_disconnected') }}
      </span>
    </div>

    <template v-if="hubriseStatus?.connected">
      <div class="info-row"><span>{{ t('hubrise_account_label') }}</span><span>{{ hubriseStatus.account_name }}</span></div>
      <div class="info-row"><span>{{ t('hubrise_location_label') }}</span><span>{{ hubriseStatus.location_name }}</span></div>
      <div v-if="hubriseStatus.catalog_name" class="info-row">
        <span>{{ t('hubrise_catalog_label') }}</span><span>{{ hubriseStatus.catalog_name }}</span>
      </div>
      <button class="add-btn" :disabled="hubriseLoading" @click="disconnectHubrise">
        {{ t('hubrise_disconnect_action') }}
      </button>
    </template>

    <template v-else>
      <button class="add-btn" :disabled="hubriseLoading" @click="connectHubrise">
        {{ t('hubrise_open_authorize_url') }}
      </button>
      <div class="add-worker">
        <h4>{{ t('hubrise_paste_code_title') }}</h4>
        <div class="add-fields">
          <input v-model="hubriseCode" class="field" :placeholder="t('hubrise_code_placeholder')" />
        </div>
        <button class="add-btn" :disabled="hubriseLoading" @click="completeHubrise">
          {{ t('hubrise_complete_connection') }}
        </button>
      </div>
    </template>

    <p v-if="hubriseError" class="add-error">{{ hubriseError }}</p>
  </div>
</template>

<style scoped>
.hubrise-section { padding: var(--space-md); border: 1px solid rgba(160,160,176,0.15); border-radius: var(--radius-md); }
.hubrise-section h3 { margin-top: 0; }
.hubrise-help { font-size: var(--font-size-xs); color: var(--text-secondary); margin-bottom: var(--space-sm); }
.status-ok { color: var(--accent-success); font-weight: var(--font-weight-bold); }
.status-off { color: var(--text-secondary); }
.info-row { display: flex; justify-content: space-between; padding: var(--space-sm) 0; border-bottom: 1px solid rgba(160,160,176,0.1); font-size: var(--font-size-sm); }
.info-row span:first-child { color: var(--text-secondary); }
.add-worker { margin-top: var(--space-md); padding: var(--space-md); background: var(--bg-secondary); border-radius: var(--radius-md); }
.add-worker h4 { font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); margin-bottom: var(--space-sm); }
.add-fields { display: flex; flex-direction: column; gap: var(--space-sm); }
.field { padding: var(--space-sm) var(--space-md); font-size: var(--font-size-sm); border-radius: var(--radius-sm); border: 1px solid rgba(160,160,176,0.2); background: var(--bg-surface); color: var(--text-primary); height: 48px; }
.add-error { color: var(--accent-danger); font-size: var(--font-size-xs); margin-top: var(--space-xs); }
.add-btn { margin-top: var(--space-sm); width: 100%; height: 52px; font-size: var(--font-size-md); font-weight: var(--font-weight-bold); border-radius: var(--radius-md); background: var(--accent-primary); color: var(--text-on-accent); transition: transform var(--tap-duration) ease; }
.add-btn:active { transform: scale(var(--tap-scale)); }
</style>
