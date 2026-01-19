# Shell API Contract (Web)

The webapp treats the BeeNode as a small computer. The only public interface is the Shell.

## Design goals

- Stable, minimal surface area.
- Same verbs across web, mobile, native.
- Internal details (BeeNode, storage) are hidden behind the Shell.

## Boot

```js
import { bootShell, autoBootShell, initWasm } from './core/shell.js'

await initWasm()
const shell = await bootShell({ storage: 'indexeddb', dbName: 'beenode-web-lab' })
// or
const shell = await autoBootShell()
```

`storage`: "memory" | "indexeddb" (default: indexeddb)

## Verbs (9S atoms)

All verbs return Promises unless noted.

### get(path)
Read a scroll by path.

- `path`: string like `/notes/hello`
- Returns: scroll JSON or `null`

### put(path, data)
Write JSON data to a path.

- `data`: any JSON-serializable object
- Returns: written scroll JSON

### all(prefix)
List paths under a prefix.

- `prefix`: string like `/notes`
- Returns: `string[]`

### on(pattern, callback)
Watch changes (fire-and-forget). Pattern matching depends on the store.

- `pattern`: string like `/**`
- `callback`: `(scroll) => void`
- Returns: subscription id (number)
- Note: unsubscribe is not implemented yet

### close()
Close storage and release resources.

## Scroll shape (current)

```json
{
  "key": "/notes/hello",
  "type": "note@v1",
  "metadata": {
    "version": 1
  },
  "data": {
    "_type": "note@v1",
    "text": "hello"
  }
}
```

## Error handling

- Errors are thrown (rejected Promises).
- The UI should catch and display the error message.

## Extension policy

- New features must add verbs or sub-paths under existing verbs.
- No UI should call wasm exports directly. Use the Shell only.

## System paths

- `/system/auth/*` — lock/unlock/status
- `/system/account/init` — initialize auth PIN (web)

## Dev magic link

The Vue lab includes an environment-aware magic-link client in `src/core/magicLink.js`.

- **dev**: local-only stub (`magicLinkDev.js`) prints a link to the console and derives a deterministic PIN.
- **prod**: calls `/auth/magic/request` and `/auth/magic/verify` on `VITE_MAGIC_LINK_BASE_URL`.

Both flows end by calling `/system/account/init` + `/system/auth/unlock`.
