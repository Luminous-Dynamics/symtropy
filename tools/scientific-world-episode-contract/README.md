# Scientific World Episode Contract

Dependency-free Rust conformance core for the Scientific World Episode Provider issue #5957.

## Run

- cargo fmt --manifest-path tools/scientific-world-episode-contract/Cargo.toml -- --check
- cargo test --locked --manifest-path tools/scientific-world-episode-contract/Cargo.toml
- cargo clippy --locked --manifest-path tools/scientific-world-episode-contract/Cargo.toml --all-targets -- -D warnings

## What is executable

- Public-packet cross-field checks: opaque ID shape, digest shape, unique channels/actions/observations, declared channel references, and initial observation visibility.
- Three-time semantics: event time, acquisition time, and policy availability time are distinct. Evidence that arrives after a decision is rejected from that decision's evidence prefix.
- Action-attempt ledger: unique IDs, contiguous ordinals, action catalogue binding, frozen costs, status chronology, complete supplied denominator, failed/rejected attempt accounting, and no retry after an unknown effect outcome.
- Counterfactual branch identity: both branches must share the same world-model and world-recipe, model profile, initial state, observation history, policy/recurrent/memory state, tool contract, clock/time origin, action set, budget, and actor-policy identities.
- Randomness discipline: paired streams require a common invariance-profile digest; independently sampled streams must be distinct.

Current source includes 17 authored Rust unit tests. They are not reported as passing until the exact-head Rust workflow executes successfully.\n\n## Claim ceiling

This crate validates typed records supplied to it. It does not parse JSON or execute the JSON Schema; it is not a JSON Schema 2020-12 implementation. It does not authenticate the runner's independent attempt counter, verify signatures or provenance, prove that an invariance profile is scientifically valid, establish sandbox isolation, execute a Symtropy model, or score a Symthaea agent. A correct local ledger is not proof that no attempt was omitted upstream.

The public JSON Schema and this semantic validator are complementary: schema validation checks serialized shape; this crate checks cross-field and temporal semantics.

No scientific discovery, physical validity, SOTA performance, or production safety claim follows from these conformance tests.
