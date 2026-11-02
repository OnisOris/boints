#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release --target wasm32-unknown-unknown --bin algorithm-playground
mkdir -p dist
cp target/wasm32-unknown-unknown/release/algorithm-playground.wasm dist/
cp web/index.html web/mq_js_bundle.js web/LOADER-LICENSE web/LOADER.md dist/
printf '' > dist/.nojekyll
gzip -9 -n -c dist/algorithm-playground.wasm > dist/algorithm-playground.wasm.gz
wc -c dist/algorithm-playground.wasm dist/algorithm-playground.wasm.gz
