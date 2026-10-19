# Subterranean mobility benchmark v0.1

A standard-library-only Rust reference kernel for fail-closed passage admissibility. It supports mine, cave, tunnel, and unknown-void research scenarios but makes no claim of physical route safety, equipment qualification, or rescue certification.

## Run

From a Rust toolchain compatible with edition 2024:

~~~sh
cargo test --offline --manifest-path crates/tools/subterranean-mobility-benchmark/Cargo.toml
cargo run --offline --manifest-path crates/tools/subterranean-mobility-benchmark/Cargo.toml --bin subterranean-mobility-benchmark
~~~

## Contract

- Evaluates a declared person/PPE/load/aid profile against supplied passage geometry and evidence.
- Uses integer units and conservative uncertainty bounds for clear width, height, turning radius, grade, steps, and traction.
- Requires freshness-bearing hazard evidence; a known blocking hazard rejects the passage.
- Supports direction-specific grade and step limits; reverse travel requires an explicit bidirectional declaration.
- Qualified support evidence is required only when the traversal profile demands it for a vertical transition. Software cannot certify an anchor or rigging system.
- Returns one of three outcomes: admissible within the declared envelope, rejected by a known hard constraint, or unknown because required evidence is missing/stale.
- Preserves reason codes. A known hard violation remains a rejection even if another required observation is missing.
- Searches only edges passing all hard checks. Unknown edges do not become safe because they are part of the shortest path.
- Uses stable lexical ordering and an explicit route-search expansion/hop cap. Truncated search reports SearchLimitExceeded, never a definitive no-route claim.

## Limits

This is an evidence filter over supplied data, not a perception system or physical simulation. It does not assess rock stability, geotechnical support, atmosphere, medical suitability, real rope/anchor strength, current water dynamics, human biomechanics, device snagging, braking in a real environment, or rescue-team competence. A status of admissible means only that the provided fresh measurements satisfy the provided profile. It must not be used as an operational authorization.

See SUBTERRANEAN-HUMAN-MOBILITY-001 in the docs/research directory, implementation task #2039, and FLOOD-EGRESS-001 #1963.
