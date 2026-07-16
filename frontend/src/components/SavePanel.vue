<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { SaveGroup } from '../types'

const props = defineProps<{
  open: boolean
  groups: SaveGroup[]
  currentId: string | null
  currentGroupId: string | null
}>()
defineEmits<{
  close: []
  load: [id: string]
  removeSave: [id: string]
  removeGroup: [id: string]
}>()

const selectedId = ref<string | null>(null)
const selected = computed(() => props.groups.find(group => group.save_group_id === selectedId.value) || props.groups[0])

watch(() => [props.open, props.currentGroupId, props.groups] as const, () => {
  if (!props.open) return
  const preferred = props.groups.find(group => group.save_group_id === props.currentGroupId)
  if (!props.groups.some(group => group.save_group_id === selectedId.value)) {
    selectedId.value = preferred?.save_group_id || props.groups[0]?.save_group_id || null
  }
}, { immediate: true })

const played = (group: SaveGroup) => {
  const total = Math.max(0, (group.year - 1) * 12 + group.month - 1)
  return `已历${Math.floor(total / 12)}年${total % 12}月`
}
const time = (value: string) => value ? new Date(value).toLocaleString('zh-CN', { hour12: false }) : '年月不详'
</script>

<template>
  <Transition name="fade">
    <div v-if="open" class="modal-overlay z-[100]" @click.self="$emit('close')">
      <div class="save-dialog save-panel">
        <div class="panel-title">江 湖 卷 宗</div>
        <p class="save-hint">横览山门槽位，点选后查阅该局全部存档。</p>

        <div v-if="!groups.length" class="empty-saves">尚无卷宗。且待开山立派，书写第一笔江湖事。</div>
        <template v-else>
          <div class="slot-strip" aria-label="存档槽位">
            <button
              v-for="group in groups"
              :key="group.save_group_id"
              class="group-card"
              :class="{ active: selected?.save_group_id === group.save_group_id }"
              @click="selectedId = group.save_group_id"
            >
              <span class="group-seal">{{ group.save_group_id === currentGroupId ? '今' : '卷' }}</span>
              <strong>{{ group.sect_name || '无名山门' }}</strong>
              <small>{{ played(group) }}</small>
              <time>{{ time(group.updated_at) }}</time>
            </button>
          </div>

          <section v-if="selected" class="save-list-wrap">
            <header class="save-list-header">
              <div>
                <strong>{{ selected.sect_name || '无名山门' }}</strong>
                <span>共 {{ selected.saves.length }} 份存档</span>
              </div>
              <button class="btn btn-sm danger" @click="$emit('removeGroup', selected.save_group_id)">删除槽位</button>
            </header>
            <div class="save-list">
              <article v-for="save in selected.saves" :key="save.id" class="save-entry" :class="{ current: save.id === currentId }">
                <span class="save-kind" :class="save.save_type">{{ save.save_type === 'manual' ? '手动' : '自动' }}</span>
                <div class="save-meta">
                  <strong>第 {{ save.year }} 年 · {{ save.month }} 月</strong>
                  <time>{{ time(save.updated_at) }}</time>
                </div>
                <span v-if="save.id === currentId" class="current-mark">当前</span>
                <button class="btn btn-sm" @click="$emit('load', save.id)">读档</button>
                <button class="btn btn-sm danger" @click="$emit('removeSave', save.id)">删除</button>
              </article>
            </div>
          </section>
        </template>

        <button class="btn save-back" @click="$emit('close')">收卷返回</button>
      </div>
    </div>
  </Transition>
</template>
