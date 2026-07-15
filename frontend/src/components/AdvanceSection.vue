<script setup lang="ts">
defineProps<{ used: number; max: number; december?: boolean; nextYear?: number; pending?: boolean }>()
defineEmits<{ advance: [] }>()
</script>
<template>
  <div class="advance-section">
    <button class="btn btn-gold btn-lg" :disabled="pending" @click="$emit('advance')">
      {{ pending ? '先决眼前江湖事' : december ? `赴年终论剑，进入${nextYear}年` : '封卷落印，推演下月 →' }}
    </button>
    <div class="advance-hint">本月已定 {{ used }}/{{ max }} 事；掌门亦可留有余地，径自推进。</div>
  </div>
</template>
