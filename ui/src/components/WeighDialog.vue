<script setup lang="ts">
// The weighing in progress: where it stands, and the only way out. The
// engine holds the weighing; closing this dialog asks it to stop and
// its refusal (`scale_cancelled`) ends the call that opened the dialog.
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'

const { itemName } = defineProps<{ itemName: string }>()

const { t } = useI18n()
const stage = ref('waiting')
const cancelling = ref(false)

const label = computed(() => {
  if (stage.value === 'stabilizing') return t('scale_stage_stabilizing')
  if (stage.value === 'checking') return t('scale_stage_checking')
  return t('scale_stage_waiting')
})

let unlisten: UnlistenFn | undefined
let gone = false

onMounted(async () => {
  const off = await listen<{ stage: string }>('scale-progress', (event) => {
    stage.value = event.payload.stage
  })
  if (gone) off()
  else unlisten = off
})
onUnmounted(() => {
  gone = true
  unlisten?.()
})

async function cancel() {
  cancelling.value = true
  await ipc.scaleCancelWeighing()
}
</script>

<template>
  <div class="backdrop">
    <div class="dialog" role="dialog" :aria-label="t('scale_title')" data-testid="weigh-dialog">
      <h2 class="title">{{ t('scale_title') }}</h2>
      <p class="item">{{ itemName }}</p>
      <p class="stage" data-testid="weigh-stage">{{ label }}</p>
      <button class="btn" :disabled="cancelling" data-testid="weigh-cancel" @click="cancel">
        {{ t('back') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
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
  min-width: 360px;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.item {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}
.stage {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  color: var(--accent-primary);
}
.btn {
  width: 100%;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
}
.btn:disabled {
  opacity: 0.5;
}
</style>
