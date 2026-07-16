<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import type { Building, BuildingKind, Decision, GameState, ManagementRequest, MartialArt, MoralDirection, SectPolicy, SectState } from '../types'
import DecisionGrid from './DecisionGrid.vue'
import MartialArtsPanel from './MartialArtsPanel.vue'
import SectViewPanel from './SectViewPanel.vue'
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
  ['righteous', '行侠仗义', '扶危济困，道德渐长'], ['neutral', '独善其身', '不涉恩怨，安定门心'], ['villainous', '为非作歹', '劫掠牟利，道德日损'],
]
const elderDuties: Record<BuildingKind, Array<[string, string, string]>> = {
  practice: [['instruct', '整饬教习', '本门志气提升 3 点'], ['drill', '主持月考', '所有在门弟子各添 2 点功绩']],
  scripture: [['curate', '校勘群籍', '每部公册的门派参研提升 4 点'], ['comprehend', '邀众合参', '集中参悟首部公册，门派参研提升 18 点']],
  warehouse: [['audit', '清点旧账', '追回 12 至 24 两库银'], ['purchase', '下山采买', '耗费 15 两，购入 5 份草药']],
  herb_hall: [['treat', '诊治掌门', '掌门伤势降低 8 点'], ['brew', '试炼小炉', '耗费 2 份草药，炼得 1 份金疮药']],
  intelligence: [['correspond', '修书诸派', '与所有门派的交情各提升 2 点'], ['scout', '查探江湖', '本门声望提升 3 点']],
  affairs: [['recruit', '代访新人', '耗费 25 两，为门中访得一名新人'], ['arbitrate', '处置事务', '依门风提升道德、志气或库银']],
  logistics: [['maintain', '巡检诸堂', '所有建筑完好度恢复 4 点'], ['supervise', '亲临督造', '所有在建工程各增加 6 点工作量']],
}
const orders = [
  ['diligent', '勤修令', '三月内门人更喜练武', 60], ['righteous', '尚义令', '四月内涵养门风', 80],
  ['frugal', '节用令', '四月内节用裕库', 50], ['rest', '调息令', '两月内静养更佳', 45],
]
const recipeRates = ['常速', '低速', '极低速'] as const
const decisionIds: Record<BuildingKind, string[]> = {
  practice: ['teach'], scripture: ['train', 'study'], warehouse: [], herb_hall: ['rest'],
  intelligence: [], affairs: ['recruit', 'mission'], logistics: [],
}
const buildingDecisions = computed(() => props.decisions.filter(decision => decisionIds[props.view].includes(decision.id)))
const currentBuilding = computed(() => props.game.sect.buildings.find(building => building.kind === props.view))
const assignedIds = computed(() => new Set(props.game.sect.buildings.map(building => building.elder_id).filter(Boolean)))
const elderCandidates = computed(() => props.game.disciples.filter(disciple =>
  disciple.alive && disciple.rank === 'inner' && (!assignedIds.value.has(disciple.id) || disciple.id === currentBuilding.value?.elder_id)))
const elderName = computed(() => props.game.disciples.find(disciple => disciple.id === currentBuilding.value?.elder_id)?.name || '暂缺')
const alive = computed(() => props.game.disciples.filter(disciple => disciple.alive))
const rankCount = (rank: 'chore' | 'outer' | 'inner') => alive.value.filter(disciple => disciple.rank === rank).length
const outerLimit = computed(() => Math.floor(alive.value.length * props.game.sect.rank_rules.outer_ratio))
const innerLimit = computed(() => Math.floor(rankCount('outer') * props.game.sect.rank_rules.inner_ratio))
const artName = (id: string) => props.arts.find(art => art.id === id)?.name || id
const countryName = (id: string) => props.game.countries.find(country => country.id === id)?.name || id
const foundation = (sectId: string) => `${sectId}_foundation`
const command = (payload: ManagementRequest) => emit('manage', payload)
const appointElder = (event: Event) => command({ action: 'assign_elder', building_id: currentBuilding.value?.id, disciple_id: (event.target as HTMLSelectElement).value || null })
const viewedSectId = ref<string | null>(null)
type EnvoyAction = 'exchange' | 'request_manual'
interface EnvoyRequest { action: EnvoyAction; sectId: string; sectName: string; martialArtId?: string }
const envoyRequest = ref<EnvoyRequest | null>(null)
const selectedEnvoy = ref('')
const innerTravelers = computed(() => props.game.disciples.filter(disciple => disciple.alive && disciple.rank === 'inner' && !disciple.away_months && disciple.condition === 'healthy'))
const viewedSect = computed(() => props.game.npc_sects.find(sect => sect.id === viewedSectId.value) || null)
const viewedDisciples = computed(() => props.game.npc_disciples.filter(disciple => disciple.sect_id === viewedSectId.value && disciple.alive))
const closeNpcSect = () => { viewedSectId.value = null; envoyRequest.value = null; selectedEnvoy.value = '' }
defineExpose({ closeNpcSect })

const manualCost = (martialArtId?: string) => 60 + (props.arts.find(art => art.id === martialArtId)?.difficulty || 0) * 5
const envoyCost = computed(() => envoyRequest.value?.action === 'exchange' ? 45 : manualCost(envoyRequest.value?.martialArtId))
const openEnvoyPicker = (action: EnvoyAction, sect: SectState) => {
  selectedEnvoy.value = ''
  envoyRequest.value = { action, sectId: sect.id, sectName: sect.name, martialArtId: action === 'request_manual' ? foundation(sect.id) : undefined }
}
const confirmEnvoy = () => {
  if (!envoyRequest.value || !selectedEnvoy.value) return
  const request = envoyRequest.value
  command(request.action === 'exchange'
    ? { action: 'exchange', sect_id: request.sectId, disciple_id: selectedEnvoy.value }
    : { action: 'request_manual', sect_id: request.sectId, martial_art_id: request.martialArtId, disciple_id: selectedEnvoy.value })
  envoyRequest.value = null
  selectedEnvoy.value = ''
}

const selectedElderDuties = reactive<Record<string, string>>({})
const recommendedElderDuty = (building: Building) => {
  switch (building.kind) {
    case 'practice': return props.game.sect.attributes.morale < 65 ? 'instruct' : 'drill'
    case 'scripture': return props.game.sect.public_books.length >= 5 ? 'curate' : 'comprehend'
    case 'warehouse': return props.game.sect.attributes.silver < 120 || (props.game.sect.inventory['草药'] || 0) >= 8 ? 'audit' : 'purchase'
    case 'herb_hall': return props.game.injury > 0 || (props.game.sect.inventory['草药'] || 0) < 2 ? 'treat' : 'brew'
    case 'intelligence': return Object.values(props.game.sect.relations).some(value => value < 10) ? 'correspond' : 'scout'
    case 'affairs': return props.game.sect.attributes.silver >= 25 ? 'recruit' : 'arbitrate'
    case 'logistics': return props.game.sect.buildings.some(item => item.work_required > item.work_invested) ? 'supervise' : 'maintain'
  }
}
const selectedDuty = (building: Building) => selectedElderDuties[building.id] || recommendedElderDuty(building)
const runSelectedDuty = (building: Building) => command({ action: 'run_elder_duty', building_id: building.id, duty_id: selectedDuty(building) })
watch(() => `${props.game.year}-${props.game.month}`, () => {
  for (const key of Object.keys(selectedElderDuties)) delete selectedElderDuties[key]
  for (const building of props.game.sect.buildings) selectedElderDuties[building.id] = recommendedElderDuty(building)
}, { immediate: true })
</script>

<template>
  <section class="management-sheet">
    <header v-if="currentBuilding" class="building-header">
      <div><b>{{ currentBuilding.name }}</b><small>第{{ currentBuilding.level }}重 · 完好 {{ currentBuilding.condition }}%</small></div>
      <label><span>{{ currentBuilding.elder_title }}</span>
        <select class="wuxia-select elder-select" :value="currentBuilding.elder_id || ''" :disabled="game.decisions_used >= game.max_decisions" @change="appointElder">
          <option value="">暂缺（掌门兼领）</option>
          <option v-for="disciple in elderCandidates" :key="disciple.id" :value="disciple.id">{{ disciple.name }}</option>
        </select>
      </label>
      <i>{{ elderName }}</i>
      <div class="elder-duty-actions">
        <small>{{ currentBuilding.elder_action_used ? '本月堂务已办' : '月首已由长老择定，可改选' }}</small>
        <button v-for="[id, label, description] in elderDuties[view]" :key="id" type="button" class="btn btn-sm elder-duty-option" :class="{ active: selectedDuty(currentBuilding) === id }" :data-tooltip="description" :aria-label="`${label}：${description}`" :disabled="!currentBuilding.elder_id || currentBuilding.elder_action_used || !!game.pending_event" @click="selectedElderDuties[currentBuilding.id] = id">{{ label }}</button>
        <button class="btn btn-sm btn-jade elder-duty-submit" :disabled="!currentBuilding.elder_id || currentBuilding.elder_action_used || !!game.pending_event" @click="runSelectedDuty(currentBuilding)">依此办理</button>
      </div>
    </header>

    <DecisionGrid v-if="buildingDecisions.length" :decisions="buildingDecisions" :arts="arts" :game="game" :used="used" :title="`${currentBuilding?.name || ''}本月议事`" @decide="$emit('decide', $event)" />

    <template v-if="view === 'practice'">
      <div class="management-row-title">传武授艺</div>
      <p class="building-prose">由传武长老总领门中教习、外门习武与同门陪练。弟子的个人行止可在左侧谱牒中安排。</p>
    </template>

    <template v-else-if="view === 'scripture'">
      <div class="library-summary">藏经阁共收公册 {{ game.sect.public_books.length }} 部，可闭关练功、研习武功或合参典籍。</div>
      <div class="manual-list">
        <article v-for="id in game.sect.public_books" :key="id" class="manual-card">
          <div><b>{{ artName(id) }}</b><span>{{ arts.find(art => art.id === id)?.type || '武学' }}</span></div>
          <small>门派参研 {{ game.sect.martial_research[id] || 0 }}</small>
          <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'research_martial', martial_art_id: id })">合参 · 40两</button>
        </article>
      </div>
      <button class="btn btn-primary research-new" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'research_new_martial' })">集众研创新武学 · 库银120两</button>
      <MartialArtsPanel :arts="arts.filter(art => art.sect_id === 'player')" :learned="game.martial_arts_learned" />
    </template>

    <template v-else-if="view === 'warehouse'">
      <div class="management-row-title">合库总簿</div>
      <div class="warehouse-ledger"><span v-for="(count, name) in game.sect.inventory" :key="name" :class="{ 'medicine-tooltip': medicineDescription(String(name)) }" :data-tooltip="medicineDescription(String(name))"><b>{{ name }}</b><em>{{ count }}</em></span></div>
      <p class="building-prose">银库、药材与内外门物资统一由司库长老造册收发，丹药可从左侧弟子卷宗中赐予。</p>
    </template>

    <template v-else-if="view === 'herb_hall'">
      <div class="management-row-title">丹房开炉</div>
      <section v-for="rate in recipeRates" :key="rate" class="recipe-section">
        <div class="recipe-rate"><b>{{ rate }}</b><span>{{ rate === '常速' ? '调养药物' : rate === '低速' ? '永久上限丹药' : '易筋延寿丹药' }}</span></div>
        <div class="recipe-grid">
          <button v-for="medicine in medicines.filter(item => item.rate === rate)" :key="medicine.recipeId" class="order-card medicine-tooltip" :data-tooltip="medicine.description" :aria-label="`${medicine.name}：${medicine.description}`" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'brew_pill', recipe_id: medicine.recipeId })"><b>{{ medicine.name }}</b><span>草药{{ medicine.herbCost }}份 · {{ medicine.months }}月 · 产{{ medicine.quantity }}份</span></button>
        </div>
      </section>
      <div v-if="game.sect.productions.length" class="active-orders">炼制中：<span v-for="task in game.sect.productions" :key="task.id">{{ task.name }}（尚余{{ task.remaining_months }}月）</span></div>
    </template>

    <template v-else-if="view === 'intelligence'">
      <SectViewPanel v-if="viewedSect" :sect="viewedSect" :disciples="viewedDisciples" :arts="arts" :countries="game.countries" :player-sect="game.sect" :npc-sects="game.npc_sects" @back="viewedSectId = null" />
      <template v-else>
        <div class="world-summary">天枢阁掌门派通问、请教与内门弟子江湖行走。通问往来需由内门弟子跋涉两个月完成。</div>
        <div class="world-grid">
          <article v-for="npc in game.npc_sects" :key="npc.id" class="world-sect-card">
            <div class="world-sect-head"><b>{{ npc.name }}</b><span>{{ countryName(npc.country_id) }}</span></div>
            <div>声望 {{ npc.attributes.prestige }} · 道德 {{ npc.attributes.morality }} · 交情 {{ game.sect.relations[npc.id] || 0 }}</div>
            <small>镇派：{{ artName(npc.public_books.at(-1) || '') }}</small>
            <div class="world-sect-actions"><button class="btn btn-sm sect-view-entry" @click="viewedSectId = npc.id">查阅</button><button class="btn btn-sm" :disabled="!innerTravelers.length || game.decisions_used >= game.max_decisions" @click="openEnvoyPicker('exchange', npc)">通问</button><button class="btn btn-sm" :disabled="!innerTravelers.length || (game.sect.relations[npc.id] || 0) < 10 || game.decisions_used >= game.max_decisions" @click="openEnvoyPicker('request_manual', npc)">请教</button></div>
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
          <small v-if="building.work_required">营造 {{ building.work_invested }}/{{ building.work_required }} 工作量</small><small v-else>完好 {{ building.condition }}%</small>
          <div class="building-actions"><button class="btn btn-sm" :disabled="!!building.work_required || game.decisions_used >= game.max_decisions" @click="command({ action: 'upgrade_building', building_id: building.id })">立项扩建</button></div>
        </div>
      </div>
      <p class="building-prose">扩建立项后须在左侧安排杂役弟子“建造升级”；日常损耗则由“建筑维护”恢复。原“修缮山门”议事已撤除。</p>
    </template>

    <Teleport to="body">
      <Transition name="fade">
        <div v-if="envoyRequest" class="modal-overlay envoy-overlay" @click.self="envoyRequest = null">
          <section class="envoy-dialog" role="dialog" aria-modal="true" aria-labelledby="envoy-dialog-title">
            <div class="envoy-seal">行</div>
            <h2 id="envoy-dialog-title">择弟子{{ envoyRequest.action === 'exchange' ? '通问' : '请教' }}</h2>
            <p>此行前往{{ envoyRequest.sectName }}，将耗库银 <b>{{ envoyCost }}</b> 两；承办弟子离山两个月。</p>
            <label>内门弟子
              <select v-model="selectedEnvoy" autofocus>
                <option value="">请选择</option>
                <option v-for="disciple in innerTravelers" :key="disciple.id" :value="disciple.id">{{ disciple.name }} · 本月{{ disciple.action ? '已有安排' : '未安排' }}</option>
              </select>
            </label>
            <div class="envoy-actions"><button class="btn" @click="envoyRequest = null">暂且搁置</button><button class="btn btn-primary" :disabled="!selectedEnvoy || game.sect.attributes.silver < envoyCost" @click="confirmEnvoy">遣其下山</button></div>
          </section>
        </div>
      </Transition>
    </Teleport>
  </section>
</template>
