<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { gameApi } from './api'
import { DECISIONS, G, gameId, MARTIAL_ARTS, resetGame, saveSlots, ui, usedDecisions } from './store'
import ScrollContainer from './components/ScrollContainer.vue'
import StartScreen from './components/StartScreen.vue'
import GameOverScreen from './components/GameOverScreen.vue'
import GameView from './components/GameView.vue'
import SavePanel from './components/SavePanel.vue'
import EventPopup from './components/EventPopup.vue'
import LoadingOverlay from './components/LoadingOverlay.vue'

import SettingsPanel from './components/SettingsPanel.vue'
import type { ManagementRequest } from './types'

const report = (prefix: string, error: unknown) => alert(`${prefix}: ${error instanceof Error ? error.message : String(error)}`)
const loading = async (task: () => Promise<void>) => { ui.loading = true; try { await task() } finally { ui.loading = false } }

const loadStaticData = async () => {
  try { const data = await gameApi.staticData(); DECISIONS.value = data.decisions; MARTIAL_ARTS.value = data.arts }
  catch (error) { console.warn('加载静态数据失败:', error) }
}
const refreshSaves = async () => {
  try { const { games = [] } = await gameApi.list(); saveSlots.value = games.map(g => ({ id: g.id, sect_name: g.sect_name || g.state?.sect_name || '', year: g.state?.year || 1, month: g.state?.month || 1, prestige: g.state?.prestige || 0, silver: g.state?.silver || 0, disciples: g.state?.disciples?.filter(d => d.alive).length || 0, updated_at: g.updated_at })) }
  catch (error) { console.warn('获取存档列表失败:', error); saveSlots.value = [] }
}
const openSaves = async (mode: 'load' | 'manage') => loading(async () => { await refreshSaves(); ui.saveMode = mode; ui.savePanel = true })
const start = (name: string) => loading(async () => { try { const data = await gameApi.create(name); gameId.value = data.id; G.value = { ...data.state, sect_name: name }; usedDecisions.value = [] } catch (e) { report('创建游戏失败，请确认后端已启动', e) } })
const loadGame = (id: string) => loading(async () => { try { const data = await gameApi.get(id); gameId.value = data.id; G.value = { ...data.state, sect_name: data.sect_name || data.state.sect_name || '' }; usedDecisions.value = []; ui.savePanel = false; if (G.value.pending_event) ui.popup = true } catch (e) { report('载入失败', e) } })
const removeSave = async (id: string) => { if (!confirm('确定删除此存档？')) return; await loading(async () => { try { await gameApi.remove(id); if (gameId.value === id) resetGame(); await refreshSaves() } catch (e) { report('删除失败', e) } }) }
const decide = (id: string) => loading(async () => { if (!gameId.value || !G.value || usedDecisions.value.includes(id)) return; try { const name = G.value.sect_name; const data = await gameApi.decide(gameId.value, id); G.value = { ...data.state, sect_name: data.state.sect_name || name }; usedDecisions.value.push(id) } catch (e) { report('决策失败', e) } })
const manage = (command: ManagementRequest) => loading(async () => { if (!gameId.value || !G.value) return; try { const name = G.value.sect_name; const data = await gameApi.manage(gameId.value, command); G.value = { ...data.state, sect_name: data.state.sect_name || name } } catch (e) { report('掌门令未能施行', e) } })
const advance = () => loading(async () => { if (!gameId.value || !G.value) return; try { const name = G.value.sect_name; const before = `${G.value.year}-${G.value.month}`; const data = await gameApi.advance(gameId.value); G.value = { ...data.state, sect_name: data.state.sect_name || name }; if (`${G.value.year}-${G.value.month}` !== before) usedDecisions.value = []; ui.popupEvents = data.events || []; ui.tournament = data.tournament || null; ui.popup = !!(data.events?.length || data.tournament || G.value.pending_event) } catch (e) { report('推演月令未成', e) } })
const resolveEvent = (optionId: string) => loading(async () => { if (!gameId.value || !G.value) return; ui.resolvingEvent = true; try { const name = G.value.sect_name; const data = await gameApi.resolveEvent(gameId.value, optionId); G.value = { ...data.state, sect_name: data.state.sect_name || name }; usedDecisions.value = []; ui.popupEvents = data.events || []; ui.tournament = data.tournament || null; ui.popup = true } catch (e) { report('此策未能施行', e) } finally { ui.resolvingEvent = false } })
const closePopup = () => { if (G.value?.pending_event) return; ui.popup = false; ui.popupEvents = []; ui.tournament = null }
const restart = () => { if (confirm('确定重开山门？')) resetGame() }
const keyboard = (event: KeyboardEvent) => { if (event.key === 'Escape') { if (ui.popup && !G.value?.pending_event) closePopup(); else if (!G.value?.pending_event) ui.savePanel = false } else if (ui.popup && !G.value?.pending_event && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); closePopup() } }
onMounted(() => { loadStaticData(); document.addEventListener('keydown', keyboard) })
onBeforeUnmount(() => document.removeEventListener('keydown', keyboard))
</script>

<template>
  <ScrollContainer>
    <StartScreen v-if="!G" @start="start" @load="openSaves('load')" />
    <GameOverScreen v-else-if="G.game_over" :game="G" @restart="resetGame" @saves="openSaves('manage')" />
    <GameView v-else :game="G" :decisions="DECISIONS" :arts="MARTIAL_ARTS" :used="usedDecisions" @saves="openSaves('manage')" @restart="restart" @decide="decide" @manage="manage" @advance="advance" />
  </ScrollContainer>
  <SavePanel :open="ui.savePanel" :mode="ui.saveMode" :slots="saveSlots" :current-id="gameId" @close="ui.savePanel = false" @load="loadGame" @remove="removeSave" />
  <EventPopup :open="ui.popup" :events="ui.popupEvents" :tournament="ui.tournament" :pending="G?.pending_event" :choosing="ui.resolvingEvent" @choose="resolveEvent" @close="closePopup" />
  <SettingsPanel :open="ui.settingsOpen" @close="ui.settingsOpen = false" />
  <LoadingOverlay :show="ui.loading" />
</template>
