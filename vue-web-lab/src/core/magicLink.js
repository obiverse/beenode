// magicLink.js: environment-aware magic-link flow.
// - dev: local stub with console link (see magicLinkDev.js)
// - prod: HTTP endpoints returning a verification token

import {
  requestMagicLink as requestMagicLinkDev,
  verifyMagicLink as verifyMagicLinkDev,
  parseMagicLinkHash,
  clearMagicLinkHash,
} from './magicLinkDev.js'

function isProdMode() {
  const mode = import.meta.env.VITE_MAGIC_LINK_MODE
  if (mode) {
    return ['prod', 'production'].includes(mode)
  }
  return !import.meta.env.DEV
}

function getBaseUrl() {
  return import.meta.env.VITE_MAGIC_LINK_BASE_URL || ''
}

async function requestMagicLinkProd(email) {
  const baseUrl = getBaseUrl()
  const response = await fetch(`${baseUrl}/auth/magic/request`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      email,
      redirect: `${window.location.origin}${window.location.pathname}#/magic`,
    }),
  })

  if (!response.ok) {
    return { error: await response.text() }
  }

  return response.json()
}

async function verifyMagicLinkProd(payload) {
  const baseUrl = getBaseUrl()
  const response = await fetch(`${baseUrl}/auth/magic/verify`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })

  if (!response.ok) {
    return { ok: false, error: await response.text() }
  }

  return response.json()
}

export async function requestMagicLink(email) {
  if (isProdMode()) {
    return requestMagicLinkProd(email)
  }
  return requestMagicLinkDev(email)
}

export async function verifyMagicLink(payload) {
  if (isProdMode()) {
    return verifyMagicLinkProd(payload)
  }
  return verifyMagicLinkDev(payload)
}

export { parseMagicLinkHash, clearMagicLinkHash }
