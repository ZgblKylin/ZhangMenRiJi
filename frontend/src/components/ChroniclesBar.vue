<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import type { ChronicleEvent } from '../types'

const props = defineProps<{ sectEntries: ChronicleEvent[]; worldEntries: ChronicleEvent[] }>()
const sectScroll = ref<HTMLElement>()
const worldScroll = ref<HTMLElement>()
watch(() => [props.sectEntries, props.worldEntries], () => nextTick(() => {
  if (sectScroll.value) sectScroll.value.scrollTop = 0
  if (worldScroll.value) worldScroll.value.scrollTop = 0
}), { deep: true })
</script>

<template>
  <aside class="chronicles-column">
    <section class="chronicles-bar">
      <span class="chronicles-title">◆ 本门纪事</span>
      <div ref="sectScroll" class="chronicles-scroll">
        <span v-if="!sectEntries.length" class="chr-empty">山门初立，尚无纪事。</span>
        <template v-else>
          <span v-for="(event, i) in sectEntries.slice(-24).reverse()" :key="i" class="chr-entry">
            <span v-if="event.year" class="chr-date">第{{ event.year }}年{{ event.month }}月 </span>
            <span :class="`chr-${event.mood}`">{{ event.text }}</span>
          </span>
        </template>
      </div>
    </section>
    <section class="chronicles-bar world-chronicles">
      <span class="chronicles-title">◆ 江湖纪事</span>
      <div ref="worldScroll" class="chronicles-scroll">
        <span v-if="!worldEntries.length" class="chr-empty">江湖风静，尚无外闻。</span>
        <template v-else>
          <span v-for="(event, i) in worldEntries.slice(-24).reverse()" :key="i" class="chr-entry">
            <span v-if="event.year" class="chr-date">第{{ event.year }}年{{ event.month }}月 </span>
            <span :class="`chr-${event.mood}`">{{ event.text }}</span>
          </span>
        </template>
      </div>
    </section>
  </aside>
</template>
