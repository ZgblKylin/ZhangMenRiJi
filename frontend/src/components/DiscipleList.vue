<script setup lang="ts">
import { reactive, ref } from 'vue'
import type { ActionKind, Disciple, DiscipleRank, ManagementRequest, MartialArt, SkillCategory, SkillEntry } from '../types'
import { artName as displayArtName, skillCategories, skillsInCategory } from '../skillDisplay'

const props = defineProps<{ disciples: Disciple[]; arts: MartialArt[]; disabled?: boolean }>()
const emit = defineEmits<{ manage: [command: ManagementRequest] }>()
const openId = ref<string | null>(null)
const selected = reactive<Record<string, ActionKind>>({})
const selectedRank = reactive<Record<string, DiscipleRank>>({})
const actions: Array<[ActionKind, string]> = [
  ['read', '研读典籍'], ['practice', '练习武功'], ['temper_body', '打熬气血'], ['cultivate_neili', '修炼内力'],
  ['meditate', '冥想养神'], ['spar', '同门切磋'], ['teach', '传功授艺'],
  ['sect_mission', '外派办事'], ['wander', '江湖历练'], ['recover', '静养调息'],
]
const rankName = { chore: '杂役', outer: '外门', inner: '内门', elder: '长老' }
const conditionName = { healthy: '安好', exhausted: '力竭', unconscious: '昏迷', seriously_injured: '重伤', dead: '亡故' }
const artName = (id: string) => displayArtName(props.arts, id)
const categorySkills = (disciple: Disciple, category: typeof skillCategories[number]['id']) =>
  skillsInCategory(disciple.skills, props.arts, category)
const art = (id: string) => props.arts.find(candidate => candidate.id === id)
const skillLevel = (disciple: Disciple, id: string) =>
  disciple.skills.find(skill => skill.martial_art_id === id)?.level || 0
const basicSkill = (disciple: Disciple, category: SkillCategory) =>
  categorySkills(disciple, category).find(skill => art(skill.martial_art_id)?.tier === 'basic')
const combatChoices = (disciple: Disciple, basic: SkillEntry) =>
  disciple.skills
    .filter(skill => {
      const candidate = art(skill.martial_art_id)
      return candidate?.is_combat && candidate.tier !== 'basic' && candidate.basic_skill === basic.martial_art_id
    })
    .sort((a, b) => b.level - a.level || a.martial_art_id.localeCompare(b.martial_art_id))
const equippedArt = (disciple: Disciple, basicId: string) => disciple.equipped_skills?.[basicId] || ''
const isEquipped = (disciple: Disciple, artId: string) => Object.values(disciple.equipped_skills || {}).includes(artId)
const highestKnowledge = (disciple: Disciple) => categorySkills(disciple, 'knowledge')[0]
const aptitudeBonus = (disciple: Disciple, aptitude: 'strength' | 'intelligence' | 'constitution' | 'agility') => {
  const source = {
    strength: skillLevel(disciple, 'basic_unarmed'),
    intelligence: highestKnowledge(disciple)?.level || 0,
    constitution: skillLevel(disciple, 'basic_force'),
    agility: skillLevel(disciple, 'basic_dodge'),
  }
  return Math.floor(source[aptitude] / 10)
}
const effectiveAptitude = (disciple: Disciple, aptitude: 'strength' | 'intelligence' | 'constitution' | 'agility') =>
  disciple.aptitudes[aptitude] + aptitudeBonus(disciple, aptitude)
const neiliTrainingCap = (disciple: Disciple) => {
  const force = equippedArt(disciple, 'basic_force') || 'basic_force'
  return Math.floor(skillLevel(disciple, force) * effectiveAptitude(disciple, 'constitution') * 2 / 3)
}
const energyTrainingCap = (disciple: Disciple) => {
  const knowledge = highestKnowledge(disciple)?.level || 0
  return Math.floor(knowledge * effectiveAptitude(disciple, 'intelligence') / 2)
}
const equip = (disciple: Disciple, basicSkillId: string, event: Event) => {
  const martialArtId = (event.target as HTMLSelectElement).value
  if (!martialArtId || martialArtId === equippedArt(disciple, basicSkillId)) return
  emit('manage', {
    action: 'equip_skill', disciple_id: disciple.id,
    basic_skill_id: basicSkillId, martial_art_id: martialArtId,
  })
}
const actionUnavailable = (disciple: Disciple, kind: ActionKind) =>
  (kind === 'cultivate_neili' && disciple.attributes.neili.maximum >= neiliTrainingCap(disciple))
  || (kind === 'meditate' && disciple.attributes.energy.maximum >= energyTrainingCap(disciple))
const assign = (disciple: Disciple) => emit('manage', {
  action: 'assign_action', disciple_id: disciple.id,
  kind: selected[disciple.id] || 'read', target_id: null, martial_art_id: null,
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
            <span>膂力<b>{{ effectiveAptitude(d, 'strength') }}<small v-if="aptitudeBonus(d, 'strength')">先天{{ d.aptitudes.strength }} +{{ aptitudeBonus(d, 'strength') }}</small></b></span>
            <span>悟性<b>{{ effectiveAptitude(d, 'intelligence') }}<small v-if="aptitudeBonus(d, 'intelligence')">先天{{ d.aptitudes.intelligence }} +{{ aptitudeBonus(d, 'intelligence') }}</small></b></span>
            <span>根骨<b>{{ effectiveAptitude(d, 'constitution') }}<small v-if="aptitudeBonus(d, 'constitution')">先天{{ d.aptitudes.constitution }} +{{ aptitudeBonus(d, 'constitution') }}</small></b></span>
            <span>身法<b>{{ effectiveAptitude(d, 'agility') }}<small v-if="aptitudeBonus(d, 'agility')">先天{{ d.aptitudes.agility }} +{{ aptitudeBonus(d, 'agility') }}</small></b></span>
            <span>福源<b>{{ d.aptitudes.fortune }}</b></span>
          </div>
          <div class="resource-lines">
            <span>气血 {{ d.attributes.qi.current }}/{{ d.attributes.qi.maximum }}</span>
            <span>精神 {{ d.attributes.spirit.current }}/{{ d.attributes.spirit.maximum }}</span>
            <span>内力 {{ d.attributes.neili.current }}/{{ d.attributes.neili.maximum }} <small>修炼上限 {{ neiliTrainingCap(d) }}</small></span>
            <span>精力 {{ d.attributes.energy.current }}/{{ d.attributes.energy.maximum }} <small>修炼上限 {{ energyTrainingCap(d) }}</small></span>
          </div>
          <div class="attainment-line">造诣 {{ d.attributes.attainment }} · 功绩 {{ d.merit }} · 声名 {{ d.attributes.reputation }} · 道德 {{ d.attributes.morality }}</div>
          <div class="disciple-skills">
            <div class="skill-caption">六艺武学谱 <small>知识限制本门战斗武学等级</small></div>
            <div class="skill-category-grid">
              <section v-for="category in skillCategories" :key="category.id" class="skill-category" :class="category.id">
                <header><b>{{ category.label }}</b><small>{{ category.hint }}</small></header>
                <label v-if="category.id !== 'knowledge' && basicSkill(d, category.id) && combatChoices(d, basicSkill(d, category.id)!).length" class="equipment-picker">
                  <span>当前装备</span>
                  <select :value="equippedArt(d, basicSkill(d, category.id)!.martial_art_id)" @change="equip(d, basicSkill(d, category.id)!.martial_art_id, $event)">
                    <option v-for="skill in combatChoices(d, basicSkill(d, category.id)!)" :key="skill.martial_art_id" :value="skill.martial_art_id">
                      {{ artName(skill.martial_art_id) }} · {{ skill.level }}级
                    </option>
                  </select>
                </label>
                <div v-else-if="category.id === 'knowledge' && highestKnowledge(d)" class="auto-equipment">
                  自动装备 {{ artName(highestKnowledge(d)!.martial_art_id) }} · {{ highestKnowledge(d)!.level }}级
                </div>
                <div v-if="categorySkills(d, category.id).length" class="category-skill-list">
                  <span v-for="skill in categorySkills(d, category.id)" :key="skill.martial_art_id" class="skill-entry" :class="{ equipped: isEquipped(d, skill.martial_art_id) }">
                    <b>{{ artName(skill.martial_art_id) }}</b>
                    <em>{{ skill.level }}级<span v-if="isEquipped(d, skill.martial_art_id)"> · 已装备</span></em>
                    <small>经验 {{ skill.experience }}</small>
                  </span>
                </div>
                <span v-else class="skill-empty">未录入</span>
              </section>
            </div>
          </div>
          <div class="action-assignment">
            <select v-model="selected[d.id]" :disabled="disabled || !!d.away_months || d.condition !== 'healthy'">
              <option v-for="[value, label] in actions" :key="value" :value="value" :disabled="actionUnavailable(d, value)">
                {{ label }}{{ actionUnavailable(d, value) ? '（已达上限）' : '' }}
              </option>
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
