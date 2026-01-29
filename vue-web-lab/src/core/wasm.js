// wasm.js: single responsibility for wasm init + node factory.
import init, { BeeNode } from '@beenode-wasm/beenode.js'

// One-time init guard (module scope).
let wasmInitPromise

// Initialize the wasm runtime once.
export async function initWasm() {
  if (!wasmInitPromise) {
    wasmInitPromise = init()
  }
  await wasmInitPromise
}

// Create a BeeNode for the chosen storage backend.
export async function createNode({ storage = 'indexeddb', dbName = 'beenode-web-lab' } = {}) {
  await initWasm()
  if (storage === 'memory') {
    return { node: new BeeNode(), backend: 'memory' }
  }
  const node = await BeeNode.withIndexedDb(dbName)
  return { node, backend: `indexeddb:${dbName}` }
}
