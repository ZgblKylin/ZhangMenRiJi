<script setup lang="ts">
import type { GameState } from '../types'
defineProps<{ game: GameState }>()
defineEmits<{ restart: []; saves: [] }>()
</script>
<template><section class="start-screen fade-in p-8">
  <h1 class="!text-4xl !text-cinnabar">山 门 落 幕</h1>
  <div class="my-6 text-lg leading-8 text-ink-light">{{ game.sect_name }}<br>第{{ game.year }}年{{ game.month }}月<br><br>{{ game.game_over_reason }}</div>
  <div class="mb-4 text-ink-fade">声望：{{ game.prestige }} · 库银：{{ game.silver }}两 · 弟子：{{ game.disciples.filter(d => d.alive).length }}人</div>
  <button class="btn btn-primary px-8 py-2.5 text-lg" @click="$emit('restart')">重 头 再 来</button>
  <div class="mt-4"><button class="btn btn-sm" @click="$emit('saves')">查看存档 / 回档</button></div>
</section></template>

