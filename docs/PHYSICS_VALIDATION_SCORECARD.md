# Physics Validation Scorecard

**Status:** proposed benchmark contract; not a claim that the current engine is qualified.  
**Scope:** Symtropy's native 3D rigid-body solver and its independent Rapier 0.18 comparison lane.  
**Principle:** `agreement between two solvers != physical truth`.

## Paradigm shift: from plausible rollouts to auditable physical claims

Do not compress physics quality into one scalar or treat a green unit-test suite as an engine qualification. Every claim should identify what evidence supports it, where that evidence applies, and what remains unknown.

Use separate evidence classes:

| Evidence class | What it establishes | What it does not establish |
|---|---|---|
| Structural / numerical integrity | Inputs, traces, identities, units, and computed metrics obey explicit invariants | Correct physical behavior |
| Analytical verification | Results agree with a closed-form or independently derived solution within a declared numerical envelope | Real-world fidelity outside that model |
| Metamorphic verification | A declared transformation preserves or changes an outcome in the mathematically expected way | Full physical correctness |
| Cross-engine comparison | Independent implementations disagree or agree on a shared, explicitly bounded scenario | Which implementation is physically correct |
| Measurement-grounded validation | Simulation outcomes agree with controlled real observations, including uncertainty and calibration metadata | Universal fidelity across other regimes |

The report must expose the evidence class per task. Do not average a severe CCD failure away with many easy free-fall passes. Report **pass**, **fail**, **inconclusive**, or **not comparable** separately from evidence class and risk.

This approach is aligned with NIST's risk-based VVUQ guidance: scale credibility work to model influence and decision consequence, and keep uncertainty visible ([NIST AMS 100-85, 2026](https://doi.org/10.6028/NIST.AMS.100-85)). Metamorphic testing is also an established technique for simulation verification and validation when a complete oracle is unavailable ([Raunak & Olsen, NIST, 2021](https://doi.org/10.1109/MET52542.2021.00015)).

## Gates and benchmark targets

All numerical limits below are **proposed engineering targets**. They are not universal tolerances; calibrate thresholds per scenario, solver profile, scale, and consequence.

### Gate 0 — evidence integrity (required before interpreting any physics metric)

- Reject unsupported schemas, empty scenario identifiers, empty backend / backend-version / solver-profile identity, duplicate semantic body IDs, invalid time sequences, and mismatched body sets.
- A missing or non-finite measurement must remain missing/explicit and must never be encoded as a zero-error result.
- Require complete finite observation coverage to issue a threshold-based pass; partial coverage is diagnostic and otherwise inconclusive.
- Acceptance profiles name every selected metric and unit, have finite nonnegative tolerances, and are versioned by a stable profile ID.
- Preserve negative controls: intentionally malformed input, injected non-finite measurements, incorrect body mapping, and a known threshold violation must be rejected or classified as expected.
- Exact source/tree identity and scenario/configuration identity must accompany each result. Queued CI is not a pass; a result is valid only for the exact tested source and configuration.

### Gate 1 — analytical verification and convergence

Start with isolated, interpretable systems:

- Constant-gravity free fall, with position and velocity compared to the closed-form trajectory.
- Elastic / inelastic sphere bounce where the idealized model's assumptions are documented.
- Frictionless sliding / inclined-plane motion where contact discretization does not dominate the oracle.
- Simple pendulum or rigid-body rotation only after angular-state conventions and the reference equations are explicitly qualified.

**Targets:**

- Run at least three stable timestep refinements for each claimed integrator order. Estimate observed order from the error ratios; for the current semi-implicit free-fall position fixture, require first-order position error within **±0.2** over each adjacent refinement pair.
- Record timestep, total simulation time, signed residual, max error, RMS error, and observed order. Keep the raw trace.
- Stop interpreting apparent order after numerical noise or a different error source dominates. If the expected order is not observed, report the result rather than widening tolerance.
- Add exact-oracle tests for both position and velocity; do not infer solver accuracy solely from agreement with Rapier.

The current corpus uses `dt = 0.04, 0.02, 0.01 s` for this first order-estimation slice.

### Gate 2 — metamorphic and determinism properties

For scenarios where the physics equations warrant the transformation:

- **Semantic insertion-order permutation:** exact trace equality only for backend profiles that claim deterministic replay and where construction order is declared non-semantic.
- **Uniform coordinate translation:** translate the initial position by a known vector; require the trajectory to preserve that offset and velocities/sample times to remain invariant within declared numerical precision.
- **Frame / axis transformations:** add only when gravity, orientation, geometry, and all vectors are transformed consistently.
- **Serialization/reload and replay:** compare canonical scenario digests before comparing outcomes.

**Targets:** 100% of declared invariants pass on the deterministic fixture set; every injected defect is detected by at least one intended guard. Do not demand bitwise equality from profiles that explicitly do not promise it.

### Gate 3 — contact, stacks, constraints, and CCD

Grow the corpus in bounded increments instead of one massive “physics test”:

| Family | First fixtures | Required observables |
|---|---|---|
| Rest / support | Sphere and box on plane; rotated contact when supported | Settling height, residual speed, peak penetration, contact stability |
| Impact | Restitution ladder; glancing / nonsmooth contact | Pre/post-impact velocity, bounce-height ratio, momentum-transfer error |
| Friction | Slope slider; static-to-kinetic transition | Acceleration, terminal speed, stopping distance, stick/slip transitions |
| Stack stability | 3 / 5 / 10 / 20 body stacks and pyramid | All-body cardinality, peak speed, stack-height drift, penetration, contact churn |
| Mass-ratio stress | Light body against heavy/static body | Constraint/contact residual, impulse bounds, drift |
| CCD | Fast small projectile against thin barrier; swept shapes | Tunnelling count/rate, time of impact, post-impact state |
| Lifecycle | Sleep/wake, remove/rebuild, reset/replay | State continuity, stale handles, deterministic identity mapping |
| Constraints | Pendulum, chain, bridge, motor/limit fixtures once supported | Constraint residual, drift, oscillation period, limit overshoot |

Record solver iterations, substeps, timestep, collision/CCD settings, and backend for every result. Normalize penetration by a documented geometric scale (for example, the smallest relevant body dimension) so cross-scale comparisons have meaning.

**Targets:** zero non-finite states; zero silent body loss; zero tunnelling in declared fixtures that require guaranteed CCD; all empirical limits tied to documented fixture expectations. Select bounds from analytical expectations or measured reference trials—not from whichever engine happens to perform better.

### Gate 4 — speed–accuracy frontier and scaling

Performance is meaningful only alongside accuracy and a reproducible workload.

- Benchmark sparse broadphase, dense contact, static-heavy, and query-heavy scenes separately.
- Sweep timestep, substeps, solver iterations, and backend. Plot wall time against trajectory/contact error instead of publishing speed alone.
- After warmup, collect at least 30 measured runs per fixed scenario/profile when runtime permits; report median, p95, dispersion, peak memory, and sample counts. Keep machine and power/thermal conditions with the receipt.
- Include sizes such as 10, 100, 1,000, and 10,000 bodies when practical; maintain separate collision density and shape complexity controls.
- Establish a versioned baseline before setting regression budgets. A candidate optimization must not silently change the acceptance profile or accuracy envelope.

**Aspirational scaling target:** on locked hardware, sparse scenes above 1,000 bodies should aim for under **2.5× median runtime when body count doubles**, without escaping accuracy limits. Treat this as an optimization objective, not an initial correctness/merge gate until repeatable baselines exist.

### Gate 5 — measurement-grounded validation

Cross-engine parity is not physical validation. After analytic and contact regression tests stabilize, introduce an openly documented set of controlled real-world trials.

[GAUGE](https://github.com/InternRobotics/GAUGE) is a candidate reference project. Its current public overview describes approximately 1,560 motion-capture trials across 22 task families, calibrated physical metadata, uncertainty estimates, and rigid / cable / textile / volumetric regimes. Begin with a small rigid-body subset—such as bouncing ball, slope slider, and nonsmooth contact—before attempting broader coverage. Review dataset, code, and attribution terms before copying or redistributing any assets.

For each trial:

- Record measured geometry, mass, material parameters, friction/restitution where available, measurement cadence, calibration, and uncertainty.
- Separate parameter-calibration trials from held-out validation trials. Do not tune and report accuracy on the same trial.
- Report trajectory discrepancy, temporal alignment, event/impact timing, momentum transfer, oscillation period, friction behavior, and relevant task-specific observables.
- Report distributions and confidence intervals across trials/materials; show outliers and failed trials. Compare residuals against measurement uncertainty, not a convenient universal millimetre threshold.
- Keep a benchmark version and immutable dataset manifest so future revisions cannot silently alter the baseline.

Initial goal: reproduce reference baseline metrics on the selected held-out task subset before claiming improvement. Expand to deformables only after separate constitutive-model, contact, and material-parameter validation exists.

## Required result receipt

Every executed scenario should produce a machine-readable receipt (the existing scenario/trace contract is the starting point; this document does **not** define another serialization schema). It must bind:

- exact source commit and tree, workspace state, compiler/toolchain, dependency versions, OS and hardware;
- scenario ID and canonical input digest; configuration/profile ID and digest;
- backend ID/version, fixed timestep, substeps, solver iterations, CCD/contact settings, and random seed if applicable;
- metric names, units, values, thresholds, coverage counts, and pass/fail/inconclusive/not-comparable decision;
- evidence class, reference dataset/trial ID and uncertainty where applicable;
- raw trace artifact or content-addressed reference, digest, and evaluator version.

Receipts should be reproducible and auditable. A future follow-on can sign receipts and link them into a content-addressed evidence graph; that should build on the canonical observation protocol, not become a third trace schema.

## Merge and promotion policy

1. Keep physics bridge, shared observation, and corpus PRs in draft until their exact-head formatting, lint, tests, and build checks finish.
2. Before merging the D1 work, resolve the protocol-convergence gate between the existing `PhysicsScenario3d / PhysicsTrace3d` comparator and `symtropy-physics-diff-core`. There must be one canonical observation/wire layer.
3. Treat each added fixture as a narrow claim with a named profile; do not label the whole engine “validated” because a small rigid-body subset passes.
4. Promote to safety-relevant or robot-control usage only with measurement-grounded validation and uncertainty appropriate to that use case.

## Research references

- NIST, *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices*, AMS 100-85 (2026): https://doi.org/10.6028/NIST.AMS.100-85
- Raunak & Olsen, *Metamorphic Testing on the Continuum of Verification and Validation of Simulation Models*, NIST / MET 2021: https://doi.org/10.1109/MET52542.2021.00015
- InternRobotics, *GAUGE: A Measurement-Grounded Benchmark for Physical Fidelity in Simulation Engines and Video World Models*: https://github.com/InternRobotics/GAUGE
