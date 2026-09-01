# Basin Causal State Identity Contract v0.6

Status: canonical contract for the v0.6 stacked CUF tranche.

## Purpose

A Basin state digest is an identity for the complete stored Basin state that can influence future Basin evolution. It is not a digest of presentation metrics, a debug snapshot, a serde payload, or Rust object memory.

The initial implementation is a read-only downstream extension in `symtropy-world`. `symtropy-basin` remains the state authority and does not depend on world orchestration.

## Typed digest

- digest domain: `symtropy.basin.state.v1`
- digest schema version: `1`
- algorithm: SHA-256
- canonical stream prefix: `symtropy.basin.causal-state.v1\0`

## Included state

The canonical stream includes, in order:

1. schema version;
2. Basin width and height as fixed-width little-endian `u64`;
3. Basin tick as little-endian `u64`;
4. every `BasinCell`, traversed y-major then x-minor;
5. every `FieldGrid` value for every frozen field layer, each layer traversed y-major then x-minor;
6. `MetabolicFlux`;
7. `TrophicMemory`;
8. `ViabilityProfile`;
9. the stored `SignalField` sequence, including `readable_by` sequence;
10. the stored `EcoCivicClaim` sequence, including evidence and opposition sequences.

Section tags and sequence lengths are included so structurally different states cannot alias through concatenation ambiguity.

## Frozen FieldLayer order

The v1 order is:

1. FoodPheromone
2. HomePheromone
3. DangerPheromone
4. Moisture
5. Obstacle
6. Nutrient
7. Toxin
8. Biomass
9. Heat
10. Light
11. Oxygen
12. Disease
13. SignalNoise
14. NullContamination

A source-level change to `FieldLayer` that is not reflected in this order must fail compilation or qualification rather than silently changing digest meaning.

## Scalar encoding

Integers use explicit fixed-width little-endian encoding. Platform-sized lengths are checked before conversion to `u64`.

`f32` values use canonical IEEE-754 bits:

- `+0.0` and `-0.0` both encode as `0x00000000`;
- every NaN payload/sign encodes as canonical quiet NaN `0x7fc00000`;
- finite non-zero values use their exact IEEE-754 bit representation;
- positive and negative infinity retain their standard IEEE-754 encodings.

This rule avoids state-identity differences caused only by irrelevant zero sign or NaN payload variation while preserving exact finite simulation state.

## Enum encoding

Rust enum discriminants and declaration-layout assumptions are forbidden. Every enum participating in the digest has an explicit hand-written stable `u8` code in the v1 implementation.

Changing those codes changes digest semantics and requires a new digest schema/domain version.

## Sequence semantics

Stored vector order is identity-bearing in v1. This includes signals, signal readers, civic claims, claim evidence, and opposition lists.

The digest must not sort these collections unless the owning domain first changes their semantics to an explicitly unordered canonical set.

## Authority boundary

The digest implementation may read Basin state but may not mutate it. `symtropy-basin` remains authoritative for:

- Basin cells;
- living-system fields;
- metabolic flux;
- trophic memory;
- viability;
- signals;
- civic claims;
- tick progression and interventions.

The downstream identity layer does not infer, repair, sanitize, interpolate, or substitute missing Basin state.

## Non-equivalence to metrics

`BasinMetrics` is deliberately not sufficient state identity. Two Basin states may share the same visible/aggregate metrics while differing in spatial fields, trophic memory, signals, claims, or other future-relevant state.

Any receipt that claims to bind a Basin transformation must bind this complete causal-state digest (or a later explicitly versioned successor), not a metrics digest.

## Future use

The next environmental ingest contract may bind:

- exact-time `ObservationEvidence` inputs;
- prior Basin causal-state digest;
- versioned transformation-policy digest;
- resulting Basin causal-state digest;
- causal parents / event evidence.

That future receipt must remain evidence of a transformation performed by an owning domain; the common world layer must not become a second Basin authority.
