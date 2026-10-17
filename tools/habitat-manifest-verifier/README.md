# Habitat manifest verifier

A standalone Rust structural verifier for the frozen whole-property and terrain-native habitat scenario manifests. This is intentionally a separate Cargo workspace so verification does not pull the Bevy application and its large dependency graph.

## Run

From the repository root:

```sh
cargo fmt --manifest-path tools/habitat-manifest-verifier/Cargo.toml -- --check
cargo test --locked --manifest-path tools/habitat-manifest-verifier/Cargo.toml
cargo run --locked --manifest-path tools/habitat-manifest-verifier/Cargo.toml -- whole-property docs/research/fixtures/WHOLE_PROPERTY_HABITAT_SCENARIOS_V0_1.json
cargo run --locked --manifest-path tools/habitat-manifest-verifier/Cargo.toml -- terrain-native docs/research/fixtures/TERRAIN_NATIVE_HABITAT_SCENARIOS_V0_1.json
```

The verifier pins each v0.1 manifest's SHA-256, uses strict Serde types with unknown-field rejection, checks the version and status, synthetic-only claim ceiling, fixed scenario census, canonical ordered IDs, duplicate IDs, allowed expected-disposition vocabulary, invariant completeness, and negative-control count. It emits the source SHA-256, scenario census, IDs and per-case structure summary.

## Claim ceiling

The report status is always `STRUCTURAL_PASS_NOT_SEMANTIC_PASS`. This tool checks corpus integrity and contract shape. It does **not** independently prove that authored scenario dispositions are semantically or scientifically correct; it does not run a building, geotechnical, hydraulic, structural, life-safety, ecological or other physical solver. It does not qualify a real site, establish buildability, or certify a building. The frozen fixture bytes and authored expectations remain synthetic.

Any intentional corpus revision requires a reviewed update to the pinned digest, ID census, and tests. Do not change a frozen corpus merely to make a verifier pass.
