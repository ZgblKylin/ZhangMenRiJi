<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Decision, Disciple, GameState, ManagementRequest, MartialArt } from '../types'
import TitleBar from './TitleBar.vue'
import DiscipleList from './DiscipleList.vue'
import DecisionGrid from './DecisionGrid.vue'
import AdvanceSection from './AdvanceSection.vue'
import TournamentPanel from './TournamentPanel.vue'
import ChroniclesBar from './ChroniclesBar.vue'
import SectManagementPanel from './SectManagementPanel.vue'

const props = defineProps<{ game: GameState; decisions: Decision[]; arts: MartialArt[]; used: string[] }>()
defineEmits<{ save: []; load: []; restart: []; decide: [id: string]; advance: []; manage: [command: ManagementRequest]; expel: [disciple: Disciple] }>()
const section = ref<'month' | 'sect' | 'library' | 'world'>('month')
const alive = computed(() => props.game.disciples?.filter(d => d.alive) || [])
const sectChronicles = computed(() => (props.game.event_log || []).filter(event => event.category !== 'world'))
const worldChronicles = computed(() => (props.game.event_log || []).filter(event => event.category === 'world'))
const lastTournament = computed(() => {
  const last = (props.game.tournament_history || []).at(-1)
  return last?.year === props.game.year ? last : null
})
const sections = [
  ['month', '本月议事'], ['sect', '山门营造'], ['library', '藏经研武'], ['world', '江湖通问'],
] as const
</script>

<template>
  <TitleBar :game="game" @save="$emit('save')" @load="$emit('load')" @restart="$emit('restart')" />
  <div class="book-layout">
    <div class="left-page">
      <DiscipleList :disciples="alive" :arts="arts" :disabled="game.decisions_used >= game.max_decisions || !!game.pending_event" @manage="$emit('manage', $event)" @expel="$emit('expel', $event)" />
    </div>
    <div class="center-page">
      <nav class="section-tabs" aria-label="中栏内容切换">
        <button v-for="[id, label] in sections" :key="id" :class="{ active: section === id }" @click="section = id">{{ label }}</button>
        <AdvanceSection :december="game.month === 12" :next-year="game.year + 1" :pending="!!game.pending_event" @advance="$emit('advance')" />
        <span>本月尚可定夺 <b>{{ Math.max(0, game.max_decisions - game.decisions_used) }}</b> 事</span>
      </nav>
      <template v-if="section === 'month'">
        <TournamentPanel v-if="game.month === 12" :tournament="lastTournament" />
        <DecisionGrid v-else :decisions="decisions" :arts="arts" :game="game" :used="used" @decide="$emit('decide', $event)" />
      </template>
      <SectManagementPanel v-else :game="game" :arts="arts" :view="section" @manage="$emit('manage', $event)" />
    </div>
    <ChroniclesBar :sect-entries="sectChronicles" :world-entries="worldChronicles" />
  </div>
</template>
