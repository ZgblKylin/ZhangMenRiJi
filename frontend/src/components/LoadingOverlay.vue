<script setup lang="ts">defineProps<{ show: boolean }>()</script><template><Transition name="fade"><div v-if="show" class="modal-overlay z-[200] bg-ink/40"><div class="loading-spinner">⏳ 静待片刻...</div></div></Transition></template>

