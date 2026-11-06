<script setup lang="ts">
/**
 * The menu under the user button of the header (UX audit L1, R5): the user's own
 * language, "Changer mon PIN", "Fin de service" (a second tap within 5 s confirms)
 * and "Verrouiller". Every row is 56 px; nothing here is a Volt action.
 */
import { computed, onBeforeUnmount, ref } from 'vue'
import { useAuthStore } from '../stores/auth'
import { usePosStore } from '../stores/pos'
import { useI18n } from '../i18n'
import { isArmed, tapArmed } from '../utils/armedAction'
import { LANGUAGE_CHOICES, type AppLanguage } from '../utils/userLanguage'
import UiIcon from './UiIcon.vue'

const { left } = defineProps<{
  /** Where the menu starts, in px from the left of the screen: under the user button. */
  left: number
}>()

const emit = defineEmits<{
  close: []
  pin: []
  endShift: []
  lock: []
}>()

const auth = useAuthStore()
const pos = usePosStore()
const { t } = useI18n()

// The language shown as chosen: the user's own, else the terminal's they follow.
const chosen = computed(() => pos.language)

function pickLanguage(language: AppLanguage) {
  if (auth.currentUser) pos.setUserLanguage(auth.currentUser.insz, language)
}

// "Fin de service": the first tap arms it, the second one within 5 s does it.
const armedUntil = ref<number | null>(null)
const now = ref(Date.now())
let repaint: ReturnType<typeof setTimeout> | undefined
const armed = computed(() => isArmed(armedUntil.value, now.value))

function tapEndShift() {
  const at = Date.now()
  const step = tapArmed(armedUntil.value, at)
  armedUntil.value = step.armedUntil
  now.value = at
  clearTimeout(repaint)
  if (step.fire) {
    emit('endShift')
    return
  }
  // Back to the first label when the time is up.
  repaint = setTimeout(() => {
    now.value = Date.now()
    armedUntil.value = null
  }, step.armedUntil! - at)
}
onBeforeUnmount(() => clearTimeout(repaint))
</script>

<template>
  <div class="user-menu-layer">
    <div class="user-menu-backdrop" data-testid="user-menu-backdrop" @click="emit('close')" />
    <div
      class="user-menu"
      role="menu"
      :style="{ left: `${left}px` }"
      :aria-label="t('user_menu')"
      data-testid="user-menu"
    >
      <div class="menu-row menu-language" data-testid="user-menu-language">
        <span class="menu-label"
          ><UiIcon name="language" :size="22" />{{ t('user_menu_language') }}</span
        >
        <div class="lang-choices" role="group" :aria-label="t('user_menu_language')">
          <button
            v-for="language in LANGUAGE_CHOICES"
            :key="language"
            type="button"
            class="lang-choice"
            :class="{ active: chosen === language }"
            :aria-pressed="chosen === language"
            :data-testid="`user-lang-${language}`"
            @click="pickLanguage(language)"
          >
            {{ t(`lang_short_${language}`) }}
          </button>
        </div>
      </div>
      <button
        type="button"
        class="menu-row menu-item"
        role="menuitem"
        data-testid="user-menu-pin"
        @click="emit('pin')"
      >
        <span class="menu-label"><UiIcon name="pin" :size="22" />{{ t('pin_change') }}</span>
      </button>
      <button
        type="button"
        class="menu-row menu-item menu-end-shift"
        :class="{ armed }"
        role="menuitem"
        data-testid="user-menu-end-shift"
        @click="tapEndShift"
      >
        <span class="menu-label"
          ><UiIcon name="endShift" :size="22" />{{
            armed ? t('end_shift_confirm') : t('end_shift')
          }}</span
        >
      </button>
      <button
        type="button"
        class="menu-row menu-item"
        role="menuitem"
        data-testid="user-menu-lock"
        @click="emit('lock')"
      >
        <span class="menu-label"><UiIcon name="lock" :size="22" />{{ t('lock') }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.user-menu-layer {
  position: fixed;
  inset: 0;
  z-index: 50;
}
.user-menu-backdrop {
  position: absolute;
  inset: 0;
}
/* Anchored under the user button, the first thing after the logo and the lights. */
.user-menu {
  position: absolute;
  top: calc(var(--header-height) - 6px);
  width: 400px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
}
.menu-row {
  min-height: 56px;
  padding: 0 14px;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: var(--text-primary);
  font-size: 17px;
  font-weight: var(--font-weight-bold);
  text-align: left;
}
.menu-item {
  width: 100%;
  background: var(--bg-surface);
  transition: transform var(--tap-duration) ease;
}
.menu-item:active {
  transform: scale(var(--tap-scale));
}
.menu-label {
  display: flex;
  align-items: center;
  gap: 12px;
}
.menu-language {
  background: var(--bg-surface);
}
.lang-choices {
  display: flex;
  gap: 4px;
}
.lang-choice {
  min-width: 56px;
  height: 48px;
  padding: 0 12px;
  border-radius: var(--radius-sm);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
}
.lang-choice.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
/* Armed: red, the colour of what ends something. */
.menu-end-shift.armed {
  background: transparent;
  color: var(--accent-danger);
  box-shadow: inset 0 0 0 2px var(--accent-danger);
}
</style>
