# Semantic scene-state digest contract

Status: frozen for the current inhabited-studio qualification lineage.

## Current host identity

`symthaea-bevy-brain::art_scene::stable_scene_hash` is a deterministic FNV-1a
64-bit protocol identity over canonical semantic scene records. It is explicitly
not a cryptographic digest.

Therefore any `TypedDigest` whose value is directly a `stable_scene_hash`
result MUST use:

- semantic domain supplied by the Symtropy reality binding (normally
  `symtropy.scene-state.v1`);
- `DigestAlgorithm::Other("fnv1a64")`;
- the 16-hex-character FNV-1a64 scene identity as its value.

It MUST NOT be labeled `DigestAlgorithm::Blake3` merely because the surrounding
Reality Ledger uses BLAKE3 for records, genesis manifests, observation-bundle
digests or checkpoints.

## Cryptographic boundary

The non-cryptographic scene identity answers a different question:

> Are these canonical semantic scene records the same state according to the
> frozen Symtropy scene-identity protocol?

Tamper evidence is supplied outside that protocol by cryptographic structures
such as:

- the BLAKE3 Reality Ledger record chain;
- BLAKE3 `WorldGenesisManifest` digests;
- BLAKE3 inhabited-episode observation-bundle digests; and
- externally attestable Reality Ledger checkpoints.

A later scene-state protocol may hash canonical scene bytes cryptographically,
but that must receive a new semantic domain/version rather than silently
changing the meaning of `symtropy.scene-state.v1`.

## Qualification gate

Before empirical execution, tests must prove:

1. direct Symtropy scene-state typed digests report `fnv1a64`;
2. equal scene-state values under `Blake3` and `fnv1a64` are not typed-equal;
3. four-ghost source/before/after materialization digests use the same truthful
   scene-state algorithm;
4. presence entry/exit and world genesis use the same scene-state contract;
5. cryptographic bundle and ledger digests remain BLAKE3 and are not downgraded
   to FNV-1a64.
