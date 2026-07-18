<script setup lang="ts">
import { reactive, ref } from 'vue'
import { MartialTier, SkillCategory } from '../types'
import type { ActionKind, Building, Department, Disciple, DiscipleRank, ManagementRequest, MartialArt, SkillEntry } from '../types'
import { artName as displayArtName, skillCategories, skillsInCategory } from '../skillDisplay'
import { issuableItems, medicineDescription } from '../medicine'

const props = defineProps<{
  disciples: Disciple[]
  arts: MartialArt[]
  buildings: Building[]
  inventory: Record<string, number>
  publicBooks: string[]
  martialResearch: Record<string, number>
  disabled?: boolean
}>()
const emit = defineEmits<{ manage: [command: ManagementRequest]; expel: [disciple: Disciple] }>()
const openId = ref<string | null>(null)
const selected = reactive<Record<string, ActionKind>>({})
const selectedRank = reactive<Record<string, DiscipleRank>>({})
const selectedDepartment = reactive<Record<string, Department | ''>>({})
const selectedMaster = reactive<Record<string, string>>({})
const selectedTarget = reactive<Record<string, string>>({})
const selectedMartial = reactive<Record<string, string>>({})
const selectedItem = reactive<Record<string, string>>({})
const innerActions: Array<[ActionKind, string]> = [
  ['read', '研读典籍'], ['practice', '练习武功'], ['temper_body', '打熬气血'], ['cultivate_neili', '修炼内力'],
  ['meditate', '冥想养神'], ['spar', '同门切磋'], ['teach', '传武授艺'],
  ['sect_mission', '外派办事'], ['wander', '江湖历练'], ['recover', '静养调息'],
]
const outerActions: Array<[ActionKind, string]> = [
  ['read', '研读典籍'], ['practice', '习武'], ['temper_body', '打熬气血'], ['cultivate_neili', '修炼内力'],
  ['meditate', '冥想养神'], ['spar', '陪练'], ['sect_mission', '江湖事务'], ['wander', '自由历练探险'], ['recover', '静养调息'],
]
const choreActions: Array<[ActionKind, string]> = [
  ['maintain', '建筑维护'], ['construct', '建造升级'], ['produce', '门中生产'], ['business', '世俗经营'], ['gather', '入山采集'], ['recover', '静养调息'],
]
const actionsFor = (disciple: Disciple) => disciple.rank === 'chore' ? choreActions : disciple.rank === 'outer' ? outerActions : innerActions
const actionName = (disciple: Disciple) => actionsFor(disciple).find(([kind]) => kind === disciple.action?.kind)?.[1] || '未安排'
const displayedSkillCategories = skillCategories
const rankName = { chore: '杂役', outer: '外门', inner: '内门' }
const departments: Array<[Department, string]> = [
  ['transmission', '传功'],
  ['library', '藏经'],
  ['apothecary', '药务'],
  ['treasury', '司库'],
  ['stewardship', '庶务'],
  ['external_affairs', '外务'],
]
const departmentName = (department?: Department | null) =>
  departments.find(([id]) => id === department)?.[1] || '未入部'
const conditionName = { healthy: '安好', exhausted: '力竭', unconscious: '昏迷', seriously_injured: '重伤', dead: '亡故' }
const masterIdOf = (disciple: Disciple) => disciple.master_id || ''
const masterName = (disciple: Disciple) =>
  props.disciples.find(candidate => candidate.id === masterIdOf(disciple))?.name || '未定'
const wouldCreateMasterCycle = (discipleId: string, masterId: string) => {
  const visited = new Set<string>()
  let cursor = masterId
  while (cursor && !visited.has(cursor)) {
    if (cursor === discipleId) return true
    visited.add(cursor)
    const current = props.disciples.find(candidate => candidate.id === cursor)
    cursor = current ? masterIdOf(current) : ''
  }
  return false
}
const masterCandidates = (disciple: Disciple) => props.disciples.filter(candidate =>
  candidate.id !== disciple.id
  && candidate.alive
  && candidate.rank === 'inner'
  && !wouldCreateMasterCycle(disciple.id, candidate.id))
const studentCount = (masterId: string) => props.disciples.filter(candidate =>
  candidate.alive && masterIdOf(candidate) === masterId).length
const masterAtCapacity = (masterId: string) => studentCount(masterId) >= 5
const chosenDepartment = (disciple: Disciple) =>
  Object.prototype.hasOwnProperty.call(selectedDepartment, disciple.id)
    ? selectedDepartment[disciple.id]
    : disciple.department || ''
const personnelChanged = (disciple: Disciple) =>
  (selectedRank[disciple.id] || disciple.rank) !== disciple.rank
  || (chosenDepartment(disciple) || null) !== (disciple.department || null)
const chosenMaster = (disciple: Disciple) =>
  Object.prototype.hasOwnProperty.call(selectedMaster, disciple.id)
    ? selectedMaster[disciple.id]
    : masterIdOf(disciple)
const artName = (id: string) => displayArtName(props.arts, id)
const targetName = (id?: string | null) =>
  props.disciples.find(disciple => disciple.id === id)?.name
  || props.buildings.find(building => building.id === id)?.name
  || id
  || ''
const journeyTemplateNames: Record<string, string> = {
  escort_supplies: '护送粮饷',
  seek_physician: '寻访名医',
  mediate_dispute: '调停地界',
  clear_bandits: '清剿路匪',
  free_wander: '江湖游历',
}
const journeyTemplateName = (id?: string) => journeyTemplateNames[id || ''] || '江湖行程'
const journeyDestinationNames: Record<string, string> = {
  xiangyang: '襄阳',
  linan: '临安',
  luoyang: '洛阳',
  dali: '大理',
  liangzhou: '凉州',
  taihu: '太湖',
}
const journeyDestinationName = (id?: string) => journeyDestinationNames[id || ''] || '江湖'
const actionSummary = (disciple: Disciple) => {
  const reservedBy = interactionOwner(disciple)
  if (!disciple.action && reservedBy?.action) {
    const detail = reservedBy.action.kind === 'teach'
      ? `随${reservedBy.name}受教${reservedBy.action.martial_art_id ? `《${artName(reservedBy.action.martial_art_id)}》` : ''}`
      : `与${reservedBy.name}切磋`
    return `${detail} · 已列同修`
  }
  const details = [actionName(disciple)]
  if (disciple.action?.journey) {
    details.push(`${journeyDestinationName(disciple.action.journey.destination_id)}·${journeyTemplateName(disciple.action.journey.template_id)}`)
  }
  if (disciple.action?.martial_art_id) details.push(`《${artName(disciple.action.martial_art_id)}》`)
  if (disciple.action?.target_id) details.push(targetName(disciple.action.target_id))
  if ((disciple.action?.remaining_months || 0) > 1) details.push(`尚余${disciple.action!.remaining_months}月`)
  return details.join(' · ')
}
const categorySkills = (disciple: Disciple, category: typeof skillCategories[number]['id']) =>
  skillsInCategory(disciple.skills, props.arts, category)
const art = (id: string) => props.arts.find(candidate => candidate.id === id)
const skillLevel = (disciple: Disciple, id: string) =>
  disciple.skills.find(skill => skill.martial_art_id === id)?.level || 0
const basicSkill = (disciple: Disciple, category: SkillCategory) =>
  categorySkills(disciple, category).find(skill => art(skill.martial_art_id)?.tier === MartialTier.Basic)
const combatChoices = (disciple: Disciple, basic: SkillEntry) =>
  disciple.skills
    .filter(skill => {
      const candidate = art(skill.martial_art_id)
      return candidate?.is_combat && candidate.tier !== MartialTier.Basic && candidate.basic_skill === basic.martial_art_id
    })
    .sort((a, b) => b.level - a.level || a.martial_art_id.localeCompare(b.martial_art_id))
const parryGroups = (disciple: Disciple) => {
  const choices = disciple.skills
    .filter(skill => {
      const candidate = art(skill.martial_art_id)
      return !!candidate
        && candidate.is_combat
        && candidate.tier !== MartialTier.Basic
        && (candidate.usable_for_parry === true
          || candidate.category === SkillCategory.Unarmed
          || candidate.category === SkillCategory.Weapon)
    })
    .sort((a, b) => b.level - a.level || a.martial_art_id.localeCompare(b.martial_art_id))
  return [
    { label: '拳脚武学', choices: choices.filter(skill => art(skill.martial_art_id)?.category === SkillCategory.Unarmed) },
    { label: '兵器武学', choices: choices.filter(skill => art(skill.martial_art_id)?.category === SkillCategory.Weapon) },
    {
      label: '特殊招架',
      choices: choices.filter(skill => {
        const candidate = art(skill.martial_art_id)
        return candidate?.usable_for_parry === true
          && candidate.category !== SkillCategory.Unarmed
          && candidate.category !== SkillCategory.Weapon
      }),
    },
  ].filter(group => group.choices.length)
}
const parryChoices = (disciple: Disciple) => parryGroups(disciple).flatMap(group => group.choices)
const preparedArt = (disciple: Disciple, basicId: string) => disciple.prepared_skills?.[basicId] || ''
const isPrepared = (disciple: Disciple, artId: string) => Object.values(disciple.prepared_skills || {}).includes(artId)
const highestKnowledge = (disciple: Disciple) => categorySkills(disciple, SkillCategory.Knowledge)[0]
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
const effectiveForceLevel = (disciple: Disciple) => {
  const basic = skillLevel(disciple, 'basic_force')
  const prepared = preparedArt(disciple, 'basic_force')
  const special = prepared && prepared !== 'basic_force'
    ? skillLevel(disciple, prepared)
    : Math.floor(basic / 2)
  return Math.floor(basic / 2) + special
}
const neiliTrainingCap = (disciple: Disciple) => {
  return Math.floor(effectiveForceLevel(disciple) * disciple.aptitudes.constitution * 2 / 3)
}
const energyTrainingCap = (disciple: Disciple) => {
  const knowledge = highestKnowledge(disciple)?.level || 0
  return Math.floor(knowledge * effectiveAptitude(disciple, 'intelligence') / 2)
}
const requiredBuilding = (kind: ActionKind): [string, string] | null => {
  if (['read', 'meditate'].includes(kind)) return ['scripture', '藏经阁']
  if (['practice', 'teach', 'spar', 'temper_body', 'cultivate_neili'].includes(kind)) return ['practice', '传功堂']
  if (['produce', 'business'].includes(kind)) return ['warehouse', '司库房']
  if (kind === 'gather') return ['herb_hall', '百草堂']
  if (['maintain', 'construct'].includes(kind)) return ['logistics', '庶务堂']
  return null
}
const buildingOperational = (id: string) => (props.buildings.find(building => building.id === id)?.condition || 0) > 0
const prepare = (disciple: Disciple, basicSkillId: string, event: Event) => {
  const martialArtId = (event.target as HTMLSelectElement).value
  if (!martialArtId || martialArtId === preparedArt(disciple, basicSkillId)) return
  emit('manage', {
    action: 'prepare_skill', disciple_id: disciple.id,
    basic_skill_id: basicSkillId, martial_art_id: martialArtId,
  })
}
const actionUnavailableReason = (disciple: Disciple, kind: ActionKind) => {
  const hall = requiredBuilding(kind)
  if (hall && !buildingOperational(hall[0])) return `${hall[1]}已损毁`
  if (kind === 'cultivate_neili') {
    if (disciple.attributes.neili.maximum >= neiliTrainingCap(disciple)) return '已达内力上限'
    if (disciple.attributes.spirit.current * 10 < disciple.attributes.spirit.maximum * 7) return '精神不足七成'
  }
  if (kind === 'meditate') {
    if (disciple.attributes.energy.maximum >= energyTrainingCap(disciple)) return '已达精力上限'
    if (disciple.attributes.qi.current * 10 < disciple.attributes.qi.maximum * 7) return '气血不足七成'
  }
  return ''
}
const actionUnavailable = (disciple: Disciple, kind: ActionKind) => !!actionUnavailableReason(disciple, kind)

const attainmentSkillCap = (disciple: Disciple) =>
  Math.ceil(Math.cbrt(Math.max(0, disciple.attributes.attainment) * 10))
const factionKnowledgeId = (candidate: MartialArt) =>
  candidate.sect_id ? `${candidate.sect_id}_knowledge` : ''
const skillGrowthCaps = (disciple: Disciple, skill: SkillEntry) => {
  const candidate = art(skill.martial_art_id)
  if (!candidate || !candidate.is_combat) return [] as Array<{ kind: string; level: number }>
  const caps = [{ kind: '造诣', level: attainmentSkillCap(disciple) }]
  caps.push({ kind: '门派参研', level: Math.max(50, props.martialResearch?.[skill.martial_art_id] || 0) })
  if (candidate.tier !== MartialTier.Basic && candidate.basic_skill) {
    caps.push({ kind: artName(candidate.basic_skill), level: skillLevel(disciple, candidate.basic_skill) })
  }
  const knowledgeId = factionKnowledgeId(candidate)
  if (candidate.tier !== MartialTier.Basic && knowledgeId) {
    caps.push({ kind: artName(knowledgeId), level: skillLevel(disciple, knowledgeId) })
  }
  return caps
}
const limitingSkillCap = (disciple: Disciple, skill: SkillEntry) =>
  skillGrowthCaps(disciple, skill).sort((a, b) => a.level - b.level)[0]
const skillBottleneck = (disciple: Disciple, skill: SkillEntry) => {
  const limiting = limitingSkillCap(disciple, skill)
  return limiting && limiting.level - skill.level <= 3
    ? `${limiting.kind}上限${limiting.level}级`
    : ''
}
const skillAtCap = (disciple: Disciple, skill: SkillEntry) => {
  const limiting = limitingSkillCap(disciple, skill)
  return limiting && limiting.level <= skill.level
    ? `${limiting.kind}已限于${limiting.level}级`
    : ''
}
const learningPrerequisite = (disciple: Disciple, candidate: MartialArt) => {
  if (!candidate.is_combat || candidate.tier === MartialTier.Basic) return ''
  if (candidate.basic_skill && skillLevel(disciple, candidate.basic_skill) <= 0) {
    return `须先习${artName(candidate.basic_skill)}`
  }
  const knowledgeId = factionKnowledgeId(candidate)
  if (knowledgeId && skillLevel(disciple, knowledgeId) <= 0) {
    return `须先研读${artName(knowledgeId)}`
  }
  return ''
}

interface ActionMartialChoice {
  id: string
  name: string
  level?: number
  note: string
  blocked?: string
}
const selectionKey = (disciple: Disciple, kind: ActionKind) => `${disciple.id}:${kind}`
const chosenAction = (disciple: Disciple) =>
  selected[disciple.id] || disciple.action?.kind || actionsFor(disciple)[0][0]
const isInteraction = (kind?: ActionKind) => kind === 'teach' || kind === 'spar'
const interactionOwner = (disciple: Disciple) => props.disciples.find(owner =>
  owner.id !== disciple.id
  && isInteraction(owner.action?.kind)
  && owner.action?.target_id === disciple.id)
const isReservedByOther = (disciple: Disciple, actor: Disciple) => props.disciples.some(owner =>
  owner.id !== actor.id
  && isInteraction(owner.action?.kind)
  && owner.action?.target_id === disciple.id)
const peerChoices = (disciple: Disciple) => props.disciples.filter(candidate =>
  candidate.id !== disciple.id
  && candidate.alive
  && !candidate.away_months
  && candidate.condition === 'healthy'
  && !candidate.action
  && !isReservedByOther(candidate, disciple))
const rankCanReceiveTier = (disciple: Disciple, tier: MartialTier) =>
  tier === MartialTier.Basic
  || tier === MartialTier.Chore
  || (tier === MartialTier.Outer && ['outer', 'inner'].includes(disciple.rank))
  || (tier === MartialTier.Inner && disciple.rank === 'inner')
const rankLearningPrerequisite = (disciple: Disciple, candidate: MartialArt) =>
  rankCanReceiveTier(disciple, candidate.tier) ? '' : `须先晋为${candidate.tier === MartialTier.Inner ? '内门' : '外门'}`
const canTeachSkillTo = (student: Disciple, skill: SkillEntry) => {
  const candidate = art(skill.martial_art_id)
  if (!candidate || !rankCanReceiveTier(student, candidate.tier)) return false
  if (learningPrerequisite(student, candidate)) return false
  const studentLevel = student.skills.find(known => known.martial_art_id === skill.martial_art_id)?.level || 0
  if (candidate.is_combat) {
    const studentSkill = student.skills.find(known => known.martial_art_id === skill.martial_art_id)
      || { martial_art_id: skill.martial_art_id, level: 0, experience: 0 }
    const limiting = limitingSkillCap(student, studentSkill)
    if (limiting && limiting.level <= studentLevel) return false
  }
  return skill.level > studentLevel
}
const canTeachDisciple = (teacher: Disciple, student: Disciple) =>
  student.master_id === teacher.id
  || (teacher.relations?.[student.id] || 0) >= 35
  || (student.relations?.[teacher.id] || 0) >= 35
const actionTargetChoices = (disciple: Disciple, kind: ActionKind) => {
  const peers = peerChoices(disciple)
  return kind === 'teach'
    ? peers.filter(student =>
      canTeachDisciple(disciple, student)
      && disciple.skills.some(skill => canTeachSkillTo(student, skill)))
    : peers
}
const actionNeedsTarget = (kind: ActionKind) => ['maintain', 'construct', 'teach', 'spar'].includes(kind)
const actionNeedsMartial = (kind: ActionKind) => ['read', 'practice', 'teach'].includes(kind)
const actionMartialChoices = (disciple: Disciple, kind: ActionKind): ActionMartialChoice[] => {
  if (kind === 'read') {
    const privateBooks = new Set(disciple.martial_progress?.private_books || [])
    return [...new Set([...(props.publicBooks || []), ...privateBooks])]
      .flatMap<ActionMartialChoice>(id => {
        const candidate = art(id)
        if (!candidate) return []
        const known = disciple.skills.find(skill => skill.martial_art_id === id)
        return [{
          id,
          name: candidate.name,
          level: known?.level,
          note: privateBooks.has(id) ? '私藏' : '公册',
          blocked: rankLearningPrerequisite(disciple, candidate) || learningPrerequisite(disciple, candidate),
        }]
      })
      .sort((a, b) => {
        const aKnowledge = art(a.id)?.category === SkillCategory.Knowledge ? 0 : 1
        const bKnowledge = art(b.id)?.category === SkillCategory.Knowledge ? 0 : 1
        return aKnowledge - bKnowledge || a.name.localeCompare(b.name, 'zh-CN')
      })
  }
  const student = kind === 'teach'
    ? actionTargetChoices(disciple, kind).find(candidate => candidate.id === selectedTargetFor(disciple, kind))
    : undefined
  return disciple.skills
    .filter(skill => {
      const candidate = art(skill.martial_art_id)
      return !!candidate
        && (kind === 'teach' ? !!student && canTeachSkillTo(student, skill) : candidate.is_combat)
    })
    .sort((a, b) => b.level - a.level || artName(a.martial_art_id).localeCompare(artName(b.martial_art_id), 'zh-CN'))
    .map(skill => ({
      id: skill.martial_art_id,
      name: artName(skill.martial_art_id),
      level: skill.level,
      note: art(skill.martial_art_id)?.type || '武学',
      blocked: kind === 'practice' ? skillAtCap(disciple, skill) : '',
    }))
}
const selectedTargetFor = (disciple: Disciple, kind: ActionKind) => {
  const choices = ['maintain', 'construct'].includes(kind)
    ? props.buildings.map(building => building.id)
    : actionTargetChoices(disciple, kind).map(candidate => candidate.id)
  const stored = selectedTarget[selectionKey(disciple, kind)]
    || (disciple.action?.kind === kind ? disciple.action.target_id || '' : '')
  return choices.includes(stored) ? stored : choices[0] || ''
}
const selectedMartialFor = (disciple: Disciple, kind: ActionKind) => {
  const choices = actionMartialChoices(disciple, kind)
  const stored = selectedMartial[selectionKey(disciple, kind)]
    || (disciple.action?.kind === kind ? disciple.action.martial_art_id || '' : '')
  return choices.some(choice => choice.id === stored && !choice.blocked)
    ? stored
    : choices.find(choice => !choice.blocked)?.id || ''
}
const setAction = (disciple: Disciple, event: Event) => {
  selected[disciple.id] = (event.target as HTMLSelectElement).value as ActionKind
}
const setRank = (disciple: Disciple, event: Event) => {
  selectedRank[disciple.id] = (event.target as HTMLSelectElement).value as DiscipleRank
}
const setDepartment = (disciple: Disciple, event: Event) => {
  selectedDepartment[disciple.id] = (event.target as HTMLSelectElement).value as Department | ''
}
const setMaster = (disciple: Disciple, event: Event) => {
  selectedMaster[disciple.id] = (event.target as HTMLSelectElement).value
}
const setActionTarget = (disciple: Disciple, kind: ActionKind, event: Event) => {
  selectedTarget[selectionKey(disciple, kind)] = (event.target as HTMLSelectElement).value
}
const setActionMartial = (disciple: Disciple, kind: ActionKind, event: Event) => {
  selectedMartial[selectionKey(disciple, kind)] = (event.target as HTMLSelectElement).value
}
const missingActionChoice = (disciple: Disciple) => {
  const kind = chosenAction(disciple)
  return actionUnavailable(disciple, kind)
    || (actionNeedsTarget(kind) && !selectedTargetFor(disciple, kind))
    || (actionNeedsMartial(kind) && !selectedMartialFor(disciple, kind))
}
const orderBlocked = (disciple: Disciple) =>
  !!props.disabled
  || !!disciple.away_months
  || disciple.condition !== 'healthy'
  || !!interactionOwner(disciple)
const assign = (disciple: Disciple) => {
  const kind = chosenAction(disciple)
  emit('manage', {
    action: 'assign_action',
    disciple_id: disciple.id,
    kind,
    target_id: actionNeedsTarget(kind) ? selectedTargetFor(disciple, kind) : null,
    martial_art_id: actionNeedsMartial(kind) ? selectedMartialFor(disciple, kind) : null,
  })
}
const appoint = (disciple: Disciple) => emit('manage', {
  action: 'set_personnel', disciple_id: disciple.id,
  rank: selectedRank[disciple.id] || disciple.rank, department: chosenDepartment(disciple) || null,
})
const appointMaster = (disciple: Disciple) => emit('manage', {
  action: 'assign_master',
  disciple_id: disciple.id,
  master_id: chosenMaster(disciple) || null,
})
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
          <span class="disciple-brief">{{ d.age }}岁 · {{ departmentName(d.department) }} · {{ d.rank === 'chore' ? '尚未拜师' : `师承${masterName(d)}` }} · 门忠{{ d.attributes?.sect_loyalty ?? d.loyalty }} · 本月{{ actionSummary(d) }}</span>
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
          <div class="personal-resource-line">私银：{{ d.personal_silver ?? 0 }} 两 · 口粮：{{ d.personal_rations ?? 0 }} 份</div>
          <div v-if="d.action?.journey" class="personal-resource-line">
            旅程卷宗：{{ journeyDestinationName(d.action.journey.destination_id) }} ·
            {{ journeyTemplateName(d.action.journey.template_id) }} · 难度 {{ d.action.journey.difficulty }} ·
            进度 {{ d.action.journey.elapsed_months }}/{{ d.action.journey.total_months }} ·
            {{ d.action.journey.encounter_resolved ? '奇遇已结' : '奇遇待归山揭晓' }}
          </div>
          <div class="attainment-line">造诣 {{ d.attributes.attainment }} · 功绩 {{ d.merit }} · 声名 {{ d.attributes.reputation }} · 道德 {{ d.attributes.morality }}</div>
          <div class="disciple-skills">
            <div class="skill-caption">门下武学谱 <small>造诣、门派参研、基础武学与门派知识共同限制战斗武学</small></div>
            <div class="skill-category-grid">
              <section v-for="category in displayedSkillCategories" :key="category.id" class="skill-category" :class="category.id">
                <header><b>{{ category.label }}</b><small>{{ category.hint }}</small></header>
                <label v-if="category.id === SkillCategory.Parry && parryChoices(d).length" class="preparation-picker">
                  <span>当前准备</span>
                  <select class="wuxia-select" :value="preparedArt(d, 'basic_parry')" @change="prepare(d, 'basic_parry', $event)">
                    <optgroup v-for="group in parryGroups(d)" :key="group.label" :label="group.label">
                      <option v-for="skill in group.choices" :key="skill.martial_art_id" :value="skill.martial_art_id">
                        {{ artName(skill.martial_art_id) }} · {{ skill.level }}级
                      </option>
                    </optgroup>
                  </select>
                </label>
                <label v-else-if="category.id !== SkillCategory.Knowledge && basicSkill(d, category.id) && combatChoices(d, basicSkill(d, category.id)!).length" class="preparation-picker">
                  <span>当前准备</span>
                  <select class="wuxia-select" :value="preparedArt(d, basicSkill(d, category.id)!.martial_art_id)" @change="prepare(d, basicSkill(d, category.id)!.martial_art_id, $event)">
                    <option v-for="skill in combatChoices(d, basicSkill(d, category.id)!)" :key="skill.martial_art_id" :value="skill.martial_art_id">
                      {{ artName(skill.martial_art_id) }} · {{ skill.level }}级
                    </option>
                  </select>
                </label>
                <div v-else-if="category.id === SkillCategory.Knowledge && highestKnowledge(d)" class="auto-preparation">
                  自动准备 {{ artName(highestKnowledge(d)!.martial_art_id) }} · {{ highestKnowledge(d)!.level }}级
                </div>
                <div v-if="categorySkills(d, category.id).length" class="category-skill-list">
                  <span v-for="skill in categorySkills(d, category.id)" :key="skill.martial_art_id" class="skill-entry" :class="{ prepared: isPrepared(d, skill.martial_art_id), bottleneck: !!skillBottleneck(d, skill) }">
                    <b>{{ artName(skill.martial_art_id) }}</b>
                    <em>{{ skill.level }}级<span v-if="isPrepared(d, skill.martial_art_id)"> · 已准备</span></em>
                    <small>经验 {{ skill.experience }}<strong v-if="skillBottleneck(d, skill)"> · 瓶颈：{{ skillBottleneck(d, skill) }}</strong></small>
                  </span>
                </div>
                <span v-else class="skill-empty">未录入</span>
              </section>
            </div>
          </div>
          <div class="action-assignment">
            <select :value="chosenAction(d)" :disabled="orderBlocked(d)" :title="interactionOwner(d) ? `已列入${interactionOwner(d)!.name}的同修安排` : ''" @change="setAction(d, $event)">
              <option v-for="[value, label] in actionsFor(d)" :key="value" :value="value" :disabled="actionUnavailable(d, value)">
                {{ label }}{{ actionUnavailableReason(d, value) ? `（${actionUnavailableReason(d, value)}）` : '' }}
              </option>
            </select>
            <select
              v-if="['maintain', 'construct'].includes(chosenAction(d))"
              :value="selectedTargetFor(d, chosenAction(d))"
              :disabled="orderBlocked(d)"
              aria-label="选择建筑"
              @change="setActionTarget(d, chosenAction(d), $event)"
            >
              <option v-for="building in buildings" :key="building.id" :value="building.id">{{ building.name }}</option>
            </select>
            <select
              v-else-if="['teach', 'spar'].includes(chosenAction(d))"
              :value="selectedTargetFor(d, chosenAction(d))"
              :disabled="orderBlocked(d)"
              aria-label="选择同门"
              @change="setActionTarget(d, chosenAction(d), $event)"
            >
              <option v-if="!actionTargetChoices(d, chosenAction(d)).length" value="">
                {{ chosenAction(d) === 'teach' ? '暂无嫡传或亲近同门可授' : '暂无可同行门' }}
              </option>
              <option v-for="peer in actionTargetChoices(d, chosenAction(d))" :key="peer.id" :value="peer.id">{{ peer.name }} · {{ rankName[peer.rank] }}</option>
            </select>
            <select
              v-if="actionNeedsMartial(chosenAction(d))"
              :value="selectedMartialFor(d, chosenAction(d))"
              :disabled="orderBlocked(d)"
              aria-label="选择武学"
              @change="setActionMartial(d, chosenAction(d), $event)"
            >
              <option v-if="!actionMartialChoices(d, chosenAction(d)).length" value="">暂无可选武学</option>
              <option v-for="choice in actionMartialChoices(d, chosenAction(d))" :key="choice.id" :value="choice.id" :disabled="!!choice.blocked">
                {{ choice.name }}{{ choice.level === undefined ? '' : ` · ${choice.level}级` }} · {{ choice.note }}{{ choice.blocked ? `（${choice.blocked}）` : '' }}
              </option>
            </select>
            <button class="btn btn-sm" :disabled="orderBlocked(d) || missingActionChoice(d)" :title="interactionOwner(d) ? `已列入${interactionOwner(d)!.name}的同修安排` : ''" @click="assign(d)">传令</button>
          </div>
          <div class="personnel-actions">
            <div class="personnel-role-controls">
              <select :value="selectedRank[d.id] || d.rank" :disabled="disabled || !d.alive" aria-label="弟子品秩" @change="setRank(d, $event)">
                <option value="chore">杂役</option><option value="outer">外门</option><option value="inner">内门</option>
              </select>
              <select :value="chosenDepartment(d)" :disabled="disabled || !d.alive" aria-label="六部任职" @change="setDepartment(d, $event)">
                <option value="">未入六部</option>
                <option v-for="[id, name] in departments" :key="id" :value="id">{{ name }}</option>
              </select>
              <button class="btn btn-sm" :disabled="disabled || !d.alive || !personnelChanged(d)" @click="appoint(d)">考校任用</button>
            </div>
            <div v-if="d.rank !== 'chore'" class="master-controls">
              <span>师承</span>
              <select :value="chosenMaster(d)" :disabled="disabled || !d.alive" aria-label="选择师父" @change="setMaster(d, $event)">
                <option value="">解除师承</option>
                <option
                  v-if="masterIdOf(d) && !masterCandidates(d).some(master => master.id === masterIdOf(d))"
                  :value="masterIdOf(d)"
                  disabled
                >
                  原师承 {{ masterName(d) }}（不可任）
                </option>
                <option
                  v-for="master in masterCandidates(d)"
                  :key="master.id"
                  :value="master.id"
                  :disabled="masterAtCapacity(master.id)"
                >
                  {{ master.name }} · 门下{{ studentCount(master.id) }}/5{{ masterAtCapacity(master.id) ? '（已满）' : '' }}
                </option>
              </select>
              <button class="btn btn-sm" :disabled="disabled || !d.alive || chosenMaster(d) === masterIdOf(d)" @click="appointMaster(d)">定师承</button>
            </div>
            <div v-else class="master-controls master-hint">
              <span>师承</span><small>杂役须先晋为外门，方可拜师。</small>
            </div>
            <div class="personnel-item-controls">
              <label class="issue-item-picker" :class="{ 'medicine-tooltip': medicineDescription(selectedItem[d.id] || '') }" :data-tooltip="medicineDescription(selectedItem[d.id] || '')">
                <select v-model="selectedItem[d.id]" :disabled="disabled"><option value="">赐物</option><option v-for="item in issuableItems" :key="item" :value="item" :disabled="!(inventory[item] || 0)">{{ item }}（{{ inventory[item] || 0 }}）</option></select>
              </label>
              <button class="btn btn-sm" :disabled="disabled || !d.alive || !selectedItem[d.id]" @click="$emit('manage', { action: 'issue_item', disciple_id: d.id, item: selectedItem[d.id], quantity: 1 })">赐予</button>
              <button class="btn btn-sm danger" :disabled="disabled" @click="emit('expel', d)">逐出</button>
            </div>
          </div>
        </div>
      </article>
    </div>
  </section>
</template>
