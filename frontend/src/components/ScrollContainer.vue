<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

const scale = ref(1)
const updateScale = () => {
  if (window.innerWidth < 750) { scale.value = 1; return }
  const body = getComputedStyle(document.body)
  const availableWidth = window.innerWidth - parseFloat(body.paddingLeft) - parseFloat(body.paddingRight)
  const availableHeight = window.innerHeight - parseFloat(body.paddingTop) - parseFloat(body.paddingBottom)
  scale.value = Math.max(1, Math.min(availableWidth / 1048, availableHeight / 808))
}
onMounted(() => { updateScale(); window.addEventListener('resize', updateScale) })
onBeforeUnmount(() => window.removeEventListener('resize', updateScale))
</script>

<template><main class="scroll-container" :style="{ transform: scale > 1 ? `scale(${scale})` : undefined }"><slot /></main></template>

