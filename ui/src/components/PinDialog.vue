<script setup lang="ts">
// Set or change a 4-digit PIN: typed twice, saved only when both
// match. The backend allows the user's own PIN, or anyone's for a
// manager (commands/auth.rs::do_set_pin).
//
// `firstLogin`: a user without a PIN (a manager must have one) chooses
// it at login; the PIN is stored and the session opened in one step
// (commands/auth.rs::do_login_with_new_pin).
import { ref } from 'vue'
import NumPad from './NumPad.vue'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'
import { useToastStore } from '../stores/toast'
import { useAuthStore } from '../stores/auth'

const props = defineProps<{
  insz: string
  userName: string
  firstLogin?: boolean
  /** Confirm an action with the user's current PIN (typed once):
   * called on the 4th digit, a refusal is shown and the pad cleared. */
  verify?: (pin: string) => Promise<void>
  /** What the PIN confirms, shown under the name. */
  hint?: string
  /** Hand the chosen PIN to the caller instead of saving it. */
  save?: (pin: string) => Promise<void> | void
}>()

const emit = defineEmits<{
  done: []
  cancel: []
}>()

const { t } = useI18n()
const toast = useToastStore()

const step = ref<'enter' | 'confirm'>('enter')
const first = ref('')
const pin = ref('')
const error = ref<string | null>(null)
const saving = ref(false)

async function onKey(key: string) {
  if (saving.value) return
  if (key === 'BACKSPACE') pin.value = pin.value.slice(0, -1)
  else if (key === 'CLEAR') pin.value = ''
  else if (pin.value.length < 4) pin.value += key
  if (pin.value.length < 4) return

  if (props.verify) {
    saving.value = true
    try {
      await props.verify(pin.value)
      emit('done')
    } catch (e) {
      error.value = describeError(e)
      pin.value = ''
    } finally {
      saving.value = false
    }
    return
  }

  if (step.value === 'enter') {
    first.value = pin.value
    pin.value = ''
    error.value = null
    step.value = 'confirm'
    return
  }
  if (pin.value !== first.value) {
    error.value = t('pin_mismatch')
    first.value = ''
    pin.value = ''
    step.value = 'enter'
    return
  }
  saving.value = true
  try {
    if (props.save) {
      await props.save(pin.value)
      emit('done')
      return
    }
    if (props.firstLogin) {
      await useAuthStore().loginWithNewPin(props.insz, pin.value)
    } else {
      await ipc.setPin(props.insz, pin.value)
    }
    toast.show(t(props.firstLogin ? 'pin_chosen' : 'pin_changed'), 'success')
    emit('done')
  } catch (e) {
    error.value = describeError(e)
    first.value = ''
    pin.value = ''
    step.value = 'enter'
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="backdrop" @click.self="emit('cancel')">
    <div class="dialog pin-dialog" role="dialog" :aria-label="t('pin_change')">
      <h2 class="title">
        {{
          verify
            ? t('pin_enter')
            : step === 'confirm'
              ? t('pin_confirm')
              : firstLogin
                ? t('pin_choose')
                : t('pin_enter')
        }}
      </h2>
      <p class="who">{{ userName }}</p>
      <p v-if="firstLogin" class="hint">{{ t('pin_first_login_hint') }}</p>
      <p v-else-if="hint" class="hint">{{ hint }}</p>
      <p v-if="error" class="error-msg shake">{{ error }}</p>
      <div class="pin-display">
        <span v-for="i in 4" :key="i" class="pin-dot" :class="{ filled: i <= pin.length }"></span>
      </div>
      <NumPad :model-value="pin" @update:model-value="onKey" />
      <button class="btn btn-cancel" @click="emit('cancel')">{{ t('back') }}</button>
    </div>
  </div>
</template>

<style scoped>
.hint {
  margin: 0;
  max-width: 22rem;
  text-align: center;
  color: var(--text-secondary);
  font-size: 0.95rem;
}

.backdrop {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  z-index: 50;
}
.dialog {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.who {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}
.error-msg {
  color: var(--accent-danger);
  font-weight: var(--font-weight-bold);
}
.pin-display {
  display: flex;
  gap: var(--space-md);
}
.pin-dot {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid var(--text-secondary);
}
.pin-dot.filled {
  background: var(--accent-primary);
  border-color: var(--accent-primary);
}
.btn {
  width: 100%;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
}
.btn-cancel {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
</style>
