# symtropy-hdc-physics

A deterministic semantic shadow of `symtropy-physics`.

The exact physics world remains authoritative. This crate derives versioned
hypervectors for retrieval, anomaly detection, agent perception, and research.
Every emitted vector carries an encoder fingerprint and a deterministic digest
linking it to the exact source state.

The encoder supports world-space, center-of-dynamic-mass, and anchored reference
frames. Relative reference frames make translation-invariant episode retrieval
possible without modifying the simulation.

## Reference experiment

```bash
cargo run --release \
  --manifest-path domains/symtropy-hdc-physics/Cargo.toml \
  --example physics_episode_retrieval \
  > hdc-physics-results.csv
```

See [`docs/HDC_PHYSICS_RESEARCH_PROTOCOL.md`](docs/HDC_PHYSICS_RESEARCH_PROTOCOL.md)
for claim boundaries, baselines, split rules, and required provenance.
