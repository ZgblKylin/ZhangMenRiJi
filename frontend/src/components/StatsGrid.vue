<script setup lang="ts">import { computed } from 'vue'; import type { GameState } from '../types'; const props = defineProps<{ game: GameState }>(); const remaining = computed(() => Math.max(0, props.game.max_decisions - props.game.decisions_used))</script>
<template><div class="stats-grid">
  <div class="stat-card"><div class="label">本月定夺</div><div class="value" :class="remaining === 0 ? 'warn' : 'good'">{{ remaining }}/{{ game.max_decisions }}</div></div>
  <div class="stat-card"><div class="label">江湖声望</div><div class="value good">{{ game.sect.attributes.prestige }}</div></div>
  <div class="stat-card"><div class="label">库银</div><div class="value gold">{{ game.sect.attributes.silver }}<small>两</small></div></div>
  <div class="stat-card"><div class="label">门人志气</div><div class="value" :class="game.sect.attributes.morale < 25 ? 'warn' : ''">{{ game.sect.attributes.morale }}</div></div>
  <div class="stat-card"><div class="label">门风道义</div><div class="value" :class="game.sect.attributes.morality > 65 ? 'good' : ''">{{ game.sect.attributes.morality }}</div></div>
  <div class="stat-card"><div class="label">在山门人</div><div class="value">{{ game.disciples.filter(d => d.alive && !d.away_months).length }}<small>人</small></div></div>
  <div class="stat-card"><div class="label">掌门伤势</div><div class="value" :class="game.injury >= 30 ? 'warn' : 'good'">{{ game.injury }}</div></div>
</div></template>