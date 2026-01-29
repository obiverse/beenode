// shell.js: minimal 9S shell wrapper around BeeNode.
import { createNode, initWasm } from './wasm.js'

const DEFAULT_DB_NAME = 'beenode-web-lab'
let autoBootPromise

// Shell exposes only the five verbs.
export class Shell {
  constructor(node, backend) {
    this.node = node
    this.backend = backend
  }

  // get: read a scroll
  async get(path) {
    return this.node.read(path)
  }

  // put: write a scroll
  async put(path, data) {
    return this.node.write(path, data)
  }

  // all: list paths
  async all(prefix) {
    return this.node.list(prefix)
  }

  // on: watch changes
  on(pattern, callback) {
    return this.node.watch(pattern, callback)
  }

  // close: release resources
  async close() {
    return this.node.close()
  }
}

// Boot a shell for a specific storage backend.
export async function bootShell({ storage = 'indexeddb', dbName = DEFAULT_DB_NAME } = {}) {
  const { node, backend } = await createNode({ storage, dbName })
  return new Shell(node, backend)
}

// Auto-boot once; returns a shared shell promise.
export function autoBootShell(options = {}) {
  if (!autoBootPromise) {
    autoBootPromise = bootShell(options)
  }
  return autoBootPromise
}

// Re-export wasm init for explicit control when needed.
export { initWasm }
