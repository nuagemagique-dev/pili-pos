<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useIntegrationsStore } from "../stores/integrations";
import { useRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { usePosStore } from "../stores/pos";
import { useFdmStore } from "../stores/fdm";
import { useBadgeStore } from "../stores/badge";
import { goAfterLogin } from "../utils/postLogin";
import { useToastStore } from "../stores/toast";
import type { User } from "../types";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";
import { describeError, errorCode } from "../utils/errorMessages";
import { formatTimestamp, formatTime } from "../utils/datetime";
import PinDialog from "../components/PinDialog.vue";
import PiliLogo from "../components/PiliLogo.vue";

const router = useRouter();
const integrations = useIntegrationsStore();
const auth = useAuthStore();
const pos = usePosStore();
const fdm = useFdmStore();
const toast = useToastStore();
const badge = useBadgeStore();
const { t } = useI18n();

const savedUsers = ref<User[]>([]);
const shifts = ref<ipc.OpenShift[]>([]);
const showNumpad = ref(false);
const insz = ref("");
const error = ref<string | null>(null);

// ── PIN panel state ────────────────────────────────────────────
const pinMode = ref(false);
const pinUser = ref<User | null>(null);
const pin = ref("");
const pinError = ref<string | null>(null);
// The PIN ends the selected employee's shift instead of logging in.
const endShiftMode = ref(false);

// A badge scan while this mode is on clocks its holder out (plan L5-D7).
watch(endShiftMode, (on) => { badge.endShiftMode = on; }, { immediate: true });
onUnmounted(() => { badge.endShiftMode = false; });

// A manager without a PIN chooses one before logging in
// (commands/auth.rs::do_login_with_new_pin).
const firstPinUser = ref<{ insz: string; name: string } | null>(null);

function nameOf(insz: string): string {
  return savedUsers.value.find((u) => u.insz === insz)?.name ?? insz;
}

/** When the employee's open shift started, or null when off shift. */
function shiftStart(user: { insz: string }): string | null {
  return shifts.value.find((s) => s.userInsz === user.insz)?.startedAt ?? null;
}

function openPinOverlay(user: User) {
  pinUser.value = user;
  pinMode.value = true;
  pin.value = "";
  pinError.value = null;
  endShiftMode.value = false;
}

// A manager's badge never opens a session by itself: it selects the
// manager and opens their PIN pad (plan L5-D6).
watch(
  () => badge.pinRequest,
  (req) => {
    if (!req) return;
    badge.pinRequest = null;
    error.value = null;
    showNumpad.value = false;
    const known = savedUsers.value.find((u) => u.insz === req.insz);
    if (known) void quickLogin(known);
    else openPinOverlay({ insz: req.insz, name: req.name, role: "admin", active: true, has_pin: true });
  },
);

// A badge registered someone's arrival or departure: the tiles show it,
// and an open "Fin de service" pad has done its job.
watch(
  () => badge.presenceVersion,
  () => {
    if (endShiftMode.value) cancelPin();
    void loadShifts();
  },
);

function onFirstPinDone() {
  firstPinUser.value = null;
  void goAfterLogin(router);
}

async function loadShifts() {
  try {
    shifts.value = (await ipc.listOpenShifts()) ?? [];
  } catch {
    shifts.value = [];
  }
}

// ── Start-up checks (§2.3, §8.1): the devices, before the users ──
type Check = {
  key: string;
  label: string;
  state: "ok" | "warn" | "err" | "none";
  text: string;
  /** Why a device is not reachable (tooltip). */
  detail?: string;
};
const checks = ref<Check[]>([]);

// One run at a time: a slow device (the payment terminal waits for its
// answer) must not be probed again by the next tick.
let checking = false;

async function runChecks() {
  if (checking) return;
  checking = true;
  try {
    await probeDevices();
  } finally {
    checking = false;
  }
}

async function probeDevices() {
  const list: Check[] = [];

  await fdm.checkNow();
  list.push({
    key: "fdm",
    label: "FDM",
    state: fdm.severity === "blocking" ? "err" : fdm.severity === "warning" ? "warn" : "ok",
    text: fdm.severity === "blocking" ? t("check_offline") : fdm.severity === "warning" ? t("check_warning") : t("check_connected"),
  });

  const printer = await ipc.checkPrinter().catch(() => null);
  list.push({
    key: "ticket",
    label: t("check_ticket_printer"),
    state: printer == null ? "none" : printer ? "ok" : "err",
    text: printer == null ? t("check_not_configured") : printer ? t("check_ready") : t("check_unreachable"),
  });

  const kitchen = await ipc.checkKitchenPrinter().catch(() => null);
  list.push({
    key: "kitchen",
    label: t("check_kitchen_printer"),
    state: kitchen == null ? "none" : kitchen ? "ok" : "err",
    text: kitchen == null ? t("check_not_configured") : kitchen ? t("check_ready") : t("check_unreachable"),
  });

  // Informative like the printers: a terminal that does not answer never blocks the login.
  const terminal = await ipc.checkPaymentTerminal().catch(() => null);
  if (terminal?.configured) {
    list.push({
      key: "terminal",
      label: t("check_payment_terminal"),
      state: terminal.busy ? "warn" : terminal.connected ? "ok" : "err",
      text: terminal.busy ? t("check_payment_busy") : terminal.connected ? t("check_connected") : t("check_unreachable"),
      detail: terminal.detail ?? undefined,
    });
  }

  if (integrations.hubriseEnabled) {
    const hub = await ipc.hubriseGetConnectionStatus().catch(() => null);
    list.push({
      key: "hubrise",
      label: "HubRise",
      state: hub == null ? "none" : hub.connected ? "ok" : "warn",
      text: hub?.connected ? t("check_connected") : t("check_not_connected"),
    });
  }

  const mail = await ipc.checkMailStatus().catch(() => null);
  list.push({
    key: "mail",
    label: t("check_email"),
    state: !mail?.configured ? "none" : mail.reachable ? "ok" : "warn",
    text: !mail?.configured ? t("check_not_configured") : mail.reachable ? t("check_reachable") : t("check_unreachable"),
  });

  checks.value = list;
}

const footerPlace = computed(() =>
  [pos.settings?.restaurantName, pos.settings?.terminalId].filter(Boolean).join(" · "),
);

// A device that drops while the login screen stays open must not stay
// green: the list is checked again every 30 s, as the header lights are.
const CHECKS_EVERY_MS = 30_000;
let checksTimer: ReturnType<typeof setInterval> | undefined;
onUnmounted(() => clearInterval(checksTimer));

onMounted(async () => {
  void runChecks();
  checksTimer = setInterval(() => void runChecks(), CHECKS_EVERY_MS);
  void loadShifts();
  try {
    // Presence-only staff (kitchen) never log in: no tile for them.
    savedUsers.value = (await ipc.listUsers()).filter((u) => !u.presence_only);
    if (savedUsers.value.length === 0) {
      showNumpad.value = true;
    }
  } catch {
    showNumpad.value = true;
  }
});

// ── Quick login (tap a tile) ───────────────────────────────────
async function quickLogin(user: User) {
  error.value = null;
  if (user.has_pin) {
    // User has a PIN → show the PIN pad next to the tiles
    openPinOverlay(user);
    return;
  }
  if (user.role === "admin") {
    firstPinUser.value = { insz: user.insz, name: user.name };
    return;
  }
  if (shiftStart(user)) {
    // On shift without a PIN: offer to log in or to end the shift.
    openPinOverlay(user);
    return;
  }
  await loginWithoutPin(user);
}

async function loginWithoutPin(user: User) {
  // No PIN set → login directly + warning toast
  try {
    await auth.loginWithInsz(user.insz);
    toast.show(t("pin_not_set_warning"), "warning", 5000);
    void goAfterLogin(router);
  } catch (e) {
    if (errorCode(e) === "pin_setup_required") {
      firstPinUser.value = { insz: user.insz, name: user.name };
      return;
    }
    error.value = describeError(e);
  }
}

// ── PIN numpad ─────────────────────────────────────────────────
function handlePinKey(value: string) {
  if (value === "C") {
    pin.value = "";
  } else if (value === "BS") {
    pin.value = pin.value.slice(0, -1);
  } else if (pin.value.length < 4) {
    pin.value += value;
    // Auto-submit on 4th digit - no confirm button needed
    if (pin.value.length === 4) {
      submitPin();
    }
  }
}

async function submitPin() {
  if (!pinUser.value || pin.value.length !== 4) return;
  pinError.value = null;
  if (endShiftMode.value) {
    await finishShift(pinUser.value, pin.value);
    return;
  }
  try {
    await auth.loginWithInsz(pinUser.value.insz, pin.value);
    void goAfterLogin(router);
  } catch (e) {
    pinError.value = describeError(e);
    pin.value = "";
  }
}

/** Clock the employee out (S event) and stay on the login screen. */
async function finishShift(user: User, userPin?: string) {
  try {
    const done = await auth.endShift(user.insz, userPin);
    if (done) toast.show(t("shift_ended", { name: user.name }), "success");
    cancelPin();
    await loadShifts();
  } catch (e) {
    pinError.value = describeError(e);
    pin.value = "";
  }
}

function cancelPin() {
  pinMode.value = false;
  pinUser.value = null;
  pin.value = "";
  pinError.value = null;
  endShiftMode.value = false;
}

function toggleEndShift() {
  endShiftMode.value = !endShiftMode.value;
  pin.value = "";
  pinError.value = null;
}

function toggleInsz() {
  showNumpad.value = !showNumpad.value;
  insz.value = "";
  cancelPin();
}

// ── INSZ numpad (manual login) ─────────────────────────────────
function handleKey(value: string) {
  if (value === "C") { insz.value = ""; }
  else if (value === "BS") { insz.value = insz.value.slice(0, -1); }
  else if (insz.value.length < 11) { insz.value += value; }
}

async function submitInsz() {
  if (insz.value.length !== 11) {
    error.value = t("settings_err_11_digits_required");
    return;
  }
  error.value = null;
  try {
    // Manual INSZ login - no PIN (the backend will reject if PIN is
    // required, so the user must use the tile path for PIN-protected
    // accounts).
    await auth.loginWithInsz(insz.value);
    void goAfterLogin(router);
  } catch (e) {
    const code = errorCode(e);
    if (code === "pin_setup_required") {
      firstPinUser.value = { insz: insz.value, name: nameOf(insz.value) };
    } else if (code === "pin_required") {
      const known = savedUsers.value.find((u) => u.insz === insz.value);
      openPinOverlay(
        known ?? { insz: insz.value, name: insz.value, role: "waiter", active: true, has_pin: true },
      );
    } else {
      error.value = describeError(e);
    }
  }
}
</script>

<template>
  <div class="login">
    <!-- Left: the brand and the start-up checks (§2.3, §8.1). -->
    <section class="brand">
      <div class="brand-mark">
        <PiliLogo :height="120" color="var(--text-primary)" mark-color="var(--accent-primary)" />
        <span class="hanzi" lang="zh">霹雳</span>
      </div>
      <div class="checks">
        <div class="checks-label">{{ t('check_title') }}</div>
        <div v-for="c in checks" :key="c.key" class="check" :class="c.state" :title="c.detail">
          <span class="dot" />
          <span class="check-label">{{ c.label }}</span>
          <span class="check-text">{{ c.text }}</span>
        </div>
        <div class="brand-footer">
          <span>{{ footerPlace }}</span>
          <span>{{ t('by_nuage_magique') }}</span>
        </div>
        <div v-if="pos.settings?.posSwVersion" class="sw-version">
          {{ t('sw_version') }} {{ pos.settings.posSwVersion }}
        </div>
      </div>
    </section>

    <!-- Right: who is it, and their PIN, on the same screen. -->
    <section class="people">
      <div class="people-main">
        <h1 class="title">{{ t('who_are_you') }}</h1>
        <p v-if="error" class="error-msg shake">{{ error }}</p>

        <div v-if="!showNumpad && savedUsers.length > 0" class="user-grid">
          <button
            v-for="u in savedUsers"
            :key="u.insz"
            class="user-btn"
            :class="{ selected: pinUser?.insz === u.insz }"
            @click="quickLogin(u)"
          >
            <span class="user-initial">{{ u.name.charAt(0) }}</span>
            <span class="user-text">
              <span class="user-name">{{ u.name }}</span>
              <span class="user-role">{{ t(`role_${u.role}`) }}</span>
              <span v-if="shiftStart(u)" class="user-shift on">
                <span class="shift-dot" />{{ t('on_shift_since', { time: formatTimestamp(shiftStart(u)!, formatTime) }) }}
              </span>
              <span v-else class="user-shift">
                <span class="shift-dot" />{{ t('off_shift') }}
              </span>
            </span>
          </button>
        </div>

        <!-- INSZ numpad login -->
        <div v-if="showNumpad" class="numpad-section">
          <p class="numpad-label">INSZ</p>
          <div class="insz-display">
            <span
              v-for="(_, i) in 11"
              :key="i"
              class="digit"
              :class="{ filled: i < insz.length }"
            >{{ insz[i] ?? "" }}</span>
          </div>

          <div class="numpad">
            <button v-for="d in ['1','2','3','4','5','6','7','8','9']" :key="d" class="nk" @click="handleKey(d)">{{ d }}</button>
            <button class="nk nk-clear" @click="handleKey('C')">C</button>
            <button class="nk" @click="handleKey('0')">0</button>
            <button class="nk nk-back" @click="handleKey('BS')">&#9003;</button>
          </div>

          <button class="submit-btn" :disabled="insz.length !== 11" @click="submitInsz">
            {{ t('login') }}
          </button>
        </div>

        <p v-if="badge.loginOn && !showNumpad" class="badge-hint" data-testid="badge-login-hint">{{ t('login_badge_hint') }}</p>

        <button v-if="savedUsers.length > 0" class="other-btn" @click="toggleInsz">
          {{ showNumpad ? t('back') : t('other_person_insz') }}
        </button>
      </div>

      <!-- PIN pad: logs in on the 4th digit, no button to confirm. -->
      <div v-if="pinMode && pinUser" class="pin-overlay">
        <div class="pin-head">
          <span class="pin-title">{{ endShiftMode ? t('end_shift') : t('pin_code') }}</span>
          <span class="pin-user">{{ pinUser.name }}</span>
        </div>
        <p v-if="pinError" class="error-msg shake">{{ pinError }}</p>

        <template v-if="pinUser.has_pin">
          <div class="pin-display">
            <span v-for="i in 4" :key="i" class="pin-dot" :class="{ filled: i <= pin.length }"></span>
          </div>

          <div class="numpad">
            <button v-for="d in ['1','2','3','4','5','6','7','8','9']" :key="d" class="nk" @click="handlePinKey(d)">{{ d }}</button>
            <button class="nk nk-clear" @click="handlePinKey('C')">C</button>
            <button class="nk" @click="handlePinKey('0')">0</button>
            <button class="nk nk-back" @click="handlePinKey('BS')">&#9003;</button>
          </div>
          <span class="pin-hint">{{ endShiftMode ? t('end_shift_hint') : t('pin_auto_hint') }}</span>
          <span v-if="endShiftMode && badge.readerOn" class="pin-hint" data-testid="end-shift-badge-hint">{{ t('end_shift_badge_hint') }}</span>
        </template>
        <!-- On shift without a PIN: log in, or end the shift. -->
        <button v-else class="submit-btn" @click="loginWithoutPin(pinUser)">{{ t('login') }}</button>

        <div class="pin-actions">
          <button class="pin-secondary" @click="cancelPin">{{ t('back') }}</button>
          <template v-if="shiftStart(pinUser)">
            <button
              v-if="pinUser.has_pin"
              class="pin-secondary end-shift-btn"
              :class="{ active: endShiftMode }"
              @click="toggleEndShift"
            >{{ t('end_shift') }}</button>
            <button v-else class="pin-secondary end-shift-btn" @click="finishShift(pinUser)">
              {{ t('end_shift') }}
            </button>
          </template>
        </div>
      </div>
    </section>

    <PinDialog
      v-if="firstPinUser"
      :insz="firstPinUser.insz"
      :user-name="firstPinUser.name"
      first-login
      @done="onFirstPinDone"
      @cancel="firstPinUser = null"
    />
  </div>
</template>

<style scoped>
.login {
  flex: 1;
  min-height: 0;
  display: flex;
  background: var(--bg-primary);
}

/* ── Left: brand and checks ─────────────────────────────────────── */
.brand {
  width: 720px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 80px;
}
.brand-mark {
  display: flex;
  flex-direction: column;
  gap: 28px;
}
.sw-version {
  margin-top: 8px;
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-secondary);
  opacity: 0.8;
}
.hanzi {
  font-family: var(--font-zh);
  font-size: 40px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
  line-height: 1;
}

.checks {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.checks-label {
  margin-bottom: 8px;
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-secondary);
}
.check {
  display: grid;
  grid-template-columns: 24px 1fr auto;
  gap: 12px;
  align-items: center;
  height: 44px;
  border-bottom: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: 18px;
}
.check .dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
}
.check-label { color: var(--text-secondary); }
.check-text { font-weight: var(--font-weight-bold); }
.check.ok .dot { background: var(--accent-success); }
.check.ok .check-text { color: var(--accent-success); }
.check.warn .dot { background: var(--accent-warning); }
.check.warn .check-text { color: var(--accent-warning); }
.check.err .dot { background: var(--accent-danger); }
.check.err .check-text { color: var(--accent-danger); }
.check.none .dot { background: var(--text-secondary); opacity: 0.4; }
.check.none .check-text { color: var(--text-secondary); }

.brand-footer {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  margin-top: 24px;
  font-family: var(--font-mono);
  font-size: 15px;
  color: var(--text-secondary);
}

/* ── Right: people and PIN ──────────────────────────────────────── */
.people {
  flex: 1;
  min-width: 0;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 56px;
  padding: 80px 72px;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
}

.people-main {
  display: flex;
  flex-direction: column;
  gap: 28px;
  min-height: 0;
}

.title {
  font-size: 40px;
  font-weight: var(--font-weight-heavy);
}

.error-msg {
  align-self: flex-start;
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-sm);
  border: 1px solid var(--accent-danger);
  color: var(--accent-danger);
  font-size: 16px;
}

.user-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  align-content: start;
  overflow-y: auto;
}

.user-btn {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 24px;
  min-height: 150px;
  border-radius: 14px;
  background: var(--bg-surface);
  color: var(--text-primary);
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.user-btn:active { transform: scale(var(--tap-scale)); }
.user-btn.selected { box-shadow: inset 0 0 0 3px var(--selected-bg); }

.user-initial {
  width: 72px;
  height: 72px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--border);
  font-size: 32px;
  font-weight: var(--font-weight-heavy);
  display: flex;
  align-items: center;
  justify-content: center;
}
.user-btn.selected .user-initial {
  background: var(--selected-bg);
  color: var(--selected-fg);
}

.user-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.user-name {
  font-size: 26px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.user-role {
  font-size: 16px;
  color: var(--text-secondary);
}
.user-shift {
  margin-top: 4px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono);
  font-size: 14px;
  color: var(--text-secondary);
  white-space: nowrap;
}
.shift-dot {
  flex-shrink: 0;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}
.user-shift.on { color: var(--accent-success); }

.other-btn {
  margin-top: auto;
  align-self: flex-start;
  height: 56px;
  padding: 0 24px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.other-btn:active { transform: scale(var(--tap-scale)); }

/* INSZ entry */
.numpad-section {
  display: flex;
  flex-direction: column;
  gap: 16px;
  align-items: flex-start;
}
.numpad-label {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: var(--font-weight-bold);
  letter-spacing: 0.08em;
  color: var(--text-secondary);
}
.insz-display {
  display: flex;
  gap: 6px;
}
.digit {
  width: 44px;
  height: 56px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  font-family: var(--font-mono);
  font-size: 26px;
  font-weight: var(--font-weight-bold);
  display: flex;
  align-items: center;
  justify-content: center;
}
.digit.filled { border-color: var(--text-primary); }

/* PIN pad */
.pin-overlay {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 28px;
}
.pin-head {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}
.pin-title {
  font-size: 18px;
  color: var(--text-secondary);
}
.pin-user {
  font-size: 36px;
  font-weight: var(--font-weight-heavy);
}
.pin-display {
  display: flex;
  gap: 20px;
}
.pin-dot {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--bg-surface);
}
.pin-dot.filled { background: var(--accent-primary); }

.numpad {
  display: grid;
  grid-template-columns: repeat(3, 128px);
  gap: 8px;
}
.nk {
  height: 88px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-family: var(--font-mono);
  font-size: 30px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.nk:active {
  transform: scale(var(--tap-scale));
  background: var(--bg-pressed);
}
.nk-clear { color: var(--accent-warning); }
.nk-back { color: var(--accent-danger); }

.badge-hint {
  font-family: var(--font-mono);
  font-size: 16px;
  color: var(--text-secondary);
}

.pin-hint {
  font-family: var(--font-mono);
  font-size: 14px;
  color: var(--text-secondary);
}

.pin-actions {
  display: flex;
  gap: 8px;
}
.pin-secondary {
  height: 56px;
  padding: 0 24px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.pin-secondary:active { transform: scale(var(--tap-scale)); }
.end-shift-btn {
  border: 1px solid var(--border);
  background: transparent;
}
.end-shift-btn.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}

/* Volt: the screen's main action (log in). */
.submit-btn {
  height: 80px;
  min-width: 400px;
  padding: 0 28px;
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
  font-size: 22px;
  font-weight: var(--font-weight-heavy);
  transition: transform var(--tap-duration) ease;
}
.submit-btn:active { transform: scale(var(--tap-scale)); }
.submit-btn:disabled {
  opacity: 0.3;
  pointer-events: none;
}
</style>
