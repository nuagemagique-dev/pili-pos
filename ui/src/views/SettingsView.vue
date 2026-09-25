<script setup lang="ts">
import { defineAsyncComponent, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { usePosStore } from "../stores/pos";
import { useAuthStore } from "../stores/auth";
import { useToastStore } from "../stores/toast";
import { useMenuStore } from "../stores/menu";
import type { AppLanguage } from "../stores/pos";
import type { User } from "../types";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";
import { HUBRISE_ENABLED } from "../features";

// Folded at build time so the cert build drops the HubRise panel (see
// OrderView for why this is not a template v-if).
const HubriseSettings = HUBRISE_ENABLED
  ? defineAsyncComponent(() => import("../components/HubriseSettings.vue"))
  : null;
const { t } = useI18n();

const router = useRouter();
const pos = usePosStore();
const auth = useAuthStore();
const toast = useToastStore();

/** The backend refuses the switch while an order is open. */
async function toggleTraining() {
  try {
    await pos.toggleTrainingMode();
  } catch (e) {
    const msg = String(e).includes("open orders") ? t("training_open_orders") : String(e);
    toast.show(msg, "error", 6000);
  }
}
const menu = useMenuStore();

const workers = ref<User[]>([]);
const newName = ref("");
const newInsz = ref("");
const newRole = ref<"admin" | "cashier" | "waiter">("cashier");
// PROFILS_UTILISATEURS_FR.md §1: Gérant, Serveur, Apprenti.
const roles = ["admin", "cashier", "waiter"] as const;
const addError = ref<string | null>(null);

// Deactivation takes two taps: the first arms the row's button.
const confirmDeactivate = ref<string | null>(null);

async function deactivate(insz: string) {
  if (confirmDeactivate.value !== insz) {
    confirmDeactivate.value = insz;
    return;
  }
  confirmDeactivate.value = null;
  try {
    await ipc.deactivateUser(insz);
    await loadWorkers();
  } catch (e) {
    addError.value = String(e);
  }
}

async function loadWorkers() {
  try { workers.value = await ipc.listUsers(); } catch { /* empty */ }
}

async function addWorker() {
  addError.value = null;
  if (!newName.value || !newInsz.value) {
    addError.value = t("settings_err_name_insz_required");
    return;
  }
  try {
    await ipc.createUser({ insz: newInsz.value, name: newName.value, role: newRole.value });
    newName.value = "";
    newInsz.value = "";
    await loadWorkers();
  } catch (e) {
    addError.value = String(e);
  }
}

const languages: { code: AppLanguage; label: string }[] = [
  { code: "fr", label: "FR" },
  { code: "en", label: "EN" },
  { code: "zh", label: "中文" },
];

async function switchLanguage(lang: AppLanguage) {
  pos.setLanguage(lang);
  await menu.reloadWithLanguage();
}

onMounted(() => {
  pos.loadSettings();
  loadWorkers();
});
</script>

<template>
  <div class="settings">
    <header class="header">
      <h1>{{ t('settings') }}</h1>
      <button class="nav-btn" @click="router.push({ name: 'order' })">{{ t('back') }}</button>
    </header>
    <div class="content">
      <!-- UX Guide §9: training mode toggle — violet accent -->
      <div class="setting-row" :class="{ active: pos.trainingMode }">
        <div>
          <h3>{{ t('training_mode') }}</h3>
          <p>{{ t('training_desc') }}</p>
        </div>
        <button class="toggle" :class="{ on: pos.trainingMode }" @click="toggleTraining">
          <span class="track"><span class="thumb"></span></span>
          {{ pos.trainingMode ? t('active') : t('inactive') }}
        </button>
      </div>

      <!-- Language switcher -->
      <div class="setting-row">
        <div>
          <h3>{{ t('language') }}</h3>
          <p>{{ t('language_desc') }}</p>
        </div>
        <div class="lang-btns">
          <button
            v-for="l in languages"
            :key="l.code"
            class="lang-btn"
            :class="{ active: pos.language === l.code }"
            @click="switchLanguage(l.code)"
          >{{ l.label }}</button>
        </div>
      </div>

      <!-- Workers management -->
      <div class="workers-section">
        <h3>{{ t('employees') }}</h3>
        <div v-for="w in workers" :key="w.insz" class="worker-row">
          <span class="worker-name">{{ w.name }}</span>
          <span class="worker-meta">{{ t(`role_${w.role}`) }} · {{ w.insz }}</span>
          <button
            v-if="w.insz !== auth.currentUser?.insz"
            class="deactivate-btn"
            :class="{ armed: confirmDeactivate === w.insz }"
            @click="deactivate(w.insz)"
          >{{ confirmDeactivate === w.insz ? t('confirm_deactivate') : t('deactivate') }}</button>
        </div>

        <div class="add-worker">
          <h4>{{ t('add') }}</h4>
          <div class="add-fields">
            <input v-model="newName" class="field" :placeholder="t('settings_name_placeholder')" />
            <input v-model="newInsz" class="field" :placeholder="t('settings_insz_placeholder')" maxlength="11" inputmode="numeric" />
            <div class="role-btns" role="radiogroup" :aria-label="t('role')">
              <button
                v-for="r in roles"
                :key="r"
                class="role-btn"
                role="radio"
                :aria-checked="newRole === r"
                :class="{ active: newRole === r }"
                @click="newRole = r"
              >{{ t(`role_${r}`) }}</button>
            </div>
          </div>
          <p v-if="addError" class="add-error">{{ addError }}</p>
          <button class="add-btn" @click="addWorker">{{ t('add') }}</button>
        </div>
      </div>

      <div v-if="pos.settings" class="info-section">
        <h3>{{ t('info') }}</h3>
        <div class="info-row"><span>{{ t('settings_pos_id_label') }}</span><span>{{ pos.settings.posId }}</span></div>
        <div class="info-row"><span>{{ t('settings_terminal_label') }}</span><span>{{ pos.settings.terminalId }}</span></div>
        <div class="info-row"><span>{{ t('settings_restaurant_label') }}</span><span>{{ pos.settings.restaurantName }}</span></div>
        <div class="info-row"><span>{{ t('settings_fdm_label') }}</span><span>{{ pos.settings.fdmUrl }}</span></div>
        <div class="info-row"><span>{{ t('settings_printer_label') }}</span><span>{{ pos.settings.printerAddress ?? t('printer_none') }}</span></div>
      </div>

      <component :is="HubriseSettings" v-if="HubriseSettings" />
    </div>
  </div>
</template>

<style scoped>
.settings { display: flex; flex-direction: column; height: 100vh; background: var(--bg-primary); }
.header { display: flex; align-items: center; justify-content: space-between; padding: 0 var(--space-md); background: var(--bg-secondary); border-bottom: 1px solid rgba(160,160,176,0.1); height: 48px; }
.header h1 { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); }
.nav-btn { padding: var(--space-xs) var(--space-md); font-size: var(--font-size-xs); font-weight: var(--font-weight-bold); border-radius: var(--radius-sm); background: var(--bg-surface); color: var(--text-secondary); min-height: 36px; transition: transform var(--tap-duration) ease; }
.nav-btn:active { transform: scale(var(--tap-scale)); }

.content { flex: 1; padding: var(--space-lg); max-width: 600px; margin: 0 auto; width: 100%; overflow-y: auto; -webkit-overflow-scrolling: touch; }

.setting-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--space-lg); background: var(--bg-secondary); border-radius: var(--radius-lg);
  margin-bottom: var(--space-md); border: 2px solid transparent;
}
.setting-row.active { border-color: var(--accent-training); background: rgba(155,89,182,0.08); }
.setting-row h3 { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); }
.setting-row p { font-size: var(--font-size-sm); color: var(--text-secondary); }

.toggle { display: flex; align-items: center; gap: var(--space-sm); font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); color: var(--text-secondary); }
.toggle.on { color: var(--accent-training); }

.track { width: 52px; height: 28px; border-radius: 14px; background: var(--bg-surface); position: relative; }
.toggle.on .track { background: var(--accent-training); }
.thumb { width: 22px; height: 22px; border-radius: 50%; background: var(--text-on-accent); position: absolute; top: 3px; left: 3px; transition: transform var(--tap-duration) ease; }
.toggle.on .thumb { transform: translateX(24px); }

.lang-btns { display: flex; gap: var(--space-xs); }
.lang-btn {
  padding: var(--space-sm) var(--space-md); min-width: 56px; height: 48px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md); background: var(--bg-surface); color: var(--text-secondary);
  transition: transform var(--tap-duration) ease;
}
.lang-btn:active { transform: scale(var(--tap-scale)); }
.lang-btn.active { background: var(--accent-primary); color: var(--text-on-accent); }
.role-btns { display: flex; gap: var(--space-xs); }
.role-btn {
  padding: var(--space-sm) var(--space-md); min-width: 56px; height: 48px;
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md); background: var(--bg-surface); color: var(--text-secondary);
  transition: transform var(--tap-duration) ease;
}
.role-btn.active { background: var(--accent-primary); color: var(--text-on-accent); }

.workers-section { margin-top: var(--space-xl); }
.workers-section h3 { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); color: var(--text-secondary); margin-bottom: var(--space-md); }
.worker-row { display: flex; justify-content: space-between; align-items: center; padding: var(--space-sm) 0; border-bottom: 1px solid rgba(160,160,176,0.1); }
.deactivate-btn { margin-left: auto; min-height: 48px; padding: 0 var(--space-md); border-radius: var(--radius-md); background: var(--bg-surface); color: var(--text-secondary); font-weight: var(--font-weight-bold); }
.deactivate-btn.armed { background: var(--accent-danger); color: var(--text-on-accent); }
.worker-name { font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); }
.worker-meta { font-size: var(--font-size-xs); color: var(--text-secondary); }

.add-worker { margin-top: var(--space-md); padding: var(--space-md); background: var(--bg-secondary); border-radius: var(--radius-md); }
.add-worker h4 { font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); margin-bottom: var(--space-sm); }
.add-fields { display: flex; flex-direction: column; gap: var(--space-sm); }
.field { padding: var(--space-sm) var(--space-md); font-size: var(--font-size-sm); border-radius: var(--radius-sm); border: 1px solid rgba(160,160,176,0.2); background: var(--bg-surface); color: var(--text-primary); height: 48px; }
.add-error { color: var(--accent-danger); font-size: var(--font-size-xs); margin-top: var(--space-xs); }
.add-btn { margin-top: var(--space-sm); width: 100%; height: 52px; font-size: var(--font-size-md); font-weight: var(--font-weight-bold); border-radius: var(--radius-md); background: var(--accent-primary); color: var(--text-on-accent); transition: transform var(--tap-duration) ease; }
.add-btn:active { transform: scale(var(--tap-scale)); }

.info-section { margin-top: var(--space-xl); }
.info-section h3 { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); color: var(--text-secondary); margin-bottom: var(--space-md); }
.info-row { display: flex; justify-content: space-between; padding: var(--space-sm) 0; border-bottom: 1px solid rgba(160,160,176,0.1); font-size: var(--font-size-sm); }
.info-row span:first-child { color: var(--text-secondary); }

</style>
