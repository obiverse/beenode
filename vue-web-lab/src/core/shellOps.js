// shellOps.js: declarative helpers that keep 9S verb names intact.
import { ref } from 'vue'
import { useShell } from './shellContext.js'

export function useShellOps() {
  const { get, put, all, on } = useShell()
  const output = ref('')
  const logs = ref([])

  // Timestamped log stream for UI feedback.
  function log(message) {
    logs.value.unshift(`[${new Date().toISOString()}] ${message}`)
  }

  function clearLogs() {
    logs.value = []
  }

  // put: parse JSON text and write the scroll.
  async function putVerb(path, jsonText) {
    let data
    try {
      data = JSON.parse(jsonText)
    } catch (error) {
      log(`put failed: invalid JSON - ${error.message}`)
      return
    }
    try {
      output.value = JSON.stringify(await put(path, data), null, 2)
      log(`put ${path}`)
    } catch (error) {
      log(`put failed: ${error}`)
    }
  }

  // get: read a scroll and render it as JSON.
  async function getVerb(path) {
    try {
      output.value = JSON.stringify(await get(path), null, 2)
      log(`get ${path}`)
    } catch (error) {
      log(`get failed: ${error}`)
    }
  }

  // all: list paths and render the array as JSON.
  async function allVerb(prefix) {
    try {
      output.value = JSON.stringify(await all(prefix), null, 2)
      log(`all ${prefix}`)
    } catch (error) {
      log(`all failed: ${error}`)
    }
  }

  // on: subscribe to changes and log them.
  function onVerb(pattern) {
    try {
      on(pattern, (scroll) => {
        log(`on ${scroll.key}`)
      })
      log(`on ${pattern}`)
    } catch (error) {
      log(`on failed: ${error}`)
    }
  }

  return {
    output,
    logs,
    log,
    clearLogs,
    get: getVerb,
    put: putVerb,
    all: allVerb,
    on: onVerb,
  }
}
