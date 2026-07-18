<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { BuildingKind, Decision, Disciple, GameState, ManagementRequest, MartialArt } from '../types'
import TitleBar from './TitleBar.vue'
import DiscipleList from './DiscipleList.vue'
import AdvanceSection from './AdvanceSection.vue'
import TournamentPanel from './TournamentPanel.vue'
import ChroniclesBar from './ChroniclesBar.vue'
import SectManagementPanel from './SectManagementPanel.vue'

const props = defineProps<{ game: GameState; decisions: Decision[]; arts: MartialArt[]; used: string[] }>()
defineEmits<{ save: []; load: []; restart: []; decide: [id: string]; advance: []; manage: [command: ManagementRequest]; expel: [disciple: Disciple] }>()
const section = ref<BuildingKind>('practice')
const managementPanel = ref<InstanceType<typeof SectManagementPanel> | null>(null)
watch(section, () => managementPanel.value?.closeNpcSect())
const alive = computed(() => props.game.disciples?.filter(d => d.alive) || [])
const sectChronicles = computed(() => (props.game.event_log || []).filter(event => event.category !== 'world'))
const worldChronicles = computed(() => (props.game.event_log || []).filter(event => event.category === 'world'))
const lastTournament = computed(() => {
  const last = (props.game.tournament_history || []).at(-1)
  return last?.year === props.game.year ? last : null
})
const sections = [
  ['practice', '演武场'], ['scripture', '藏经阁'], ['warehouse', '仓库'], ['herb_hall', '百草堂'],
  ['intelligence', '天枢阁'], ['affairs', '执事堂'], ['logistics', '庶务堂'],
] as const
</script>

<template>
  <TitleBar :game="game" @save="$emit('save')" @load="$emit('load')" @restart="$emit('restart')" />
  <div class="book-layout">
    <div class="left-page">
      <DiscipleList :disciples="alive" :arts="arts" :buildings="game.sect.buildings" :inventory="game.sect.inventory" :public-books="game.sect.public_books" :martial-research="game.sect.martial_research" :disabled="game.decisions_used >= game.max_decisions || !!game.pending_event" @manage="$emit('manage', $event)" @expel="$emit('expel', $event)" />
    </div>
    <div class="center-page">
      <nav class="section-tabs" aria-label="中栏内容切换">
        <button v-for="[id, label] in sections" :key="id" :class="{ active: section === id }" @click="section = id">{{ label }}</button>
        <div class="advance-section">
          <span>本月尚可定夺 <b>{{ Math.max(0, game.max_decisions - game.decisions_used) }}/{{ game.max_decisions }}</b></span>
          <AdvanceSection :december="game.month === 12" :next-year="game.year + 1" :pending="!!game.pending_event" :exhausted="game.decisions_used >= game.max_decisions" @advance="$emit('advance')" />
        </div>
      </nav>
      <TournamentPanel v-if="game.month === 12" :tournament="lastTournament" />
      <SectManagementPanel ref="managementPanel" :game="game" :arts="arts" :decisions="decisions" :used="used" :view="section" @decide="$emit('decide', $event)" @manage="$emit('manage', $event)" />
    </div>
    <ChroniclesBar :sect-entries="sectChronicles" :world-entries="worldChronicles" />
  </div>
</template>
