<script setup lang="ts">import type { GameState } from '../types'; defineProps<{ game: GameState }>()</script>
<template><div class="stats-grid">
  <div class="stat-card"><div class="label">江湖声望</div><div class="value" :class="game.prestige >= 70 ? 'good' : game.prestige <= 20 ? 'warn' : ''">{{ game.prestige }}</div></div>
  <div class="stat-card"><div class="label">库银</div><div class="value" :class="game.silver >= 200 ? 'gold' : game.silver <= 30 ? 'warn' : ''">{{ game.silver }}两</div></div>
  <div class="stat-card"><div class="label">门人志气</div><div class="value" :class="game.morale >= 70 ? 'good' : game.morale <= 20 ? 'warn' : ''">{{ game.morale }}</div></div>
  <div class="stat-card"><div class="label">掌门伤势</div><div class="value" :class="game.injury >= 50 ? 'warn' : game.injury <= 10 ? 'good' : ''">{{ game.injury }}</div></div>
</div></template>

