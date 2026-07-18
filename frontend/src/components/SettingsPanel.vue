<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppConfig } from '../types'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

const form = ref<AppConfig>({
  db_path: '',
  server_host: '0.0.0.0',
  server_port: '3000',
})
const saving = ref(false)
const message = ref('')
const isTauri = ref(false)

const loadConfig = async () => {
  try {
    const cfg = await invoke<AppConfig>('get_config')
    form.value = cfg
    isTauri.value = true
  } catch {
    isTauri.value = false
    try {
      const response = await fetch('/api/config')
      if (!response.ok) throw new Error(await response.text())
      form.value = await response.json() as AppConfig
    } catch (e) {
      message.value = `读取配置失败: ${e}`
    }
  }
}

const saveConfig = async () => {
  saving.value = true
  message.value = ''
  try {
    if (isTauri.value) {
      await invoke('save_config', { config: form.value })
    } else {
      const response = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(form.value),
      })
      if (!response.ok) throw new Error(await response.text())
    }
    message.value = '配置已存，请重启案牍使其生效。'
  } catch (e) {
    message.value = `保存失败: ${e}`
  } finally {
    saving.value = false
  }
}

watch(() => props.open, (v) => { if (v) { loadConfig(); message.value = '' } })
</script>

<template>
  <div v-if="open" class="modal-overlay z-[110]" @click.self="emit('close')">
    <div class="save-dialog" style="max-width: 520px;">
      <h2 style="font-family: var(--font-title); font-size: 1.2rem; text-align: center; margin-bottom: 1rem; letter-spacing: .15em;">
        ⚙️ 应用配置
      </h2>

      <div class="config-form">
        <div class="form-row">
          <label>数据库文件</label>
          <input v-model="form.db_path" class="db-path-input" placeholder="数据库文件路径" />
        </div>

        <div v-if="message" class="config-msg" :class="{ error: message.includes('失败') }">{{ message }}</div>

        <div class="form-actions">
          <button class="btn" @click="emit('close')">作罢</button>
          <button class="btn btn-primary" :disabled="saving" @click="saveConfig">
            {{ saving ? '誊录中…' : '存下配置' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-form {
  display: flex;
  flex-direction: column;
  gap: .5rem;
}
.form-row {
  display: flex;
  align-items: center;
  gap: .5rem;
  min-width: 0;
}
.form-row label {
  width: 80px;
  text-align: right;
  font-size: .82rem;
  color: var(--color-ink-light);
  flex-shrink: 0;
}
.form-row input {
  flex: 1;
  min-width: 0;
  font-family: var(--font-serif);
  font-size: .85rem;
  padding: .35rem .5rem;
  border: 1px solid var(--color-border);
  border-radius: 2px;
  background: var(--color-paper-light);
  color: var(--color-ink);
}
.form-row input:focus {
  outline: none;
  border-color: var(--color-cinnabar);
}
.config-msg {
  text-align: center;
  font-size: .82rem;
  padding: .4rem;
  color: var(--color-jade);
}
.config-msg.error {
  color: var(--color-cinnabar);
}
.form-actions {
  display: flex;
  justify-content: center;
  gap: .5rem;
  margin-top: .5rem;
}
</style>
