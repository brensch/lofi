# Mesh Lab

The product-facing browser application for developing and demonstrating Lofi
Mesh. It is a Vite + React + TypeScript app, but React never renders audio or
simulates device behavior.

## Boundaries

- `src/components`: product UI and controls.
- `src/hooks/useMeshAudio.ts`: typed browser audio lifecycle and command bridge.
- `src/types/mesh.ts`: main-thread/worklet message contract.
- `src/audio/mesh-worklet.js`: realtime multi-instance host, packet substrate,
  and listener-side mix.
- `crates/lofi-web`: raw `no_std` ABI instantiated once per virtual device.
- `crates/lofi-app`: actual device behavior shared with firmware.

Each virtual module owns a distinct WASM instance and communicates only by
encoded mesh frames copied through the worklet's fixed packet pool. The React
main thread receives telemetry at 30 Hz and cannot block the audio callback.

## Commands

From the repository root:

```sh
npm install
npm run dev
npm run check:web
npm run build:web
```

`predev` and `prebuild` compile `lofi-web` for `wasm32-unknown-unknown` and copy
the generated module into Vite's ignored `public/` staging directory.
