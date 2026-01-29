// shellService.js: singleton service with verb-level helpers.
import { autoBootShell } from './shell.js'

// Boot once at import time (app-wide singleton).
const shellPromise = autoBootShell({ storage: 'indexeddb', dbName: 'beenode-web-lab' })

// Explicit init hook (used by app bootstrap).
export function initShell() {
  return shellPromise
}

// Resolve backend label for UI status.
export async function getBackend() {
  const shell = await shellPromise
  return shell.backend
}

// Internal helper to apply an operation to the singleton shell.
async function withShell(fn) {
  const shell = await shellPromise
  return fn(shell)
}

// get: read a scroll
export function get(path) {
  return withShell((shell) => shell.get(path))
}

// put: write a scroll
export function put(path, data) {
  return withShell((shell) => shell.put(path, data))
}

// all: list paths
export function all(prefix) {
  return withShell((shell) => shell.all(prefix))
}

// on: watch changes
export async function on(pattern, callback) {
  const shell = await shellPromise
  return shell.on(pattern, callback)
}

// close: release resources
export function close() {
  return withShell((shell) => shell.close())
}
