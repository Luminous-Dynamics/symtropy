# Fusion equilibrium interoperability contract v0.1

An isolated, dependency-free Rust research crate for validating declared Grad–Shafranov equilibrium field semantics before solver outputs or benchmark records are consumed by Symthaea or Symtropy.

## Scope

The crate validates and normalizes only explicit metadata and array structure:

- source profile, producer revision, geometry and artifact digest identities;
- static-forward, static-inverse, evolving-forward and dataset-reconstruction task classes;
- explicit R/Z units, time units, flux units and grid layout;
- right-handed cylindrical (R, φ, Z) coordinates and the declared poloidal-field definition;
- explicit COCOS flags and quantity sign metadata;
- monotonic axes and timestamps, frame counts, array dimensions and finite values;
- explicit normalization receipts and stale-identity checks.

Canonical numerical representations are metres, seconds and Wb/rad. The only v0.1 unit conversions are declared millimetres → metres, milliseconds → seconds, and Wb → Wb/rad by division by 2π. Wb/rad is passed through unchanged.

## Why the convention gate is strict

FreeGSNKE 3.1.1 documents an internal Wb/rad poloidal flux representation, right-handed cylindrical coordinates, and a COCOS-7-like convention with:

- exp_Bp = 0
- sigma_Bp = -1
- sigma_RpZ = +1
- sigma_rhotp = +1

Its documentation also warns that the low-level G-EQDSK COCOS helper does not implement every sign transformation for arbitrary COCOS conventions. The crate therefore does not guess transformations from a numeric COCOS label. It requires an exact expected convention fingerprint and preserves the source signs. A mismatch is rejected rather than silently repaired.

Reference: https://docs.freegsnke.com/

The sign values used in the unit-test fixture are synthetic declared metadata; they are not claimed as universal FreeGSNKE defaults.

## Fusion Equilibrium Challenge boundary

The public challenge dataset documents a 65×65 (R,Z) grid in metres and nested per-shot time series. Its public data description does not establish every downstream solver's flux convention. The adapter must not infer missing flux units, signs or orientation from array shape. Shot identity and shot-level split rules remain upstream benchmark responsibilities.

References:
- https://fusion-equilibrium-challenge.sophelio.io/data/
- https://fusion-equilibrium-challenge.sophelio.io/rules/
- https://fusion-equilibrium-challenge.sophelio.io/results/

## Run locally

From this directory:

```sh
cargo test --manifest-path Cargo.toml
cargo run --manifest-path Cargo.toml --bin verify_fixture_corpus
```

The binary checks only fixture-corpus structure and required case identities. It does not call the production normalizer. Unit tests exercise the typed validator and normalization behavior.

## Claim ceiling

A passing unit test or corpus audit establishes only the software behavior covered by that test and exact source revision. It does not establish that source bytes were fetched or hashed by this crate, that FreeGSNKE is numerically accurate, that EFIT is physical truth, or that any equilibrium is experimentally validated.

No solver is launched. No model is trained. No physical measurement is created. No plasma control, hardware authorization or reactor qualification is possible from this crate.
