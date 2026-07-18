<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { MartialTier, SkillCategory } from '../types'
import type { ActionKind, Building, BuildingKind, Decision, Disciple, GameState, ManagementRequest, MartialArt, MoralDirection, SectPolicy, SectState } from '../types'
import DecisionGrid from './DecisionGrid.vue'
import SectViewPanel from './SectViewPanel.vue'
import TournamentBracket from './TournamentBracket.vue'
import { medicineDescription, medicines } from '../medicine'

const props = defineProps<{
  game: GameState
  arts: MartialArt[]
  decisions: Decision[]
  used: string[]
  view: BuildingKind
}>()
const emit = defineEmits<{ manage: [command: ManagementRequest]; decide: [id: string] }>()
const policyNames: Array<[SectPolicy, string, string]> = [
  ['balanced', '持中守成', '诸务均衡'], ['martial', '崇武精进', '偏重练武'], ['scholarly', '研经明理', '偏重研读'],
  ['chivalrous', '行侠尚义', '积攒道义'], ['mercantile', '通商裕库', '增益进项'], ['reclusive', '闭门清修', '加快调息'],
]
const moralDirections: Array<[MoralDirection, string, string]> = [
  ['righteous', '行侠仗义', '扶危济困，道德渐长'], ['neutral', '独善其身', '不涉恩怨，安定门心'], ['villainous', '为非作歹', '劫掠牟利，道德声望渐损、反噬增多'],
]
const elderDuties: Record<BuildingKind, Array<[string, string, string]>> = {
  practice: [['instruct', '整饬教习', '本门志气提升 3 点'], ['drill', '主持月考', '所有在门弟子各添 2 点功绩']],
  scripture: [['curate', '校勘群籍', '每部武学公册的可授上限提升 4 级'], ['comprehend', '邀众合参', '集中参悟首部武学公册，可授上限提升 18 级']],
  warehouse: [['audit', '清点旧账', '追回 12 至 24 两库银'], ['purchase', '下山采买', '耗费 15 两，购入 5 份草药']],
  herb_hall: [['treat', '诊治掌门', '掌门伤势降低 8 点'], ['brew', '开炉炼丹', '选择一种丹药额外炼制']],
  intelligence: [['correspond', '修书诸派', '与所有门派的交情各提升 2 点'], ['scout', '查探江湖', '本门声望提升 3 点']],
  affairs: [['recruit', '代访新人', '耗费 25 两，为门中访得一名新人'], ['arbitrate', '处置事务', '依门风提升道德、志气或库银']],
  logistics: [['maintain', '巡检诸堂', '耗库银 8 两、精铁 2 份恢复 4 点；不足时仅恢复 2 点'], ['supervise', '亲临督造', '所有在建工程各增加 6 点工作量'], ['expand', '督造扩建', '选取一栋建筑，按其等级耗库银与精铁立项扩建']],
}
const dispatchTasks: Partial<Record<BuildingKind, Array<[ActionKind, string, string]>>> = {
  practice: [['practice', '演武练习', '持续期间每月耗私银 2 两；不足则效果减半']],
  scripture: [['read', '入阁研读', '按月研读藏经阁典籍']],
  warehouse: [['business', '下山采买', '为门派带回库银并赚取私银'], ['produce', '盘库整理', '整理物资并赚取私银']],
  herb_hall: [['gather', '入山采药', '按月为百草堂采回草药']],
}
const dispatchDisciple = reactive<Partial<Record<BuildingKind, string>>>({})
const dispatchKind = reactive<Partial<Record<BuildingKind, ActionKind>>>({})
const dispatchDuration = reactive<Partial<Record<BuildingKind, number>>>({})
const orders = [
  ['diligent', '勤修令', '三月内门人更喜练武', 60], ['righteous', '尚义令', '四月内涵养门风', 80],
  ['frugal', '节用令', '四月内节用裕库', 50], ['rest', '调息令', '两月内静养更佳', 45],
]
const recipeRates = ['常速', '低速', '极低速'] as const
const decisionIds: Record<BuildingKind, string[]> = {
  practice: ['train', 'teach'], scripture: ['study', 'research'], warehouse: [], herb_hall: ['rest'],
  intelligence: [], affairs: ['recruit', 'mission'], logistics: [],
}
const buildingDecisions = computed(() => props.decisions.filter(decision => decisionIds[props.view].includes(decision.id)))
const currentBuilding = computed(() => props.game.sect.buildings.find(building => building.kind === props.view))
const upgradeableBuildings = computed(() => props.game.sect.buildings.filter(building => building.work_required === 0))
const assignedIds = computed(() => new Set(props.game.sect.buildings.map(building => building.elder_id).filter(Boolean)))
const elderCandidates = computed(() => props.game.disciples.filter(disciple =>
  disciple.alive && disciple.rank === 'inner' && (!assignedIds.value.has(disciple.id) || disciple.id === currentBuilding.value?.elder_id)))
const currentElder = computed(() => props.game.disciples.find(disciple => disciple.id === currentBuilding.value?.elder_id))
const elderName = computed(() => currentElder.value?.name || '暂缺')
const alive = computed(() => props.game.disciples.filter(disciple => disciple.alive))
const rankCount = (rank: 'chore' | 'outer' | 'inner') => alive.value.filter(disciple => disciple.rank === rank).length
const outerLimit = computed(() => Math.floor(alive.value.length * props.game.sect.rank_rules.outer_ratio))
const innerLimit = computed(() => rankCount('outer') > 0
  ? Math.max(1, Math.floor(rankCount('outer') * props.game.sect.rank_rules.inner_ratio))
  : 0)
const artName = (id: string) => props.arts.find(art => art.id === id)?.name || id
const isCombatArt = (id: string) => props.arts.find(art => art.id === id)?.is_combat === true
const martialResearchCap = (id: string) => Math.max(50, props.game.sect.martial_research[id] || 0)
const countryState = (id: string) => props.game.countries.find(country => country.id === id)
const countryName = (id: string) => countryState(id)?.name || id
const prosperityLabel = (value: number) => value >= 80 ? '鼎盛' : value >= 60 ? '丰足' : value >= 40 ? '疲敝' : '凋敝'
const orderLabel = (value: number) => value >= 80 ? '清平' : value >= 60 ? '尚安' : value >= 40 ? '多事' : '动荡'
const countryTone = (value: number) => value >= 80 ? 'flourishing' : value >= 60 ? 'steady' : value >= 40 ? 'strained' : 'unrest'
const boundedCountryValue = (value: number) => Math.max(0, Math.min(100, value))
const marketPercent = (value: number) => Math.max(85, Math.min(115, 100 + Math.trunc((boundedCountryValue(value) - 70) / 2)))
const safetyPercent = (value: number) => Math.max(85, Math.min(115, 100 + Math.trunc((boundedCountryValue(value) - 65) / 2)))
const bilateralRelation = (sect: SectState) => {
  const playerToSect = props.game.sect.relations?.[sect.id] ?? 0
  const sectToPlayer = sect.relations?.[props.game.sect.id] ?? 0
  return {
    playerToSect,
    sectToPlayer,
    mutual: Math.min(playerToSect, sectToPlayer),
  }
}
const isAllianceEligible = (sect: SectState) => !['court', 'song_court'].includes(sect.id)
const hasAllianceWith = (sect: SectState) =>
  isAllianceEligible(sect) && bilateralRelation(sect).mutual >= 70
const alliedSects = computed(() => props.game.npc_sects
  .map(sect => ({ sect, relation: bilateralRelation(sect) }))
  .filter(entry => isAllianceEligible(entry.sect) && entry.relation.mutual >= 70)
  .sort((left, right) =>
    right.relation.mutual - left.relation.mutual
    || left.sect.name.localeCompare(right.sect.name, 'zh-CN')))
// --- 自创武学表单 ---
const createMartialForm = reactive({
  name: '',
  category: 'unarmed' as string,
  basicSkill: 'basic_unarmed',
  weaponBasic: 'basic_sword',
})

const categoryOptions = [
  ['unarmed', '拳脚'],
  ['dodge', '轻功'],
  ['force', '内功'],
  ['weapon', '兵器'],
] as const

const basicSkillOptions = computed(() => {
  const weaponBasics = ['basic_sword', 'basic_blade', 'basic_staff', 'basic_spear', 'basic_whip']
  if (createMartialForm.category === 'weapon') {
    return weaponBasics.map(id => [id, props.arts.find(a => a.id === id)?.name || id])
  }
  const mapping: Record<string, string> = {
    unarmed: 'basic_unarmed', dodge: 'basic_dodge', force: 'basic_force'
  }
  const basicId = mapping[createMartialForm.category]
  return [[basicId, props.arts.find(a => a.id === basicId)?.name || basicId]]
})

const weaponOptions = [
  ['basic_sword', '剑法'],
  ['basic_blade', '刀法'],
  ['basic_staff', '棍法'],
  ['basic_spear', '枪法'],
  ['basic_whip', '鞭法'],
]

const createdArts = computed(() => {
  const publicIds = new Set(props.game.sect.public_books)
  return [
    ...props.arts.filter(art => art.sect_id === 'player' && art.is_combat && publicIds.has(art.id)),
    ...(props.game.sect.created_martial_arts || []),
  ]
})

const isHeritageArt = (artId: string) => {
  return (props.game.sect.heritage_arts || []).includes(artId)
}

const availableCreationArts = computed(() => props.arts.filter(art => art.sect_id === 'player' && art.is_combat && !props.game.martial_arts_learned?.includes(art.id)))

const createEstimateSilver = computed(() => {
  const knowledgePower = Math.max(...Object.values(props.game.sect.martial_research || {}), 0)
  if (knowledgePower >= 260) return 280
  if (knowledgePower >= 130) return 160
  return 80
})

const onSubmitCreateMartial = () => {
  if (!createMartialForm.name.trim() || createMartialForm.name.trim().length > 8) return
  emit('manage', {
    action: 'create_martial_art',
    name: createMartialForm.name.trim(),
    category: createMartialForm.category,
    basic_skill: createMartialForm.basicSkill,
    weapon_basic: createMartialForm.category === 'weapon' ? createMartialForm.weaponBasic : undefined,
  })
  createMartialForm.name = ''
}


const tradeItem = ref('草药')
const tradeQty = ref(10)
const innerCandidates = computed(() => props.game.disciples.filter(d => d.alive && d.rank === 'inner' && d.condition === 'healthy'))

const command = (payload: ManagementRequest) => emit('manage', payload)
const appointElder = (event: Event) => command({ action: 'assign_elder', building_id: currentBuilding.value?.id, disciple_id: (event.target as HTMLSelectElement).value || null })
const viewedSectId = ref<string | null>(null)
type EnvoyAction = 'exchange' | 'request_manual'
interface EnvoyRequest { action: EnvoyAction; sectId: string; sectName: string }
interface ManualCandidate {
  art: MartialArt
  tierLabel: string
  requiredRelation: number
  requiredPrestige: number
  cost: number
  priority: number
}
interface PrivateBookHolding {
  id: string
  name: string
  ownerName: string
  ownerAlive: boolean
}
const envoyRequest = ref<EnvoyRequest | null>(null)
const selectedEnvoy = ref('')
const selectedManualId = ref('')
const interactionTargetIds = computed(() => new Set(props.game.disciples
  .filter(disciple => ['teach', 'spar'].includes(disciple.action?.kind || ''))
  .map(disciple => disciple.action?.target_id)
  .filter((id): id is string => !!id)))
const innerTravelers = computed(() => props.game.disciples.filter(disciple =>
  disciple.alive
  && disciple.rank === 'inner'
  && !disciple.away_months
  && disciple.condition === 'healthy'
  && !disciple.action
  && !interactionTargetIds.value.has(disciple.id)))
const viewedSect = computed(() => props.game.npc_sects.find(sect => sect.id === viewedSectId.value) || null)
const viewedDisciples = computed(() => props.game.npc_disciples.filter(disciple => disciple.sect_id === viewedSectId.value && disciple.alive))
const selectedTournamentYear = ref<number | null>(null)
const tournamentRecords = computed(() =>
  [...(props.game.tournament_history || [])].sort((left, right) => right.year - left.year))
const selectedTournament = computed(() =>
  tournamentRecords.value.find(record => record.year === selectedTournamentYear.value)
  || tournamentRecords.value[0]
  || null)
watch(
  () => tournamentRecords.value.map(record => record.year).join('|'),
  () => {
    if (!tournamentRecords.value.some(record => record.year === selectedTournamentYear.value)) {
      selectedTournamentYear.value = tournamentRecords.value[0]?.year ?? null
    }
  },
  { immediate: true },
)
const closeNpcSect = () => {
  viewedSectId.value = null
  envoyRequest.value = null
  selectedEnvoy.value = ''
  selectedManualId.value = ''
}
defineExpose({ closeNpcSect })

const manualRule = (candidate: MartialArt) => {
  if (candidate.category === SkillCategory.Knowledge) {
    return { tierLabel: '知识', requiredRelation: 10, requiredPrestige: 0, priority: 0 }
  }
  if (candidate.tier === MartialTier.Basic) {
    return { tierLabel: '基础', requiredRelation: 10, requiredPrestige: 0, priority: 1 }
  }
  if (candidate.tier === MartialTier.Chore) {
    return { tierLabel: '杂役', requiredRelation: 20, requiredPrestige: 40, priority: 2 }
  }
  if (candidate.tier === MartialTier.Outer) {
    return { tierLabel: '外门', requiredRelation: 45, requiredPrestige: 90, priority: 3 }
  }
  return { tierLabel: '内门', requiredRelation: 75, requiredPrestige: 180, priority: 4 }
}
const manualCandidatesFor = (sect: SectState): ManualCandidate[] => {
  const collected = new Set(props.game.sect.public_books || [])
  return [...new Set(sect.public_books || [])]
    .filter(id => !collected.has(id))
    .map(id => props.arts.find(candidate => candidate.id === id))
    .filter((candidate): candidate is MartialArt => !!candidate)
    .map(candidate => {
      const rule = manualRule(candidate)
      return {
        art: candidate,
        ...rule,
        cost: 60 + (candidate.difficulty || 0) * 5,
      }
    })
    .sort((a, b) => a.priority - b.priority || a.cost - b.cost || a.art.name.localeCompare(b.art.name, 'zh-CN'))
}
const currentManualSect = computed(() =>
  envoyRequest.value?.action === 'request_manual'
    ? props.game.npc_sects.find(sect => sect.id === envoyRequest.value?.sectId) || null
    : null)
const manualCandidates = computed(() => currentManualSect.value ? manualCandidatesFor(currentManualSect.value) : [])
const privateBookHoldings = computed<PrivateBookHolding[]>(() => {
  const publicBooks = new Set(props.game.sect.public_books || [])
  const seen = new Set<string>()
  const holdings: PrivateBookHolding[] = []
  for (const disciple of props.game.disciples) {
    for (const id of disciple.martial_progress?.private_books || []) {
      if (publicBooks.has(id) || seen.has(id)) continue
      const candidate = props.arts.find(art => art.id === id)
      if (!candidate) continue
      seen.add(id)
      holdings.push({
        id,
        name: candidate.name,
        ownerName: disciple.name,
        ownerAlive: disciple.alive,
      })
    }
  }
  return holdings.sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'))
})
const defaultBrewQueue = medicines.map(medicine => medicine.recipeId)
const brewQueueDraft = ref<string[]>([])
const brewQueueChoice = ref('')
watch(
  () => props.game.sect.auto_brew_queue,
  queue => {
    brewQueueDraft.value = [...(queue ?? defaultBrewQueue)]
    brewQueueChoice.value = ''
  },
  { immediate: true },
)
const liveBrewQueue = computed(() => props.game.sect.auto_brew_queue ?? defaultBrewQueue)
const availableBrewRecipes = computed(() => medicines.filter(medicine => !brewQueueDraft.value.includes(medicine.recipeId)))
const currentAutoRecipe = computed(() => {
  if (!liveBrewQueue.value.length) return null
  const id = liveBrewQueue.value[props.game.sect.auto_brew_index % liveBrewQueue.value.length]
  return medicines.find(medicine => medicine.recipeId === id) || null
})
const brewQueueChanged = computed(() => brewQueueDraft.value.join('|') !== liveBrewQueue.value.join('|'))
const addBrewRecipe = () => {
  if (!brewQueueChoice.value || brewQueueDraft.value.includes(brewQueueChoice.value)) return
  brewQueueDraft.value.push(brewQueueChoice.value)
  brewQueueChoice.value = ''
}
const removeBrewRecipe = (index: number) => brewQueueDraft.value.splice(index, 1)
const moveBrewRecipe = (index: number, offset: number) => {
  const target = index + offset
  if (target < 0 || target >= brewQueueDraft.value.length) return
  const [recipe] = brewQueueDraft.value.splice(index, 1)
  brewQueueDraft.value.splice(target, 0, recipe)
}
const clearBrewQueue = () => {
  brewQueueDraft.value = []
}
const saveBrewQueue = () => command({ action: 'set_auto_brew_queue', recipe_ids: [...brewQueueDraft.value] })
const selectedManual = computed(() => manualCandidates.value.find(candidate => candidate.art.id === selectedManualId.value) || null)
const manualLockReasons = (candidate: ManualCandidate, sectId = envoyRequest.value?.sectId || '') => {
  const reasons: string[] = []
  const relation = props.game.sect.relations[sectId] || 0
  const prestige = props.game.sect.attributes.prestige
  const silver = props.game.sect.attributes.silver
  if (relation < candidate.requiredRelation) reasons.push(`交情须达${candidate.requiredRelation}（当前${relation}）`)
  if (prestige < candidate.requiredPrestige) reasons.push(`声望须达${candidate.requiredPrestige}（当前${prestige}）`)
  if (silver < candidate.cost) reasons.push(`库银尚缺${candidate.cost - silver}两`)
  return reasons
}
const selectedManualLockReasons = computed(() => selectedManual.value ? manualLockReasons(selectedManual.value) : ['尚未选定典籍'])
const manualBlocked = computed(() => envoyRequest.value?.action === 'request_manual' && selectedManualLockReasons.value.length > 0)
const manualOptionText = (candidate: ManualCandidate) => {
  const locks = manualLockReasons(candidate)
  return `${candidate.art.name} · ${candidate.tierLabel} · 交情${candidate.requiredRelation}/声望${candidate.requiredPrestige} · ${candidate.cost}两${locks.length ? `（${locks.join('；')}）` : ''}`
}
const envoyCost = computed(() => envoyRequest.value?.action === 'exchange' ? 45 : selectedManual.value?.cost || 0)
const openEnvoyPicker = (action: EnvoyAction, sect: SectState) => {
  selectedEnvoy.value = ''
  selectedManualId.value = ''
  envoyRequest.value = { action, sectId: sect.id, sectName: sect.name }
  if (action === 'request_manual') selectedManualId.value = manualCandidatesFor(sect)[0]?.art.id || ''
}
const confirmEnvoy = () => {
  if (!envoyRequest.value || !selectedEnvoy.value || manualBlocked.value) return
  const request = envoyRequest.value
  command(request.action === 'exchange'
    ? { action: 'exchange', sect_id: request.sectId, disciple_id: selectedEnvoy.value }
    : { action: 'request_manual', sect_id: request.sectId, martial_art_id: selectedManual.value!.art.id, disciple_id: selectedEnvoy.value })
  envoyRequest.value = null
  selectedEnvoy.value = ''
  selectedManualId.value = ''
}

const recommendedElderDuty = (building: Building) => {
  switch (building.kind) {
    case 'practice': return props.game.sect.attributes.morale < 65 ? 'instruct' : 'drill'
    case 'scripture': return props.game.sect.public_books.length >= 5 ? 'curate' : 'comprehend'
    case 'warehouse': return props.game.sect.attributes.silver < 120 || (props.game.sect.inventory['草药'] || 0) >= 8 ? 'audit' : 'purchase'
    case 'herb_hall': return props.game.injury > 0 ? 'treat' : 'brew'
    case 'intelligence': return Object.values(props.game.sect.relations).some(value => value < 10) ? 'correspond' : 'scout'
    case 'affairs': return props.game.sect.attributes.silver >= 25 ? 'recruit' : 'arbitrate'
    case 'logistics':
      if (props.game.sect.buildings.some(item => item.work_required > 0)) return 'supervise'
      if (props.game.sect.buildings.some(item => item.condition < 70)) return 'maintain'
      return 'expand'
  }
}
const selectedDuty = (building: Building) => building.selected_duty || recommendedElderDuty(building)
const expansionTarget = (building: Building) => upgradeableBuildings.value.find(item => item.id === building.duty_target)?.id || upgradeableBuildings.value[0]?.id || null
const brewTarget = (building: Building) => medicines.find(item => item.recipeId === building.duty_target)?.recipeId || medicines[0]?.recipeId || null
const selectElderDuty = (building: Building, dutyId: string, dutyTarget?: string | null) => command({ action: 'set_elder_duty', building_id: building.id, duty_id: dutyId, duty_target: dutyTarget })
const selectExpansionTarget = (building: Building, event: Event) => selectElderDuty(building, 'expand', (event.target as HTMLSelectElement).value || null)
const selectBrewTarget = (building: Building, event: Event) => selectElderDuty(building, 'brew', (event.target as HTMLSelectElement).value || null)
const dispatchCandidates = computed(() => props.game.disciples.filter(disciple =>
  disciple.alive && !disciple.away_months && disciple.condition === 'healthy'))
const currentDispatchTasks = computed(() => dispatchTasks[props.view] || [])
const dispatchTask = () => {
  const discipleId = dispatchDisciple[props.view]
  const kind = dispatchKind[props.view] || currentDispatchTasks.value[0]?.[0]
  if (!currentBuilding.value || !discipleId || !kind) return
  command({
    action: 'dispatch_task', building_id: currentBuilding.value.id, disciple_id: discipleId,
    kind, duration_months: dispatchDuration[props.view] || 1,
  })
}
const warehouseInventory = computed(() => {
  const names = ['精铁', '粮秣', ...Object.keys(props.game.sect.inventory).filter(name => !['精铁', '粮秣'].includes(name))]
  return names.map(name => ({ name, count: props.game.sect.inventory[name] || 0 }))
})
const upgradeCost = (building: Building) => ({
  silver: 80 + building.level * 60,
  iron: 3 + building.level * 2,
})
const repairCost = (building: Building) => {
  const missing = Math.max(0, 100 - building.condition)
  return {
    silver: Math.max(10, missing * 2),
    iron: Math.max(1, Math.ceil(missing / 25)),
  }
}
const canAffordBuildingCost = (cost: { silver: number; iron: number }) =>
  props.game.sect.attributes.silver >= cost.silver
  && (props.game.sect.inventory['精铁'] || 0) >= cost.iron
const buildingCostText = (cost: { silver: number; iron: number }) => `库银${cost.silver}两 · 精铁${cost.iron}份`
const buildingEffectiveness = (building?: Building) => building
  ? Math.floor((80 + Math.max(1, building.level) * 20) * Math.max(0, Math.min(100, building.condition)) / 100)
  : 0
const currentEffectiveness = computed(() => buildingEffectiveness(currentBuilding.value))
const discipleSkillLevel = (disciple: Disciple, artId: string) =>
  disciple.martial_progress?.proficiencies?.[artId]?.level
  ?? disciple.skills?.find(skill => skill.martial_art_id === artId)?.level
  ?? 0
const discipleKnowledgeLevel = (disciple: Disciple) =>
  Math.max(0, ...Object.entries(disciple.martial_progress?.proficiencies || {})
    .filter(([id]) => props.arts.find(art => art.id === id)?.category === SkillCategory.Knowledge)
    .map(([, progress]) => progress.level))
const effectiveAptitudes = (disciple: Disciple) => ({
  strength: disciple.aptitudes.strength + Math.floor(Math.max(0, discipleSkillLevel(disciple, 'basic_unarmed')) / 10),
  intelligence: disciple.aptitudes.intelligence + Math.floor(discipleKnowledgeLevel(disciple) / 10),
  constitution: disciple.aptitudes.constitution + Math.floor(Math.max(0, discipleSkillLevel(disciple, 'basic_force')) / 10),
  agility: disciple.aptitudes.agility + Math.floor(Math.max(0, discipleSkillLevel(disciple, 'basic_dodge')) / 10),
  fortune: disciple.aptitudes.fortune,
})
const effectiveForceLevel = (disciple: Disciple) => {
  const basic = Math.max(0, discipleSkillLevel(disciple, 'basic_force'))
  const prepared = disciple.prepared_skills?.basic_force
  const special = prepared && prepared !== 'basic_force'
    ? discipleSkillLevel(disciple, prepared)
    : Math.floor(basic / 2)
  return Math.floor(basic / 2) + special
}
const expectedElderDepartment: Record<BuildingKind, Disciple['department']> = {
  practice: 'transmission',
  scripture: 'library',
  warehouse: 'treasury',
  herb_hall: 'apothecary',
  intelligence: 'external_affairs',
  affairs: 'stewardship',
  logistics: 'stewardship',
}
const elderCompetence = (disciple: Disciple, kind: BuildingKind) => {
  const expected = expectedElderDepartment[kind]
  const departmentAdjustment = disciple.department === expected ? 8 : disciple.department ? -4 : 0
  const aptitude = effectiveAptitudes(disciple)
  let relevantAptitude = 20
  let expertise = discipleKnowledgeLevel(disciple)
  if (kind === 'practice') {
    relevantAptitude = Math.trunc((aptitude.strength + aptitude.constitution + aptitude.agility) / 3)
    expertise = effectiveForceLevel(disciple)
  } else if (kind === 'scripture') {
    relevantAptitude = aptitude.intelligence
  } else if (kind === 'warehouse' || kind === 'affairs') {
    relevantAptitude = Math.trunc((aptitude.intelligence + aptitude.fortune) / 2)
  } else if (kind === 'herb_hall') {
    relevantAptitude = Math.trunc((aptitude.intelligence + aptitude.constitution) / 2)
  } else if (kind === 'intelligence') {
    relevantAptitude = Math.trunc((aptitude.intelligence + aptitude.agility + aptitude.fortune) / 3)
  } else {
    relevantAptitude = Math.trunc((aptitude.strength + aptitude.constitution) / 2)
    expertise = effectiveForceLevel(disciple)
  }
  const aptitudeAdjustment = Math.max(-5, Math.min(8, Math.trunc((relevantAptitude - 20) / 4)))
  const expertiseAdjustment = Math.max(0, Math.min(8, Math.trunc((Math.max(20, expertise) - 20) / 10)))
  const meritAdjustment = Math.trunc(Math.max(-150, Math.min(150, disciple.merit)) / 25)
  return Math.max(85, Math.min(130, 100 + departmentAdjustment + aptitudeAdjustment + expertiseAdjustment + meritAdjustment))
}
const elderCanServe = (disciple?: Disciple) => !!disciple
  && disciple.alive
  && disciple.rank === 'inner'
  && disciple.sect_id === props.game.sect.id
  && disciple.away_months <= 0
  && disciple.condition === 'healthy'
const currentElderCompetence = computed(() =>
  currentElder.value && currentBuilding.value
    ? elderCompetence(currentElder.value, currentBuilding.value.kind)
    : null)
const currentDutyEffectiveness = computed(() =>
  currentElderCompetence.value === null
    ? 0
    : Math.floor(currentEffectiveness.value * currentElderCompetence.value / 100))
const scaledBuildingOutput = (base: number, building?: Building) =>
  Math.max(1, Math.floor(base * buildingEffectiveness(building) / 100))
const scriptureBuilding = computed(() => props.game.sect.buildings.find(building => building.id === 'scripture'))
const herbHallBuilding = computed(() => props.game.sect.buildings.find(building => building.id === 'herb_hall'))
const researchGain = () => {
  const building = scriptureBuilding.value
  return scaledBuildingOutput(20 + (building?.level || 0) * 6, building)
}
const quickBrewMonths = (medicine: typeof medicines[number]) => {
  const quick = Math.floor((medicine.months * 2 + 2) / 3)
  const effectiveness = buildingEffectiveness(herbHallBuilding.value)
  return effectiveness > 0 ? Math.ceil(quick * 100 / effectiveness) : 0
}
const backgroundBrewMonths = (medicine: typeof medicines[number]) => {
  const effectiveness = buildingEffectiveness(herbHallBuilding.value)
  if (effectiveness <= 0) return 0
  const workMonths = Math.ceil(medicine.months * 100 / effectiveness)
  const herbElder = props.game.disciples.find(disciple => disciple.id === herbHallBuilding.value?.elder_id)
  return workMonths * (elderCanServe(herbElder) ? 1 : 2)
}
const totalBuildingLevels = computed(() => props.game.sect.buildings.reduce((total, building) => total + Math.max(0, building.level), 0))
const monthlyMaintenanceCost = computed(() => ({
  grain: Math.floor((totalBuildingLevels.value + 6) / 7),
  iron: Math.floor((totalBuildingLevels.value + 13) / 14),
}))
const recipeSilverCost = (medicine: typeof medicines[number]) => medicine.months * (medicine.rate === '常速' ? 2 : medicine.rate === '低速' ? 3 : 4)
</script>

<template>
  <section class="management-sheet">
    <header v-if="currentBuilding" class="building-header">
      <div>
        <b>{{ currentBuilding.name }}</b>
        <small>
          第{{ currentBuilding.level }}重 · 完好 {{ currentBuilding.condition }}% · 堂效 {{ currentEffectiveness }}%
          <template v-if="currentElderCompetence !== null"> · 主事 {{ currentElderCompetence }}% · 堂务合效 {{ currentDutyEffectiveness }}%</template>
        </small>
      </div>
      <label><span>{{ currentBuilding.elder_title }}</span>
        <select class="wuxia-select elder-select" :value="currentBuilding.elder_id || ''" :disabled="game.decisions_used >= game.max_decisions" @change="appointElder">
          <option value="">暂缺（掌门兼领）</option>
          <option v-for="disciple in elderCandidates" :key="disciple.id" :value="disciple.id">{{ disciple.name }} · 主事 {{ elderCompetence(disciple, view) }}%{{ elderCanServe(disciple) ? '' : '（暂不可理事）' }}</option>
        </select>
      </label>
      <i>{{ elderName }}<template v-if="currentElder && !elderCanServe(currentElder)"> · 暂不可理事</template></i>
      <div class="elder-duty-actions">
        <small>{{ currentBuilding.elder_action_used ? '本月堂务已办，过月仍照此办理' : '事务已择定，过月自动办理' }}</small>
        <button v-for="[id, label, description] in elderDuties[view]" :key="id" type="button" class="btn btn-sm elder-duty-option" :class="{ active: selectedDuty(currentBuilding) === id }" :data-tooltip="`${description}；堂效与主事能力合成为 ${currentDutyEffectiveness}%`" :aria-label="`${label}：${description}`" :disabled="!currentBuilding.elder_id || !elderCanServe(currentElder) || currentEffectiveness <= 0 || !!game.pending_event" @click="selectElderDuty(currentBuilding, id, id === 'expand' ? expansionTarget(currentBuilding) : id === 'brew' ? brewTarget(currentBuilding) : undefined)">{{ label }}</button>
        <label v-if="selectedDuty(currentBuilding) === 'expand'" class="elder-duty-target"><span>扩建目标</span>
          <select class="wuxia-select" :value="expansionTarget(currentBuilding) || ''" :disabled="!currentBuilding.elder_id || !!game.pending_event || !upgradeableBuildings.length" @change="selectExpansionTarget(currentBuilding, $event)">
            <option v-if="!upgradeableBuildings.length" value="">暂无可扩建建筑</option>
            <option v-for="building in upgradeableBuildings" :key="building.id" :value="building.id">{{ building.name }} · 第{{ building.level }}重 · {{ buildingCostText(upgradeCost(building)) }}</option>
          </select>
        </label>
        <label v-if="selectedDuty(currentBuilding) === 'brew'" class="elder-duty-target"><span>丹药</span>
          <select class="wuxia-select" :value="brewTarget(currentBuilding) || ''" :disabled="!currentBuilding.elder_id || !!game.pending_event || !medicines.length" @change="selectBrewTarget(currentBuilding, $event)">
            <option v-for="medicine in medicines" :key="medicine.recipeId" :value="medicine.recipeId">{{ medicine.name }} · 草药{{ medicine.herbCost }}份 · 产{{ medicine.quantity }}份</option>
          </select>
        </label>
      </div>
    </header>

    <DecisionGrid v-if="buildingDecisions.length" :decisions="buildingDecisions" :arts="arts" :game="game" :used="used" :building-effectiveness="currentEffectiveness" :title="`${currentBuilding?.name || ''}本月议事`" @decide="$emit('decide', $event)" />

    <section v-if="currentDispatchTasks.length" class="dispatch-panel">
      <div class="management-row-title">建筑派发任务</div>
      <div class="dispatch-controls">
        <select v-model="dispatchDisciple[view]" class="wuxia-select">
          <option value="">选择弟子</option>
          <option v-for="disciple in dispatchCandidates" :key="disciple.id" :value="disciple.id">{{ disciple.name }} · 私银{{ disciple.personal_silver ?? 0 }}两</option>
        </select>
        <select v-model="dispatchKind[view]" class="wuxia-select">
          <option v-for="[kind, label, description] in currentDispatchTasks" :key="kind" :value="kind">{{ label }} · {{ description }}</option>
        </select>
        <select v-model="dispatchDuration[view]" class="wuxia-select"><option :value="1">1个月</option><option :value="2">2个月</option><option :value="3">3个月</option></select>
        <button class="btn btn-sm" :disabled="!dispatchDisciple[view] || currentEffectiveness <= 0 || game.decisions_used >= game.max_decisions || !!game.pending_event" @click="dispatchTask">派发</button>
      </div>
    </section>

    <template v-if="view === 'practice'">
      <div class="management-row-title">传武授艺</div>
      <p class="building-prose">由传武长老总领门中教习、外门习武与同门陪练。弟子的个人行止可在左侧谱牒中安排。</p>
    </template>

    <template v-else-if="view === 'scripture'">
      <div class="library-summary">藏经阁共收公册 {{ game.sect.public_books.length }} 部，可闭关练功、研习武功或合参典籍。</div>
      <!-- 自创武学面板 -->
      <section class="create-martial-panel">
        <div class="management-row-title">自创武学</div>
        <p class="building-prose">融汇门中诸般造诣，掌门亲创一门新武学。名称须为一至八字，所悟层次以本门研究深浅而定。</p>
        <div class="create-martial-form">
          <label><span>武学名称</span><input v-model="createMartialForm.name" class="wuxia-input" placeholder="如：破云剑诀" maxlength="8" /></label>
          <label><span>武学门类</span><select v-model="createMartialForm.category" class="wuxia-select"><option v-for="[key, label] in categoryOptions" :key="key" :value="key">{{ label }}</option></select></label>
          <label v-if="createMartialForm.category === 'weapon'"><span>兵器类型</span><select v-model="createMartialForm.weaponBasic" class="wuxia-select"><option v-for="[key, label] in weaponOptions" :key="key" :value="key">{{ label }}</option></select></label>
          <label><span>根基武学</span><select v-model="createMartialForm.basicSkill" class="wuxia-select"><option v-for="[key, label] in basicSkillOptions" :key="key" :value="key">{{ label }}</option></select></label>
          <button class="btn btn-sm" :disabled="!createMartialForm.name.trim() || createMartialForm.name.trim().length > 8 || game.decisions_used >= game.max_decisions || game.sect.attributes.silver < createEstimateSilver || currentEffectiveness <= 0 || !!game.pending_event" @click="onSubmitCreateMartial">开创新学 · {{ createEstimateSilver }}两</button>
        </div>
        <div v-if="availableCreationArts.length && !game.sect.created_martial_arts?.length" class="building-prose" style="margin-top:0.5rem">另有{{ availableCreationArts.length }}部待研创武学可由掌门召众合参。</div>
      </section>
      <section v-if="privateBookHoldings.length" class="private-manual-shelf">
        <div class="management-row-title">门人私藏与遗卷</div>
        <div>
          <article v-for="holding in privateBookHoldings" :key="holding.id">
            <span><b>《{{ holding.name }}》</b><small>{{ holding.ownerAlive ? `${holding.ownerName}私藏` : `${holding.ownerName}遗卷` }}</small></span>
            <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions || !!game.pending_event" @click="command({ action: 'library_add', martial_art_id: holding.id })">献入公册</button>
          </article>
        </div>
      </section>
      <div class="manual-list">
        <article v-for="id in game.sect.public_books" :key="id" class="manual-card">
          <div><b>{{ artName(id) }}</b><span>{{ arts.find(art => art.id === id)?.type || '武学' }}</span></div>
          <small v-if="isCombatArt(id)">门派可授上限 {{ martialResearchCap(id) }} 级</small>
          <small v-else>义理根基 · 不设参研上限</small>
          <button v-if="isCombatArt(id)" class="btn btn-sm" :disabled="currentEffectiveness <= 0 || game.decisions_used >= game.max_decisions || game.sect.attributes.silver < 40 || !!game.pending_event" @click="command({ action: 'research_martial', martial_art_id: id })">合参 +{{ researchGain() }} · 40两</button>
        </article>
      </div>
    </template>

    <template v-else-if="view === 'warehouse'">
      <div class="management-row-title">合库总簿</div>
      <div class="warehouse-ledger"><span v-for="item in warehouseInventory" :key="item.name" :class="{ 'medicine-tooltip': medicineDescription(item.name) }" :data-tooltip="medicineDescription(item.name)"><b>{{ item.name }}</b><em>{{ item.count }}</em></span></div>
      <p class="building-prose">银库、药材与内外门物资统一由司库长老造册收发，丹药可从左侧弟子卷宗中赐予。</p>
    </template>

    <template v-else-if="view === 'herb_hall'">
      <section class="auto-brew-panel">
        <div class="management-row-title">常设药炉</div>
        <p v-if="currentAutoRecipe">正在依药序炼制{{ currentAutoRecipe.name }}：炉功 {{ game.sect.auto_brew_progress }} 月，按当前堂效与长老配置约需 {{ backgroundBrewMonths(currentAutoRecipe) }} 个历月；草药或库银不足时自动候料。</p>
        <p v-else>常设药序为空，后台药炉现已停火。</p>
        <div v-if="brewQueueDraft.length" class="auto-brew-queue">
          <article v-for="(recipeId, index) in brewQueueDraft" :key="recipeId">
            <b>{{ index + 1 }}. {{ medicines.find(medicine => medicine.recipeId === recipeId)?.name || recipeId }}</b>
            <span>
              <button type="button" class="btn btn-sm" :disabled="index === 0" @click="moveBrewRecipe(index, -1)">上移</button>
              <button type="button" class="btn btn-sm" :disabled="index === brewQueueDraft.length - 1" @click="moveBrewRecipe(index, 1)">下移</button>
              <button type="button" class="btn btn-sm" @click="removeBrewRecipe(index)">移出</button>
            </span>
          </article>
        </div>
        <div class="auto-brew-controls">
          <select v-model="brewQueueChoice" class="wuxia-select">
            <option value="">选择待列入药方</option>
            <option v-for="medicine in availableBrewRecipes" :key="medicine.recipeId" :value="medicine.recipeId">{{ medicine.name }} · {{ medicine.months }}炉功月</option>
          </select>
          <button type="button" class="btn btn-sm" :disabled="!brewQueueChoice" @click="addBrewRecipe">列入末位</button>
          <button type="button" class="btn btn-sm" :disabled="!brewQueueDraft.length" @click="clearBrewQueue">清空停炉</button>
          <button type="button" class="btn btn-sm" :disabled="!brewQueueChanged" @click="saveBrewQueue">誊录药序</button>
        </div>
        <small>药序会循环执行；调整与暂停不耗掌门本月定夺。</small>
      </section>
      <div class="management-row-title">丹房开炉</div>
      <section v-for="rate in recipeRates" :key="rate" class="recipe-section">
        <div class="recipe-rate"><b>{{ rate }}</b><span>{{ rate === '常速' ? '调养药物' : rate === '低速' ? '永久上限丹药' : '易筋延寿丹药' }}</span></div>
        <div class="recipe-grid">
          <button v-for="medicine in medicines.filter(item => item.rate === rate)" :key="medicine.recipeId" class="order-card medicine-tooltip" :data-tooltip="medicine.description" :aria-label="`${medicine.name}：${medicine.description}`" :disabled="currentEffectiveness <= 0 || game.decisions_used >= game.max_decisions" @click="command({ action: 'brew_pill', recipe_id: medicine.recipeId })"><b>{{ medicine.name }}</b><span>草药{{ medicine.herbCost }}份 · 库银{{ recipeSilverCost(medicine) }}两 · {{ quickBrewMonths(medicine) }}月 · 产{{ medicine.quantity }}份</span></button>
        </div>
      </section>
      <div v-if="game.sect.productions.length" class="active-orders">炼制中：<span v-for="task in game.sect.productions" :key="task.id">{{ task.name }}（尚余{{ task.remaining_months }}月）</span></div>
    </template>

    <template v-else-if="view === 'intelligence'">
      <SectViewPanel v-if="viewedSect" :sect="viewedSect" :disciples="viewedDisciples" :arts="arts" :countries="game.countries" :player-sect="game.sect" :npc-sects="game.npc_sects" @back="viewedSectId = null" />
      <template v-else>
        <div class="world-summary">天枢阁掌门派通问、请教与内门弟子江湖行走。通问往来需由内门弟子跋涉两个月完成。</div>
        <section class="alliance-ledger" aria-labelledby="alliance-ledger-title">
          <header>
            <div>
              <b id="alliance-ledger-title">盟约录</b>
              <small>双方关系均达七十，方为真实盟契</small>
            </div>
            <i aria-hidden="true">盟</i>
          </header>
          <div v-if="alliedSects.length" class="alliance-list">
            <article v-for="{ sect: ally, relation } in alliedSects" :key="ally.id">
              <i aria-hidden="true">盟契</i>
              <span>
                <b>{{ ally.name }}</b>
                <small>{{ countryName(ally.country_id) }} · 双向交情 {{ relation.mutual }}</small>
              </span>
              <em>本门→彼 {{ relation.playerToSect }} · 彼→本门 {{ relation.sectToPlayer }}</em>
            </article>
          </div>
          <div v-else class="alliance-empty">
            尚无双向盟友；须本门与对方账面关系都达到 70，盟契才会录入。
          </div>
          <p>盟友会联合巡行，并在危急时驰援；低道德盟友仍可能背盟，盟约并非一劳永逸。</p>
        <section class="alliance-actions" v-if="alliedSects.length" aria-labelledby="alliance-actions-title">
          <div class="management-row-title" id="alliance-actions-title">盟务署令</div>
          <p class="building-prose">遣内门弟子出使列盟，共襄武备、通问论道、互通有无。每次行事消耗一次掌门定夺。</p>
          <div class="alliance-action-list">
            <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions || currentEffectiveness <= 0 || game.sect.attributes.silver < 60 || !!game.pending_event" @click="command({ action: 'joint_patrol', sect_id: alliedSects[0]?.id, disciple_id: innerCandidates[0]?.id })">联合巡行 · 60两</button>
            <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions || currentEffectiveness <= 0 || game.sect.attributes.silver < 80 || !!game.pending_event" @click="command({ action: 'call_aid', sect_id: alliedSects[0]?.id })">召集援手 · 80两</button>
            <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions || currentEffectiveness <= 0 || game.sect.attributes.silver < 40 || !!game.pending_event" @click="command({ action: 'host_exchange', sect_id: alliedSects[0]?.id, disciple_id: innerCandidates[0]?.id })">武学论道 · 40两</button>
            <label class="alliance-trade-control"><span>通商易货</span>
              <select v-model="tradeItem" class="wuxia-select"><option value="草药">草药</option><option value="精铁">精铁</option><option value="粮秣">粮秣</option></select>
              <select v-model="tradeQty" class="wuxia-select"><option :value="5">5份</option><option :value="10">10份</option><option :value="20">20份</option></select>
              <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions || currentEffectiveness <= 0 || !!game.pending_event" @click="command({ action: 'trade_with_ally', sect_id: alliedSects[0]?.id, item: tradeItem, quantity: tradeQty })">发货</button>
            </label>
          </div>
        </section>
        </section>
        <section class="tournament-archive" aria-labelledby="tournament-archive-title">
          <header>
            <div>
              <b id="tournament-archive-title">历届论剑谱</b>
              <small>天枢阁藏卷</small>
            </div>
            <label v-if="tournamentRecords.length">
              <span>择年</span>
              <select v-model.number="selectedTournamentYear" class="wuxia-select">
                <option v-for="record in tournamentRecords" :key="record.year" :value="record.year">
                  第{{ record.year }}年 · 第{{ record.rank }}名
                </option>
              </select>
            </label>
          </header>
          <template v-if="selectedTournament">
            <div class="tournament-archive-summary">
              <span><small>本门名次</small><strong>第 {{ selectedTournament.rank }} 名</strong></span>
              <span><small>与会门派</small><strong>{{ selectedTournament.total_sects }} 派</strong></span>
              <span><small>本届魁首</small><strong>{{ selectedTournament.champion || '旧卷未载' }}</strong></span>
              <span><small>开赛战力</small><strong>{{ selectedTournament.power }}</strong></span>
            </div>
            <div class="tournament-lineup">
              <b>本门出阵</b>
              <ol v-if="selectedTournament.player_lineup.length">
                <li v-for="(member, index) in selectedTournament.player_lineup" :key="member.disciple_id">
                  <i>{{ index + 1 }}</i>
                  <span><strong>{{ member.name }}</strong><small>《{{ artName(member.martial_art_id) }}》 · 战力 {{ member.combat_score }}</small></span>
                </li>
              </ol>
              <small v-else>旧卷未载出阵名册。</small>
            </div>
            <details class="tournament-archive-bracket">
              <summary>展阅第{{ selectedTournament.year }}年逐场签表</summary>
              <TournamentBracket
                :tournament="selectedTournament"
                :player-sect-id="game.sect.id"
                compact
              />
            </details>
          </template>
          <div v-else class="tournament-archive-empty">尚未举办年终论剑，谱架空待来年。</div>
        </section>
        <section class="country-situation" aria-label="四国形势">
          <div class="country-situation-title"><b>四国形势</b><small>天枢月报</small></div>
          <article v-for="country in game.countries" :key="country.id" class="country-situation-card">
            <b>{{ country.name }}</b>
            <span :class="countryTone(country.prosperity)"><small>繁荣</small><strong>{{ country.prosperity }}</strong><em>{{ prosperityLabel(country.prosperity) }}</em></span>
            <span :class="countryTone(country.order)"><small>治安</small><strong>{{ country.order }}</strong><em>{{ orderLabel(country.order) }}</em></span>
            <small class="country-impact">市况 {{ marketPercent(country.prosperity) }}% · 行路 {{ safetyPercent(country.order) }}%</small>
          </article>
        </section>
        <div class="world-grid">
          <article v-for="npc in game.npc_sects" :key="npc.id" class="world-sect-card" :class="{ allied: hasAllianceWith(npc) }">
            <div class="world-sect-head">
              <b>{{ npc.name }}</b>
              <i v-if="hasAllianceWith(npc)" class="alliance-seal" aria-label="已缔结双向盟契">盟契</i>
              <span class="world-sect-country">
                {{ countryName(npc.country_id) }}
                <small v-if="countryState(npc.country_id)">繁 {{ countryState(npc.country_id)!.prosperity }} · 治 {{ countryState(npc.country_id)!.order }}</small>
              </span>
            </div>
            <div class="world-sect-relation">
              声望 {{ npc.attributes.prestige }} · 道德 {{ npc.attributes.morality }} · 双向交情 <b>{{ bilateralRelation(npc).mutual }}</b>
              <small>本门→彼 {{ bilateralRelation(npc).playerToSect }} · 彼→本门 {{ bilateralRelation(npc).sectToPlayer }}</small>
            </div>
            <small>镇派：{{ artName(npc.public_books.at(-1) || '') }}</small>
            <div class="world-sect-actions"><button class="btn btn-sm sect-view-entry" @click="viewedSectId = npc.id">查阅</button><button class="btn btn-sm" :disabled="!innerTravelers.length || game.decisions_used >= game.max_decisions" @click="openEnvoyPicker('exchange', npc)">通问</button><button class="btn btn-sm" :title="manualCandidatesFor(npc).length ? '查阅可请教的典籍与门槛' : '对方已无本门未藏之册'" :disabled="!innerTravelers.length || !manualCandidatesFor(npc).length || game.decisions_used >= game.max_decisions" @click="openEnvoyPicker('request_manual', npc)">请教</button></div>
          </article>
        </div>
      </template>
    </template>

    <template v-else-if="view === 'affairs'">
      <div class="management-row-title">门籍名额</div>
      <div class="rank-ledger"><span>杂役 <b>{{ rankCount('chore') }}</b><small>不限</small></span><span>外门 <b>{{ rankCount('outer') }}/{{ outerLimit }}</b><small>总数 × {{ Math.round(game.sect.rank_rules.outer_ratio * 100) }}%</small></span><span>内门 <b>{{ rankCount('inner') }}/{{ innerLimit }}</b><small>外门 × {{ Math.round(game.sect.rank_rules.inner_ratio * 100) }}%</small></span></div>
      <div class="management-row-title">掌门方略</div>
      <div class="moral-grid"><button v-for="[id, name, note] in moralDirections" :key="id" class="policy-card" :class="{ active: game.sect.moral_direction === id }" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'set_moral_direction', direction: id })"><b>{{ name }}</b><small>{{ note }}</small></button></div>
      <div class="management-row-title">经营侧重</div>
      <div class="policy-grid"><button v-for="[id, name, note] in policyNames" :key="id" class="policy-card" :class="{ active: game.sect.policy === id }" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'set_policy', policy: id })"><b>{{ name }}</b><small>{{ note }}</small></button></div>
      <div class="management-row-title">掌门令</div>
      <div class="order-grid"><button v-for="[id, name, desc, cost] in orders" :key="id" class="order-card" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'issue_order', order_id: id })"><b>{{ name }}</b><span>{{ desc }}</span><small>库银 {{ cost }} 两</small></button></div>
      <div v-if="game.sect.active_orders.length" class="active-orders">施行中：<span v-for="order in game.sect.active_orders" :key="order.id">{{ order.name }}（{{ order.remaining_months }}月）</span></div>
    </template>

    <template v-else>
      <div class="management-row-title">山门营造</div>
      <div class="building-grid">
        <div v-for="building in game.sect.buildings" :key="building.id" class="building-card">
          <div><b>{{ building.name }}</b><span>第{{ building.level }}重</span></div>
          <div class="condition-bar"><i :style="{ width: `${building.condition}%` }"></i></div>
          <small v-if="building.work_required">营造 {{ building.work_invested }}/{{ building.work_required }} 工作量 · 堂效 {{ buildingEffectiveness(building) }}%</small><small v-else>完好 {{ building.condition }}% · 堂效 {{ buildingEffectiveness(building) }}%</small>
          <div class="building-costs">
            <small>扩建：{{ buildingCostText(upgradeCost(building)) }}</small>
            <small v-if="building.condition < 100">修缮：{{ buildingCostText(repairCost(building)) }}</small>
          </div>
          <div class="building-actions">
            <button class="btn btn-sm" :title="buildingCostText(upgradeCost(building))" :disabled="!!building.work_required || !canAffordBuildingCost(upgradeCost(building)) || game.decisions_used >= game.max_decisions || !!game.pending_event" @click="command({ action: 'upgrade_building', building_id: building.id })">立项扩建</button>
            <button class="btn btn-sm" :title="building.condition < 100 ? buildingCostText(repairCost(building)) : '此处完好，无须修缮'" :disabled="building.condition >= 100 || !canAffordBuildingCost(repairCost(building)) || game.decisions_used >= game.max_decisions || !!game.pending_event" @click="command({ action: 'repair_building', building_id: building.id })">彻底修缮</button>
          </div>
        </div>
      </div>
      <p class="building-prose">堂效由建筑重数与完好度共同决定，一级完好为 100%，每升一重增益 20%；归零时该堂停办。当前每月月修应耗粮秣 {{ monthlyMaintenanceCost.grain }} 份、精铁 {{ monthlyMaintenanceCost.iron }} 份，欠供会令所有堂舍额外损耗。长老“巡检诸堂”可另行恢复完好度。</p>
    </template>

    <Teleport to="body">
      <Transition name="fade">
        <div v-if="envoyRequest" class="modal-overlay envoy-overlay" @click.self="envoyRequest = null">
          <section class="envoy-dialog" role="dialog" aria-modal="true" aria-labelledby="envoy-dialog-title">
            <div class="envoy-seal">行</div>
            <h2 id="envoy-dialog-title">择弟子{{ envoyRequest.action === 'exchange' ? '通问' : '请教' }}</h2>
            <p v-if="envoyRequest.action === 'exchange'">此行前往{{ envoyRequest.sectName }}通问，将耗库银 <b>{{ envoyCost }}</b> 两；承办弟子离山两个月。</p>
            <p v-else-if="selectedManual">此行前往{{ envoyRequest.sectName }}请教《{{ selectedManual.art.name }}》，将耗库银 <b>{{ envoyCost }}</b> 两；承办弟子离山两个月。</p>
            <p v-else>{{ envoyRequest.sectName }}已无本门未藏之册可供请教。</p>
            <label v-if="envoyRequest.action === 'request_manual'" class="manual-picker">请教典籍
              <select v-model="selectedManualId">
                <option v-if="!manualCandidates.length" value="">暂无可请教典籍</option>
                <option v-for="candidate in manualCandidates" :key="candidate.art.id" :value="candidate.art.id">{{ manualOptionText(candidate) }}</option>
              </select>
            </label>
            <div v-if="envoyRequest.action === 'request_manual' && selectedManual" class="manual-requirement" :class="{ unlocked: !selectedManualLockReasons.length }">
              <span>{{ selectedManual.tierLabel }}典籍 · 需交情 {{ selectedManual.requiredRelation }} · 本门声望 {{ selectedManual.requiredPrestige }} · 库银 {{ selectedManual.cost }} 两</span>
              <b>{{ selectedManualLockReasons.length ? selectedManualLockReasons.join('；') : '门槛俱足，可以遣使请教' }}</b>
            </div>
            <label>内门弟子
              <select v-model="selectedEnvoy" autofocus>
                <option value="">请选择</option>
                <option v-for="disciple in innerTravelers" :key="disciple.id" :value="disciple.id">{{ disciple.name }} · 本月未安排</option>
              </select>
            </label>
            <div class="envoy-actions"><button class="btn" @click="envoyRequest = null">暂且搁置</button><button class="btn btn-primary" :disabled="!selectedEnvoy || game.sect.attributes.silver < envoyCost || manualBlocked" @click="confirmEnvoy">遣其下山</button></div>
          </section>
        </div>
      </Transition>
    </Teleport>
  </section>
</template>

<style scoped>
.alliance-ledger {
  margin: .4rem 0 .55rem;
  padding: .42rem .52rem;
  border: 1px solid #c43a316b;
  background: linear-gradient(120deg, #c43a310d, #fff8e780 42%, #faf3e4b8);
}

.alliance-ledger > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: .28rem;
  border-bottom: 1px dashed var(--color-border);
}

.alliance-ledger > header b,
.alliance-ledger > header small {
  display: block;
}

.alliance-ledger > header b {
  color: var(--color-cinnabar);
  font: .8rem var(--font-title);
  letter-spacing: .14em;
}

.alliance-ledger > header small {
  color: var(--color-ink-fade);
  font-size: .52rem;
}

.alliance-ledger > header > i {
  display: grid;
  place-items: center;
  width: 1.4rem;
  height: 1.4rem;
  border: 2px solid var(--color-cinnabar);
  color: var(--color-cinnabar);
  font: .65rem var(--font-title);
  font-style: normal;
  transform: rotate(-5deg);
}

.alliance-list {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: .25rem;
  margin-top: .35rem;
}

.alliance-list article {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  min-width: 0;
  padding: .28rem .35rem;
  border: 1px solid #b8860b66;
  background: #fffaf099;
}

.alliance-list article > i {
  grid-row: 1 / span 2;
  margin-right: .32rem;
  padding: .1rem .16rem;
  border: 1px solid var(--color-cinnabar);
  color: var(--color-cinnabar);
  font: .5rem var(--font-title);
  font-style: normal;
  writing-mode: vertical-rl;
}

.alliance-list span,
.alliance-list b,
.alliance-list small,
.alliance-list em {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.alliance-list b {
  color: var(--color-ink);
  font-size: .64rem;
}

.alliance-list small,
.alliance-list em {
  color: var(--color-ink-fade);
  font-size: .52rem;
  font-style: normal;
}

.alliance-list em {
  grid-column: 2;
}

.alliance-empty {
  margin-top: .35rem;
  padding: .35rem;
  border: 1px dashed var(--color-border);
  color: var(--color-ink-fade);
  font-size: .56rem;
  text-align: center;
}

.alliance-ledger > p {
  margin: .3rem 0 0;
  color: var(--color-ink-light);
  font-size: .52rem;
  line-height: 1.5;
  text-align: center;
}

.world-sect-card.allied {
  border-color: #b8860b99;
  box-shadow: inset 3px 0 #c43a3199;
}

.world-sect-head .alliance-seal {
  flex: 0 0 auto;
  padding: .08rem .16rem;
  border: 1px solid var(--color-cinnabar);
  color: var(--color-cinnabar);
  font: .5rem var(--font-title);
  font-style: normal;
  transform: rotate(-3deg);
}

.world-sect-country {
  margin-left: auto;
}

.world-sect-relation b {
  color: var(--color-cinnabar);
}

.world-sect-relation small {
  display: block;
  color: var(--color-ink-fade);
  font-size: .52rem;
}

.tournament-archive {
  margin: .4rem 0 .55rem;
  padding: .45rem .55rem;
  border: 1px solid #b8860b80;
  background: linear-gradient(135deg, #fff8e780, #faf3e4b8);
}

.tournament-archive > header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: .5rem;
  padding-bottom: .35rem;
  border-bottom: 1px dashed var(--color-border);
}

.tournament-archive > header > div b,
.tournament-archive > header > div small {
  display: block;
}

.tournament-archive > header > div b {
  color: var(--color-cinnabar);
  font: .82rem var(--font-title);
  letter-spacing: .12em;
}

.tournament-archive > header > div small,
.tournament-archive > header label span {
  color: var(--color-ink-fade);
  font-size: .52rem;
}

.tournament-archive > header label {
  display: flex;
  align-items: center;
  gap: .3rem;
}

.tournament-archive > header select {
  min-width: 8.5rem;
  font-size: .6rem;
}

.tournament-archive-summary {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: .25rem;
  margin: .4rem 0;
}

.tournament-archive-summary > span {
  min-width: 0;
  padding: .25rem .3rem;
  border: 1px solid var(--color-border);
  background: #fffaf080;
  text-align: center;
}

.tournament-archive-summary small,
.tournament-archive-summary strong {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tournament-archive-summary small {
  color: var(--color-ink-fade);
  font-size: .5rem;
}

.tournament-archive-summary strong {
  color: var(--color-ink);
  font-size: .66rem;
}

.tournament-lineup {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  gap: .35rem;
  margin-bottom: .4rem;
}

.tournament-lineup > b {
  color: var(--color-cinnabar);
  font: .62rem var(--font-title);
}

.tournament-lineup > ol {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: .2rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.tournament-lineup li {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  min-width: 0;
  padding: .2rem .25rem;
  background: #e8d5a84d;
}

.tournament-lineup li i {
  display: grid;
  place-items: center;
  width: 1rem;
  height: 1rem;
  margin-right: .2rem;
  border: 1px solid var(--color-cinnabar);
  color: var(--color-cinnabar);
  font-size: .52rem;
  font-style: normal;
}

.tournament-lineup li span,
.tournament-lineup li strong,
.tournament-lineup li small {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tournament-lineup li strong {
  font-size: .58rem;
}

.tournament-lineup li small,
.tournament-lineup > small {
  color: var(--color-ink-fade);
  font-size: .48rem;
}

.tournament-archive-bracket > summary {
  width: max-content;
  margin: .25rem auto;
  color: var(--color-cinnabar);
  cursor: pointer;
  font: .65rem var(--font-title);
}

.tournament-archive-bracket > summary:focus-visible {
  outline: 2px solid var(--color-cinnabar);
  outline-offset: 2px;
}

.tournament-archive-empty {
  padding: .55rem;
  color: var(--color-ink-fade);
  font-size: .62rem;
  text-align: center;
}
</style>
