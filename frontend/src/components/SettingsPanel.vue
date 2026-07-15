<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppConfig } from '../types'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

const form = ref<AppConfig>({
  pg_host: '',
  pg_port: '5432',
  pg_user: 'ruoruo',
  pg_password: '',
  pg_database: 'zhangmenriji',
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
    // 非 Tauri 环境（浏览器开发模式），invoke 不可用
    isTauri.value = false
  }
}

const saveConfig = async () => {
  saving.value = true
  message.value = ''
  try {
    await invoke('save_config', { config: form.value })
    message.value = '✅ 配置已保存，请重启应用使其生效喵～'
  } catch (e) {
    message.value = `❌ 保存失败: ${e}`
  } finally {
    saving.value = false
  }
}

watch(() => props.open, (v) => { if (v) { loadConfig(); message.value = '' } })
</script>

<template>
  <div v-if="open" class="modal-overlay" @click.self="emit('close')">
    <div class="save-dialog" style="max-width: 520px;">
      <h2 style="font-family: var(--font-title); font-size: 1.2rem; text-align: center; margin-bottom: 1rem; letter-spacing: .15em;">
        ⚙️ 数据库配置
      </h2>

      <div v-if="!isTauri" style="text-align: center; color: var(--color-ink-fade); padding: 1rem; font-size: .85rem;">
        请在桌面应用中打开此设置面板喵～<br>浏览器开发模式下配置通过 <code>.env</code> 文件管理。
      </div>

      <div v-else class="config-form">
        <div class="form-row">
          <label>数据库地址</label>
          <input v-model="form.pg_host" placeholder="192.168.50.150" />
        </div>
        <div class="form-row">
          <label>端口</label>
          <input v-model="form.pg_port" placeholder="5432" />
        </div>
        <div class="form-row">
          <label>用户名</label>
          <input v-model="form.pg_user" placeholder="ruoruo" />
        </div>
        <div class="form-row">
          <label>密码</label>
          <input v-model="form.pg_password" type="password" placeholder="请输入密码" />
        </div>
        <div class="form-row">
          <label>数据库名</label>
          <input v-model="form.pg_database" placeholder="zhangmenriji" />
        </div>

        <div v-if="message" class="config-msg" :class="{ error: message.startsWith('❌') }">{{ message }}</div>

        <div class="form-actions">
          <button class="btn" @click="emit('close')">取消</button>
          <button class="btn btn-primary" :disabled="saving" @click="saveConfig">
            {{ saving ? '保存中…' : '保存配置' }}
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
