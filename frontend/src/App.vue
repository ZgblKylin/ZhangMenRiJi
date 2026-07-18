<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive } from 'vue'
import { gameApi, isMissingGameError, isStaleRevisionError } from './api'
import {
  adoptGameResponse,
  adoptGameRevision,
  clearTransientGameUi,
  DECISIONS,
  G,
  gameGroupId,
  gameId,
  gameRevision,
  MARTIAL_ARTS,
  preferredContinuationId,
  resetGame,
  saveGroups,
  ui,
  usedDecisions,
} from './store'
import ScrollContainer from './components/ScrollContainer.vue'
import StartScreen from './components/StartScreen.vue'
import GameOverScreen from './components/GameOverScreen.vue'
import GameView from './components/GameView.vue'
import SavePanel from './components/SavePanel.vue'
import EventPopup from './components/EventPopup.vue'
import LoadingOverlay from './components/LoadingOverlay.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import ChangelogPanel from './components/ChangelogPanel.vue'

import SettingsPanel from './components/SettingsPanel.vue'
import type { Disciple, ManagementRequest } from './types'

const report = (prefix: string, error: unknown) => alert(`${prefix}: ${error instanceof Error ? error.message : String(error)}`)
const loading = async (task: () => Promise<void>) => { ui.loading = true; try { await task() } finally { ui.loading = false } }
const confirmation = reactive({ open: false, title: '掌门定夺', message: '', confirmLabel: '确认施行', danger: false })
let settleConfirmation: ((accepted: boolean) => void) | null = null
const askConfirmation = (options: { title: string; message: string; confirmLabel: string; danger?: boolean }) => {
  settleConfirmation?.(false)
  Object.assign(confirmation, options, { danger: options.danger ?? false, open: true })
  return new Promise<boolean>(resolve => { settleConfirmation = resolve })
}
const settleConfirmDialog = (accepted: boolean) => {
  confirmation.open = false
  const settle = settleConfirmation
  settleConfirmation = null
  settle?.(accepted)
}

const loadStaticData = async () => {
  try { const data = await gameApi.staticData(); DECISIONS.value = data.decisions; MARTIAL_ARTS.value = data.arts }
  catch (error) { console.warn('加载静态数据失败:', error) }
}
const refreshSaves = async () => {
  try { const { groups = [] } = await gameApi.list(); saveGroups.value = groups }
  catch (error) { console.warn('获取存档列表失败:', error); saveGroups.value = [] }
}
const enterGame = (data: import('./types').GameResponse, preferredSectName = '') => {
  adoptGameResponse(data, preferredSectName)
  clearTransientGameUi()
}
const recoverStaleRevision = async (error: unknown) => {
  if (!isStaleRevisionError(error)) return false
  const current = error.body.current
  if (
    current?.id
    && current.save_group_id
    && current.state
    && Number.isSafeInteger(current.revision)
    && current.revision > 0
  ) {
    enterGame(current, G.value?.sect_name)
    await refreshSaves()
    alert('卷宗已在别处更新，现已载入最新进度；本次指令没有重复施行。')
    return true
  }
  resetGame()
  await refreshSaves()
  ui.savePanel = saveGroups.value.length > 0
  alert('存档版本已冲突，且服务端没有可接管的当前卷宗。已安全退出本局，请重新选择存档。')
  return true
}
const recoverMissingGame = async (error: unknown) => {
  if (!isMissingGameError(error) || !gameGroupId.value) return false
  const saveGroupId = gameGroupId.value
  await refreshSaves()
  const group = saveGroups.value.find(item => item.save_group_id === saveGroupId)
  const currentId = group?.current_id || group?.saves[0]?.id
  if (currentId) {
    try {
      enterGame(await gameApi.get(currentId), G.value?.sect_name)
      alert('当前卷宗已被裁剪或删除，现已载入同槽位的最新进度；本次指令没有重复施行。')
      return true
    } catch (loadError) {
      console.warn('接管槽位当前卷宗失败:', loadError)
    }
  }
  resetGame()
  ui.savePanel = saveGroups.value.length > 0
  alert('当前卷宗已不存在，且该槽位没有可接管的进度。已安全退出本局。')
  return true
}
const handleMutationError = async (prefix: string, error: unknown) => {
  if (!await recoverStaleRevision(error) && !await recoverMissingGame(error)) report(prefix, error)
}
const handleDeleteError = async (prefix: string, error: unknown, targetGroupId: string) => {
  if (isStaleRevisionError(error)) {
    const current = error.body.current
    if (current && gameGroupId.value === targetGroupId) enterGame(current, G.value?.sect_name)
    await refreshSaves()
    alert('卷宗已在别处更新，本次删除没有施行；存档列表已刷新。')
    return
  }
  if (isMissingGameError(error)) {
    if (gameGroupId.value === targetGroupId) await recoverMissingGame(error)
    else {
      await refreshSaves()
      alert('目标卷宗已不存在，存档列表已刷新。')
    }
    return
  }
  report(prefix, error)
}
const openSaves = async () => loading(async () => { await refreshSaves(); ui.savePanel = true })
const start = (name: string) => loading(async () => {
  try {
    enterGame(await gameApi.create(name), name)
    await refreshSaves()
  } catch (e) {
    report('创建游戏失败，请确认后端已启动', e)
  }
})
const loadGame = (id: string) => loading(async () => {
  try {
    enterGame(await gameApi.get(id))
    ui.savePanel = false
  } catch (e) {
    report('载入失败', e)
  }
})
const continueGame = () => loading(async () => {
  await refreshSaves()
  const id = preferredContinuationId(saveGroups.value)
  if (!id) return
  try {
    enterGame(await gameApi.get(id))
  } catch (e) {
    report('再入江湖失败', e)
  }
})
const manualSave = () => loading(async () => {
  if (!gameId.value || !G.value) return
  try {
    const name = G.value.sect_name
    adoptGameResponse(await gameApi.save(gameId.value, gameRevision.value), name)
    await refreshSaves()
  } catch (e) {
    await handleMutationError('存档失败', e)
  }
})
const removeSave = async (id: string) => {
  if (!await askConfirmation({ title: '焚毁存档', message: '确定删除此份存档？卷宗焚毁后无法复原。', confirmLabel: '焚毁存档', danger: true })) return
  const group = saveGroups.value.find(item => item.saves.some(save => save.id === id))
  await loading(async () => {
    try {
      const result = await gameApi.remove(id, group?.revision ?? null)
      if (gameGroupId.value === result.save_group_id) {
        if (gameId.value === id) {
          if (result.current) enterGame(result.current, G.value?.sect_name)
          else resetGame()
        } else if (result.current) {
          adoptGameRevision(result.current.revision)
        }
      }
      await refreshSaves()
    } catch (error) {
      await handleDeleteError('删除失败', error, group?.save_group_id || '')
    }
  })
}
const removeGroup = async (id: string) => {
  if (!await askConfirmation({ title: '撤去槽位', message: '此举将删除该槽位及其中全部存档，且不可撤回。当真要撤去？', confirmLabel: '撤去槽位', danger: true })) return
  const group = saveGroups.value.find(item => item.save_group_id === id)
  await loading(async () => {
    try {
      await gameApi.removeGroup(id, group?.revision ?? null)
      if (gameGroupId.value === id) resetGame()
      await refreshSaves()
    } catch (error) {
      await handleDeleteError('删除槽位失败', error, id)
    }
  })
}
const decide = (id: string) => loading(async () => {
  if (!gameId.value || !G.value || usedDecisions.value.includes(id)) return
  try {
    const name = G.value.sect_name
    adoptGameResponse(await gameApi.decide(gameId.value, id, gameRevision.value), name)
    usedDecisions.value.push(id)
  } catch (e) {
    await handleMutationError('决策失败', e)
  }
})
const manage = (command: ManagementRequest) => loading(async () => {
  if (!gameId.value || !G.value) return
  try {
    const name = G.value.sect_name
    adoptGameResponse(await gameApi.manage(gameId.value, command, gameRevision.value), name)
  } catch (e) {
    await handleMutationError('掌门令未能施行', e)
  }
})
const setPopupEvents = (data: { events?: import('./types').ChronicleEvent[]; sect_events?: import('./types').ChronicleEvent[]; world_events?: import('./types').ChronicleEvent[] }) => {
  const all = data.events || []
  ui.popupSectEvents = data.sect_events || all.filter(event => event.category !== 'world')
  ui.popupWorldEvents = data.world_events || all.filter(event => event.category === 'world')
}
const advance = () => loading(async () => {
  if (!gameId.value || !G.value) return
  try {
    const name = G.value.sect_name
    const before = `${G.value.year}-${G.value.month}`
    const data = await gameApi.advance(gameId.value, gameRevision.value)
    adoptGameResponse(data, name)
    if (`${G.value?.year}-${G.value?.month}` !== before) {
      usedDecisions.value = []
      await refreshSaves()
    }
    setPopupEvents(data)
    ui.tournament = data.tournament || null
    ui.popup = !!(ui.popupSectEvents.length || ui.popupWorldEvents.length || data.tournament || G.value?.pending_event)
  } catch (e) {
    await handleMutationError('推演月令未成', e)
  }
})
const resolveEvent = (optionId: string) => loading(async () => {
  if (!gameId.value || !G.value) return
  ui.resolvingEvent = true
  try {
    const name = G.value.sect_name
    const data = await gameApi.resolveEvent(gameId.value, optionId, gameRevision.value)
    adoptGameResponse(data, name)
    usedDecisions.value = []
    await refreshSaves()
    setPopupEvents(data)
    ui.tournament = data.tournament || null
    ui.popup = true
  } catch (e) {
    await handleMutationError('此策未能施行', e)
  } finally {
    ui.resolvingEvent = false
  }
})
const closePopup = () => { if (G.value?.pending_event) return; ui.popup = false; ui.popupSectEvents = []; ui.popupWorldEvents = []; ui.tournament = null }
const restart = async () => { if (await askConfirmation({ title: '重开山门', message: '现有推演将就此搁下，确定返回山门初立之时？', confirmLabel: '重开山门', danger: true })) resetGame() }
const expel = async (disciple: Disciple) => { if (await askConfirmation({ title: '逐出门墙', message: `当真要将${disciple.name}逐出山门？此令一出，再难挽回。`, confirmLabel: '逐出山门', danger: true })) await manage({ action: 'expel', disciple_id: disciple.id }) }
const keyboard = (event: KeyboardEvent) => { if (event.key === 'Escape') { if (confirmation.open) settleConfirmDialog(false); else if (ui.changelogOpen) ui.changelogOpen = false; else if (ui.popup && !G.value?.pending_event) closePopup(); else if (!G.value?.pending_event) ui.savePanel = false } else if (!confirmation.open && ui.popup && !G.value?.pending_event && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); closePopup() } }
onMounted(() => { loadStaticData(); refreshSaves(); document.addEventListener('keydown', keyboard) })
onBeforeUnmount(() => document.removeEventListener('keydown', keyboard))
</script>

<template>
  <ScrollContainer>
    <StartScreen v-if="!G" :can-continue="!!saveGroups.length" @start="start" @continue="continueGame" @load="openSaves" />
    <GameOverScreen v-else-if="G.game_over" :game="G" @restart="resetGame" @saves="openSaves" />
    <GameView v-else :game="G" :decisions="DECISIONS" :arts="MARTIAL_ARTS" :used="usedDecisions" @save="manualSave" @load="openSaves" @restart="restart" @decide="decide" @manage="manage" @expel="expel" @advance="advance" />
  </ScrollContainer>
  <SavePanel :open="ui.savePanel" :groups="saveGroups" :current-id="gameId" :current-group-id="gameGroupId" @close="ui.savePanel = false" @load="loadGame" @remove-save="removeSave" @remove-group="removeGroup" />
  <EventPopup :open="ui.popup" :sect-events="ui.popupSectEvents" :world-events="ui.popupWorldEvents" :tournament="ui.tournament" :pending="G?.pending_event" :choosing="ui.resolvingEvent" @choose="resolveEvent" @close="closePopup" />
  <SettingsPanel :open="ui.settingsOpen" @close="ui.settingsOpen = false" />
  <ChangelogPanel :open="ui.changelogOpen" @close="ui.changelogOpen = false" />
  <ConfirmDialog v-bind="confirmation" @confirm="settleConfirmDialog(true)" @cancel="settleConfirmDialog(false)" />
  <LoadingOverlay :show="ui.loading" />
</template>
