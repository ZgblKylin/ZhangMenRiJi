<script setup lang="ts">
import { computed } from 'vue'
import type {
  ActionKind,
  Building,
  Country,
  Department,
  Disciple,
  MartialArt,
  SectState,
} from '../types'
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
const rankNames = { chore: '杂役', outer: '外门', inner: '内门' }
const legacyLeaderId = computed(() => `npc_${props.sect.id}_1`)
const positionedLeader = computed(() =>
  props.disciples.find(disciple => disciple.alive && disciple.npc_position === '掌门')
  || props.disciples.find(disciple => disciple.npc_position === '掌门'))
const leader = computed(() =>
  positionedLeader.value
  || props.disciples.find(disciple => disciple.id === legacyLeaderId.value))
const leaderId = computed(() => leader.value?.id || legacyLeaderId.value)
const isLeader = (disciple: Disciple) => disciple.id === leaderId.value
const leaderName = computed(() => leader.value?.name || '暂缺')
const country = computed(() => props.countries.find(item => item.id === props.sect.country_id))
const countryName = computed(() => country.value?.name || props.sect.country_id)
const playerToSectRelation = computed(() => props.playerSect.relations?.[props.sect.id] ?? 0)
const sectToPlayerRelation = computed(() => props.sect.relations?.[props.playerSect.id] ?? 0)
const mutualPlayerRelation = computed(() =>
  Math.min(playerToSectRelation.value, sectToPlayerRelation.value))
const allianceEligible = computed(() => !['court', 'song_court'].includes(props.sect.id))
const hasPlayerAlliance = computed(() =>
  allianceEligible.value && mutualPlayerRelation.value >= 70)
const prosperityLabel = (value: number) => value >= 80 ? '鼎盛' : value >= 60 ? '丰足' : value >= 40 ? '疲敝' : '凋敝'
const orderLabel = (value: number) => value >= 80 ? '清平' : value >= 60 ? '尚安' : value >= 40 ? '多事' : '动荡'
const boundedCountryValue = (value: number) => Math.max(0, Math.min(100, value))
const marketPercent = (value: number) => Math.max(85, Math.min(115, 100 + Math.trunc((boundedCountryValue(value) - 70) / 2)))
const safetyPercent = (value: number) => Math.max(85, Math.min(115, 100 + Math.trunc((boundedCountryValue(value) - 65) / 2)))
const artName = (id: string) => displayArtName(props.arts, id)
const categorySkills = (disciple: Disciple, category: typeof skillCategories[number]['id']) =>
  skillsInCategory(disciple.skills, props.arts, category)
const discipleElderTitles = (disciple: Disciple) =>
  [...new Set(props.sect.buildings
    .filter(building => building.elder_id === disciple.id)
    .map(building => building.elder_title)
    .filter(Boolean))]
const discipleRole = (disciple: Disciple) => {
  const elderTitles = discipleElderTitles(disciple)
  if (isLeader(disciple)) {
    return elderTitles.length ? `掌门兼${elderTitles.join('、')}` : '掌门'
  }
  const position = disciple.npc_position?.trim()
  const base = position && position !== '掌门' && position !== '长老'
    ? position
    : rankNames[disciple.rank]
  return elderTitles.length ? `${base} · ${elderTitles.join('、')}` : base
}
const buildingOfficer = (building: Building) => {
  const holder = props.disciples.find(disciple => disciple.id === building.elder_id)
  if (!holder) return `${building.elder_title}：暂缺`
  const title = isLeader(holder) ? `掌门兼${building.elder_title}` : building.elder_title
  return `${title}：${holder.name}`
}
const departments: Record<Department, string> = {
  transmission: '传功',
  library: '藏经',
  apothecary: '药务',
  treasury: '司库',
  stewardship: '庶务',
  external_affairs: '外务',
}
const departmentName = (department?: Department | null) =>
  department ? departments[department] : '未入六部'
const masterName = (disciple: Disciple) =>
  disciple.master_id
    ? props.disciples.find(candidate => candidate.id === disciple.master_id)?.name || disciple.master_id
    : '未定'
const conditionNames: Record<Disciple['condition'], string> = {
  healthy: '安好',
  exhausted: '力竭',
  unconscious: '昏迷',
  seriously_injured: '重伤',
  dead: '亡故',
}
const actionNames: Record<ActionKind, string> = {
  read: '研读典籍',
  practice: '练习武功',
  teach: '传武授艺',
  spar: '切磋武艺',
  temper_body: '打熬气血',
  cultivate_neili: '修炼内力',
  meditate: '冥想养神',
  sect_mission: '外派办事',
  wander: '江湖历练',
  recover: '静养调息',
  maintain: '维护建筑',
  construct: '建造升级',
  produce: '门中生产',
  business: '世俗经营',
  gather: '入山采集',
}
const targetName = (id?: string | null) =>
  props.disciples.find(disciple => disciple.id === id)?.name
  || props.sect.buildings.find(building => building.id === id)?.name
  || id
  || ''
const actionSummary = (disciple: Disciple) => {
  if (!disciple.action) return disciple.away_months ? `在外，尚余${disciple.away_months}月` : '本月未有定令'
  const details = [actionNames[disciple.action.kind]]
  if (disciple.action.martial_art_id) details.push(`《${artName(disciple.action.martial_art_id)}》`)
  if (disciple.action.target_id) details.push(targetName(disciple.action.target_id))
  if (disciple.action.remaining_months > 0) details.push(`尚余${disciple.action.remaining_months}月`)
  return details.join(' · ')
}
const journeyTemplateNames: Record<string, string> = {
  escort_supplies: '护送粮饷',
  seek_physician: '寻访名医',
  mediate_dispute: '调停地界',
  clear_bandits: '清剿路匪',
  free_wander: '江湖游历',
}
const journeyDestinationNames: Record<string, string> = {
  xiangyang: '襄阳',
  linan: '临安',
  luoyang: '洛阳',
  dali: '大理',
  liangzhou: '凉州',
  taihu: '太湖',
}
const journeyTemplateName = (id?: string) => journeyTemplateNames[id || ''] || '江湖游历'
const journeyDestinationName = (id?: string) => journeyDestinationNames[id || ''] || id || '江湖'
const journeyOutcomeNames = { success: '功成', partial: '勉成', failed: '失利' }
const journeyOutcomeName = (outcome?: keyof typeof journeyOutcomeNames | null) =>
  outcome ? journeyOutcomeNames[outcome] : '未结'
const skillLevel = (disciple: Disciple, id: string) =>
  disciple.skills.find(skill => skill.martial_art_id === id)?.level || 0
const highestKnowledgeLevel = (disciple: Disciple) =>
  categorySkills(disciple, 'knowledge')[0]?.level || 0
type TrainedAptitude = 'strength' | 'intelligence' | 'constitution' | 'agility'
const aptitudeBonus = (disciple: Disciple, aptitude: TrainedAptitude) => {
  const sources: Record<TrainedAptitude, number> = {
    strength: skillLevel(disciple, 'basic_unarmed'),
    intelligence: highestKnowledgeLevel(disciple),
    constitution: skillLevel(disciple, 'basic_force'),
    agility: skillLevel(disciple, 'basic_dodge'),
  }
  return Math.floor(sources[aptitude] / 10)
}
const effectiveAptitude = (disciple: Disciple, aptitude: TrainedAptitude) =>
  disciple.aptitudes[aptitude] + aptitudeBonus(disciple, aptitude)
const preparedEntries = (disciple: Disciple) =>
  Object.entries(disciple.prepared_skills || {}).map(([slot, artId]) => ({
    slot: slot === 'knowledge' ? '知识' : artName(slot),
    art: artName(artId),
  }))
const isPrepared = (disciple: Disciple, artId: string) =>
  Object.values(disciple.prepared_skills || {}).includes(artId)
const relationName = (id: string) => {
  if (id === props.playerSect.id) return props.playerSect.name
  return props.npcSects.find(sect => sect.id === id)?.name || id
}
const relations = computed(() => {
  const values = { ...props.sect.relations }
  values[props.playerSect.id] ??= props.playerSect.relations[props.sect.id] || 0
  return Object.entries(values)
    .filter(([id]) => id !== props.playerSect.id)
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
        <small v-if="country" class="sect-country-status">{{ countryName }} · 繁荣 {{ country.prosperity }}（{{ prosperityLabel(country.prosperity) }}） · 治安 {{ country.order }}（{{ orderLabel(country.order) }}） · 市况 {{ marketPercent(country.prosperity) }}% · 行路 {{ safetyPercent(country.order) }}%</small>
        <small v-else class="sect-country-status">{{ countryName }}</small>
        <small>{{ policyNames[sect.policy] }} · {{ sect.landmark }}</small>
        <small v-if="sect.description">{{ sect.description }}</small>
      </div>
      <div class="sect-view-header-stats">
        <span><small>声望</small><b>{{ sect.attributes.prestige }}</b></span>
        <span><small>库银</small><b>{{ sect.attributes.silver }}<em>两</em></b></span>
        <span><small>道德</small><b>{{ sect.attributes.morality }}</b></span>
        <span><small>志气</small><b>{{ sect.attributes.morale }}</b></span>
      </div>
      <i :class="{ allied: hasPlayerAlliance }">{{ hasPlayerAlliance ? '盟' : '阅' }}</i>
    </header>

    <section class="bilateral-alliance-status" :class="{ allied: hasPlayerAlliance }" aria-label="与本门的双向关系及盟约状态">
      <i aria-hidden="true">{{ !allianceEligible ? '不盟' : hasPlayerAlliance ? '盟契' : '未盟' }}</i>
      <span>
        <b>{{ !allianceEligible
          ? `${sect.name}不入江湖盟契`
          : hasPlayerAlliance
            ? `与${playerSect.name}盟契在册`
            : `与${playerSect.name}尚未成盟` }}</b>
        <small>
          双向交情 {{ mutualPlayerRelation }} ·
          本门→{{ sect.name }} {{ playerToSectRelation }} ·
          {{ sect.name }}→本门 {{ sectToPlayerRelation }}
        </small>
      </span>
      <em>{{ hasPlayerAlliance
        ? '双方关系均已达到 70；盟友会联合巡行、危急驰援，低道德盟友仍可能背盟。'
        : !allianceEligible
          ? '朝廷势力不参与江湖门派盟契，交情再高也不会进入盟友互动池。'
          : '双方账面关系都达到 70 方能缔结盟契；旧卷缺失的关系按 0 计。' }}</em>
    </section>

    <section class="sect-ledger-section">
      <div class="sect-ledger-title">掌门：{{ leaderName }}</div>
      <div class="sect-ledger-title">门人名录与武学卷宗 · 点击姓名展阅</div>
      <div v-if="!disciples.length" class="npc-dossier-empty">卷宗中暂无在籍门人。</div>
      <div v-else class="npc-disciple-grid">
        <details
          v-for="disciple in disciples"
          :key="disciple.id"
          class="npc-disciple-card npc-disciple-record"
          :open="isLeader(disciple)"
        >
          <summary class="npc-disciple-head">
            <span class="npc-disciple-identity">
              <b>{{ disciple.name }}</b>
              <small>{{ discipleRole(disciple) }}</small>
            </span>
            <span class="npc-disciple-brief">
              {{ disciple.age }}岁 · {{ departmentName(disciple.department) }} ·
              师承{{ masterName(disciple) }} ·
              {{ disciple.away_months ? `外出${disciple.away_months}月` : conditionNames[disciple.condition] }}
            </span>
            <i aria-hidden="true"></i>
          </summary>

          <div class="npc-disciple-dossier">
            <div class="aptitude-row npc-aptitude-row">
              <span>膂力<b>{{ effectiveAptitude(disciple, 'strength') }}<small v-if="aptitudeBonus(disciple, 'strength')">先天{{ disciple.aptitudes.strength }} +{{ aptitudeBonus(disciple, 'strength') }}</small></b></span>
              <span>悟性<b>{{ effectiveAptitude(disciple, 'intelligence') }}<small v-if="aptitudeBonus(disciple, 'intelligence')">先天{{ disciple.aptitudes.intelligence }} +{{ aptitudeBonus(disciple, 'intelligence') }}</small></b></span>
              <span>根骨<b>{{ effectiveAptitude(disciple, 'constitution') }}<small v-if="aptitudeBonus(disciple, 'constitution')">先天{{ disciple.aptitudes.constitution }} +{{ aptitudeBonus(disciple, 'constitution') }}</small></b></span>
              <span>身法<b>{{ effectiveAptitude(disciple, 'agility') }}<small v-if="aptitudeBonus(disciple, 'agility')">先天{{ disciple.aptitudes.agility }} +{{ aptitudeBonus(disciple, 'agility') }}</small></b></span>
              <span>福源<b>{{ disciple.aptitudes.fortune }}</b></span>
            </div>

            <div class="resource-lines npc-resource-lines">
              <span>气血 <b>{{ disciple.attributes.qi.current }}/{{ disciple.attributes.qi.maximum }}</b></span>
              <span>精神 <b>{{ disciple.attributes.spirit.current }}/{{ disciple.attributes.spirit.maximum }}</b></span>
              <span>内力 <b>{{ disciple.attributes.neili.current }}/{{ disciple.attributes.neili.maximum }}</b></span>
              <span>精力 <b>{{ disciple.attributes.energy.current }}/{{ disciple.attributes.energy.maximum }}</b></span>
            </div>

            <div class="attainment-line npc-attainment-line">
              造诣 {{ disciple.attributes.attainment }} · 门忠 {{ disciple.attributes.sect_loyalty ?? disciple.loyalty }} ·
              功绩 {{ disciple.merit }} · 声名 {{ disciple.attributes.reputation }} · 道德 {{ disciple.attributes.morality }}
            </div>
            <div class="npc-record-line">
              <b>任职师承</b>
              <span>{{ departmentName(disciple.department) }} · 师承{{ masterName(disciple) }}</span>
            </div>
            <div class="npc-record-line">
              <b>当前行动</b>
              <span>{{ actionSummary(disciple) }}</span>
            </div>
            <div v-if="disciple.action?.journey" class="npc-journey-line">
              <b>旅程卷宗</b>
              <span>
                {{ journeyDestinationName(disciple.action.journey.destination_id) }} ·
                {{ journeyTemplateName(disciple.action.journey.template_id) }} ·
                进度 {{ disciple.action.journey.elapsed_months }}/{{ disciple.action.journey.total_months }} ·
                难度 {{ disciple.action.journey.difficulty }} ·
                {{ disciple.action.journey.encounter_resolved ? '途中见闻已结' : '途中见闻未结' }} ·
                {{ journeyOutcomeName(disciple.action.journey.outcome) }}
              </span>
            </div>

            <div class="npc-prepared-line">
              <b>准备武学</b>
              <span v-for="entry in preparedEntries(disciple)" :key="`${entry.slot}:${entry.art}`">
                {{ entry.slot }}：{{ entry.art }}
              </span>
              <small v-if="!preparedEntries(disciple).length">尚无准备记录</small>
            </div>

            <div class="npc-skill-caption">六类武学谱 <small>等级与本级经验</small></div>
            <div class="npc-skill-groups npc-skill-dossier">
              <section v-for="category in skillCategories" :key="category.id">
                <header>{{ category.label }}<small>{{ category.hint }}</small></header>
                <span
                  v-for="skill in categorySkills(disciple, category.id)"
                  :key="skill.martial_art_id"
                  :class="{ prepared: isPrepared(disciple, skill.martial_art_id) }"
                >
                  <b>{{ artName(skill.martial_art_id) }}</b>
                  <em>{{ skill.level }}级<template v-if="isPrepared(disciple, skill.martial_art_id)"> · 已准备</template></em>
                  <small>经验 {{ skill.experience }}</small>
                </span>
                <small v-if="!categorySkills(disciple, category.id).length">未录入</small>
              </section>
            </div>
          </div>
        </details>
      </div>
    </section>

    <div class="sect-ledger-columns">
      <section class="sect-ledger-section">
        <div class="sect-ledger-title">山门建筑</div>
        <div class="readonly-building-list">
          <span v-for="building in sect.buildings" :key="building.id">
            <b>{{ building.name }}</b><small>第{{ building.level }}重 · 完好 {{ building.condition }}% · {{ buildingOfficer(building) }}</small>
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

<style scoped>
.npc-disciple-record {
  padding: 0;
}

.sect-view-header > i.allied {
  border-color: var(--color-gold);
  color: var(--color-cinnabar);
  box-shadow: 0 0 0 3px #b8860b1f;
}

.bilateral-alliance-status {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) minmax(180px, .9fr);
  align-items: center;
  gap: .5rem;
  margin-top: .42rem;
  padding: .38rem .5rem;
  border: 1px solid var(--color-border);
  background: #e8d5a84d;
}

.bilateral-alliance-status.allied {
  border-color: #b8860b99;
  background: linear-gradient(90deg, #c43a3112, #fff8e799 34%, #f2d57924);
  box-shadow: inset 3px 0 var(--color-cinnabar);
}

.bilateral-alliance-status > i {
  padding: .16rem .24rem;
  border: 1px solid var(--color-border);
  color: var(--color-ink-fade);
  font: .58rem var(--font-title);
  font-style: normal;
  writing-mode: vertical-rl;
}

.bilateral-alliance-status.allied > i {
  border-color: var(--color-cinnabar);
  color: var(--color-cinnabar);
}

.bilateral-alliance-status span,
.bilateral-alliance-status b,
.bilateral-alliance-status small {
  display: block;
  min-width: 0;
}

.bilateral-alliance-status b {
  color: var(--color-ink);
  font-size: .68rem;
}

.bilateral-alliance-status small,
.bilateral-alliance-status em {
  color: var(--color-ink-fade);
  font-size: .52rem;
}

.bilateral-alliance-status em {
  font-style: normal;
  line-height: 1.45;
  text-align: right;
}

.npc-disciple-head {
  position: relative;
  align-items: center;
  padding: .32rem .38rem;
  cursor: pointer;
  list-style: none;
}

.npc-disciple-head::-webkit-details-marker {
  display: none;
}

.npc-disciple-head > i {
  flex: 0 0 auto;
  width: 1.35rem;
  color: var(--color-cinnabar);
  font: .52rem var(--font-title);
  font-style: normal;
  text-align: right;
}

.npc-disciple-head > i::before {
  content: '展阅';
}

.npc-disciple-record[open] .npc-disciple-head > i::before {
  content: '收卷';
}

.npc-disciple-identity {
  display: flex;
  flex: 0 0 auto;
  align-items: baseline;
  gap: .22rem;
}

.npc-disciple-identity b {
  color: var(--color-ink);
  font-size: .76rem;
}

.npc-disciple-identity small {
  color: var(--color-cinnabar);
  font-size: .52rem;
}

.npc-disciple-brief {
  flex: 1;
  min-width: 0;
  color: var(--color-ink-fade);
  overflow: hidden;
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.npc-disciple-dossier {
  padding: .4rem;
  border-top: 1px dotted var(--color-border);
  background: #e8d5a826;
}

.npc-resource-lines {
  grid-template-columns: repeat(4, minmax(0, 1fr));
  margin: .32rem 0;
}

.npc-resource-lines span {
  display: flex;
  justify-content: space-between;
  gap: .2rem;
  padding: .16rem .22rem;
  border: 1px solid var(--color-border);
  background: #faf3e480;
}

.npc-resource-lines b {
  color: var(--color-ink);
  font-weight: 600;
}

.npc-attainment-line {
  margin-bottom: .28rem;
  color: var(--color-ink-light);
}

.npc-record-line,
.npc-journey-line {
  display: grid;
  grid-template-columns: 4.2rem 1fr;
  gap: .28rem;
  padding: .18rem .22rem;
  border-top: 1px dotted var(--color-border);
  color: var(--color-ink-light);
  font-size: .6rem;
}

.npc-record-line b,
.npc-journey-line b,
.npc-prepared-line > b {
  color: var(--color-cinnabar);
  font-family: var(--font-title);
}

.npc-journey-line {
  background: #9b7a3c12;
}

.npc-prepared-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: .18rem;
  margin-top: .25rem;
  padding: .22rem;
  border: 1px solid var(--color-border);
  background: #faf3e480;
  font-size: .58rem;
}

.npc-prepared-line > b {
  margin-right: .08rem;
}

.npc-prepared-line > span {
  padding: .1rem .2rem;
  border-left: 2px solid var(--color-cinnabar);
  background: var(--color-paper-light);
  color: var(--color-ink-light);
}

.npc-prepared-line > small,
.npc-dossier-empty {
  color: var(--color-ink-fade);
  font-size: .56rem;
}

.npc-skill-caption {
  display: flex;
  justify-content: space-between;
  margin-top: .32rem;
  color: var(--color-cinnabar);
  font: .7rem var(--font-title);
}

.npc-skill-caption small {
  color: var(--color-ink-fade);
  font: .52rem sans-serif;
}

.npc-skill-dossier {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.npc-skill-dossier section {
  border: 1px solid transparent;
}

.npc-skill-dossier header {
  display: flex;
  justify-content: space-between;
  gap: .2rem;
}

.npc-skill-dossier header small {
  color: var(--color-ink-fade);
  font-size: .46rem;
}

.npc-skill-dossier span {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  margin-top: .08rem;
  padding: .12rem .16rem;
  border-left: 2px solid var(--color-jade);
  background: #faf3e4b3;
}

.npc-skill-dossier span.prepared {
  border-left-color: var(--color-cinnabar);
}

.npc-skill-dossier span > small {
  grid-column: 1 / -1;
}

.npc-skill-dossier span.prepared em {
  color: var(--color-cinnabar);
}

@media (max-width: 749px) {
  .npc-disciple-head {
    flex-wrap: wrap;
  }

  .npc-disciple-brief {
    flex-basis: calc(100% - 2rem);
    order: 3;
    text-align: left;
  }

  .npc-resource-lines,
  .npc-skill-dossier {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
