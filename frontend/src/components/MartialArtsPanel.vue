<script setup lang="ts">import type { MartialArt } from '../types'; defineProps<{ arts: MartialArt[]; learned: string[] }>()</script>
<template><section class="panel"><div class="panel-title">本派武学（{{ learned?.length || 0 }}/{{ arts.length }}）</div><div class="art-tags"><span v-for="art in arts" :key="art.id" class="art-tag" :class="{ unlearned: !learned?.includes(art.id) }">{{ art.name }}<span class="text-[.6rem] text-ink-fade">({{ art.type }})</span><span v-if="!learned?.includes(art.id)" class="unlearned-label"> 未习</span></span></div></section></template>

