<script setup lang="ts">
import { computed } from 'vue'
import type {
  Tournament,
  TournamentBout,
  TournamentLineupMember,
  TournamentMatch,
  TournamentSide,
} from '../types'

const props = withDefaults(defineProps<{
  tournament: Tournament
  playerSectId?: string
  compact?: boolean
}>(), {
  playerSectId: '',
  compact: false,
})

const rounds = computed(() => props.tournament.rounds || [])
const hasBracket = computed(() => rounds.value.some(round => round.matches.length))
const champion = computed(() => props.tournament.champion || winnerName(rounds.value.at(-1)?.matches.at(-1)))
const bracketStyle = computed(() => ({ '--round-count': Math.max(1, rounds.value.length) }))

function winnerName(match?: TournamentMatch) {
  if (!match) return ''
  return [match.left, match.right]
    .find(side => side?.sect_id === match.winner_sect_id)?.sect_name || ''
}

const isPlayerSide = (side?: TournamentSide | null) =>
  !!side && !!props.playerSectId && side.sect_id === props.playerSectId

const isPlayerMatch = (match: TournamentMatch) =>
  isPlayerSide(match.left) || isPlayerSide(match.right)

const isWinner = (match: TournamentMatch, side?: TournamentSide | null) =>
  !!side && side.sect_id === match.winner_sect_id

const boutName = (position: number) => ['首阵', '次阵', '末阵'][position - 1] || `第${position}阵`

const memberLabel = (member?: TournamentLineupMember | null) =>
  member ? `${member.name} · ${member.martial_art_id || '未备武学'}` : '缺阵'

const boutWinner = (bout: TournamentBout, side?: TournamentSide | null) =>
  !!side && bout.winner_sect_id === side.sect_id

const matchLabel = (match: TournamentMatch) => {
  const left = match.left?.sect_name || '空签'
  const right = match.right?.sect_name || '空签'
  return `${left}对阵${right}${match.bye ? '，本场轮空' : ''}`
}
</script>

<template>
  <section class="tournament-bracket" :class="{ compact }" aria-label="年终论剑签表">
    <header v-if="champion" class="bracket-champion">
      <span>本届魁首</span>
      <strong>{{ champion }}</strong>
      <i aria-hidden="true">魁</i>
    </header>

    <div v-if="hasBracket" class="bracket-scroll" tabindex="0" aria-label="可横向与纵向滚动查看完整签表">
      <div class="bracket-grid" :style="bracketStyle">
        <section v-for="round in rounds" :key="round.number" class="bracket-round">
          <h3><span>第{{ round.number }}轮</span>{{ round.name }}</h3>
          <div class="round-matches">
            <details
              v-for="match in round.matches"
              :key="match.id"
              class="bracket-match"
              :class="{
                'player-match': isPlayerMatch(match),
                'player-win': match.winner_sect_id === playerSectId,
                bye: match.bye,
              }"
            >
              <summary :aria-label="matchLabel(match)">
                <span
                  v-for="(side, index) in [match.left, match.right]"
                  :key="side?.sect_id || `empty-${index}`"
                  class="match-side"
                  :class="{ player: isPlayerSide(side), winner: isWinner(match, side), empty: !side }"
                >
                  <small v-if="side">第{{ side.seed }}签</small>
                  <b>{{ side?.sect_name || '空签' }}</b>
                  <em v-if="side">{{ match.bye ? '轮空' : `${side.bout_wins} 阵胜` }}</em>
                  <i v-if="isWinner(match, side)" aria-label="胜方">胜</i>
                </span>
              </summary>

              <div v-if="match.bye" class="bye-note">本签轮空，{{ winnerName(match) || '胜方' }}不战晋级。</div>
              <ol v-else class="bout-list" aria-label="三阵详情">
                <li v-for="bout in match.bouts" :key="bout.position">
                  <strong>{{ boutName(bout.position) }}</strong>
                  <span :class="{ won: boutWinner(bout, match.left), absent: !bout.left }">
                    <b>{{ memberLabel(bout.left) }}</b>
                    <small>{{ bout.left ? `${bout.left_score}分` : '—' }}</small>
                  </span>
                  <i aria-hidden="true">对</i>
                  <span :class="{ won: boutWinner(bout, match.right), absent: !bout.right }">
                    <b>{{ memberLabel(bout.right) }}</b>
                    <small>{{ bout.right ? `${bout.right_score}分` : '—' }}</small>
                  </span>
                </li>
              </ol>
            </details>
          </div>
        </section>
      </div>
    </div>

    <div v-else class="legacy-tournament">
      <strong>第 {{ tournament.rank }} 名</strong>
      <span>天下{{ tournament.total_sects }}派 · 本派战力{{ tournament.power }}</span>
      <small>旧卷未录逐场签表，仅存本届名次摘要。</small>
    </div>
  </section>
</template>

<style scoped>
.tournament-bracket {
  min-width: 0;
  color: var(--color-ink);
  text-align: left;
}

.bracket-champion {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: .65rem;
  min-height: 2.25rem;
  margin: 0 auto .55rem;
  padding: .35rem 3rem .35rem 1rem;
  overflow: hidden;
  border: 1px solid #b8860b99;
  background: linear-gradient(90deg, #fff4c900, #fff4c9 24%, #f2d579 50%, #fff4c9 76%, #fff4c900);
}

.bracket-champion span {
  color: var(--color-ink-fade);
  font-size: .68rem;
  letter-spacing: .18em;
}

.bracket-champion strong {
  color: #8f2b23;
  font: 1.15rem var(--font-title);
  letter-spacing: .12em;
}

.bracket-champion i {
  position: absolute;
  right: .65rem;
  display: grid;
  place-items: center;
  width: 1.55rem;
  height: 1.55rem;
  border: 2px solid var(--color-cinnabar);
  color: var(--color-cinnabar);
  font: .85rem var(--font-title);
  font-style: normal;
  transform: rotate(-5deg);
}

.bracket-scroll {
  max-height: min(51vh, 610px);
  overflow: auto;
  overscroll-behavior: contain;
  padding: .45rem;
  border: 1px solid var(--color-border);
  background: #f8eedb80;
  scrollbar-color: var(--color-border) #f4e5c8;
}

.bracket-scroll:focus-visible {
  outline: 2px solid var(--color-cinnabar);
  outline-offset: 2px;
}

.bracket-grid {
  display: grid;
  grid-template-columns: repeat(var(--round-count), 205px);
  align-items: start;
  gap: .65rem;
  width: max-content;
  min-width: 100%;
}

.bracket-round {
  min-width: 0;
}

.bracket-round > h3 {
  position: sticky;
  z-index: 5;
  top: -.45rem;
  margin: 0 0 .38rem;
  padding: .32rem;
  border-bottom: 2px solid var(--color-gold);
  background: #f5e6c8ee;
  color: var(--color-cinnabar);
  font: .9rem var(--font-title);
  text-align: center;
  letter-spacing: .1em;
}

.bracket-round > h3 span {
  display: block;
  color: var(--color-ink-fade);
  font: .52rem var(--font-serif);
  letter-spacing: .05em;
}

.round-matches {
  display: flex;
  flex-direction: column;
  gap: .38rem;
}

.bracket-match {
  min-width: 0;
  border: 1px solid var(--color-border);
  border-left: 3px solid var(--color-border);
  background: #fffaf0cc;
  box-shadow: 0 1px 3px #2c181012;
}

.bracket-match.player-match {
  border-color: #c43a3173;
  border-left-color: var(--color-cinnabar);
  background: linear-gradient(90deg, #c43a3114, #fffaf0 36%);
  box-shadow: 0 1px 7px #8f2b2329;
}

.bracket-match.player-win {
  box-shadow: inset 2px 0 var(--color-cinnabar), 0 1px 8px #8f2b233d;
}

.bracket-match > summary {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: .2rem;
  min-width: 0;
  padding: .25rem;
  cursor: pointer;
  list-style: none;
}

.bracket-match > summary::-webkit-details-marker {
  display: none;
}

.bracket-match > summary:focus-visible {
  outline: 2px solid var(--color-cinnabar);
  outline-offset: -1px;
}

.match-side {
  position: relative;
  display: grid;
  grid-template-columns: 1fr auto;
  min-width: 0;
  padding: .22rem .25rem;
  background: #e8d5a847;
}

.match-side + .match-side {
  border-left: 1px dotted var(--color-border);
}

.match-side small {
  grid-column: 1 / -1;
  color: var(--color-ink-fade);
  font-size: .5rem;
}

.match-side b {
  overflow: hidden;
  font-size: .68rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.match-side em {
  color: var(--color-ink-fade);
  font-size: .53rem;
  font-style: normal;
  white-space: nowrap;
}

.match-side i {
  position: absolute;
  right: .15rem;
  top: .08rem;
  color: var(--color-jade);
  font: .58rem var(--font-title);
  font-style: normal;
}

.match-side.winner {
  background: #4a7c5914;
}

.match-side.player {
  background: #c43a311c;
  color: #8f2b23;
}

.match-side.player.winner i {
  color: var(--color-cinnabar);
}

.match-side.empty {
  color: var(--color-ink-fade);
  opacity: .65;
}

.bye-note {
  padding: .28rem .4rem .4rem;
  color: var(--color-ink-fade);
  font-size: .58rem;
  line-height: 1.5;
}

.bout-list {
  margin: 0;
  padding: .15rem .3rem .3rem;
  list-style: none;
}

.bout-list li {
  display: grid;
  grid-template-columns: 2rem minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: .18rem;
  padding: .2rem 0;
  border-top: 1px dotted var(--color-border);
}

.bout-list li > strong {
  color: var(--color-cinnabar);
  font: .56rem var(--font-title);
}

.bout-list li > span {
  display: flex;
  min-width: 0;
  flex-direction: column;
  padding: .12rem .18rem;
}

.bout-list li > span b,
.bout-list li > span small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.bout-list li > span b {
  font-size: .52rem;
  font-weight: 600;
}

.bout-list li > span small {
  color: var(--color-ink-fade);
  font-size: .48rem;
}

.bout-list li > span.won {
  background: #4a7c5917;
  color: var(--color-jade);
}

.bout-list li > span.absent {
  color: var(--color-cinnabar);
  font-style: italic;
}

.bout-list li > i {
  color: var(--color-ink-fade);
  font-size: .45rem;
  font-style: normal;
}

.legacy-tournament {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  gap: .18rem;
  padding: .65rem;
  border: 1px dashed var(--color-border);
  background: #faf3e480;
  text-align: center;
}

.legacy-tournament strong {
  color: var(--color-gold);
  font: 1.3rem var(--font-title);
}

.legacy-tournament span {
  color: var(--color-ink-light);
  font-size: .68rem;
}

.legacy-tournament small {
  color: var(--color-ink-fade);
  font-size: .56rem;
}

.compact .bracket-scroll {
  max-height: 285px;
  padding: .3rem;
}

.compact .bracket-grid {
  grid-template-columns: repeat(var(--round-count), 180px);
  gap: .4rem;
}

.compact .bracket-champion {
  min-height: 1.8rem;
  margin-bottom: .35rem;
}

.compact .bracket-round > h3 {
  top: -.3rem;
  font-size: .72rem;
}

@media (max-width: 900px) {
  .bracket-scroll {
    max-height: 56vh;
  }

  .bracket-grid {
    grid-template-columns: repeat(var(--round-count), 190px);
  }
}
</style>
