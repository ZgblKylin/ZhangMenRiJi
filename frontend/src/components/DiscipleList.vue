<script setup lang="ts">
import { reactive, ref } from 'vue'
import type { ActionKind, Disciple, DiscipleRank, ManagementRequest, MartialArt } from '../types'

const props = defineProps<{ disciples: Disciple[]; arts: MartialArt[]; disabled?: boolean }>()
const emit = defineEmits<{ manage: [command: ManagementRequest] }>()
const openId = ref<string | null>(null)
const selected = reactive<Record<string, ActionKind>>({})
const selectedRank = reactive<Record<string, DiscipleRank>>({})
const actions: Array<[ActionKind, string]> = [
  ['read', '研读典籍'], ['temper_body', '打熬气血'], ['cultivate_neili', '修炼内力'],
  ['meditate', '冥想养神'], ['spar', '同门切磋'], ['teach', '传功授艺'],
  ['sect_mission', '外派办事'], ['wander', '江湖历练'], ['recover', '静养调息'],
]
const rankName = { chore: '杂役', outer: '外门', inner: '内门', elder: '长老' }
const conditionName = { healthy: '安好', exhausted: '力竭', unconscious: '昏迷', seriously_injured: '重伤', dead: '亡故' }
const artName = (id: string) => props.arts.find(art => art.id === id)?.name || id
const assign = (disciple: Disciple) => emit('manage', {
  action: 'assign_action', disciple_id: disciple.id,
  kind: selected[disciple.id] || 'cultivate_neili', target_id: null, martial_art_id: null,
})
const appoint = (disciple: Disciple) => emit('manage', {
  action: 'set_personnel', disciple_id: disciple.id,
  rank: selectedRank[disciple.id] || disciple.rank, department: disciple.department || null,
})
const expel = (disciple: Disciple) => {
  if (confirm(`当真要将${disciple.name}逐出山门？`)) emit('manage', { action: 'expel', disciple_id: disciple.id })
}
</script>

<template>
  <section class="panel disciple-panel">
    <div class="panel-title">门下谱牒（{{ disciples.length }}人）</div>
    <div v-if="!disciples.length" class="empty-message">山门冷清，尚无弟子。</div>
    <div v-else class="disciple-list">
      <article v-for="d in disciples" :key="d.id" class="disciple-card" :class="{ expanded: openId === d.id }">
        <button class="disciple-summary" @click="openId = openId === d.id ? null : d.id">
          <span class="disciple-name">{{ d.name }}</span>
          <span class="rank-seal">{{ rankName[d.rank] }}</span>
          <span class="disciple-brief">{{ d.age }}岁 · {{ artName(d.martial_art) }} · 门忠{{ d.attributes?.sect_loyalty ?? d.loyalty }}</span>
          <span class="condition" :class="d.condition">{{ d.away_months ? `外出${d.away_months}月` : conditionName[d.condition] }}</span>
        </button>
        <div v-if="openId === d.id" class="disciple-detail">
          <div class="aptitude-row">
            <span>膂力<b>{{ d.aptitudes.strength }}</b></span><span>悟性<b>{{ d.aptitudes.intelligence }}</b></span>
            <span>根骨<b>{{ d.aptitudes.constitution }}</b></span><span>身法<b>{{ d.aptitudes.agility }}</b></span><span>福源<b>{{ d.aptitudes.fortune }}</b></span>
          </div>
          <div class="resource-lines">
            <span>气血 {{ d.attributes.qi.current }}/{{ d.attributes.qi.maximum }}</span>
            <span>精神 {{ d.attributes.spirit.current }}/{{ d.attributes.spirit.maximum }}</span>
            <span>内力 {{ d.attributes.neili.current }}/{{ d.attributes.neili.maximum }}</span>
            <span>精力 {{ d.attributes.energy.current }}/{{ d.attributes.energy.maximum }}</span>
          </div>
          <div class="attainment-line">造诣 {{ d.attributes.attainment }} · 功绩 {{ d.merit }} · 声名 {{ d.attributes.reputation }} · 道德 {{ d.attributes.morality }}</div>
          <div class="action-assignment">
            <select v-model="selected[d.id]" :disabled="disabled || !!d.away_months || d.condition !== 'healthy'">
              <option v-for="[value, label] in actions" :key="value" :value="value">{{ label }}</option>
            </select>
            <button class="btn btn-sm" :disabled="disabled || !!d.away_months || d.condition !== 'healthy'" @click="assign(d)">传令</button>
          </div>
          <div class="personnel-actions">
            <select v-model="selectedRank[d.id]" :disabled="disabled">
              <option value="chore">杂役</option><option value="outer">外门</option><option value="inner">内门</option><option value="elder">长老</option>
            </select>
            <button class="btn btn-sm" :disabled="disabled" @click="appoint(d)">考校任用</button>
            <button class="btn btn-sm" :disabled="disabled || !d.alive" @click="$emit('manage', { action: 'issue_item', disciple_id: d.id, item: '草药', quantity: 1 })">赐草药</button>
            <button class="btn btn-sm danger" :disabled="disabled" @click="expel(d)">逐出</button>
          </div>
        </div>
      </article>
    </div>
  </section>
</template>
