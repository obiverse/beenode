// identityOps.js: read-only identity helpers.
import { ref } from 'vue'
import { useShell } from './shellContext.js'

export function useIdentityOps() {
  const { get } = useShell()
  const identity = ref(null)
  const error = ref('')

  async function refresh() {
    error.value = ''
    try {
      const result = await get('/system/identity')
      const resolved = result?.data ?? result ?? null
      identity.value = resolved
      if (!resolved) {
        error.value = 'identity unavailable (rebuild wasm or unlock session)'
      }
    } catch (err) {
      error.value = String(err)
    }
  }

  return { identity, error, refresh }
}
