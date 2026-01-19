<script setup>
const props = defineProps({
  verbs: {
    type: Array,
    default: () => ['get', 'put', 'all', 'on', 'close'],
  },
})
</script>

<template>
  <div class="verb-palette">
    <span class="verb-palette__label">verbs</span>
    <div class="verb-palette__list">
      <span v-for="verb in props.verbs" :key="verb" class="verb-chip">{{ verb }}</span>
    </div>
  </div>
</template>

<style scoped>
.verb-palette {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  font-size: 12px;
  opacity: 0.8;
}

.verb-palette__label {
  text-transform: uppercase;
  letter-spacing: 0.18em;
}

.verb-palette__list {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.verb-chip {
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: rgba(255, 255, 255, 0.06);
}
</style>
