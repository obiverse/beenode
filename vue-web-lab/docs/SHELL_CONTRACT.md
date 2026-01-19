# Shell Contract Fixtures

These fixtures define the minimal, portable behavior expected from any BeeNode shell implementation (native or wasm).

## Goals

- Keep the shell as the only public interface.
- Guarantee parity between native and wasm implementations.
- Provide a simple, shared test corpus for verification.

## Files

- `contracts/shell-contract.json` — machine-readable fixtures

## Shape rules

- `get` returns a scroll or `null`.
- `put` returns the written scroll.
- `all` returns string paths.
- `on` emits events on writes; unsubscribe is optional.

## Running against wasm

The Vue lab can load these fixtures to validate behavior against the wasm shell.

## Running against native

A small harness can read the same fixtures and run them against `Node` for parity.
