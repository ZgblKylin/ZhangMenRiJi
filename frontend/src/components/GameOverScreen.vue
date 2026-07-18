<script setup lang="ts">
import { computed } from 'vue'
import type { GameState, Tournament, TournamentMatch, TournamentSide } from '../types'
import TournamentBracket from './TournamentBracket.vue'

const props = defineProps<{ game: GameState }>()
defineEmits<{ restart: []; saves: [] }>()
const finalTournaments = computed(() => (props.game.tournament_history || []).slice(-2))
const finalTournament = computed(() => finalTournaments.value.at(-1))

const matchSide = (match: TournamentMatch, player = true) =>
  [match.left, match.right].find(side =>
    player ? side?.sect_id === props.game.sect.id : side?.sect_id !== props.game.sect.id)

const opponentSide = (match: TournamentMatch): TournamentSide | undefined => {
  const sides = [match.left, match.right].filter((side): side is TournamentSide => !!side)
  return sides.find(side => side.sect_id !== props.game.sect.id)
}

const inferredChampion = (record: Tournament) => {
  if (record.champion) return record.champion
  const finalMatch = record.rounds?.at(-1)?.matches.at(-1)
  return [finalMatch?.left, finalMatch?.right]
    .find(side => side?.sect_id === finalMatch?.winner_sect_id)?.sect_name || '旧卷未载'
}

const playerPath = computed(() => {
  const record = finalTournament.value
  if (!record?.rounds?.length) return []
  return record.rounds.flatMap(round =>
    round.matches
      .filter(match => !!matchSide(match))
      .map(match => {
        const opponent = opponentSide(match)
        const won = match.winner_sect_id === props.game.sect.id
        return {
          round: round.name,
          result: match.bye ? '轮空晋级' : won ? '取胜' : '止步',
          opponent: match.bye ? '' : opponent?.sect_name || '无名之师',
          score: match.bye || !opponent
            ? ''
            : `${matchSide(match)?.bout_wins ?? 0}:${opponent.bout_wins}`,
          won,
        }
      }))
})
</script>

<template>
  <section class="start-screen game-over-screen fade-in p-8">
    <h1 class="!text-4xl" :class="game.game_won ? '!text-gold' : '!text-cinnabar'">{{ game.game_won ? '名 动 江 湖' : '山 门 落 幕' }}</h1>
    <div class="ending-prose text-lg leading-8 text-ink-light">
      {{ game.sect_name }}<br>
      <template v-if="game.game_won">门中经略有成，山门卷宗至此告一段落。</template>
      <template v-else>第{{ game.year }}年{{ game.month }}月</template>
      <br><br>{{ game.game_over_reason }}
    </div>

    <section v-if="game.game_won" class="tournament-ending">
      <div class="ending-ledger">
        <div class="ledger-title">两 届 论 剑</div>
        <div v-for="record in finalTournaments" :key="record.year" class="ledger-row">
          <span>第{{ record.year }}年</span>
          <strong>第 {{ record.rank }} 名</strong>
          <small>天下{{ record.total_sects }}派 · 魁首{{ inferredChampion(record) }}</small>
        </div>
      </div>

      <article v-if="finalTournament" class="final-campaign">
        <header>
          <span>第 二 届 · 本 门 征 途</span>
          <strong>{{ inferredChampion(finalTournament) }}</strong>
          <small>本届魁首</small>
        </header>

        <ol v-if="playerPath.length" class="player-path" aria-label="第二届本门晋级路径">
          <li v-for="step in playerPath" :key="step.round" :class="{ won: step.won }">
            <b>{{ step.round }}</b>
            <span>{{ step.result }}<template v-if="step.opponent"> · {{ step.opponent }}</template></span>
            <em v-if="step.score">{{ step.score }}</em>
          </li>
        </ol>
        <div v-else class="legacy-path">旧卷未录逐轮对阵，第二届仅存最终名次。</div>

        <details class="ending-bracket">
          <summary>展开第二届完整签表</summary>
          <TournamentBracket
            :tournament="finalTournament"
            :player-sect-id="game.sect.id"
            compact
          />
        </details>
      </article>
    </section>

    <div class="mb-4 text-ink-fade">声望：{{ game.prestige }} · 库银：{{ game.silver }}两 · 弟子：{{ game.disciples.filter(d => d.alive).length }}人</div>
    <button class="btn btn-primary px-8 py-2.5 text-lg" @click="$emit('restart')">{{ game.game_won ? '另 开 山 门' : '重 头 再 来' }}</button>
    <div class="mt-4"><button class="btn btn-sm" @click="$emit('saves')">{{ game.game_won ? '查 看 存 档' : '查看存档 / 回档' }}</button></div>
  </section>
</template>

<style scoped>
.game-over-screen {
  width: min(1120px, 94%);
  max-height: calc(100% - 2rem);
  margin: auto;
  overflow-y: auto;
}

.ending-prose {
  margin: 1rem 0;
}

.tournament-ending {
  width: min(1050px, 100%);
  margin: 0 auto 1rem;
}

.ending-ledger {
  display: grid;
  grid-template-columns: auto 1fr 1fr;
  align-items: center;
  border-block: 1px solid #6d55384d;
  color: var(--color-ink-light);
}

.ledger-title {
  grid-row: 1 / span 2;
  padding: .55rem 1rem;
  color: var(--color-ink-fade);
  font-size: .72rem;
  letter-spacing: .3em;
  writing-mode: vertical-rl;
}

.ledger-row {
  grid-column: 2 / -1;
  display: grid;
  grid-template-columns: 5rem 6rem minmax(0, 1fr);
  align-items: center;
  gap: .45rem;
  padding: .3rem .6rem;
  text-align: left;
}

.ledger-row + .ledger-row {
  border-top: 1px dotted var(--color-border);
}

.ledger-row strong {
  color: var(--color-cinnabar);
}

.ledger-row small {
  color: var(--color-ink-fade);
}

.final-campaign {
  margin-top: .75rem;
  padding: .65rem .75rem;
  border: 2px solid #b8860b80;
  background: linear-gradient(135deg, #fff8e799, #faf3e4d9);
}

.final-campaign > header {
  display: grid;
  grid-template-columns: 1fr auto auto;
  align-items: center;
  gap: .6rem;
  padding-bottom: .45rem;
  border-bottom: 1px dashed var(--color-border);
  text-align: left;
}

.final-campaign > header span {
  color: var(--color-cinnabar);
  font: .85rem var(--font-title);
  letter-spacing: .18em;
}

.final-campaign > header strong {
  color: var(--color-gold);
  font: 1.2rem var(--font-title);
  letter-spacing: .1em;
}

.final-campaign > header small {
  color: var(--color-ink-fade);
}

.player-path {
  display: flex;
  align-items: stretch;
  gap: .25rem;
  margin: .55rem 0;
  padding: 0;
  overflow-x: auto;
  list-style: none;
}

.player-path li {
  position: relative;
  flex: 1 0 120px;
  min-width: 0;
  padding: .3rem .45rem;
  border: 1px solid var(--color-border);
  background: #e8d5a84d;
  text-align: left;
}

.player-path li + li::before {
  content: '›';
  position: absolute;
  left: -.25rem;
  top: 50%;
  color: var(--color-cinnabar);
  transform: translate(-50%, -50%);
}

.player-path li.won {
  border-color: #c43a3173;
  background: #c43a3114;
}

.player-path b,
.player-path span {
  display: block;
}

.player-path b {
  color: var(--color-cinnabar);
  font: .68rem var(--font-title);
}

.player-path span {
  overflow: hidden;
  color: var(--color-ink-light);
  font-size: .58rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.player-path em {
  position: absolute;
  right: .3rem;
  top: .3rem;
  color: var(--color-jade);
  font-size: .58rem;
  font-style: normal;
}

.legacy-path {
  margin: .55rem 0;
  color: var(--color-ink-fade);
  font-size: .68rem;
}

.ending-bracket {
  text-align: left;
}

.ending-bracket > summary {
  width: max-content;
  margin: .3rem auto;
  color: var(--color-cinnabar);
  cursor: pointer;
  font: .75rem var(--font-title);
}

.ending-bracket > summary:focus-visible {
  outline: 2px solid var(--color-cinnabar);
  outline-offset: 2px;
}
</style>
