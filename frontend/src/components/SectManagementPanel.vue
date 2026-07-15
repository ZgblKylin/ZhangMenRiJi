<script setup lang="ts">
import type { GameState, ManagementRequest, MartialArt, SectPolicy } from '../types'
import MartialArtsPanel from './MartialArtsPanel.vue'

const props = defineProps<{ game: GameState; arts: MartialArt[]; view: 'sect' | 'library' | 'world' }>()
const emit = defineEmits<{ manage: [command: ManagementRequest] }>()
const policyNames: Array<[SectPolicy, string, string]> = [
  ['balanced', '持中守成', '诸务均衡'], ['martial', '崇武精进', '偏重练武'], ['scholarly', '研经明理', '偏重研读'],
  ['chivalrous', '行侠尚义', '积攒道义'], ['mercantile', '通商裕库', '增益进项'], ['reclusive', '闭门清修', '加快调息'],
]
const orders = [
  ['diligent', '勤修令', '三月内门人更喜练武', 60], ['righteous', '尚义令', '四月内涵养门风', 80],
  ['frugal', '节用令', '四月内节用裕库', 50], ['rest', '调息令', '两月内静养更佳', 45],
]
const artName = (id: string) => props.arts.find(art => art.id === id)?.name || id
const countryName = (id: string) => props.game.countries.find(country => country.id === id)?.name || id
const foundation = (sectId: string) => `${sectId}_foundation`
const command = (payload: ManagementRequest) => emit('manage', payload)
</script>

<template>
  <section v-if="view === 'sect'" class="management-sheet">
    <div class="panel-title">掌门方略</div>
    <div class="policy-grid">
      <button v-for="[id, name, note] in policyNames" :key="id" class="policy-card" :class="{ active: game.sect.policy === id }"
        :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'set_policy', policy: id })">
        <b>{{ name }}</b><small>{{ note }}</small>
      </button>
    </div>
    <div class="management-row-title">山门营造</div>
    <div class="building-grid">
      <div v-for="building in game.sect.buildings" :key="building.id" class="building-card">
        <div><b>{{ building.name }}</b><span>第{{ building.level }}重</span></div>
        <div class="condition-bar"><i :style="{ width: `${building.condition}%` }"></i></div>
        <small v-if="building.upgrading_months">尚需 {{ building.upgrading_months }} 月</small>
        <small v-else>完好 {{ building.condition }}%</small>
        <div class="building-actions">
          <button class="btn btn-sm" :disabled="!!building.upgrading_months || game.decisions_used >= game.max_decisions" @click="command({ action: 'upgrade_building', building_id: building.id })">扩建</button>
          <button class="btn btn-sm" :disabled="building.condition >= 100 || game.decisions_used >= game.max_decisions" @click="command({ action: 'repair_building', building_id: building.id })">修葺</button>
        </div>
      </div>
    </div>
    <div class="management-row-title">掌门令</div>
    <div class="order-grid">
      <button v-for="[id, name, desc, cost] in orders" :key="id" class="order-card" :disabled="game.decisions_used >= game.max_decisions"
        @click="command({ action: 'issue_order', order_id: id })"><b>{{ name }}</b><span>{{ desc }}</span><small>库银 {{ cost }} 两</small></button>
    </div>
    <div v-if="game.sect.active_orders.length" class="active-orders">施行中：<span v-for="order in game.sect.active_orders" :key="order.id">{{ order.name }}（{{ order.remaining_months }}月）</span></div>
    <div class="inventory-strip"><b>五库簿：</b><span v-for="(count, name) in game.sect.inventory" :key="name">{{ name }} {{ count }}</span>
      <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'recruit' })">张榜纳徒 · 50两</button></div>
  </section>

  <section v-else-if="view === 'library'" class="management-sheet">
    <div class="panel-title">藏经阁录</div>
    <div class="library-summary">藏经阁第 {{ game.sect.buildings.find(b => b.id === 'scripture')?.level || 0 }} 重，共收公册 {{ game.sect.public_books.length }} 部。</div>
    <div class="manual-list">
      <article v-for="id in game.sect.public_books" :key="id" class="manual-card">
        <div><b>{{ artName(id) }}</b><span>{{ arts.find(art => art.id === id)?.type || '武学' }}</span></div>
        <small>门派参研 {{ game.sect.martial_research[id] || 0 }}</small>
        <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'research_martial', martial_art_id: id })">合参 · 40两</button>
      </article>
    </div>
    <button class="btn btn-primary research-new" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'research_new_martial' })">集众研创新武学 · 库银120两</button>
    <MartialArtsPanel :arts="arts.filter(art => art.sect_id === 'player')" :learned="game.martial_arts_learned" />
  </section>

  <section v-else class="management-sheet">
    <div class="panel-title">江湖门派谱</div>
    <div class="world-summary">四国并立，天下 {{ game.npc_sects.length }} 派各行其道。先通问修好，交情足时方可请教典籍。</div>
    <div class="world-grid">
      <article v-for="npc in game.npc_sects" :key="npc.id" class="world-sect-card">
        <div class="world-sect-head"><b>{{ npc.name }}</b><span>{{ countryName(npc.country_id) }}</span></div>
        <div>声望 {{ npc.attributes.prestige }} · 道德 {{ npc.attributes.morality }} · 交情 {{ game.sect.relations[npc.id] || 0 }}</div>
        <small>镇派：{{ artName(npc.public_books.at(-1) || '') }}</small>
        <div>
          <button class="btn btn-sm" :disabled="game.decisions_used >= game.max_decisions" @click="command({ action: 'exchange', sect_id: npc.id })">通问 · 45两</button>
          <button class="btn btn-sm" :disabled="(game.sect.relations[npc.id] || 0) < 10 || game.decisions_used >= game.max_decisions" @click="command({ action: 'request_manual', sect_id: npc.id, martial_art_id: foundation(npc.id) })">请教入门册</button>
        </div>
      </article>
    </div>
  </section>
</template>
