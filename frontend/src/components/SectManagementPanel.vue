<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import type { ActionKind, Building, BuildingKind, Decision, GameState, ManagementRequest, MartialArt, MoralDirection, SectPolicy, SectState } from '../types'
import DecisionGrid from './DecisionGrid.vue'
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
  herb_hall: [['treat', '诊治掌门', '掌门伤势降低 8 点'], ['brew', '开炉炼丹', '选择一种丹药额外炼制']],
  intelligence: [['correspond', '修书诸派', '与所有门派的交情各提升 2 点'], ['scout', '查探江湖', '本门声望提升 3 点']],
  affairs: [['recruit', '代访新人', '耗费 25 两，为门中访得一名新人'], ['arbitrate', '处置事务', '依门风提升道德、志气或库银']],
  logistics: [['maintain', '巡检诸堂', '耗库银 8 两、精铁 2 份恢复 4 点；不足时仅恢复 2 点'], ['supervise', '亲临督造', '所有在建工程各增加 6 点工作量'], ['expand', '督造扩建', '选取一栋建筑立项扩建，低级建筑优先']],
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
  practice: ['teach'], scripture: ['train', 'study', 'research'], warehouse: [], herb_hall: ['rest'],
  intelligence: [], affairs: ['recruit', 'mission'], logistics: [],
}
const buildingDecisions = computed(() => props.decisions.filter(decision => decisionIds[props.view].includes(decision.id)))
const currentBuilding = computed(() => props.game.sect.buildings.find(building => building.kind === props.view))
const upgradeableBuildings = computed(() => props.game.sect.buildings.filter(building => building.work_required === 0))
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
const recipeSilverCost = (medicine: typeof medicines[number]) => medicine.months * (medicine.rate === '常速' ? 2 : medicine.rate === '低速' ? 3 : 4)
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
        <small>{{ currentBuilding.elder_action_used ? '本月堂务已办，过月仍照此办理' : '事务已择定，过月自动办理' }}</small>
        <button v-for="[id, label, description] in elderDuties[view]" :key="id" type="button" class="btn btn-sm elder-duty-option" :class="{ active: selectedDuty(currentBuilding) === id }" :data-tooltip="description" :aria-label="`${label}：${description}`" :disabled="!currentBuilding.elder_id || !!game.pending_event" @click="selectElderDuty(currentBuilding, id, id === 'expand' ? expansionTarget(currentBuilding) : id === 'brew' ? brewTarget(currentBuilding) : undefined)">{{ label }}</button>
        <label v-if="selectedDuty(currentBuilding) === 'expand'" class="elder-duty-target"><span>扩建目标</span>
          <select class="wuxia-select" :value="expansionTarget(currentBuilding) || ''" :disabled="!currentBuilding.elder_id || !!game.pending_event || !upgradeableBuildings.length" @change="selectExpansionTarget(currentBuilding, $event)">
            <option v-if="!upgradeableBuildings.length" value="">暂无可扩建建筑</option>
            <option v-for="building in upgradeableBuildings" :key="building.id" :value="building.id">{{ building.name }} · 第{{ building.level }}重</option>
          </select>
        </label>
        <label v-if="selectedDuty(currentBuilding) === 'brew'" class="elder-duty-target"><span>丹药</span>
          <select class="wuxia-select" :value="brewTarget(currentBuilding) || ''" :disabled="!currentBuilding.elder_id || !!game.pending_event || !medicines.length" @change="selectBrewTarget(currentBuilding, $event)">
            <option v-for="medicine in medicines" :key="medicine.recipeId" :value="medicine.recipeId">{{ medicine.name }} · 草药{{ medicine.herbCost }}份 · 产{{ medicine.quantity }}份</option>
          </select>
        </label>
      </div>
    </header>

    <DecisionGrid v-if="buildingDecisions.length" :decisions="buildingDecisions" :arts="arts" :game="game" :used="used" :title="`${currentBuilding?.name || ''}本月议事`" @decide="$emit('decide', $event)" />

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
        <button class="btn btn-sm" :disabled="!dispatchDisciple[view] || game.decisions_used >= game.max_decisions || !!game.pending_event" @click="dispatchTask">派发</button>
      </div>
    </section>

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
    </template>

    <template v-else-if="view === 'warehouse'">
      <div class="management-row-title">合库总簿</div>
      <div class="warehouse-ledger"><span v-for="item in warehouseInventory" :key="item.name" :class="{ 'medicine-tooltip': medicineDescription(item.name) }" :data-tooltip="medicineDescription(item.name)"><b>{{ item.name }}</b><em>{{ item.count }}</em></span></div>
      <p class="building-prose">银库、药材与内外门物资统一由司库长老造册收发，丹药可从左侧弟子卷宗中赐予。</p>
    </template>

    <template v-else-if="view === 'herb_hall'">
      <div class="management-row-title">丹房开炉</div>
      <section v-for="rate in recipeRates" :key="rate" class="recipe-section">
        <div class="recipe-rate"><b>{{ rate }}</b><span>{{ rate === '常速' ? '调养药物' : rate === '低速' ? '永久上限丹药' : '易筋延寿丹药' }}</span></div>
        <div class="recipe-grid">
          <button v-for="medicine in medicines.filter(item => item.rate === rate)" :key="medicine.recipeId" class="order-card medicine-tooltip" :data-tooltip="medicine.description" :aria-label="`${medicine.name}：${medicine.description}`" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'brew_pill', recipe_id: medicine.recipeId })"><b>{{ medicine.name }}</b><span>草药{{ medicine.herbCost }}份 · 库银{{ recipeSilverCost(medicine) }}两 · {{ medicine.months }}月 · 产{{ medicine.quantity }}份</span></button>
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
      <p class="building-prose">建筑每月自然损耗 1–3 点，低于 30% 后会加速损坏。长老“巡检诸堂”需库银 8 两与精铁 2 份，可恢复 4 点；资源不足时仅作最低维护，恢复 2 点。</p>
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
