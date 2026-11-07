# CivDyn Causal Path v0.1

A small causal-path conformance checker built on the existing `civdyn-resource-ledger`. It is not a second simulator, accounting engine or empirical causal estimator.

## Reference experiment

The synthetic task uses a reservoir and one household in a one-tick resource-ledger world. The primary intervention is whether a transfer-authorization safeguard is enabled. The control, treatment and sham negative-control arms share the same exogenous snapshot. A fourth mechanism-ablation arm keeps the safeguard enabled but blocks the transfer-eligibility mediator.

The required path is:

`safeguard enabled → transfer eligibility → authorized water transfer → delivered water → household water coverage`

The household coverage value and served-water amount are read from the actual `World::step` report. The CSV includes a deterministic row key constructed from arm, tick and region to identify that ledger-derived record in the artifact; this is not a ledger-issued signature or external digest. The resource ledger independently checks water conservation.

## Conformance gates

- Acyclic declared graph and a contiguous intervention-to-outcome path.
- Control and treatment differ in exactly one declared initial intervention variable.
- Control, treatment and negative-control arms carry the same exogenous snapshot identifier.
- The mechanism-ablation arm differs from treatment only in its declared block switch and preserves the primary safeguard assignment.
- The ablated mediator must return to its control value; the independent outcome is then checked to see whether the treatment effect disappears. A surviving effect is evidence of an alternative route, not a forced pass.
- Every transition receipt reconciles against the run's initial/final state snapshots.
- Each declared path edge must change its source input between control and treatment as well as change its target; an outcome movement caused by an unmodelled alternate route cannot complete the declared path.
- The outcome variable agrees with an independent resource-ledger receipt and its unit.
- A sham negative-control counter changes while every physical/outcome state and the ledger result remain unchanged.
- Mediator necessity is reported separately from path conformance: a complete path does not by itself prove that the path is necessary.
- The paired effect is withheld (`qualified_treatment_effect = null`) unless all gates pass.

All four arms must measure the same resource, region and tick. The exogenous snapshot identifier is an asserted protocol key, not a cryptographically verified digest. This v0.1 verifies internal model conformance; it does not prove that all possible confounders were measured or establish a causal effect in the real world.

## Run

```sh
cargo fmt --manifest-path research/civdyn-causal-path/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-causal-path/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-causal-path/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-causal-path/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-causal-path/Cargo.toml --bin causal-path-sweep
```


## Paired-run statistical controls

The `paired_effects` module evaluates at least 30 matched control/treatment runs by unique seed. It reports the paired mean difference, sample standard deviation and a conservative two-sided 95% Student-t interval in the outcome's native unit. The v0.1 critical values are conservative lookup buckets, not a general-purpose statistical library.

- A positive control passes only when the complete confidence interval is in the declared positive direction.
- A negative/equivalence control passes only when the *entire* interval lies inside the predeclared equivalence margin. Failure to reject zero is not an equivalence pass.
- An outcome that moves without a declared graph path is reported as movement without a declared path, not credited to the asserted mechanism.
- Matched observational rows remain descriptive-only; the confirmatory gate requires the caller to declare a common-random-number simulation design.

The pairing design and seed correspondence are protocol assertions supplied to this crate; it does not independently prove that PRNG state or all exogenous inputs match. The paired t interval assumes the paired differences and sampling design make that summary appropriate. The minimum of 30 is a v0 gate, not a universal sample-size guarantee.

## Time-lagged dependency graphs

The `temporal_graph` module requires the zero-lag graph to be acyclic while permitting explicit delayed feedback loops. Each edge declares a mechanism ID and, when units change, a unit-transformation ID. This makes temporal feedback and dimensional changes visible rather than silently flattening them into a static DAG.

These modules strengthen computational conformance; they do not establish real-world causal identification, rule out unmeasured confounding or validate the model's causal assumptions.
