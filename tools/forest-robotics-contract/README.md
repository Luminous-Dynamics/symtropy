# Forestry Robotics Mission Contract

Standalone Rust preflight/evaluation contract for a synthetic forestry-robot mission corpus.

Run from the repository root:

    cargo fmt --manifest-path tools/forest-robotics-contract/Cargo.toml -- --check
    cargo test --manifest-path tools/forest-robotics-contract/Cargo.toml
    cargo run --quiet --manifest-path tools/forest-robotics-contract/Cargo.toml -- docs/tech/forest-robotics-corpus-v0.1.json --source-commit "$(git rev-parse HEAD)"

The decision eligible_for_simulation_only is explicitly not dispatch, motion, actuator or safety authorization. v0.1 limits tasks to non-contact mapping, surveying, tree measurement and log-deck inspection. Soil sampling, planting, load carrying, cutting/felling and chemical treatment are out of scope. A real robot requires separate hardware safety design, validated interlocks, physical E-stop, site risk assessment and jurisdiction-appropriate conformity work.

The corpus is synthetic; it does not control hardware or contain field/worker location data. Passing means the fixture matches the contract's expected mission decisions only. See docs/tech/FORESTRY_ROBOTICS_PLATFORM_V0_1.md.
