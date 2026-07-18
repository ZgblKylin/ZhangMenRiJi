<script setup lang="ts">
import { computed } from 'vue'
import { G, MARTIAL_ARTS } from '../store'
import { MartialTier } from '../types'
import type { Disciple, Tournament } from '../types'
import TournamentBracket from './TournamentBracket.vue'

defineProps<{ tournament: Tournament | null }>()

const rankName = (n: number) =>
  (n >= 0 && n <= 10
    ? ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九', '十'][n]
    : String(n))

const skillLevel = (disciple: Disciple, id: string) =>
  disciple.martial_progress?.proficiencies?.[id]?.level
  ?? disciple.skills?.find(skill => skill.martial_art_id === id)?.level
  ?? 0

const attainmentCap = (attainment: number) => {
  const target = Math.max(0, attainment) * 10
  if (target <= 0) return 0
  let cap = Math.ceil(Math.cbrt(target))
  while (cap > 0 && (cap - 1) ** 3 >= target) cap--
  while (cap ** 3 < target) cap++
  return cap
}

/**
 * 与后端 get_combat_score 保持相同口径，让十二月签位预览不会另造一套战力。
 */
const combatScore = (disciple: Disciple) => {
  let skillTotal = 0
  let artPower = 0
  for (const basic of MARTIAL_ARTS.value.filter(art => art.is_combat && art.tier === MartialTier.Basic)) {
    skillTotal += Math.floor(Math.max(0, skillLevel(disciple, basic.id)) / 2)
    const preparedId = disciple.prepared_skills?.[basic.id]
    if (!preparedId) continue
    skillTotal += skillLevel(disciple, preparedId)
    const prepared = MARTIAL_ARTS.value.find(art => art.id === preparedId)
    if (prepared) artPower += prepared.atk + prepared.def + prepared.spd
  }
  const strength = disciple.aptitudes.strength + Math.floor(Math.max(0, skillLevel(disciple, 'basic_unarmed')) / 10)
  const constitution = disciple.aptitudes.constitution + Math.floor(Math.max(0, skillLevel(disciple, 'basic_force')) / 10)
  const agility = disciple.aptitudes.agility + Math.floor(Math.max(0, skillLevel(disciple, 'basic_dodge')) / 10)
  const fortune = disciple.aptitudes.fortune
  const combatTalent = Math.trunc((strength + constitution + agility + fortune) / 4)
  const energyRatio = Math.max(0, disciple.attributes.energy.current)
    / Math.max(1, disciple.attributes.energy.maximum)
  const depth = Math.min(500, attainmentCap(disciple.attributes.attainment))
  return Math.trunc(
    combatTalent * .2
    + disciple.attributes.neili.maximum * .3
    + Math.min(2500, skillTotal) * .025
    + artPower * 3
    + depth * .08
    + Math.min(1.5, energyRatio) * 10
    + disciple.attributes.sect_loyalty * .05,
  )
}

const available = computed(() => {
  const game = G.value
  if (!game) return []
  return game.disciples
    .filter(disciple =>
      disciple.alive
      && disciple.sect_id === game.sect.id
      && disciple.condition === 'healthy'
      && disciple.away_months <= 0)
    .map(disciple => ({ disciple, score: combatScore(disciple) }))
    .sort((left, right) =>
      right.score - left.score
      || (left.disciple.id < right.disciple.id ? -1 : left.disciple.id > right.disciple.id ? 1 : 0))
})

const projectedLineup = computed(() => available.value.slice(0, 3))
const missingPositions = computed(() => Math.max(0, 3 - available.value.length))
</script>

<template>
  <section class="panel tournament-panel">
    <div class="panel-title">年 终 论 剑</div>

    <template v-if="tournament">
      <div class="rank-display">第 {{ rankName(tournament.rank) }} 名</div>
      <div class="tournament-result">
        天下共 <strong>{{ tournament.total_sects }}</strong> 派与会 · 本派战力：{{ tournament.power }}
        <template v-if="tournament.champion"><br>本届魁首：<b>{{ tournament.champion }}</b></template>
      </div>
      <details class="recent-bracket">
        <summary>查看本届逐场赛程</summary>
        <TournamentBracket
          :tournament="tournament"
          :player-sect-id="G?.sect.id"
          compact
        />
      </details>
    </template>

    <template v-else>
      <div class="tournament-result">
        岁末将至，天下各派蓄势待发。封卷后将依当前战力自动遣健康在门的前三人出阵。
      </div>
      <div class="lineup-status" :class="{ warning: missingPositions > 0 }">
        <strong>当前可用 {{ available.length }} 人</strong>
        <span v-if="missingPositions">尚缺 {{ missingPositions }} 个阵位，空位将以缺阵判负。</span>
        <span v-else>三阵齐备，以下三人暂列出阵签。</span>
      </div>
      <ol v-if="projectedLineup.length" class="projected-lineup" aria-label="预计出阵弟子">
        <li v-for="({ disciple, score }, index) in projectedLineup" :key="disciple.id">
          <i>{{ index + 1 }}</i>
          <b>{{ disciple.name }}</b>
          <span>战力 {{ score }}</span>
        </li>
      </ol>
      <div v-else class="empty-lineup">眼下无人可战，请先召回游历弟子并疗愈伤员。</div>
      <small class="lineup-caveat">月末事件、伤势或离门变化会在开赛时重新排定。</small>
    </template>
  </section>
</template>

<style scoped>
.tournament-panel {
  padding: .55rem .75rem;
}

.lineup-status {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: .5rem;
  margin: .35rem 0;
  padding: .3rem .45rem;
  border: 1px solid #4a7c5966;
  background: #4a7c590d;
  color: var(--color-jade);
  font-size: .65rem;
}

.lineup-status.warning {
  border-color: #c43a3166;
  background: #c43a310d;
  color: var(--color-cinnabar);
}

.lineup-status span {
  color: var(--color-ink-light);
}

.projected-lineup {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: .3rem;
  margin: .35rem 0 .2rem;
  padding: 0;
  list-style: none;
}

.projected-lineup li {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: center;
  min-width: 0;
  padding: .28rem .35rem;
  border: 1px solid var(--color-border);
  background: #fffaf080;
  text-align: left;
}

.projected-lineup i {
  grid-row: 1 / span 2;
  display: grid;
  place-items: center;
  width: 1.2rem;
  height: 1.2rem;
  margin-right: .3rem;
  border: 1px solid var(--color-cinnabar);
  color: var(--color-cinnabar);
  font: .65rem var(--font-title);
  font-style: normal;
}

.projected-lineup b,
.projected-lineup span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.projected-lineup b {
  font-size: .7rem;
}

.projected-lineup span {
  color: var(--color-ink-fade);
  font-size: .55rem;
}

.empty-lineup {
  margin: .3rem 0;
  color: var(--color-cinnabar);
  font-size: .68rem;
}

.lineup-caveat {
  display: block;
  color: var(--color-ink-fade);
  font-size: .55rem;
}

.recent-bracket {
  margin-top: .45rem;
  text-align: left;
}

.recent-bracket > summary {
  width: max-content;
  margin: 0 auto .35rem;
  color: var(--color-cinnabar);
  cursor: pointer;
  font: .68rem var(--font-title);
}

.recent-bracket > summary:focus-visible {
  outline: 2px solid var(--color-cinnabar);
  outline-offset: 2px;
}
</style>
