// authOps.js: declarative auth helpers using only shell verbs.
import { ref } from 'vue'
import { useShell } from './shellContext.js'
import { requestMagicLink, verifyMagicLink, parseMagicLinkHash, clearMagicLinkHash } from './magicLink.js'

export function useAuthOps() {
  const { get, put } = useShell()

  const status = ref({ locked: false, initialized: false })
  const email = ref('')
  const pendingLink = ref('')
  const lastPin = ref('')
  const error = ref('')

  async function refreshStatus() {
    try {
      const result = await get('/system/auth/status')
      status.value = result?.data ?? result ?? status.value
    } catch (err) {
      error.value = String(err)
    }
  }

  async function initAccount(pin) {
    await put('/system/account/init', { pin })
  }

  async function unlock(pin) {
    return put('/system/auth/unlock', { pin })
  }

  async function lock() {
    await put('/system/auth/lock', {})
    await refreshStatus()
  }

  async function requestLink() {
    error.value = ''
    if (!email.value) {
      error.value = 'email required'
      return
    }
    const result = await requestMagicLink(email.value)
    if (result?.error) {
      error.value = result.error
      return
    }
    const link = result?.link || ''
    pendingLink.value = link
    if (link) {
      console.info('magic link:', link)
    }
    return link
  }

  async function verifyLink(payload) {
    error.value = ''
    const result = await verifyMagicLink(payload)
    if (!result.ok) {
      error.value = result.error
      return
    }
    lastPin.value = result.pin
    if (!status.value.initialized) {
      await initAccount(result.pin)
    }
    await unlock(result.pin)
    await refreshStatus()
    if (status.value.locked) {
      // In dev, allow magic-link to reset the local account.
      await initAccount(result.pin)
      await unlock(result.pin)
      await refreshStatus()
    }
    if (status.value.locked) {
      error.value = 'unlock failed; check email or reset account'
    }
  }

  async function verifyFromHash() {
    const payload = parseMagicLinkHash()
    if (!payload) return
    if (!email.value) {
      email.value = payload.email
    }
    await verifyLink(payload)
    clearMagicLinkHash()
  }

  return {
    status,
    email,
    pendingLink,
    lastPin,
    error,
    refreshStatus,
    requestLink,
    verifyLink,
    verifyFromHash,
    lock,
  }
}
