<script setup lang="ts">
defineProps<{ december?: boolean; nextYear?: number; pending?: boolean; exhausted?: boolean }>()
defineEmits<{ advance: [] }>()
</script>
<template>
  <button class="btn advance-button" :class="{ exhausted }" :disabled="pending" @click="$emit('advance')">
    {{ pending ? '先决眼前江湖事' : december ? `赴年终论剑，进入${nextYear}年` : '封卷落印，推演下月' }}
  </button>
</template>