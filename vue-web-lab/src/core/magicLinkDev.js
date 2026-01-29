// magicLinkDev.js: in-browser dev stub for magic-link auth.
// In production, this lives on the server.

const STORAGE_KEY = 'beenode.magiclink.dev.v1'

function loadState() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    return raw ? JSON.parse(raw) : {}
  } catch {
    return {}
  }
}

function saveState(state) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state))
}

function toBase64Url(bytes) {
  let binary = ''
  bytes.forEach((b) => {
    binary += String.fromCharCode(b)
  })
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '')
}

function fromBase64Url(value) {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(value.length / 4) * 4, '=')
  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}

function randomBytes(length) {
  const buf = new Uint8Array(length)
  crypto.getRandomValues(buf)
  return buf
}

async function sha256(bytes) {
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return new Uint8Array(digest)
}

function concatBytes(a, b) {
  const out = new Uint8Array(a.length + b.length)
  out.set(a)
  out.set(b, a.length)
  return out
}

async function deriveToken(secret, nonce) {
  const input = concatBytes(secret, nonce)
  const hash = await sha256(input)
  return toBase64Url(hash)
}

async function derivePin(secret) {
  const input = concatBytes(secret, new TextEncoder().encode('pin'))
  const hash = await sha256(input)
  const view = new DataView(hash.buffer)
  const value = view.getUint32(0, false) % 1_000_000
  return String(value).padStart(6, '0')
}

function getOrCreateSecret(state, email) {
  if (!state[email]?.secret) {
    const secret = toBase64Url(randomBytes(32))
    state[email] = { secret, tokens: {} }
  }
  return fromBase64Url(state[email].secret)
}

export async function requestMagicLink(email) {
  const state = loadState()
  const secret = getOrCreateSecret(state, email)
  const nonce = randomBytes(16)
  const token = await deriveToken(secret, nonce)

  const nonceId = toBase64Url(nonce)
  state[email].tokens[nonceId] = { token, createdAt: Date.now() }
  saveState(state)

  const params = new URLSearchParams({ email, nonce: nonceId, token })
  const link = `${window.location.origin}${window.location.pathname}#/magic?${params.toString()}`
  return { link, token, nonce: nonceId }
}

export async function verifyMagicLink({ email, nonce, token }) {
  const state = loadState()
  const entry = state[email]
  if (!entry?.secret) {
    return { ok: false, error: 'unknown email' }
  }
  const expected = entry.tokens?.[nonce]?.token
  if (!expected || expected !== token) {
    return { ok: false, error: 'invalid token' }
  }
  const secret = fromBase64Url(entry.secret)
  const pin = await derivePin(secret)
  return { ok: true, pin }
}

export function parseMagicLinkHash() {
  if (!window.location.hash.startsWith('#/magic?')) return null
  const query = window.location.hash.slice('#/magic?'.length)
  const params = new URLSearchParams(query)
  const email = params.get('email') || ''
  const nonce = params.get('nonce') || ''
  const token = params.get('token') || ''
  if (!email || !nonce || !token) return null
  return { email, nonce, token }
}

export function clearMagicLinkHash() {
  window.location.hash = ''
}
