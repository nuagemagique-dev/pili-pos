<script setup lang="ts">
import { formatDate, formatDayTime, formatTime, formatTimestamp } from "../utils/datetime";
import { useIntegrationsStore } from "../stores/integrations";
import IntegrationSwitch from "../components/IntegrationSwitch.vue";
import { computed, defineAsyncComponent, onMounted, ref, watch } from "vue";
import { usePosStore } from "../stores/pos";
import { useAuthStore } from "../stores/auth";
import { useToastStore } from "../stores/toast";
import { useMenuStore } from "../stores/menu";
import { useDeliveryStore } from "../stores/delivery";
import type { AppLanguage, ProductLabel } from "../stores/pos";
import type { DeliveryView, User } from "../types";
import * as ipc from "../api/ipc";
import AppHeader from "../components/AppHeader.vue";
import PinDialog from "../components/PinDialog.vue";
import ExportStatusPanel from "../components/ExportStatusPanel.vue";
import BadgeSettings from "../components/BadgeSettings.vue";
import BadgeEnrolDialog from "../components/BadgeEnrolDialog.vue";
import { useBadgeStore } from "../stores/badge";
import { hasMessage, useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";
import { useSupportStore } from "../stores/support";
import { useFdmStore } from "../stores/fdm";

// The HubRise panel, while the integration is shown (its own chunk).
const HubriseSettings = defineAsyncComponent(() => import("../components/HubriseSettings.vue"));
const integrations = useIntegrationsStore();
const { t } = useI18n();

const pos = usePosStore();
const auth = useAuthStore();
const toast = useToastStore();
const fdm = useFdmStore();
const badge = useBadgeStore();

/** The backend refuses the switch while an order is open. */
async function toggleTraining() {
  try {
    await pos.toggleTrainingMode();
  } catch (e) {
    toast.show(describeError(e), "error", 6000);
  }
}
const menu = useMenuStore();
const delivery = useDeliveryStore();

// Remote support: started with the manager's PIN, stopped here.
const support = useSupportStore();
const confirmSupport = ref(false);

async function startSupport(pin: string) {
  await support.start(auth.currentUser?.insz ?? "", pin);
}

function onSupportStarted() {
  confirmSupport.value = false;
  toast.show(t("support_started"), "success");
}

async function stopSupport() {
  try {
    await support.stop();
    toast.show(t("support_stopped"), "success");
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  }
}

function supportSince(): string {
  const s = support.status;
  if (!s?.startedAt) return "";
  return t("support_running_since", {
    time: formatTimestamp(s.startedAt, formatDayTime),
    name: s.startedBy ?? "",
  });
}

const workers = ref<User[]>([]);
// The employee whose PIN the manager is setting (PinDialog open).
const pinFor = ref<User | null>(null);
// Another employee's PIN, or a new manager's first PIN: chosen first,
// then confirmed with the manager's own PIN.
const chosenPin = ref<string | null>(null);
const newManager = ref(false);

function openPin(w: User) {
  chosenPin.value = null;
  pinFor.value = w;
}

function closePin() {
  pinFor.value = null;
  newManager.value = false;
  chosenPin.value = null;
}

async function confirmWithManagerPin(operatorPin: string) {
  const pin = chosenPin.value ?? "";
  if (newManager.value) {
    await ipc.createUser({
      insz: newInsz.value,
      name: newName.value,
      role: "admin",
      initialPin: pin,
      operatorPin,
    });
    newName.value = "";
    newInsz.value = "";
  } else if (pinFor.value) {
    await ipc.setPin(pinFor.value.insz, pin, operatorPin);
  }
}

const ownPin = computed(() => pinFor.value?.insz === auth.currentUser?.insz);
const newName = ref("");
const newInsz = ref("");
const newRole = ref<"admin" | "cashier" | "waiter" | "presence">("cashier");
// PROFILS_UTILISATEURS_FR.md §1: Gérant, Serveur, Apprenti, and the
// presence-only profile (a waiter row that cannot log in).
const roles = ["admin", "cashier", "waiter", "presence"] as const;
const addError = ref<string | null>(null);

// Deactivation takes two taps: the first arms the row's button.
const confirmDeactivate = ref<string | null>(null);

// Open shifts by INSZ (the local index of S events), with the start time.
const openShifts = ref<Record<string, string>>({});
// The INSZ whose presence call is running (one at a time).
const presenceBusy = ref<string | null>(null);

function roleLabel(w: User): string {
  return w.presence_only ? t("role_presence") : t(`role_${w.role}`);
}

function shiftLabel(w: User): string {
  const since = openShifts.value[w.insz];
  if (!since) return t("settings_off_shift");
  return t("settings_on_shift_since", { time: formatTimestamp(since, formatTime) });
}

// Why the presence buttons are off, or null when they work.
const presenceBlock = computed<string | null>(() => {
  if (pos.trainingMode) return describeError("presence_training");
  if (!fdm.connected) return t("settings_presence_fdm_down");
  return null;
});

async function togglePresence(w: User) {
  if (presenceBlock.value || presenceBusy.value) return;
  const clockIn = !openShifts.value[w.insz];
  presenceBusy.value = w.insz;
  try {
    const r = clockIn ? await ipc.clockInEmployee(w.insz) : await ipc.clockOutEmployee(w.insz);
    if (r.success) {
      toast.show(t(clockIn ? "settings_presence_done_in" : "settings_presence_done_out", { name: w.name }), "success");
    } else {
      toast.show(describeError(r.error), "error", 6000);
    }
  } catch (e) {
    toast.show(describeError(e), "error", 6000);
  } finally {
    presenceBusy.value = null;
    await loadWorkers();
  }
}

// ── Badges (managers, while the reader is on) ──────────────────────
// The employee a badge is being linked to (capture + PIN dialog open),
// and the one whose badge removal waits for confirmation.
const enrolFor = ref<User | null>(null);
const revokeFor = ref<User | null>(null);

const showBadges = computed(() => auth.isManager && badge.readerOn);

function badgeOf(w: User) {
  return badge.overview?.badges.find((b) => b.insz === w.insz) ?? null;
}

function badgeLabel(w: User): string {
  const b = badgeOf(w);
  return b
    ? t("badge_linked_on", { date: formatTimestamp(b.linkedAt, formatDate) })
    : t("badge_none_label");
}

function onBadgeButton(w: User) {
  if (badgeOf(w)) revokeFor.value = w;
  else enrolFor.value = w;
}

async function confirmRevoke() {
  const w = revokeFor.value;
  revokeFor.value = null;
  if (!w) return;
  try {
    await badge.revoke(w.insz);
    toast.show(t("badge_revoked_done", { name: w.name }), "success");
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
  }
}

// A badge scan registered someone's arrival or departure.
watch(() => badge.presenceVersion, () => void loadWorkers());

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
    addError.value = describeError(e);
  }
}

async function loadWorkers() {
  try {
    const [users, shifts] = await Promise.all([ipc.listUsers(), ipc.listOpenShifts()]);
    workers.value = users;
    openShifts.value = Object.fromEntries(shifts.map((s) => [s.userInsz, s.startedAt]));
  } catch { /* empty */ }
}

async function addWorker() {
  addError.value = null;
  if (!newName.value || !newInsz.value) {
    addError.value = t("settings_err_name_insz_required");
    return;
  }
  if (newRole.value === "admin") {
    chosenPin.value = null;
    newManager.value = true;
    return;
  }
  try {
    await ipc.createUser(
      newRole.value === "presence"
        ? { insz: newInsz.value, name: newName.value, role: "waiter", presenceOnly: true }
        : { insz: newInsz.value, name: newName.value, role: newRole.value },
    );
    newName.value = "";
    newInsz.value = "";
    await loadWorkers();
  } catch (e) {
    addError.value = describeError(e);
  }
}

/** First letter of a name for the avatar (handles surrogate pairs). */
function initial(name: string): string {
  return Array.from(name.trim())[0]?.toUpperCase() ?? "?";
}

// French first: it is the default UI language.
const languages: AppLanguage[] = ["fr", "zh", "en"];

const productLabels: ProductLabel[] = ["name", "code"];

// ── Table plan (managers) ────────────────────────────────────────
const tablePlan = ref<ipc.TablePlan | null>(null);
const tablePlanInput = ref("");
const tablePlanError = ref<string | null>(null);

async function loadTablePlan() {
  try {
    tablePlan.value = await ipc.getTablePlan();
    tablePlanInput.value = tablePlan.value.plan;
  } catch { /* the panel stays empty */ }
}

async function saveTablePlan() {
  tablePlanError.value = null;
  try {
    tablePlan.value = await ipc.setTablePlan(tablePlanInput.value);
    tablePlanInput.value = tablePlan.value.plan;
    toast.show(t("settings_tables_saved"), "success");
  } catch (e) {
    tablePlanError.value = describeError(e);
  }
}

async function switchLanguage(lang: AppLanguage) {
  pos.setLanguage(lang);
  await menu.reloadWithLanguage();
}

// ── Digital tickets (admin) ──────────────────────────────────────
const deliveries = ref<DeliveryView[]>([]);
const deliveriesLoading = ref(false);

async function loadDeliveries() {
  deliveriesLoading.value = true;
  try {
    deliveries.value = await ipc.listTicketDeliveries(50);
  } catch {
    deliveries.value = [];
  } finally {
    deliveriesLoading.value = false;
  }
}

function formatDeliveryDate(iso: string): string {
  return formatTimestamp(iso, formatDayTime);
}

/** "Worldline", "CCV cloud", "CCV réseau local", or none. */
const paymentTerminalLabel = computed(() => {
  const kind = pos.settings?.paymentTerminal;
  if (!kind) return t("settings_payment_terminal_none");
  return hasMessage(`eft_kind_${kind}`) ? t(`eft_kind_${kind}`) : kind;
});

onMounted(() => {
  pos.loadSettings();
  loadWorkers();
  delivery.refreshMailStatus();
  loadDeliveries();
  void support.refresh();
  if (auth.isManager) {
    void loadTablePlan();
    void badge.load().then(() => badge.loadOverview());
  }
});
</script>

<template>
  <div class="settings">
    <AppHeader />

    <div class="grid">
      <!-- Column 1: training mode, language, remote support -->
      <div class="column">
        <!-- UX Guide §9: training mode toggle, effect described next to it -->
        <section class="panel training-panel" :class="{ active: pos.trainingMode }">
          <div class="panel-head">
            <h2>{{ t('training_mode') }}</h2>
            <button class="toggle" :class="{ on: pos.trainingMode }" @click="toggleTraining">
              {{ pos.trainingMode ? t('settings_training_on') : t('settings_training_off') }}
              <span class="track"><span class="thumb"></span></span>
            </button>
          </div>
          <p class="panel-text">{{ t('settings_training_effect') }}</p>
        </section>

        <section class="panel">
          <h2>{{ t('language') }}</h2>
          <div class="segmented lang-btns">
            <button
              v-for="code in languages"
              :key="code"
              class="lang-btn"
              :class="{ active: pos.language === code }"
              @click="switchLanguage(code)"
            >{{ t(`settings_lang_name_${code}`) }}</button>
          </div>
          <p class="panel-note">{{ t('settings_language_note') }}</p>
        </section>

        <section class="panel">
          <h2>{{ t('settings_product_label') }}</h2>
          <div class="segmented label-btns">
            <button
              v-for="label in productLabels"
              :key="label"
              class="label-btn"
              :class="{ active: pos.productLabel === label }"
              @click="pos.setProductLabel(label)"
            >{{ t(`settings_product_label_${label}`) }}</button>
          </div>
          <p class="panel-note">{{ t('settings_product_label_note') }}</p>
        </section>

        <!-- Table plan (manager only; the engine refuses the change to the others) -->
        <section v-if="auth.isManager" class="panel table-plan">
          <h2>{{ t('settings_tables') }}</h2>
          <p v-if="tablePlan" class="table-plan-current">
            {{ t('settings_tables_current', { count: tablePlan.table_count, plan: tablePlan.plan }) }}
          </p>
          <div class="add-fields">
            <input
              v-model="tablePlanInput"
              class="field mono"
              :placeholder="t('settings_tables_placeholder')"
              @keyup.enter="saveTablePlan"
            />
            <button class="add-btn table-plan-save" @click="saveTablePlan">{{ t('settings_tables_save') }}</button>
          </div>
          <p v-if="tablePlanError" class="add-error">{{ tablePlanError }}</p>
          <p class="panel-note">{{ t('settings_tables_note') }}</p>
        </section>

        <!-- Nightly export (manager only; the engine refuses it to the others) -->
        <ExportStatusPanel v-if="auth.isManager" class="panel" />

        <!-- Remote support (manager) -->
        <section class="panel panel-grow support-section">
          <h2>{{ t('support_title') }}</h2>
          <div class="pills">
            <span v-if="support.status && !support.status.available" class="pill">
              <span class="dot"></span>{{ t('settings_support_unavailable') }}
            </span>
            <span v-else-if="support.status?.running" class="pill pill-danger">
              <span class="dot"></span>{{ t('settings_support_running') }}
            </span>
            <span v-else class="pill">
              <span class="dot"></span>{{ t('settings_support_idle') }}
            </span>
          </div>
          <p class="panel-text">{{ t('settings_support_banner_note') }}</p>
          <p class="panel-text">{{ t('support_desc') }}</p>
          <div class="spacer"></div>
          <p v-if="support.status && !support.status.available" class="support-state">
            {{ t('support_not_installed') }}
          </p>
          <template v-else-if="support.status?.running">
            <p class="support-state running">{{ supportSince() }}</p>
            <button class="support-btn stop" @click="stopSupport">{{ t('support_stop') }}</button>
          </template>
          <button v-else class="support-btn start" @click="confirmSupport = true">
            {{ t('support_start') }}
          </button>
        </section>
      </div>

      <!-- Column 2: employees -->
      <section class="panel workers-section">
        <h2>{{ t('employees') }}</h2>
        <p class="panel-text">{{ t('settings_presence_note') }}</p>
        <p v-if="auth.isManager && presenceBlock" class="presence-block" data-testid="presence-block">{{ presenceBlock }}</p>
        <div class="worker-list">
          <div v-for="w in workers" :key="w.insz" class="worker-row">
            <span class="avatar">{{ initial(w.name) }}</span>
            <span class="worker-text">
              <span class="worker-name">{{ w.name }}</span>
              <span v-if="w.presence_only" class="worker-meta">{{ roleLabel(w) }}</span>
              <span v-else class="worker-meta" :class="{ 'no-pin': !w.has_pin }">
                {{ roleLabel(w) }} · {{ w.has_pin ? t('settings_pin_set') : t('settings_pin_missing') }}
              </span>
              <span class="worker-presence" :class="{ on: !!openShifts[w.insz] }" data-testid="presence-state">{{ shiftLabel(w) }}</span>
              <span v-if="showBadges" class="worker-badge">
                <span class="worker-presence" :class="{ on: !!badgeOf(w) }" data-testid="badge-state">{{ badgeLabel(w) }}</span>
                <button class="badge-btn" data-testid="badge-btn" @click="onBadgeButton(w)">{{ badgeOf(w) ? t('badge_revoke') : t('badge_link') }}</button>
              </span>
            </span>
            <span class="worker-actions">
              <button
                v-if="auth.isManager && w.insz !== auth.currentUser?.insz"
                class="presence-btn"
                data-testid="presence-btn"
                :disabled="!!presenceBlock || presenceBusy !== null"
                @click="togglePresence(w)"
              >{{ openShifts[w.insz] ? t('settings_presence_out') : t('settings_presence_in') }}</button>
              <button v-if="!w.presence_only" class="pin-btn" @click="openPin(w)">{{ t('settings_pin_button') }}</button>
              <button
                v-if="w.insz !== auth.currentUser?.insz"
                class="deactivate-btn"
                :class="{ armed: confirmDeactivate === w.insz }"
                @click="deactivate(w.insz)"
              >{{ confirmDeactivate === w.insz ? t('confirm_deactivate') : t('deactivate') }}</button>
            </span>
          </div>
        </div>

        <div class="add-worker">
          <div class="section-label">{{ t('add') }}</div>
          <div class="add-fields">
            <input v-model="newName" class="field" :placeholder="t('settings_name_placeholder')" />
            <input v-model="newInsz" class="field mono" :placeholder="t('settings_insz_hint')" maxlength="11" inputmode="numeric" />
          </div>
          <div class="segmented role-btns" role="radiogroup" :aria-label="t('role')">
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
          <p v-if="addError" class="add-error">{{ addError }}</p>
          <button class="add-btn" @click="addWorker">{{ t('settings_add_employee') }}</button>
        </div>
      </section>

      <!-- Column 3: info, email tickets, HubRise -->
      <div class="column">
        <section class="panel info-section">
          <h2>{{ t('info') }}</h2>
          <div v-if="pos.settings" class="info-rows">
            <div class="info-row"><span>{{ t('settings_pos_id_label') }}</span><span class="info-value">{{ pos.settings.posId }}</span></div>
            <div class="info-row"><span>{{ t('settings_terminal_label') }}</span><span class="info-value">{{ pos.settings.terminalId }}</span></div>
            <div class="info-row"><span>{{ t('settings_restaurant_label') }}</span><span class="info-value">{{ pos.settings.restaurantName }}</span></div>
            <div class="info-row"><span>{{ t('settings_fdm_label') }}</span><span class="info-value">{{ pos.settings.fdmUrl }}</span></div>
            <div class="info-row"><span>{{ t('settings_printer_label') }}</span><span class="info-value">{{ pos.settings.printerAddress ?? t('printer_none') }}</span></div>
            <div class="info-row" data-testid="payment-terminal">
              <span>{{ t('settings_payment_terminal_label') }}</span>
              <span class="info-value">{{ paymentTerminalLabel }}</span>
            </div>
          </div>
        </section>

        <!-- Digital tickets (admin) - never shows the address itself -->
        <IntegrationSwitch class="panel" />

        <section class="panel deliveries-section" :class="{ 'panel-grow': !integrations.hubriseVisible }">
          <div class="panel-head">
            <h2>{{ t('settings_email_title') }}</h2>
            <button class="refresh-btn" :disabled="deliveriesLoading" @click="loadDeliveries">
              {{ t('delivery_settings_refresh') }}
            </button>
          </div>
          <div class="pills mail-status-row">
            <span
              class="pill mail-status-pill"
              :class="delivery.mailStatus?.configured ? 'mail-status-ok' : 'mail-status-off'"
            ><span class="dot"></span>{{ delivery.mailStatus?.configured ? t('delivery_settings_configured') : t('delivery_settings_not_configured') }}</span>
            <span
              v-if="delivery.mailStatus?.configured"
              class="pill mail-status-pill"
              :class="delivery.mailStatus?.reachable ? 'mail-status-ok' : 'mail-status-err'"
            ><span class="dot"></span>{{ delivery.mailStatus?.reachable ? t('delivery_settings_reachable') : t('delivery_settings_unreachable') }}</span>
          </div>

          <p v-if="deliveries.length === 0" class="panel-note empty">{{ t('delivery_settings_empty') }}</p>
          <ul v-else class="delivery-list">
            <li v-for="d in deliveries" :key="d.id" class="delivery-row">
              <div class="delivery-row-main">
                <span class="delivery-hint">{{ d.recipientHint }}</span>
                <span class="delivery-status-badge" :class="`delivery-status-${d.status}`">
                  {{ t(`delivery_status_${d.status}`) }}
                </span>
              </div>
              <div class="delivery-row-meta">
                <span>{{ formatDeliveryDate(d.createdAt) }}</span>
                <span>{{ t('delivery_attempts') }}: {{ d.attempts }}</span>
              </div>
              <p v-if="d.lastError" class="delivery-error">{{ t('delivery_last_error') }}: {{ d.lastError }}</p>
            </li>
          </ul>
        </section>

        <BadgeSettings v-if="auth.isManager" />

        <HubriseSettings v-if="integrations.hubriseVisible" class="panel panel-grow" />
      </div>
    </div>

    <BadgeEnrolDialog
      v-if="enrolFor"
      :insz="enrolFor.insz"
      :name="enrolFor.name"
      @done="enrolFor = null"
      @cancel="enrolFor = null"
    />

    <div v-if="revokeFor" class="revoke-backdrop" data-testid="badge-revoke-dialog" @click.self="revokeFor = null">
      <div class="revoke-dialog" role="dialog" :aria-label="t('badge_revoke')">
        <p class="revoke-text">{{ t('badge_revoke_confirm', { name: revokeFor.name }) }}</p>
        <div class="revoke-actions">
          <button class="pin-btn" @click="revokeFor = null">{{ t('back') }}</button>
          <button class="deactivate-btn armed" data-testid="badge-revoke-confirm" @click="confirmRevoke">{{ t('badge_revoke') }}</button>
        </div>
      </div>
    </div>

    <PinDialog
      v-if="confirmSupport && auth.currentUser"
      :insz="auth.currentUser.insz"
      :user-name="auth.currentUser.name"
      :verify="startSupport"
      @done="onSupportStarted"
      @cancel="confirmSupport = false"
    />

    <PinDialog
      v-if="pinFor && ownPin"
      :insz="pinFor.insz"
      :user-name="pinFor.name"
      @done="closePin(); loadWorkers()"
      @cancel="closePin"
    />
    <PinDialog
      v-else-if="(pinFor || newManager) && chosenPin === null"
      :insz="pinFor?.insz ?? newInsz"
      :user-name="pinFor?.name ?? newName"
      :save="(pin: string) => { chosenPin = pin; }"
      @cancel="closePin"
    />
    <PinDialog
      v-else-if="(pinFor || newManager) && auth.currentUser"
      :insz="auth.currentUser.insz"
      :user-name="auth.currentUser.name"
      :hint="t('pin_confirm_manager')"
      :verify="confirmWithManagerPin"
      @done="closePin(); loadWorkers()"
      @cancel="closePin"
    />
  </div>
</template>

<style scoped>
.settings { display: flex; flex-direction: column; flex: 1; min-height: 0; background: var(--bg-primary); }

/* §12: three columns, no page scrolling; a panel scrolls inside if needed. */
.grid {
  flex: 1; min-height: 0;
  display: grid; grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 24px; padding: 24px; overflow: hidden;
}
.column { display: flex; flex-direction: column; gap: 24px; min-height: 0; }

.panel {
  display: flex; flex-direction: column; gap: 16px; min-height: 0;
  padding: 24px; background: var(--bg-secondary);
  border: 1px solid var(--border); border-radius: var(--radius-lg);
}
.panel-grow { flex: 1; overflow-y: auto; -webkit-overflow-scrolling: touch; }
.panel h2 { font-size: 28px; font-weight: var(--font-weight-heavy); line-height: 1.2; }
.panel-head { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
.panel-text { font-size: 16px; color: var(--text-secondary); text-wrap: pretty; }
.panel-note { font-size: 15px; color: var(--text-secondary); }
.spacer { flex: 1; }

.section-label {
  font-family: var(--font-mono); font-size: 14px; font-weight: var(--font-weight-bold);
  color: var(--text-secondary); letter-spacing: 0.08em; text-transform: uppercase;
}

/* Status pills */
.pills { display: flex; gap: 8px; flex-wrap: wrap; }
.pill {
  display: inline-flex; align-items: center; gap: 8px;
  height: 36px; padding: 0 14px; border-radius: 18px; border: 1px solid var(--border);
  font-family: var(--font-mono); font-size: 14px; font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
.pill .dot { width: 8px; height: 8px; border-radius: 50%; background: currentColor; }
.pill-danger { color: var(--accent-danger); }

/* ── Training mode (§9) ─────────────────────────────────────── */
.training-panel.active { border-color: var(--accent-training); }
.toggle {
  display: flex; align-items: center; gap: 12px; flex-shrink: 0;
  height: 56px; padding: 0 8px 0 16px; border-radius: 28px;
  background: var(--bg-surface); color: var(--text-secondary);
  font-size: 16px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.toggle.on { color: var(--text-primary); }
.toggle:active { transform: scale(var(--tap-scale)); }
.track { width: 64px; height: 40px; border-radius: 20px; background: var(--border); position: relative; display: block; }
.toggle.on .track { background: var(--accent-training); }
.thumb {
  position: absolute; left: 4px; top: 4px; width: 32px; height: 32px; border-radius: 50%;
  background: var(--text-secondary); transition: transform var(--tap-duration) ease;
}
.toggle.on .thumb { transform: translateX(24px); background: var(--text-primary); }

/* ── Segmented controls (language, product label, role) ─────── */
.segmented {
  display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px; padding: 4px;
  background: var(--bg-surface); border-radius: 12px;
}
.label-btns { grid-template-columns: repeat(2, 1fr); }
.role-btns { grid-template-columns: repeat(4, 1fr); }
.lang-btn, .label-btn, .role-btn {
  border-radius: var(--radius-sm); font-weight: var(--font-weight-bold); color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.lang-btn, .label-btn { height: 56px; font-size: 18px; }
.role-btn { height: 48px; font-size: 15px; padding: 0 4px; }
.lang-btn.active, .label-btn.active, .role-btn.active { background: var(--selected-bg); color: var(--selected-fg); }
.lang-btn:active, .label-btn:active, .role-btn:active { transform: scale(var(--tap-scale)); }

/* ── Remote support ─────────────────────────────────────────── */
.support-state { font-size: 16px; font-weight: var(--font-weight-bold); color: var(--text-secondary); }
.support-state.running { color: var(--text-primary); font-family: var(--font-mono); }
.support-btn {
  height: 64px; padding: 0 20px; border-radius: var(--radius-md);
  font-size: 17px; font-weight: var(--font-weight-bold);
  background: var(--bg-surface); color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.support-btn.stop { background: transparent; border: 2px solid var(--accent-danger); color: var(--accent-danger); }
.support-btn:active { transform: scale(var(--tap-scale)); }

/* ── Employees ──────────────────────────────────────────────── */
.worker-list { flex: 1; min-height: 0; overflow-y: auto; -webkit-overflow-scrolling: touch; }
.worker-row {
  display: grid; grid-template-columns: 48px minmax(0, 1fr) auto; gap: 16px; align-items: center;
  min-height: 72px; border-bottom: 1px solid var(--border);
}
.avatar {
  width: 48px; height: 48px; border-radius: 50%; background: var(--bg-surface);
  display: flex; align-items: center; justify-content: center;
  font-size: 20px; font-weight: var(--font-weight-heavy);
}
.worker-text { display: flex; flex-direction: column; min-width: 0; }
.worker-name { font-size: 19px; font-weight: var(--font-weight-bold); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.worker-meta { font-size: 15px; color: var(--text-secondary); }
.worker-meta.no-pin { color: var(--accent-warning); }
.worker-presence { font-size: 15px; color: var(--text-secondary); }
.worker-presence.on { color: var(--text-primary); font-weight: var(--font-weight-bold); }
.presence-block { color: var(--accent-warning); font-size: 15px; }
.worker-badge .worker-presence { white-space: nowrap; }
.worker-badge { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin-top: 4px; }
.badge-btn {
  height: 40px; padding: 0 14px; border-radius: var(--radius-sm);
  background: var(--bg-surface); color: var(--text-primary);
  font-size: 14px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.badge-btn:active { transform: scale(var(--tap-scale)); }
.worker-actions { display: flex; gap: 8px; }
.presence-btn:disabled { opacity: 0.4; }
.presence-btn, .pin-btn, .deactivate-btn {
  height: 48px; padding: 0 16px; border-radius: var(--radius-sm);
  background: var(--bg-surface); font-size: 15px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.presence-btn, .pin-btn { color: var(--text-primary); }
.deactivate-btn { color: var(--text-secondary); }
.deactivate-btn.armed { background: var(--accent-danger); color: var(--text-on-accent); }
.presence-btn:active:not(:disabled), .pin-btn:active, .deactivate-btn:active { transform: scale(var(--tap-scale)); }

.revoke-backdrop {
  position: fixed; inset: 0; z-index: 50; display: flex; align-items: center; justify-content: center;
  background: rgba(0, 0, 0, 0.6);
}
.revoke-dialog {
  display: flex; flex-direction: column; gap: 24px; padding: 32px;
  border-radius: var(--radius-md); background: var(--bg-secondary);
}
.revoke-text { font-size: 22px; font-weight: var(--font-weight-bold); }
.revoke-actions { display: flex; gap: 12px; justify-content: flex-end; }

.add-worker { display: flex; flex-direction: column; gap: 12px; flex-shrink: 0; }
.add-fields { display: flex; gap: 8px; }
.field {
  flex: 1; min-width: 0; height: 56px; padding: 0 16px;
  border-radius: var(--radius-md); border: 1px solid var(--border);
  background: transparent; color: var(--text-primary); font-size: 17px;
}
.field::placeholder { color: var(--text-secondary); }
.field.mono { font-family: var(--font-mono); }
.field.mono::placeholder { font-family: var(--font-family); }
.add-error { color: var(--accent-danger); font-size: 15px; }
.add-btn {
  height: 64px; padding: 0 20px; border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-primary);
  font-size: 17px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.add-btn:active { transform: scale(var(--tap-scale)); }
.table-plan-current { font-size: 16px; color: var(--text-primary); }
.table-plan-save { height: 56px; }

/* ── Information ────────────────────────────────────────────── */
.info-section { flex-shrink: 0; }
.info-row {
  display: flex; justify-content: space-between; align-items: center; gap: 16px;
  min-height: 48px; border-bottom: 1px solid var(--border); font-size: 16px;
}
.info-row span:first-child { color: var(--text-secondary); flex-shrink: 0; }
.info-value { font-family: var(--font-mono); text-align: right; overflow-wrap: anywhere; }

/* ── Email tickets ──────────────────────────────────────────── */
.deliveries-section { flex-shrink: 1; max-height: 45%; }
.deliveries-section.panel-grow { max-height: none; }
.refresh-btn {
  height: 48px; padding: 0 16px; border-radius: var(--radius-sm);
  background: var(--bg-surface); color: var(--text-primary);
  font-size: 15px; font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.refresh-btn:active { transform: scale(var(--tap-scale)); }
.refresh-btn:disabled { opacity: 0.5; }
.mail-status-ok { color: var(--accent-success); }
.mail-status-off { color: var(--accent-warning); }
.mail-status-err { color: var(--accent-danger); }

.delivery-list {
  list-style: none; margin: 0; padding: 0; flex: 1 1 auto; min-height: 0; overflow-y: auto;
  -webkit-overflow-scrolling: touch; display: flex; flex-direction: column;
}
.delivery-row { padding: 12px 0; border-bottom: 1px solid var(--border); }
.delivery-row-main { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.delivery-hint { font-family: var(--font-mono); font-size: 15px; font-weight: var(--font-weight-bold); }
.delivery-status-badge {
  font-size: 14px; font-weight: var(--font-weight-bold); padding: 2px 10px;
  border-radius: var(--radius-sm); background: var(--bg-surface); color: var(--text-secondary);
}
.delivery-status-sent, .delivery-status-fallback_printed { color: var(--accent-success); }
.delivery-status-failed { color: var(--accent-danger); }
.delivery-status-sending, .delivery-status-pending { color: var(--accent-warning); }
.delivery-row-meta { display: flex; gap: 16px; font-family: var(--font-mono); font-size: 13px; color: var(--text-secondary); margin-top: 4px; }
.delivery-error { font-size: 13px; color: var(--accent-danger); margin-top: 4px; }
</style>
