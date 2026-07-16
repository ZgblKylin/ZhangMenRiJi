<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'

const scale = ref(1)
const updateScale = () => {
  if (window.innerWidth < 750) { scale.value = 1; return }
  const body = getComputedStyle(document.body)
  const availableWidth = window.innerWidth - parseFloat(body.paddingLeft) - parseFloat(body.paddingRight)
  const availableHeight = window.innerHeight - parseFloat(body.paddingTop) - parseFloat(body.paddingBottom)
  // 卷轴本体为 1510×800，外圈纸边阴影还各占约 12px。桌面窗口只在
  // 放不下时缩小，避免 1600×900 的默认窗口反被旧基准放大而溢出。
  scale.value = Math.min(1, availableWidth / 1534, availableHeight / 824)
}
onMounted(() => { updateScale(); window.addEventListener('resize', updateScale) })
onBeforeUnmount(() => window.removeEventListener('resize', updateScale))
</script>

<template><main class="scroll-container" :style="{ transform: Math.abs(scale - 1) > .001 ? `scale(${scale})` : undefined }"><slot /></main></template>
