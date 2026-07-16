<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'

const props = withDefaults(defineProps<{
  open: boolean
  title?: string
  message: string
  confirmLabel?: string
  cancelLabel?: string
  danger?: boolean
}>(), {
  title: '掌门定夺',
  confirmLabel: '确认施行',
  cancelLabel: '容后再议',
  danger: false,
})

const emit = defineEmits<{ confirm: []; cancel: [] }>()
const cancelButton = ref<HTMLButtonElement>()

watch(() => props.open, open => {
  if (open) nextTick(() => cancelButton.value?.focus())
})
</script>

<template>
  <Transition name="fade">
    <div
      v-if="open"
      class="modal-overlay confirm-overlay z-[200]"
      role="presentation"
      @click.self="emit('cancel')"
      @keydown.esc.stop.prevent="emit('cancel')"
    >
      <section class="confirm-dialog" role="alertdialog" aria-modal="true" aria-labelledby="confirm-dialog-title" aria-describedby="confirm-dialog-message">
        <div class="confirm-seal" aria-hidden="true">令</div>
        <h2 id="confirm-dialog-title">{{ title }}</h2>
        <div class="confirm-divider" aria-hidden="true"><span>◆</span></div>
        <p id="confirm-dialog-message">{{ message }}</p>
        <div class="confirm-actions">
          <button ref="cancelButton" class="btn" @click="emit('cancel')">{{ cancelLabel }}</button>
          <button class="btn" :class="danger ? 'btn-primary' : 'btn-jade'" @click="emit('confirm')">{{ confirmLabel }}</button>
        </div>
      </section>
    </div>
  </Transition>
</template>
