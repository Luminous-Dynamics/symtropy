# RRVR Temporal Closure Reference v0.1

This isolated crate evaluates a supplied temporal-closure envelope. It does not operate ROS, DDS, Gazebo, a controller, a driver, a PLC, firmware, or a physical endpoint.

## Run

```sh
cargo fmt --manifest-path research/mobility-rrvr-temporal-closure/Cargo.toml -- --check
cargo test --locked --manifest-path research/mobility-rrvr-temporal-closure/Cargo.toml
cargo clippy --locked --manifest-path research/mobility-rrvr-temporal-closure/Cargo.toml --all-targets -- -D warnings
```

## Decision semantics

The reference checker has two proof paths:

- **Fenced-and-drained:** every declared holder rejects the old epoch; accepted and retired high-water marks share the declared queue generation and match; no holder reports outstanding work; every forwarding edge has no in-flight message and proves no later old-epoch enqueue; terminal endpoints enforce the fence at actuation.
- **Bounded expiry:** every holder rejects the old epoch and enforces receiver-side expiry; each node has a finite retention bound; each edge has a finite transit bound and preserves the original command age; a conservative path horizon includes the aggregate clock-uncertainty and scheduling bounds; the horizon has elapsed after ingress fencing.

A forwarding graph must be a complete, reachable DAG in this version. Cycles are held until a separate cycle-termination or non-renewable age-budget proof is modeled. Duplicate endpoint-pair edges are rejected rather than collapsed during path analysis.

A Hold result is not a fault diagnosis. A Closed result is not physical NoEffect, a STOP, a RecoveryPermit, or permission to dispatch. The caller must compose the result with the existing physical-effect observer and native authority owners.

## Known-answer cases

The v0.1 fixture corpus contains:
- a fenced-and-drained positive control;
- a bounded-expiry positive control with a command in flight;
- negative cases for unbounded late arrival, forwarder age refresh, invalid clock profile, and incomplete command-path inventory.

The JSON Schema defines fixture structure. The GitHub Actions workflow parses the schema/fixture JSON and checks the frozen fixture invariants, then runs rustfmt, unit tests and strict Clippy. This is not a physical campaign or proof of inventory completeness.

## Bounded formal model

The TLA+ abstraction is in `model/RRVRTemporalClosure.tla`. It includes two command branches, in-flight delivery, monotonic elapsed age, endpoint expiry, a drained-certificate path, a bounded-expiry path, and hidden post-fence ingress as an unsafe mutation.

The safe configuration is `model/RRVRTemporalClosure_Safe.cfg`. The negative configurations target:
- quiet-interval-only closure;
- late physical effect after bounded expiry;
- forwarding that refreshes message age;
- a hidden command path omitted from the declared inventory;
- an unbounded transit/retention path;
- an invalid clock profile.

The focused workflow pins the TLC JAR and Git blob identities for the model/configurations. It requires the safe model to exhaust and each unsafe configuration to produce its expected invariant counterexample. A queued workflow is not a formal PASS; report the exact head and completed run before making that claim.

## Scope of the model

The TLA+ model uses a small finite abstraction with a precomputed aggregate age horizon. It does not yet refine per-edge/per-node timing values, clock-domain conversions, a real ROS/DDS graph, or the Rust data types. The Rust checker and TLA+ model are separate reference implementations at this stage; a Rust/TLA refinement map is a future acceptance item. No transport, firmware, HIL or physical behavior is established.

