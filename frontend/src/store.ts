import { computed, reactive, ref } from 'vue'
import type { ChronicleEvent, Decision, GameState, MartialArt, SaveGroup, Tournament } from './types'

export const gameId = ref<string | null>(null)
export const gameGroupId = ref<string | null>(null)
export const G = ref<GameState | null>(null)
export const usedDecisions = ref<string[]>([])
export const saveGroups = ref<SaveGroup[]>([])
export const DECISIONS = ref<Decision[]>([])
export const MARTIAL_ARTS = ref<MartialArt[]>([])

export const ui = reactive({
  loading: false,
  savePanel: false,
  popup: false,
  popupEvents: [] as ChronicleEvent[],
  tournament: null as Tournament | null,
  resolvingEvent: false,
  settingsOpen: false,
})

export const aliveDisciples = computed(() => G.value?.disciples?.filter(d => d.alive) || [])

export function resetGame() {
  gameId.value = null
  gameGroupId.value = null
  G.value = null
  usedDecisions.value = []
}
