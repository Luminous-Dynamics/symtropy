# Mobility Observer Assurance Contract v0.1

Dependency-free Rust reference contract for **fault-model-relative** observer independence and measurement-channel availability.

## Core rule

Observer independence is not inferred from different observer IDs. Each channel is linked to a source component and transitive dependency graph. Every component has a failure-domain label. Channels sharing any declared failure domain are not treated as independent for this contract.

For each declared fault scenario, the evaluator removes channels whose dependency closure intersects the failed components, validates the sample's calibration/scope/clock/freshness, then calculates the maximum set of surviving channels with pairwise-disjoint failure-domain footprints.

The contract returns `QuorumQualifiedForDeclaredFaultModel` only when every declared scenario preserves the required independent observer count.

## Run

```sh
cargo fmt --manifest-path research/mobility-observer-assurance/Cargo.toml -- --check
cargo test --manifest-path research/mobility-observer-assurance/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-observer-assurance/Cargo.toml --all-targets --locked -- -D warnings
```

## Non-claims

The crate does not authenticate profiles, samples or dependency graphs; prove the inventory complete; establish actual electrical/clock isolation; prove calibration accuracy; determine physical NoEffect; or authorize an action. Dependency-domain labels and scenario lists are declared inputs. A pass means only that the supplied model satisfies the contract for its declared scenarios.

A real qualification requires an exact sensor/DAQ/clock/power/firmware/configuration profile, traceable calibration, independent injected physical events, common-cause fault injection and a third-party evaluator.
