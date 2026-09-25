<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { usePosStore } from "../stores/pos";
import { useToastStore } from "../stores/toast";
import type { User } from "../types";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";

const router = useRouter();
const auth = useAuthStore();
const pos = usePosStore();
const toast = useToastStore();
const { t } = useI18n();

const savedUsers = ref<User[]>([]);
const showNumpad = ref(false);
const insz = ref("");
const error = ref<string | null>(null);

// ── PIN overlay state ──────────────────────────────────────────
const pinMode = ref(false);
const pinUser = ref<User | null>(null);
const pin = ref("");
const pinError = ref<string | null>(null);

onMounted(async () => {
  try {
    savedUsers.value = await ipc.listUsers();
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
    // User has a PIN → show the PIN entry overlay
    pinUser.value = user;
    pinMode.value = true;
    pin.value = "";
    pinError.value = null;
    return;
  }
  // No PIN set → login directly + warning toast
  try {
    await auth.loginWithInsz(user.insz);
    toast.show(t("pin_not_set_warning"), "warning", 5000);
    router.push({ name: "order" });
  } catch (e) {
    error.value = String(e);
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
  try {
    await auth.loginWithInsz(pinUser.value.insz, pin.value);
    router.push({ name: "order" });
  } catch (e) {
    const msg = String(e);
    if (msg.includes("pin_wrong")) {
      pinError.value = t("pin_wrong");
    } else if (msg.includes("pin_required")) {
      pinError.value = t("pin_required");
    } else {
      pinError.value = msg;
    }
    pin.value = "";
  }
}

function cancelPin() {
  pinMode.value = false;
  pinUser.value = null;
  pin.value = "";
  pinError.value = null;
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
    router.push({ name: "order" });
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<template>
  <div class="login">
    <h1 class="title">{{ pos.settings?.restaurantName || "moyun-pos" }}</h1>

    <p v-if="error" class="error-msg shake">{{ error }}</p>

    <!-- PIN overlay - shown when tapping a user tile that has a PIN -->
    <div v-if="pinMode && pinUser" class="pin-overlay">
      <h2 class="pin-title">{{ t('pin_enter') }}</h2>
      <p class="pin-user">{{ pinUser.name }}</p>
      <p v-if="pinError" class="error-msg shake">{{ pinError }}</p>

      <div class="pin-display">
        <span v-for="i in 4" :key="i" class="pin-dot" :class="{ filled: i <= pin.length }"></span>
      </div>

      <div class="numpad">
        <button v-for="d in ['1','2','3','4','5','6','7','8','9']" :key="d" class="nk" @click="handlePinKey(d)">{{ d }}</button>
        <button class="nk nk-clear" @click="handlePinKey('C')">C</button>
        <button class="nk" @click="handlePinKey('0')">0</button>
        <button class="nk nk-back" @click="handlePinKey('BS')">&#9003;</button>
      </div>

      <button class="other-link" @click="cancelPin">{{ t('back') }}</button>
    </div>

    <!-- Quick login: tap on a saved worker (hidden during PIN mode) -->
    <template v-else>
      <div v-if="savedUsers.length > 0 && !showNumpad" class="user-grid">
        <button
          v-for="u in savedUsers"
          :key="u.insz"
          class="user-btn"
          @click="quickLogin(u)"
        >
          <span class="user-initial">{{ u.name.charAt(0) }}</span>
          <span class="user-name">{{ u.name }}</span>
          <span class="user-role">{{ t(`role_${u.role}`) }}</span>
        </button>
      </div>

      <button
        v-if="savedUsers.length > 0 && !showNumpad"
        class="other-link"
        @click="showNumpad = true"
      >{{ t('other_insz') }}</button>

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

        <button
          v-if="savedUsers.length > 0"
          class="other-link"
          @click="showNumpad = false; insz = ''"
        >{{ t('back') }}</button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.login {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
  gap: var(--space-lg);
}

.title {
  font-size: 36px;
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
}

.error-msg {
  color: var(--accent-danger);
  font-size: var(--font-size-sm);
  padding: var(--space-sm) var(--space-md);
  background: rgba(231, 76, 60, 0.1);
  border-radius: var(--radius-sm);
}

/* Quick user buttons */
.user-grid {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-md);
  justify-content: center;
  max-width: 600px;
}

.user-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-lg);
  min-width: 140px;
  min-height: 120px;
  border-radius: var(--radius-lg);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.user-btn:active { transform: scale(var(--tap-scale)); }

.user-initial {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: var(--accent-primary);
  color: var(--text-on-accent);
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  display: flex;
  align-items: center;
  justify-content: center;
}

.user-name {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}

.user-role {
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
}

.other-link {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  padding: var(--space-sm);
}

/* PIN overlay */
.pin-overlay {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-lg);
}

.pin-title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}

.pin-user {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}

.pin-display {
  display: flex;
  gap: var(--space-md);
}

.pin-dot {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--bg-surface);
  border: 2px solid var(--bg-surface);
  transition: background 100ms ease;
}
.pin-dot.filled {
  background: var(--accent-primary);
  border-color: var(--accent-primary);
}

/* Numpad section */
.numpad-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-lg);
}

.numpad-label {
  font-size: var(--font-size-lg);
  color: var(--text-secondary);
}

.insz-display {
  display: flex;
  gap: var(--space-xs);
}

.digit {
  width: 36px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  color: var(--text-primary);
  border: 2px solid var(--bg-surface);
}
.digit.filled { border-color: var(--accent-primary); }

.numpad {
  display: grid;
  grid-template-columns: repeat(3, 96px);
  gap: var(--space-xs);
}

.nk {
  height: 72px;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.nk:active { transform: scale(var(--tap-scale)); background: var(--bg-pressed); }
.nk-clear { color: var(--accent-warning); }
.nk-back { color: var(--accent-danger); }

.submit-btn {
  width: 300px;
  height: 72px;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
  transition: transform var(--tap-duration) ease;
}
.submit-btn:disabled { opacity: 0.3; pointer-events: none; }
.submit-btn:active { transform: scale(var(--tap-scale)); }
</style>
