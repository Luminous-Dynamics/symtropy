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


## Field layout acceptance benchmark v0.1

The field campaign is governed by the field layout acceptance benchmark profile at docs/field-layout-acceptance-benchmark-v0.1.md. It separates vendor-condition reproduction from a common independent referee. The primary output is independently accepted design-intent entities per total crew-hour, not print-head speed or robot self-reported completion.

The machine-readable per-run observation contract is schema/field-layout-run-observation-v0.1.schema.json. It binds the design work package, robot build/mode/stationing/sensors, site and metrology identities, U95 reference uncertainty, whole-workflow accounting and per-entity observations. It intentionally stores robot-claimed state separately from independent referee state.

The synthetic example at examples/field-layout-run-observation-synthetic-v0.1.json shows a robot that claims a line was printed while the independent referee rejects the measured output. The hashes and values are synthetic placeholders, not field evidence. The dependency-free Rust scorer in src/field_layout_acceptance.rs now performs typed manifest/observation/referee reconciliation and independently derives entity states and whole-workflow metrics. JSON ingestion and full Draft 2020-12 schema validation are still not wired into the CLI/CI lane.

The profile tracks HP SitePrint, Dusty FieldPrinter 2, Civ Robotics outdoor staking/marking, Rugged Mark1 and Hilti Jaibot as separate task tracks; it includes Dusty's November 2026 features and the 2025 multi-trade field study. It references AMRA-201:2026, ISO 10218-1/-2:2025, ISO 12100 and relevant ASTM F45/NIST test-method work. The CivDot+ published "3/100 ft (8 mm)" value is preserved with a unit-conversion discrepancy flagged for clarification. It does not claim certification or standards conformity.


### Independent acceptance scorer

The typed Rust API is exported as field_layout_acceptance::score_field_layout_run. It accepts an evaluator-owned design manifest, a list of robot observations, referee measurements, explicit human waivers, a frozen tolerance/resolution profile and whole-workflow crew-hour accounting.

The scorer rejects duplicate manifest IDs, duplicate/unknown observations and duplicate/unknown waivers rather than silently deduplicating or ignoring extra output. Every manifest ID receives exactly one reconciled state: ACCEPTED, REJECTED, HELD, WAIVED or NOT_ATTEMPTED. The robot claim is preserved for audit but never overrides referee measurement. Insufficient independent reference U95 produces HELD/ReferenceResolutionInsufficient, not an accuracy pass or a geometry failure. A waiver remains WAIVED and is excluded from accepted throughput. Zero crew-hours yields no productivity rate instead of infinity.

The result reports accepted/rejected/held/waived/not-attempted counts, attempted count, first-pass accepted count, rework entity count, total crew-hours, accepted entities per crew-hour, first-pass yield and rework-hour fraction. Unit tests cover false robot completion, geometric rejection, insufficient reference resolution, human waiver separation, duplicate/unknown IDs and zero-hour denominators.

This first scoring owner is typed Rust logic, not a JSON reader, digital signature verifier, physical metrology system or field report generator. The observation schema and synthetic JSON shape fixture remain separate; a future ingestion step must verify the manifest and receipt digests before building typed inputs.
