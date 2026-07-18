<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { G } from '../store'
import type { ChronicleEvent, PendingWorldEvent, Tournament } from '../types'
import TournamentBracket from './TournamentBracket.vue'

const props = defineProps<{ open: boolean; sectEvents: ChronicleEvent[]; worldEvents: ChronicleEvent[]; tournament: Tournament | null; pending?: PendingWorldEvent | null; choosing?: boolean }>()
const emit = defineEmits<{ close: []; choose: [id: string] }>()
const button = ref<HTMLButtonElement>()
const rankName = (n: number) =>
  (n >= 0 && n <= 10
    ? ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九', '十'][n]
    : String(n))
const dialogLabel = computed(() => props.pending?.title || (props.tournament ? '年终论剑与本月纪事' : '本月纪事'))

const bracketTournament = computed<Tournament | null>(() => {
  if (!props.tournament) return null
  const history = G.value?.tournament_history || []
  const recorded = props.tournament.year
    ? history.find(record => record.year === props.tournament?.year)
    : history.at(-1)
  return {
    ...(recorded || {}),
    ...props.tournament,
    champion: props.tournament.champion || recorded?.champion || '',
    player_lineup: props.tournament.player_lineup?.length
      ? props.tournament.player_lineup
      : recorded?.player_lineup || [],
    rounds: props.tournament.rounds?.length ? props.tournament.rounds : recorded?.rounds || [],
  }
})

watch(() => props.open, open => { if (open) nextTick(() => button.value?.focus({ preventScroll: true })) })
</script>

<template>
  <Transition name="fade">
    <div v-if="open" class="modal-overlay z-[150]">
      <div
        class="event-dialog"
        :class="{ 'choice-dialog': pending, 'tournament-dialog': tournament && !pending }"
        role="dialog"
        aria-modal="true"
        :aria-label="dialogLabel"
      >
        <template v-if="pending">
          <div class="event-category">{{ pending.category }}</div>
          <div class="event-title">{{ pending.title }}</div>
          <div class="event-prose">{{ pending.text }}</div>
          <div class="event-choices">
            <button v-for="choice in pending.choices" :key="choice.id" :disabled="choosing" @click="emit('choose', choice.id)">
              <b>{{ choice.label }}</b><span>{{ choice.result_text }}</span>
            </button>
          </div>
          <div class="choice-warning">此事不决，月中诸务暂且搁下。</div>
        </template>
        <template v-else>
          <template v-if="tournament">
            <div class="event-title">年 终 论 剑</div>
            <div class="my-2 text-center text-3xl text-gold">第 {{ rankName(tournament.rank) }} 名</div>
            <div class="mb-3 text-center text-sm text-ink-light">天下共 <strong>{{ tournament.total_sects }}</strong> 派与会 · 战力：{{ tournament.power }}<br>{{ tournament.desc_text }}</div>
            <TournamentBracket
              v-if="bracketTournament"
              class="event-bracket"
              :tournament="bracketTournament"
              :player-sect-id="G?.sect.id"
            />
          </template>
          <template v-if="sectEvents.length || worldEvents.length">
            <div class="event-title">本 月 纪 事</div>
            <div class="event-columns">
              <section>
                <h3>本 门 纪 事</h3>
                <div v-if="!sectEvents.length" class="event-empty">本月门中无事。</div>
                <div v-for="(event, i) in sectEvents" :key="i" class="event-item" :class="event.mood">{{ event.text }}</div>
              </section>
              <section>
                <h3>江 湖 纪 事</h3>
                <div v-if="!worldEvents.length" class="event-empty">本月江湖无大事。</div>
                <div v-for="(event, i) in worldEvents" :key="i" class="event-item" :class="event.mood">{{ event.text }}</div>
              </section>
            </div>
          </template>
          <button ref="button" class="btn btn-primary event-dismiss" @click="emit('close')">阅毕收卷</button>
        </template>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.tournament-dialog {
  display: block;
  width: min(96%, 1160px);
  overflow-y: auto;
}

.event-bracket {
  min-height: 0;
}

.tournament-dialog .event-columns {
  height: 13rem;
  max-height: 13rem;
  margin-top: .8rem;
}

.tournament-dialog .event-bracket:deep(.bracket-scroll) {
  max-height: min(42vh, 520px);
}

@media (max-width: 749px) {
  .tournament-dialog .event-columns {
    height: 11rem;
    max-height: 11rem;
  }
}
</style>
