<script setup>
import { useShellReady } from '../core/shellContext.js'

// Reusable status banner for shell boot/errors.
const { ready, backend, error } = useShellReady()
</script>

<template>
  <section v-if="error" class="shell-banner">
    <strong>Shell error:</strong>
    <span>{{ error?.message || error }}</span>
  </section>
  <section v-else-if="!ready" class="shell-banner shell-banner--loading">
    <strong>Shell booting:</strong>
    <span>initializing {{ backend }}</span>
  </section>
</template>

<style scoped>
.shell-banner {
  display: grid;
  gap: 6px;
  padding: 12px 16px;
  margin-bottom: 20px;
  border-radius: 12px;
  border: 1px solid rgba(255, 120, 120, 0.4);
  background: rgba(255, 120, 120, 0.12);
  color: #ffd6d6;
  font-size: 14px;
}

.shell-banner--loading {
  border-color: rgba(255, 200, 120, 0.4);
  background: rgba(255, 200, 120, 0.12);
  color: #ffe7c7;
}
</style>
