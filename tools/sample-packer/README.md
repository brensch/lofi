# Sample Packer

Offline converter for the embedded sample bank. It accepts FLAC, downmixes to
mono, resamples to 22.05 kHz, and writes headerless 8-bit G.711 mu-law.

```sh
cargo run -p lofi-sample-packer -- source.flac output.ulaw
```

This tool may allocate and use `std`; it never ships in firmware. The runtime
decoder in `lofi-core` is allocation-free and performs constant-time random
access, which keeps playback deterministic from mesh time.
