import { computed, reactive, ref, shallowRef } from 'vue'
import type { ChronicleEvent, Decision, GameResponse, GameState, MartialArt, SaveGroup, Tournament } from './types'

interface GameSession {
  id: string
  saveGroupId: string
  revision: number | null
  state: GameState
}

const session = shallowRef<GameSession | null>(null)
export const gameId = computed(() => session.value?.id ?? null)
export const gameGroupId = computed(() => session.value?.saveGroupId ?? null)
export const gameRevision = computed(() => session.value?.revision ?? null)
export const G = computed(() => session.value?.state ?? null)
export const usedDecisions = ref<string[]>([])
export const saveGroups = ref<SaveGroup[]>([])
export const DECISIONS = ref<Decision[]>([])
export const MARTIAL_ARTS = ref<MartialArt[]>([])

export const ui = reactive({
  loading: false,
  savePanel: false,
  popup: false,
  popupSectEvents: [] as ChronicleEvent[],
  popupWorldEvents: [] as ChronicleEvent[],
  tournament: null as Tournament | null,
  resolvingEvent: false,
  settingsOpen: false,
  changelogOpen: false,
})

export const aliveDisciples = computed(() => G.value?.disciples?.filter(d => d.alive) || [])

const validRevision = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0

export function adoptGameResponse(data: GameResponse, preferredSectName = '') {
  const previous = session.value
  const state = {
    ...data.state,
    sect_name: data.sect_name || data.state.sect_name || preferredSectName || previous?.state.sect_name || '',
  }
  session.value = {
    id: data.id || previous?.id || '',
    saveGroupId: data.save_group_id || previous?.saveGroupId || '',
    revision: validRevision(data.revision) ? data.revision : null,
    state,
  }
}

export function adoptGameRevision(revision: number) {
  if (!session.value || !validRevision(revision)) return
  session.value = { ...session.value, revision }
}

export function clearTransientGameUi() {
  usedDecisions.value = []
  ui.popupSectEvents = []
  ui.popupWorldEvents = []
  ui.tournament = null
  ui.resolvingEvent = false
  ui.popup = !!G.value?.pending_event
}

export function preferredContinuationId(groups: SaveGroup[]) {
  const latestGroup = groups[0]
  return latestGroup?.current_id || latestGroup?.saves[0]?.id || null
}

export function resetGame() {
  session.value = null
  clearTransientGameUi()
}
