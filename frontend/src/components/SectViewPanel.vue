<script setup lang="ts">
import { computed } from 'vue'
import type { Country, Disciple, MartialArt, SectState } from '../types'
import { artName as displayArtName, skillCategories, skillsInCategory } from '../skillDisplay'

const props = defineProps<{
  sect: SectState
  disciples: Disciple[]
  arts: MartialArt[]
  countries: Country[]
  playerSect: SectState
  npcSects: SectState[]
}>()
defineEmits<{ back: [] }>()

const policyNames = {
  balanced: '持中守成', martial: '崇武精进', scholarly: '研经明理',
  chivalrous: '行侠尚义', mercantile: '通商裕库', reclusive: '闭门清修',
}
const rankNames = { chore: '杂役', outer: '外门', inner: '内门', elder: '长老' }
const countryName = computed(() => props.countries.find(country => country.id === props.sect.country_id)?.name || props.sect.country_id)
const artName = (id: string) => displayArtName(props.arts, id)
const categorySkills = (disciple: Disciple, category: typeof skillCategories[number]['id']) =>
  skillsInCategory(disciple.skills, props.arts, category)
const relationName = (id: string) => {
  if (id === props.playerSect.id) return props.playerSect.name
  return props.npcSects.find(sect => sect.id === id)?.name || id
}
const relations = computed(() => {
  const values = { ...props.sect.relations }
  values[props.playerSect.id] ??= props.playerSect.relations[props.sect.id] || 0
  return Object.entries(values)
    .map(([id, value]) => ({ id, name: relationName(id), value }))
    .sort((a, b) => b.value - a.value)
})
const relationTone = (value: number) => value >= 40 ? 'friendly' : value < 0 ? 'hostile' : 'neutral'
</script>

<template>
  <div class="sect-view">
    <header class="sect-view-header">
      <button class="btn btn-sm" @click="$emit('back')">← 返回门派谱</button>
      <div>
        <span>江湖门派卷宗 · 只读</span>
        <h2>{{ sect.name }}</h2>
        <small>{{ countryName }} · {{ policyNames[sect.policy] }}</small>
      </div>
      <div class="sect-view-header-stats">
        <span><small>声望</small><b>{{ sect.attributes.prestige }}</b></span>
        <span><small>库银</small><b>{{ sect.attributes.silver }}<em>两</em></b></span>
        <span><small>道德</small><b>{{ sect.attributes.morality }}</b></span>
        <span><small>志气</small><b>{{ sect.attributes.morale }}</b></span>
      </div>
      <i>阅</i>
    </header>

    <section class="sect-ledger-section">
      <div class="sect-ledger-title">门人名录与所习武学</div>
      <div class="npc-disciple-grid">
        <article v-for="disciple in disciples" :key="disciple.id" class="npc-disciple-card">
          <div class="npc-disciple-head">
            <b>{{ disciple.name }}</b>
            <span>{{ rankNames[disciple.rank] }} · {{ disciple.age }}岁</span>
          </div>
          <small>内力 {{ disciple.attributes.neili.current }}/{{ disciple.attributes.neili.maximum }} · 造诣 {{ disciple.attributes.attainment }} · 声名 {{ disciple.attributes.reputation }}</small>
          <div class="npc-skill-groups">
            <section v-for="category in skillCategories" :key="category.id">
              <header>{{ category.label }}</header>
              <span v-for="skill in categorySkills(disciple, category.id)" :key="skill.martial_art_id">
                <b>{{ artName(skill.martial_art_id) }}</b><em>{{ skill.level }}</em>
              </span>
              <small v-if="!categorySkills(disciple, category.id).length">—</small>
            </section>
          </div>
        </article>
      </div>
    </section>

    <div class="sect-ledger-columns">
      <section class="sect-ledger-section">
        <div class="sect-ledger-title">山门建筑</div>
        <div class="readonly-building-list">
          <span v-for="building in sect.buildings" :key="building.id">
            <b>{{ building.name }}</b><small>第{{ building.level }}重 · 完好 {{ building.condition }}%</small>
          </span>
        </div>
      </section>
      <section class="sect-ledger-section">
        <div class="sect-ledger-title">仓库簿册</div>
        <div class="readonly-inventory">
          <span v-for="(count, name) in sect.inventory" :key="name"><b>{{ name }}</b>{{ count }}</span>
        </div>
        <div class="sect-ledger-title relation-title">门派关系</div>
        <div class="readonly-relations">
          <span v-for="relation in relations" :key="relation.id" :class="relationTone(relation.value)">
            {{ relation.name }} <b>{{ relation.value > 0 ? '+' : '' }}{{ relation.value }}</b>
          </span>
        </div>
      </section>
    </div>
  </div>
</template>
