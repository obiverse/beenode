<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { useShellReady } from './core/shellContext.js'
import { useShellOps } from './core/shellOps.js'
import { useAuthOps } from './core/authOps.js'
import { useIdentityOps } from './core/identityOps.js'
import ShellStatusBanner from './components/ShellStatusBanner.vue'
import VerbPalette from './components/VerbPalette.vue'

const { ready, backend } = useShellReady()
const { output, logs, clearLogs, get, put, all, on } = useShellOps()
const {
  status,
  email,
  pendingLink,
  lastPin,
  error: authError,
  refreshStatus,
  requestLink,
  verifyFromHash,
  lock,
} = useAuthOps()
const { identity, error: identityError, refresh: refreshIdentity } = useIdentityOps()

const path = ref('/notes/hello')
const payload = ref('{"_type":"note@v1","text":"hello from wasm"}')
const prefix = ref('/notes')
const watchPattern = ref('/**')

const isUnlocked = computed(() => ready.value && status.value.initialized && !status.value.locked)

function setViewFromAuth(unlocked) {
  if (window.location.hash.startsWith('#/magic?')) {
    return
  }
  window.location.hash = unlocked ? '#/dashboard' : '#/auth'
}

watch(
  ready,
  async (value) => {
    if (!value) return
    await refreshStatus()
    await verifyFromHash()
    setViewFromAuth(isUnlocked.value)
    if (isUnlocked.value) {
      await refreshIdentity()
    }
  },
  { immediate: true }
)

watch(isUnlocked, (value) => {
  setViewFromAuth(value)
  if (value) {
    refreshIdentity()
  }
})

const hashHandler = () => verifyFromHash()

onMounted(() => {
  window.addEventListener('hashchange', hashHandler)
})

onBeforeUnmount(() => {
  window.removeEventListener('hashchange', hashHandler)
})
</script>

<template>
  <main class="lab">
    <header class="lab__header">
      <div>
        <p class="lab__kicker">vue-web-lab</p>
        <h1>BeeNode Shell</h1>
        <p class="lab__subtitle">Singleton shell service, no configuration required.</p>
        <VerbPalette />
      </div>
      <div class="lab__status">
        <span :class="['chip', ready ? 'chip--ok' : 'chip--idle']">
          shell {{ ready ? backend : 'booting' }}
        </span>
      </div>
    </header>

    <ShellStatusBanner />

    <section v-if="!isUnlocked" class="panel">
      <h2>auth</h2>
      <div class="grid">
        <label>
          email
          <input v-model="email" placeholder="you@example.com" />
        </label>
        <div class="row">
          <button type="button" :disabled="!ready" @click="requestLink">magic link</button>
          <button type="button" :disabled="!ready" @click="verifyFromHash">verify</button>
        </div>
        <div class="meta">
          <div>dev: magic link prints to console</div>
          <div>initialized: {{ status.initialized }}</div>
          <div>locked: {{ status.locked }}</div>
          <div v-if="lastPin">pin (dev): {{ lastPin }}</div>
          <div v-if="pendingLink">link staged (console)</div>
          <div v-if="authError" class="error">{{ authError }}</div>
        </div>
      </div>
    </section>

    <section v-else class="panel">
      <div class="row row--space">
        <h2>dashboard</h2>
        <button type="button" :disabled="!ready" @click="lock">lock</button>
      </div>
      <p class="meta">authenticated shell session active.</p>
    </section>

    <section v-if="isUnlocked" class="panel">
      <div class="row row--space">
        <h2>identity</h2>
        <button type="button" :disabled="!ready" @click="refreshIdentity">refresh</button>
      </div>
      <div class="grid">
        <div v-if="identity" class="meta">
          <div>pubkey_hex</div>
          <pre class="output output--compact">{{ identity.pubkey_hex }}</pre>
          <div>npub</div>
          <pre class="output output--compact">{{ identity.npub }}</pre>
          <div>mobi</div>
          <pre class="output output--compact">{{ identity.mobi }}</pre>
        </div>
        <div v-else class="meta">identity not loaded.</div>
        <div v-if="identityError" class="error">{{ identityError }}</div>
      </div>
    </section>

    <section v-if="isUnlocked" class="panel">
      <h2>scrolls</h2>
      <div class="grid">
        <label>
          path
          <input v-model="path" placeholder="/notes/hello" />
        </label>
        <label>
          json
          <textarea v-model="payload" rows="5" />
        </label>
        <div class="row">
          <button type="button" :disabled="!ready" @click="put(path, payload)">put</button>
          <button type="button" :disabled="!ready" @click="get(path)">get</button>
        </div>
        <label>
          prefix
          <input v-model="prefix" placeholder="/notes" />
        </label>
        <button type="button" :disabled="!ready" @click="all(prefix)">all</button>
      </div>
      <pre class="output">{{ output }}</pre>
    </section>

    <section v-if="isUnlocked" class="panel">
      <h2>watch</h2>
      <div class="row">
        <input v-model="watchPattern" placeholder="/**" />
        <button type="button" :disabled="!ready" @click="on(watchPattern)">on</button>
      </div>
    </section>

    <section v-if="isUnlocked" class="panel">
      <div class="row">
        <h2>log</h2>
        <button type="button" @click="clearLogs">clear</button>
      </div>
      <div class="log">
        <div v-if="logs.length === 0" class="log__empty">no events yet.</div>
        <div v-for="entry in logs" :key="entry" class="log__entry">{{ entry }}</div>
      </div>
    </section>
  </main>
</template>

<style scoped>
:global(body) {
  margin: 0;
  font-family: 'Space Grotesk', system-ui, sans-serif;
  background: #0f0f14;
  color: #f5f5f7;
}

.lab {
  max-width: 960px;
  margin: 0 auto;
  padding: 32px 20px 80px;
}

.lab__header {
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin-bottom: 28px;
}

.lab__kicker {
  text-transform: uppercase;
  letter-spacing: 0.18em;
  font-size: 12px;
  opacity: 0.7;
  margin: 0;
}

.lab__subtitle {
  margin: 8px 0 6px;
  opacity: 0.7;
}

.lab__status {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
}

.panel {
  background: #161622;
  border: 1px solid #2b2b3a;
  border-radius: 16px;
  padding: 20px;
  margin-bottom: 20px;
  box-shadow: 0 12px 30px rgba(0, 0, 0, 0.25);
}

.grid {
  display: grid;
  gap: 12px;
}

.row {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
}

.row--space {
  justify-content: space-between;
}

label {
  display: grid;
  gap: 6px;
  font-size: 14px;
  opacity: 0.9;
}

input,
textarea {
  background: #0f0f14;
  border: 1px solid #2b2b3a;
  border-radius: 10px;
  padding: 10px 12px;
  color: #f5f5f7;
  font-size: 14px;
  width: 100%;
  box-sizing: border-box;
}

button {
  background: #ffcf5b;
  border: none;
  border-radius: 10px;
  padding: 10px 16px;
  font-weight: 600;
  cursor: pointer;
  color: #1a1a1a;
  text-transform: lowercase;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.output {
  background: #0f0f14;
  border-radius: 12px;
  padding: 12px;
  min-height: 80px;
  overflow-x: auto;
}

.output--compact {
  min-height: 0;
  margin: 6px 0 12px;
}

.log {
  border: 1px solid #2b2b3a;
  border-radius: 12px;
  padding: 12px;
  background: #0f0f14;
  min-height: 120px;
  max-height: 240px;
  overflow-y: auto;
}

.log__entry {
  font-size: 12px;
  padding: 4px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.log__entry:last-child {
  border-bottom: none;
}

.log__empty {
  font-size: 12px;
  opacity: 0.6;
  text-transform: lowercase;
}

.meta {
  display: grid;
  gap: 6px;
  font-size: 13px;
  opacity: 0.8;
}

.error {
  color: #ffb3b3;
}

.chip {
  padding: 6px 12px;
  border-radius: 999px;
  font-size: 12px;
  border: 1px solid #2b2b3a;
}

.chip--ok {
  background: rgba(120, 255, 175, 0.2);
  border-color: rgba(120, 255, 175, 0.4);
}

.chip--idle {
  background: rgba(255, 255, 255, 0.08);
}

@media (min-width: 768px) {
  .lab__header {
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
  }
}
</style>
