<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from '../i18n'
import { TIMES, typeKey, type CodePadTone } from '../utils/codeEntry'

/**
 * The pad of the order entry by menu code: the field (a quantity pill and
 * the code, a clear button), one line telling what OK will do or what went
 * wrong, the digits with the quantity key and backspace, the letters the
 * menu codes use, and OK. While a code is typed the order view filters the
 * grid. A physical keyboard types into it too while `listening`. The
 * grammar and what OK does belong to the order view.
 */
const {
  modelValue,
  letters,
  message,
  tone,
  listening = true,
} = defineProps<{
  modelValue: string
  /** Letter keys, from the codes of the menu. */
  letters: string[]
  message: string
  tone: CodePadTone
  /** A physical keyboard types into the pad (false while a dialog is open). */
  listening?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  submit: []
}>()

const { t } = useI18n()

function press(key: string) {
  emit('update:modelValue', typeKey(modelValue, key))
}
function backspace() {
  emit('update:modelValue', modelValue.slice(0, -1))
}
function clear() {
  emit('update:modelValue', '')
}

function onKeydown(e: KeyboardEvent) {
  if (!listening || e.ctrlKey || e.metaKey || e.altKey) return
  const target = e.target as HTMLElement | null
  if (
    target &&
    (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
  )
    return
  if (e.key === 'Enter') emit('submit')
  else if (e.key === 'Backspace') backspace()
  else if (e.key === 'Escape') clear()
  else if (e.key === '*' || /^[a-zA-Z0-9]$/u.test(e.key)) press(e.key)
  else return
  e.preventDefault()
}

onMounted(() => window.addEventListener('keydown', onKeydown))
onUnmounted(() => window.removeEventListener('keydown', onKeydown))

const DIGITS = ['7', '8', '9', '4', '5', '6', '1', '2', '3']

// The field: "3 ×" as a pill before the code, "× 3" after a code typed first.
const field = computed(() => {
  const before = /^(?<qty>\d{1,2})\*(?<code>.*)$/u.exec(modelValue)?.groups
  if (before) return { pill: `${before.qty} ×`, code: before.code, pillAfter: false }
  const after = /^(?<code>.+)\*(?<qty>\d*)$/u.exec(modelValue)?.groups
  if (after) return { pill: `× ${after.qty}`, code: after.code, pillAfter: true }
  return { pill: null, code: modelValue, pillAfter: false }
})
</script>

<template>
  <div class="code-pad" data-testid="code-pad">
    <div class="field" :class="tone" data-testid="code-pad-display">
      <span v-if="field.pill && !field.pillAfter" class="pill" data-testid="code-pad-qty">{{
        field.pill
      }}</span>
      <span v-if="modelValue === ''" class="placeholder">{{ t('code_pad_placeholder') }}</span>
      <span v-else class="code">{{ field.code }}</span>
      <span v-if="field.pill && field.pillAfter" class="pill" data-testid="code-pad-qty">{{
        field.pill
      }}</span>
      <button
        v-if="modelValue !== ''"
        class="field-clear"
        :aria-label="t('code_pad_clear')"
        data-testid="code-key-clear"
        @click="clear"
      >
        ✕
      </button>
    </div>
    <div
      class="message"
      :class="tone"
      role="status"
      aria-live="polite"
      data-testid="code-pad-message"
    >
      {{ message }}
    </div>
    <div class="keys">
      <button
        v-for="d in DIGITS"
        :key="d"
        class="key"
        :data-testid="`code-key-${d}`"
        @click="press(d)"
      >
        {{ d }}
      </button>
      <button
        class="key qty"
        :aria-label="t('code_pad_quantity')"
        data-testid="code-key-times"
        @click="press(TIMES)"
      >
        <span class="qty-sign">×</span>
        <span class="qty-label">{{ t('code_pad_quantity') }}</span>
      </button>
      <button class="key" data-testid="code-key-0" @click="press('0')">0</button>
      <button
        class="key back"
        :aria-label="t('code_pad_backspace')"
        data-testid="code-key-back"
        @click="backspace"
      >
        &#9003;
      </button>
    </div>
    <div v-if="letters.length > 0" class="letters">
      <button
        v-for="l in letters"
        :key="l"
        class="key letter"
        :data-testid="`code-key-${l}`"
        @click="press(l)"
      >
        {{ l }}
      </button>
    </div>
    <button class="ok" data-testid="code-key-ok" @click="emit('submit')">OK</button>
  </div>
</template>

<style scoped>
.code-pad {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  width: 360px;
  flex-shrink: 0;
  padding: 16px 0 16px 16px;
  overflow-y: auto;
}

.field {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  min-height: 72px;
  padding: 0 56px 0 var(--space-md);
  border-radius: var(--radius-lg);
  background: var(--bg-secondary);
  border: 2px solid var(--border);
  transition: border-color var(--tap-duration) ease;
}
.field.match {
  border-color: var(--selected-bg);
}
.field.error {
  border-color: var(--accent-danger);
}
.field.done {
  border-color: var(--accent-success);
}
.placeholder {
  color: var(--text-secondary);
  font-size: var(--font-size-lg);
}
.code {
  font-family: var(--font-mono);
  font-size: 34px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  letter-spacing: 0.04em;
}
.pill {
  padding: 4px 10px;
  border-radius: 999px;
  background: var(--selected-bg);
  color: var(--selected-fg);
  font-family: var(--font-mono);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.field-clear {
  position: absolute;
  right: 8px;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}

.message {
  min-height: 44px;
  font-size: var(--font-size-md);
  line-height: 1.3;
  color: var(--text-secondary);
}
.message.match {
  color: var(--text-primary);
  font-weight: var(--font-weight-bold);
}
.message.error {
  color: var(--accent-danger);
  font-weight: var(--font-weight-bold);
}
.message.done {
  color: var(--accent-success);
  font-weight: var(--font-weight-bold);
}

.keys {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-xs);
}
.letters {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: var(--space-xs);
}

.key {
  height: 68px;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.key:active,
.ok:active,
.field-clear:active {
  transform: scale(var(--tap-scale));
}
.key:active {
  background: var(--bg-pressed);
}
.key.letter {
  height: 52px;
  font-size: var(--font-size-lg);
}
.key.back {
  color: var(--text-secondary);
}
.key.qty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
}
.qty-sign {
  line-height: 1;
}
.qty-label {
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}

.ok {
  height: 72px;
  margin-top: var(--space-xs);
  border-radius: var(--radius-md);
  background: var(--accent-success);
  color: var(--text-on-accent);
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
</style>
