# Embedded Sample Bank

The starter bank contains six acoustic drum one-shots from
[Sample Pi](https://github.com/alex-esc/sample-pi), which declares its samples
public domain under CC0 1.0. They were taken from upstream commit
`f14596be37abf227bf77d198c9b0b60d9eb4dca9`.

The committed `.ulaw` files are mono, headerless G.711 mu-law at 22.05 kHz.
They are decoded directly from read-only flash by `lofi-core`; no source audio,
decoder state, heap allocation, or filesystem is needed at runtime.

| Output | Upstream path | Source SHA-256 |
| --- | --- | --- |
| `kick-hard.ulaw` | `drums/one-shots/kick/drum_bass_hard.flac` | `ed3ac2187d679ca1c3fec533ea1c3576c840c81ba98b8d967eb12a8f27471bc1` |
| `kick-soft.ulaw` | `drums/one-shots/kick/drum_bass_soft.flac` | `87bfb846ea7adf747515b1c59602555c2dde5da40ba4cb5a19244df06385ff23` |
| `snare-hard.ulaw` | `drums/one-shots/snare/drum_snare_hard.flac` | `1b2325523ed2a93da49df338ef861b4c35bfcbdc2e5e726ce1e873201883ffe4` |
| `snare-soft.ulaw` | `drums/one-shots/snare/drum_snare_soft.flac` | `3da004f932da94c4a4ccf1066f973c08ebe314875304333c1b257dbd6725ad9e` |
| `hat-closed.ulaw` | `drums/one-shots/cymbal/drum_cymbal_closed.flac` | `f3b9d6bb14f75ba06ef633baf58b1f75f0e2ea5e07edd491f0a22aedb2480d62` |
| `hat-pedal.ulaw` | `drums/one-shots/cymbal/drum_cymbal_pedal.flac` | `af2cf5e259f3671d9b363418b9d222b742cf6e41dd3e6871b23f2b4ce8b36635` |

Rebuild one file with:

```sh
cargo run -p lofi-sample-packer -- source.flac assets/samples/encoded/name.ulaw
```

## License

Sample Pi states: "All samples in this directory ... have been placed in the
public domain via the Creative Commons 0 License." See its
[README and source credits](https://github.com/alex-esc/sample-pi/blob/f14596be37abf227bf77d198c9b0b60d9eb4dca9/README.md)
and the [CC0 1.0 legal code](https://creativecommons.org/publicdomain/zero/1.0/legalcode).
