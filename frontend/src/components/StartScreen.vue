<script setup lang="ts">
import { nextTick, onMounted, ref } from 'vue'
import { API_BASE } from '../api'
import { ui } from '../store'
defineProps<{ canContinue: boolean }>()
const emit = defineEmits<{ start: [name: string]; continue: []; load: [] }>()
const name = ref('')
const input = ref<HTMLInputElement>()
const start = () => emit('start', name.value.trim() || '无名派')
onMounted(() => nextTick(() => input.value?.focus()))
</script>
<template>
  <section class="start-screen fade-in">
    <h1>掌 门 日 记</h1><div class="subtitle">武侠门派经营模拟器 · v3.0</div>
    <div class="poem">白手起家开山门<br>招贤纳士聚英魂<br>江湖风云多变幻<br>论剑之日定乾坤</div>
    <div class="start-input"><label>请为山门赐名：</label><input ref="input" v-model="name" maxlength="10" placeholder="如：青云门" autocomplete="off" @keydown.enter="start"></div>
    <div class="start-actions">
      <button class="btn btn-primary px-10 py-2.5 text-lg" @click="start">开 山 立 派</button>
      <button class="btn btn-jade px-10 py-2.5 text-lg" :disabled="!canContinue" @click="emit('continue')">再 入 江 湖</button>
      <button class="btn" @click="emit('load')">载 入 进 度</button>
    </div>
    <div class="mt-2 text-xs text-ink-fade">
      后端: <code>{{ API_BASE }}</code>
      <button class="ml-2 text-ink-fade hover:text-cinnabar" title="数据库设置" @click="ui.settingsOpen = true">⚙</button>
    </div>
  </section>
</template>
