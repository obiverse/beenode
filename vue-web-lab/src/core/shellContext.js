// shellContext.js: app-wide provide/inject for the singleton shell.
import { inject, ref } from 'vue'
import { initShell, getBackend, get, put, all, on, close } from './shellService.js'

export const shellKey = Symbol('shell')

// Install the shell into Vue's dependency graph.
export function installShell(app) {
  const ready = ref(false)
  const backend = ref('booting')
  const error = ref(null)

  // Provide a tiny, stable API for components.
  const api = { ready, backend, error, get, put, all, on, close }

  app.provide(shellKey, api)

  // Boot once and update reactive status.
  initShell()
    .then(getBackend)
    .then((name) => {
      backend.value = name
      ready.value = true
    })
    .catch((err) => {
      error.value = err
    })

  return api
}

// Access full shell API (verbs + status).
export function useShell() {
  const api = inject(shellKey)
  if (!api) {
    throw new Error('Shell not provided')
  }
  return api
}

// Access status-only view (no verbs).
export function useShellReady() {
  // Narrow read-only view for status UI.
  const { ready, backend, error } = useShell()
  return { ready, backend, error }
}
