# Macroquad WASM loader

`mq_js_bundle.js` is vendored with one compatibility fix from the published
`macroquad` 0.4.16 crate, `js/mq_js_bundle.js`.
Upstream: https://github.com/not-fl3/macroquad/tree/0.4.16/js
License: MIT (see LOADER-LICENSE; upstream also offers Apache-2.0).

Keep this file in sync with Cargo.lock when upgrading Macroquad/Miniquad.
It contains upstream optional plugin support; the application does not use
network, audio, or those plugins. No CDN or package manager is required.

Local patch: declare `var register_plugin` inside the optional quad-net IIFE.
The published bundle assigns an undeclared variable under strict mode, throwing
`ReferenceError: register_plugin is not defined` on script load. No plugin is
activated by this fix; it only makes the vendored bundle valid JavaScript.
