<script setup lang="ts">
// Settings panel "Carte" (managers): opens the menu editor, or says that a catalog manages the
// menu and it is edited there. The engine decides (`menu_admin_get` answers `managed_by`).
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import * as ipc from '../api/ipc'
import { useI18n } from '../i18n'

const { t } = useI18n()
const router = useRouter()
const managedBy = ref<string | null>(null)
const loaded = ref(false)

onMounted(async () => {
  try {
    managedBy.value = (await ipc.menuAdminGet()).managed_by
  } catch {
    // The panel then offers the editor, which shows the real error.
  } finally {
    loaded.value = true
  }
})
</script>

<template>
  <section class="menu-editor-panel" data-testid="menu-editor-panel">
    <h2>{{ t('menu_editor_title') }}</h2>
    <p v-if="managedBy" class="locked" data-testid="menu-editor-locked">
      {{ t('menu_editor_locked', { source: managedBy === 'hubrise' ? 'HubRise' : managedBy }) }}
    </p>
    <template v-else>
      <p class="note">{{ t('menu_editor_note') }}</p>
      <button
        class="open-btn"
        data-testid="menu-editor-open"
        :disabled="!loaded"
        @click="router.push({ name: 'menu-editor' })"
      >
        {{ t('menu_editor_open') }}
      </button>
    </template>
  </section>
</template>

<style scoped>
.menu-editor-panel {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.menu-editor-panel h2 {
  font-size: 28px;
  font-weight: var(--font-weight-heavy);
  line-height: 1.2;
}
.note {
  font-size: 16px;
  color: var(--text-secondary);
}
.locked {
  font-size: 17px;
  color: var(--accent-warning);
  font-weight: var(--font-weight-bold);
}
.open-btn {
  height: 64px;
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 2px solid var(--accent-info);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.open-btn:active {
  transform: scale(var(--tap-scale));
}
.open-btn:disabled {
  opacity: 0.4;
}
</style>
