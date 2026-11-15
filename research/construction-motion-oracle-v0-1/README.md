# Construction Motion Oracle & Differential Benchmark v0.1

**Status:** synthetic analytic reference for two bounded geometry families. Not a general collision checker, planner or safety qualification.

Parent implementation: [motion envelope #5345 / PR #5347](https://github.com/Luminous-Dynamics/symtropy/pull/5347). Programme issue: [#5363](https://github.com/Luminous-Dynamics/symtropy/issues/5363).

## Independent oracle

This crate uses separate mathematical formulas for:
- closest approach between a straight 3D segment and a stationary sphere;
- closest approach between an upper semicircle and a stationary sphere;
- exact coordinate extrema for those path families.

It consumes the envelope producer's output only to test containment and broad-phase candidates. It does not reuse the producer's AABB expansion helpers.

The semicircle case is deliberately adversarial: its endpoint chord is the diameter, which misses an obstacle at the arc apex. Endpoints-only and chord-only baselines therefore return false while the analytical continuous oracle returns true.

## Current synthetic corpus

The frozen corpus now contains **272 cases**:
- 212 development cases (the original 12 plus 200 generated fixtures);
- 60 public parameter-validation cases (30 linear and 30 upper-semicircle);
- 12 controlled mutations covering identity, geometry, time and ordering.

The generated fixture set contains 100 development and 30 validation samples per path family. It is deterministic, generated with a fixed 32-bit LCG seed `0xc0c57a11`, and has a checked-in Rust generator at `src/bin/generate_corpus.rs`. Regenerate the supplemental 260-case JSON array with:

```sh
cargo run --locked --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --bin generate-corpus > generated-cases-v0.2.json
```

The generator is deliberately dependency-free. It uses independent point-to-segment and point-to-semicircle distance formulas and asserts that any analytic collision is also a broad-phase AABB candidate. The checked-in combined corpus has been structurally validated against `schema/oracle-corpus-v0.1.schema.json`.

Across the generated fixtures there are 7 analytic contacts, 7 endpoint-only misses, 2 chord misses, 16 broad-phase false-positive candidates, and 0 broad-phase false negatives. These are deterministic fixture counts, not field frequencies or estimates of real-world collision prevalence.

The 60 public validation fixtures are parameter-stratified and intentionally differ in scale ranges from the development fixtures. They are **public validation data, not a secret blind test set**. A future genuinely blinded evaluation needs a separate custodian-controlled corpus and a frozen scoring protocol. The held-out labels must not be used for iterative tuning.

See `schema/oracle-corpus-v0.1.schema.json`. Adapter results use `schema/adapter-result-v0.1.schema.json`.

The machine-readable fixture contains 12 explicit geometry cases with path endpoints/arc parameters, body and obstacle radii, all seven per-axis margin channels, expected analytic/endpoints/chord/physical/stopping/coordination outcomes, primitive-manifest completeness, and six controlled mutations. All current cases are development cases; there is no claim of a statistically sufficient held-out corpus yet. See schema/oracle-corpus-v0.1.schema.json. Adapter results use schema/adapter-result-v0.1.schema.json.

The Rust unit suite includes:
- straight-segment analytic containment;
- explicit detection of an understated chord-deviation claim;
- arc collision missed by endpoints and chord baselines;
- thin barrier crossed between safe endpoints;
- conservative AABB false-positive candidate;
- separation of physical collision from stopping/policy overlap;
- uncertainty monotonicity;
- uniform-scale collision invariance.

All values are synthetic and expressed in millimetres. The upper semicircle is planar, has a fixed z coordinate and covers exactly 180 degrees. The linear reference is piecewise straight. The oracle does not prove general articulated-robot containment or arbitrary mesh swept volumes.

## Industry differential adapters

The frozen inputs/results can be adapted to:
- MoveIt continuous robot-world checking with the exact detector backend and collision matrix recorded;
- Tesseract CONTINUOUS and LVS_CONTINUOUS evaluators;
- additional independently versioned checkers.

The MoveIt continuous robot-world API and self-collision checks are distinct scopes; backend support must also be verified. The currently inspected MoveIt FCL continuous overload is a stub, while the Bullet tutorial documents CCD. Tesseract evaluator interpolation/cast rules must be recorded. No adapter result is accepted as analytical truth, and a missing backend is NOT_RUN, never PASS.

See docs/differential-adapter-contract.md.

## Commands

- cargo fmt --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml -- --check
- cargo test --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --locked
- cargo clippy --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --locked --all-targets -- -D warnings
- cargo run --release --manifest-path research/construction-motion-oracle-v0-1/Cargo.toml --bin differential-bench

## Claim ceiling

Passing tests establish only the implemented mathematics for the declared line and planar semicircle families and the adapter's tested structural behavior. They do not establish arbitrary continuous collision detection, physical robot model accuracy, localization/tracking/stopping truth, safe separation, construction productivity, standards conformity or vendor interoperability.
