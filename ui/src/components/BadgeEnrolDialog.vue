<script setup lang="ts">
// Link a badge to an employee (plan L5-D5): the manager presents the
// badge (the next scan is captured, 30 s), then confirms with their OWN PIN.
import { onMounted, onUnmounted, ref } from "vue";
import PinDialog from "./PinDialog.vue";
import { useAuthStore } from "../stores/auth";
import { useBadgeStore } from "../stores/badge";
import { useToastStore } from "../stores/toast";
import { useI18n } from "../i18n";
import { describeError, errorCode } from "../utils/errorMessages";

const props = defineProps<{ insz: string; name: string }>();
const emit = defineEmits<{ done: []; cancel: [] }>();

/** How long the dialog waits for the badge. */
const CAPTURE_MS = 30_000;

const { t } = useI18n();
const auth = useAuthStore();
const badge = useBadgeStore();
const toast = useToastStore();

const token = ref<string | null>(null);
let stop: (() => void) | undefined;
let timer: ReturnType<typeof setTimeout> | undefined;
let failed = false;

onMounted(() => {
  const capture = badge.startCapture();
  stop = capture.cancel;
  timer = setTimeout(() => {
    toast.show(t("badge_capture_timeout"), "warning");
    emit("cancel");
  }, CAPTURE_MS);
  void capture.token.then((tok) => {
    clearTimeout(timer);
    token.value = tok;
  });
});

onUnmounted(() => {
  clearTimeout(timer);
  stop?.();
});

/** The manager's PIN confirms the link; a refusal about the badge (in use,
 * employee already linked...) ends the dialog, a PIN refusal is retried. */
async function verify(pin: string) {
  try {
    await badge.link(props.insz, token.value ?? "", pin);
  } catch (e) {
    if (errorCode(e)?.startsWith("pin_")) throw e;
    failed = true;
    toast.show(describeError(e), "error", 5000);
    emit("cancel");
    throw e;
  }
}

function onDone() {
  if (failed) return;
  toast.show(t("badge_linked_done", { name: props.name }), "success");
  emit("done");
}
</script>

<template>
  <PinDialog
    v-if="token !== null && auth.currentUser"
    :insz="auth.currentUser.insz"
    :user-name="auth.currentUser.name"
    :hint="t('pin_confirm_manager')"
    :verify="verify"
    @done="onDone"
    @cancel="emit('cancel')"
  />
  <div v-else class="backdrop" data-testid="badge-enrol" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" :aria-label="t('badge_link')">
      <h2 class="title">{{ t('badge_link') }}</h2>
      <p class="present" data-testid="badge-present">{{ t('badge_present_for', { name }) }}</p>
      <span class="wave" aria-hidden="true"></span>
      <button class="btn" @click="emit('cancel')">{{ t('back') }}</button>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed; inset: 0; z-index: 50;
  display: flex; align-items: center; justify-content: center;
  background: rgba(0, 0, 0, 0.6);
}
.dialog {
  display: flex; flex-direction: column; align-items: center; gap: var(--space-md);
  min-width: 420px; padding: var(--space-lg);
  border-radius: var(--radius-md); background: var(--bg-secondary);
}
.title { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); }
.present { font-size: var(--font-size-md); color: var(--text-primary); text-align: center; }
.wave {
  width: 64px; height: 64px; border-radius: 50%;
  border: 4px solid var(--accent-primary); animation: pulse 1.4s ease-out infinite;
}
@keyframes pulse { from { transform: scale(0.7); opacity: 1; } to { transform: scale(1.2); opacity: 0.2; } }
.btn {
  width: 100%; height: 56px; border-radius: var(--radius-md);
  background: var(--bg-surface); color: var(--text-secondary);
  font-size: var(--font-size-md); font-weight: var(--font-weight-bold);
}
</style>
