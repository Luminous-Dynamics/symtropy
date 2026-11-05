# ROAD-EGCI-RECIPROCAL-HANDOFF-001 v0.1

A standalone Rust adapter for the Symtropy #3744 -> Symthaea SAM #10514 episode handoff.

## Purpose and boundary

This crate validates a finite reciprocal-episode receipt bundle and emits the exact four-field SAM trajectory-query shape:

- `target_policy_digest`
- `context_digest`
- `complete`
- `steps` containing `tick`, `history_digest`, and complete joint action vectors

It does **not** implement a simulator, reciprocal policy loop, trajectory-support estimator, propensity model, human behaviour model, vehicle safety controller, or physical validation. SAM #10514 remains the only owner of path-support assessment. This crate does not calculate `TrajectorySupported`, `UnsupportedSequence`, or any other SAM disposition.

The current #3532 reciprocal policy loop is still a specification. The fixture in this crate is a synthetic receipt conformance example, not an exported run from that simulator. The physics replay dependency #3539 is also an unverified draft; no physical or actor-model claim is implied.

## Adapter invariants

1. The target policy digest binds the complete actor-policy portfolio, configuration and initial policy-state identities.
2. The context digest binds scenario/ODD, initial world snapshot, physics/execution profile, observation schedule, action domain, units and tick rate.
3. Each per-policy permitted-history digest is reconstructed from the policy's initial/prior state, its observation-profile identity, delivered cue payload digests and prior own-action receipt IDs.
4. A cue must be generated and delivered before the decision that can see or consume it. The recipient must be the policy actor. Consumed cues must be a subset of visible cues.
5. Every SAM step is built from the complete joint action vector at that tick; marginal actor/action coverage is never substituted for joint-path coverage.
6. Alternative correspondence hypotheses remain separate SAM query envelopes. Their draw-bundle digests must match independently supplied anchor values.
7. Missing, failed, timed-out, censored or un-replayed execution remains incomplete. A task failure with a complete recorded path is still a complete path observation.
8. Every branch retains its own receipt root, replay profile and terminal disposition.
9. The evidence class is fixed to `SimulationGenerated` and eligibility to `FormalConformanceOnly`. This adapter cannot relabel synthetic outcomes as empirical decision evidence.

## External anchoring

BLAKE3 digests are unkeyed content commitments, not signatures. `ExternalAnchor` represents a separately controlled expected episode root and the expected draw-bundle digest for each branch/hypothesis. The adapter checks equality against those independently supplied values; it does not prove who controlled the anchor, authenticate source evidence, or independently rerun #3743's Philox verifier. A production cross-system deployment must obtain the anchor from an independently controlled evidence registry or signature-verification boundary.

## Run

```sh
cargo fmt --manifest-path research/road-egci-handoff-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/road-egci-handoff-001/Cargo.toml --all-targets
cargo clippy --locked --manifest-path research/road-egci-handoff-001/Cargo.toml --all-targets -- -D warnings
```

## Claim ceiling

A pass establishes exact adapter bookkeeping for the declared finite receipt and SAM-query fixtures. It does not prove that the reciprocal episode producer is correct, that candidate universes or correspondence hypotheses are scientifically exhaustive, that source evidence is authentic, that SAM support is positive, that the simulator is physically valid, or that a vehicle is safe or SAE L4/L5 capable.
