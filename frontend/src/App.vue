<script setup lang="ts">
import { onBeforeUnmount, onMounted } from 'vue'
import { gameApi } from './api'
import { DECISIONS, G, gameGroupId, gameId, MARTIAL_ARTS, resetGame, saveGroups, ui, usedDecisions } from './store'
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
  try { const { groups = [] } = await gameApi.list(); saveGroups.value = groups }
  catch (error) { console.warn('获取存档列表失败:', error); saveGroups.value = [] }
}
const openSaves = async () => loading(async () => { await refreshSaves(); ui.savePanel = true })
const start = (name: string) => loading(async () => { try { const data = await gameApi.create(name); gameId.value = data.id; gameGroupId.value = data.save_group_id; G.value = { ...data.state, sect_name: name }; usedDecisions.value = []; await refreshSaves() } catch (e) { report('创建游戏失败，请确认后端已启动', e) } })
const loadGame = (id: string) => loading(async () => { try { const data = await gameApi.get(id); gameId.value = data.id; gameGroupId.value = data.save_group_id; G.value = { ...data.state, sect_name: data.sect_name || data.state.sect_name || '' }; usedDecisions.value = []; ui.savePanel = false; if (G.value.pending_event) ui.popup = true } catch (e) { report('载入失败', e) } })
const continueGame = () => loading(async () => { await refreshSaves(); const latest = saveGroups.value[0]?.saves[0]; if (!latest) return; try { const data = await gameApi.get(latest.id); gameId.value = data.id; gameGroupId.value = data.save_group_id; G.value = { ...data.state, sect_name: data.sect_name || data.state.sect_name || '' }; usedDecisions.value = []; if (G.value.pending_event) ui.popup = true } catch (e) { report('再入江湖失败', e) } })
const manualSave = () => loading(async () => { if (!gameId.value || !G.value) return; try { const data = await gameApi.save(gameId.value); gameId.value = data.id; gameGroupId.value = data.save_group_id; G.value = { ...data.state, sect_name: data.sect_name || G.value.sect_name }; await refreshSaves() } catch (e) { report('存档失败', e) } })
const removeSave = async (id: string) => { if (!confirm('确定删除此存档？')) return; await loading(async () => { try { await gameApi.remove(id); if (gameId.value === id) resetGame(); await refreshSaves() } catch (e) { report('删除失败', e) } }) }
const removeGroup = async (id: string) => { if (!confirm('确定删除此槽位及其全部存档？此举不可撤回。')) return; await loading(async () => { try { await gameApi.removeGroup(id); if (gameGroupId.value === id) resetGame(); await refreshSaves() } catch (e) { report('删除槽位失败', e) } }) }
const decide = (id: string) => loading(async () => { if (!gameId.value || !G.value || usedDecisions.value.includes(id)) return; try { const name = G.value.sect_name; const data = await gameApi.decide(gameId.value, id); G.value = { ...data.state, sect_name: data.state.sect_name || name }; usedDecisions.value.push(id) } catch (e) { report('决策失败', e) } })
const manage = (command: ManagementRequest) => loading(async () => { if (!gameId.value || !G.value) return; try { const name = G.value.sect_name; const data = await gameApi.manage(gameId.value, command); G.value = { ...data.state, sect_name: data.state.sect_name || name } } catch (e) { report('掌门令未能施行', e) } })
const advance = () => loading(async () => { if (!gameId.value || !G.value) return; try { const name = G.value.sect_name; const before = `${G.value.year}-${G.value.month}`; const data = await gameApi.advance(gameId.value); gameId.value = data.id; G.value = { ...data.state, sect_name: data.state.sect_name || name }; if (`${G.value.year}-${G.value.month}` !== before) { usedDecisions.value = []; await refreshSaves() } ui.popupEvents = data.events || []; ui.tournament = data.tournament || null; ui.popup = !!(data.events?.length || data.tournament || G.value.pending_event) } catch (e) { report('推演月令未成', e) } })
const resolveEvent = (optionId: string) => loading(async () => { if (!gameId.value || !G.value) return; ui.resolvingEvent = true; try { const name = G.value.sect_name; const data = await gameApi.resolveEvent(gameId.value, optionId); gameId.value = data.id; G.value = { ...data.state, sect_name: data.state.sect_name || name }; usedDecisions.value = []; await refreshSaves(); ui.popupEvents = data.events || []; ui.tournament = data.tournament || null; ui.popup = true } catch (e) { report('此策未能施行', e) } finally { ui.resolvingEvent = false } })
const closePopup = () => { if (G.value?.pending_event) return; ui.popup = false; ui.popupEvents = []; ui.tournament = null }
const restart = () => { if (confirm('确定重开山门？')) resetGame() }
const keyboard = (event: KeyboardEvent) => { if (event.key === 'Escape') { if (ui.popup && !G.value?.pending_event) closePopup(); else if (!G.value?.pending_event) ui.savePanel = false } else if (ui.popup && !G.value?.pending_event && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); closePopup() } }
onMounted(() => { loadStaticData(); refreshSaves(); document.addEventListener('keydown', keyboard) })
onBeforeUnmount(() => document.removeEventListener('keydown', keyboard))
</script>

<template>
  <ScrollContainer>
    <StartScreen v-if="!G" :can-continue="!!saveGroups.length" @start="start" @continue="continueGame" @load="openSaves" />
    <GameOverScreen v-else-if="G.game_over" :game="G" @restart="resetGame" @saves="openSaves" />
    <GameView v-else :game="G" :decisions="DECISIONS" :arts="MARTIAL_ARTS" :used="usedDecisions" @save="manualSave" @load="openSaves" @restart="restart" @decide="decide" @manage="manage" @advance="advance" />
  </ScrollContainer>
  <SavePanel :open="ui.savePanel" :groups="saveGroups" :current-id="gameId" :current-group-id="gameGroupId" @close="ui.savePanel = false" @load="loadGame" @remove-save="removeSave" @remove-group="removeGroup" />
  <EventPopup :open="ui.popup" :events="ui.popupEvents" :tournament="ui.tournament" :pending="G?.pending_event" :choosing="ui.resolvingEvent" @choose="resolveEvent" @close="closePopup" />
  <SettingsPanel :open="ui.settingsOpen" @close="ui.settingsOpen = false" />
  <LoadingOverlay :show="ui.loading" />
</template>
