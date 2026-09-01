# CUF v0.6 Basin causal-state identity design

This note freezes the intended digest boundary before implementation.

`BasinWorld` state identity must include every stored component capable of changing future evolution:

- dimensions and tick;
- every `BasinCell` in canonical y-major/x-minor order;
- every `FieldGrid` channel in an explicit frozen `FieldLayer` order and canonical y-major/x-minor order;
- `MetabolicFlux`;
- `TrophicMemory`;
- `ViabilityProfile`;
- `SignalField` sequence, including `readable_by` ordering;
- `EcoCivicClaim` sequence, including evidence and opposition ordering.

The digest must not use Rust struct memory layout, enum discriminants, serde output, pointer values, or platform-sized integer encodings.

## Canonical scalar rules

- integers use explicit little-endian fixed-width encodings;
- `usize` dimensions are converted to `u64` after checked conversion;
- `f32` values use IEEE-754 bits with `-0.0` canonicalized to `+0.0` and all NaNs canonicalized to `0x7fc00000`;
- infinities retain their IEEE-754 encodings;
- enums use explicit hand-written stable codes;
- vectors are length-prefixed with `u64` and hashed in stored order.

## Domain

The resulting typed digest domain is `symtropy.basin.state.v1`, schema version `1`.

## Non-goals

This tranche does not ingest external environmental evidence and does not claim that `BasinMetrics` is authoritative state. It establishes the complete state identity required before any environmental ingest receipt can truthfully bind prior and resulting Basin states.
