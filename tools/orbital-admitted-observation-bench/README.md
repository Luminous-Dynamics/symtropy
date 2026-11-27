# Orbital admitted observation handoff benchmark v0.1

A synthetic Rust evaluator for recording every commanded acquisition attempt and validating its public realized outcome before that outcome may enter the predeclared #6887 sequential observation process.

## Core rule

**No attempt is silently dropped.** Valid measurement outcomes and failures of the observation process are different public data types.

The handoff has two update-permitting dispositions:
- `MEASUREMENT_ADMISSIBLE`: a source-bound measurement payload passed the required same-acquisition scope, action-realization, calibration, timing, motion, geometry, feature, metrology, photometry and diversity gates.
- `MODELLED_OBSERVATION_PROCESS_OUTCOME`: the acquisition was missing or physically unusable, but the exact public outcome class is declared in the frozen observation kernel and its failure owner agrees with the event.

Neither disposition calculates a likelihood, e-value, confidence sequence, surviving-world set or decision. It only permits the exact typed public outcome to be handed to #6887. Unmodelled outcomes abstain; malformed or contradictory events are invalid; known scope/calibration/motion/feature/measurability failures are not promoted to measurement.

## Frozen kernel controls

The predeclared kernel profile binds a source receipt digest, source state, revision, allowed public outcomes, missingness semantics, action-realization requirement and exact required-owner set. The attempt cannot turn off a required owner by marking it optional. The owner bindings must match the profile exactly. A stale/revoked kernel cannot admit an observation.

The profile itself is an opaque upstream contract reference. SHA-256 shape is not issuer authentication; full custody/authentication remains outside this adapter.

## Campaign continuity

A campaign declares the expected attempt count. The frozen campaign receipt set must contain exactly the contiguous sequence 1..N, with unique attempt IDs and sequence numbers. Missing attempts, gaps, duplicates and replayed IDs are rejected rather than quietly treating successful images as the entire history.

## Reproduce

```sh
cargo fmt --manifest-path tools/orbital-admitted-observation-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-admitted-observation-bench/Cargo.toml
cargo clippy --locked --manifest-path tools/orbital-admitted-observation-bench/Cargo.toml --all-targets -- -D warnings
cargo run --quiet --locked --manifest-path tools/orbital-admitted-observation-bench/Cargo.toml --bin orbital-admitted-observation-bench -- docs/research/orbital-truss/fixtures/admitted-observation-v0_1.json <40-hex-source-commit>
python3 tools/orbital-admitted-observation-bench/tests/reference_oracle.py docs/research/orbital-truss/fixtures/admitted-observation-v0_1.json <40-hex-source-commit>
```

The independent Python oracle uses only the standard library. It is test-only.

## Ownership and limits

- #7339/#5631 own calibration validity and clock mapping.
- #7384/#7399/#7422 own capture/motion/time bounds.
- #7426 owns exact acquisition scope coherence.
- #7043/#6846 own feature measurability and metrology.
- #7341/#4258/Mycelix #5762 own dependency/common-cause qualification.
- #6659/#6715 own capture completeness and HIL.
- #6887 owns the conditional observation kernel, e-process, sequential world set and stopping guarantee.
- #6835 owns exact decision projection; #7292 bridges the source-bound world-set receipt.

This crate does not estimate missingness probabilities, infer likelihoods, update an e-process, make engineering decisions, prove POD, authenticate external issuers/bytes, authorize hardware, or establish physical/flight qualification.
