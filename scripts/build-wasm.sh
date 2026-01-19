#!/usr/bin/env bash
set -euo pipefail

wasm-pack build --target web --no-default-features --features wasm
