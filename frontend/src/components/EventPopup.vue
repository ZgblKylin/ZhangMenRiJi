<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import type { ChronicleEvent, PendingWorldEvent, Tournament } from '../types'
const props = defineProps<{ open: boolean; events: ChronicleEvent[]; tournament: Tournament | null; pending?: PendingWorldEvent | null; choosing?: boolean }>()
const emit = defineEmits<{ close: []; choose: [id: string] }>()
const button = ref<HTMLButtonElement>()
const rankName = (n: number) => ['零', '一', '二', '三', '四', '五', '六', '七', '八', '九', '十'][Math.min(n, 10)] || n
watch(() => props.open, open => { if (open) nextTick(() => button.value?.focus()) })
</script>

<template>
  <Transition name="fade">
    <div v-if="open" class="modal-overlay z-[150]">
      <div class="event-dialog" :class="{ 'choice-dialog': pending }">
        <template v-if="pending">
          <div class="event-category">{{ pending.category }}</div>
          <div class="event-title">{{ pending.title }}</div>
          <div class="event-prose">{{ pending.text }}</div>
          <div class="event-choices">
            <button v-for="choice in pending.choices" :key="choice.id" :disabled="choosing" @click="emit('choose', choice.id)">
              <b>{{ choice.label }}</b><span>{{ choice.result_text }}</span>
            </button>
          </div>
          <div class="choice-warning">此事不决，月中诸务暂且搁下。</div>
        </template>
        <template v-else>
          <template v-if="tournament">
            <div class="event-title">年 终 论 剑</div>
            <div class="my-2 text-center text-3xl text-gold">第 {{ rankName(tournament.rank) }} 名</div>
            <div class="mb-3 text-center text-sm text-ink-light">天下共 <strong>{{ tournament.total_sects }}</strong> 派与会 · 战力：{{ tournament.power }}<br>{{ tournament.desc_text }}</div>
          </template>
          <template v-if="events.length">
            <div class="event-title">本 月 纪 事</div>
            <div v-for="(event, i) in events" :key="i" class="event-item" :class="event.mood">{{ event.text }}</div>
          </template>
          <button ref="button" class="btn btn-primary event-dismiss" @click="emit('close')">阅毕收卷</button>
        </template>
      </div>
    </div>
  </Transition>
</template>
