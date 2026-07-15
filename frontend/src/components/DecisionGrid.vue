<script setup lang="ts">
import type { Decision, GameState, MartialArt } from '../types'
const props = defineProps<{ decisions: Decision[]; arts: MartialArt[]; game: GameState; used: string[] }>()
const emit = defineEmits<{ decide: [id: string] }>()
const available = (d: Decision) => !props.used.includes(d.id) && !(d.costType === 'silver' && props.game.silver < d.cost) && !(d.id === 'train' && props.game.injury >= 30) && !(d.id === 'study' && props.game.martial_arts_learned?.length >= props.arts.length) && !(d.id === 'rest' && props.game.injury <= 5)
const choose = (d: Decision) => { if (available(d)) emit('decide', d.id) }
</script>
<template><section class="panel"><div class="panel-title">本月决策（尚余 {{ game.max_decisions - used.length }} 次）</div><div v-if="game.max_decisions - used.length > 0" class="decision-grid"><button v-for="d in decisions" :key="d.id" class="decision-card transition-all duration-150" :class="{ used: used.includes(d.id), unavailable: !available(d) && !used.includes(d.id) }" :disabled="!available(d)" @click="choose(d)"><div class="card-title">{{ d.title }}</div><div class="card-cost">{{ d.costType === 'silver' ? `花费银${d.cost}两` : '不耗银两' }}</div><div class="card-desc">{{ d.desc }}</div></button></div></section></template>

