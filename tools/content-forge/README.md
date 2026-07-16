# Content Forge

The content forge is an offline workstation pipeline that repeatedly:

1. chooses a constrained random lo-fi theme;
2. generates it through the local ACE-Step API;
3. separates six stems with Demucs;
4. slices loops and one-shots without producing runtime MIDI;
5. estimates root pitch only to tag repitchable audio;
6. rejects silence, weak pitch detections, duplicates, and pack overflow;
7. encodes accepted audio as 22.05 kHz G.711 mu-law;
8. writes one binary pack, a JSON manifest, and a listening contact sheet.

AI, Demucs, librosa, NumPy, and SoundFile are workstation dependencies. They are
not firmware or web-runtime dependencies.

```sh
~/.cache/lofi-tools/audio-analysis/.venv/bin/python \
  tools/content-forge/forge.py --start-server --count 4
```

Use `--forever` for an unattended generation loop. Each run is restartable and
records its prompt, random seed, key, mode, tempo, progression, and status under
`target/content-forge/runs/`. Rebuild a pack without generating more audio:

```sh
~/.cache/lofi-tools/audio-analysis/.venv/bin/python \
  tools/content-forge/forge.py --rebuild-only
```

The pack defaults to a 12 MiB ceiling. The manifest separates unrestricted
drums/textures from harmonically tagged loops and root-tagged one-shots. Pitched
loops may only be combined when key, mode, progression, and phrase phase agree.
One-shots can be repitched within a conservative range by the embedded sampler.

Generated output is ignored by Git. A commercial release still requires a
documented review of model/output rights and listening approval of every pack.
