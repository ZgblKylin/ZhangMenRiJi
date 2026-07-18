<script setup lang="ts">
import type { Decision, GameState, MartialArt } from '../types'
const props = defineProps<{ decisions: Decision[]; arts: MartialArt[]; game: GameState; used: string[]; buildingEffectiveness?: number; title?: string }>()
const emit = defineEmits<{ decide: [id: string] }>()
const hasUnlearnedPlayerArt = () => props.arts.some(art => art.is_combat && art.sect_id === 'player' && !props.game.martial_arts_learned?.includes(art.id))
const availableDisciples = () => props.game.disciples.filter(disciple => disciple.alive && disciple.sect_id === props.game.sect.id && disciple.condition === 'healthy' && !disciple.away_months && !disciple.action)
const hasTeachingRelationship = () => availableDisciples().some(teacher => availableDisciples().some(student => teacher.id !== student.id && (student.master_id === teacher.id || (teacher.relations?.[student.id] || 0) >= 35 || (student.relations?.[teacher.id] || 0) >= 35)))
const available = (d: Decision) => props.game.decisions_used < props.game.max_decisions && !props.game.pending_event && !props.used.includes(d.id) && (props.buildingEffectiveness ?? 100) > 0 && !(d.costType === 'silver' && props.game.silver < d.cost) && !(d.id === 'train' && (props.game.injury >= 30 || availableDisciples().length < 1)) && !(d.id === 'teach' && !hasTeachingRelationship()) && !(d.id === 'mission' && availableDisciples().length < 1) && !(d.id === 'research' && !hasUnlearnedPlayerArt()) && !(d.id === 'rest' && props.game.injury <= 5)
const choose = (d: Decision) => { if (available(d)) emit('decide', d.id) }
</script>
<template><section class="panel"><div class="panel-title">{{ title || '本月议事' }}（尚余 {{ Math.max(0, game.max_decisions - game.decisions_used) }} 次）</div><div class="decision-grid"><button v-for="d in decisions" :key="d.id" class="decision-card transition-all duration-150" :class="{ used: used.includes(d.id), unavailable: !available(d) && !used.includes(d.id) }" :disabled="!available(d)" @click="choose(d)"><div class="card-title">{{ d.title }}</div><div class="card-cost">{{ d.costType === 'silver' ? `花费银${d.cost}两` : '不耗银两' }}</div><div class="card-desc">{{ d.desc }}</div></button></div></section></template>
