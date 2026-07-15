<script setup lang="ts">
import type { Decision, GameState, MartialArt } from '../types'
const props = defineProps<{ decisions: Decision[]; arts: MartialArt[]; game: GameState; used: string[] }>()
const emit = defineEmits<{ decide: [id: string] }>()
const available = (d: Decision) => props.game.decisions_used < props.game.max_decisions && !props.game.pending_event && !props.used.includes(d.id) && !(d.costType === 'silver' && props.game.silver < d.cost) && !(d.id === 'train' && props.game.injury >= 30) && !(d.id === 'study' && props.game.martial_arts_learned?.length >= props.arts.filter(a => a.sect_id === 'player').length) && !(d.id === 'rest' && props.game.injury <= 5)
const choose = (d: Decision) => { if (available(d)) emit('decide', d.id) }
</script>
<template><section class="panel"><div class="panel-title">本月议事（尚余 {{ Math.max(0, game.max_decisions - game.decisions_used) }} 次）</div><div class="decision-grid"><button v-for="d in decisions" :key="d.id" class="decision-card transition-all duration-150" :class="{ used: used.includes(d.id), unavailable: !available(d) && !used.includes(d.id) }" :disabled="!available(d)" @click="choose(d)"><div class="card-title">{{ d.title }}</div><div class="card-cost">{{ d.costType === 'silver' ? `花费银${d.cost}两` : '不耗银两' }}</div><div class="card-desc">{{ d.desc }}</div></button></div></section></template>
