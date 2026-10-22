# Mechanics Validation Program

**Status:** research proposal; acceptance gates are proposed, not measured results.  
**Scope:** extend the existing Symtropy physics validation and benchmark work, then connect reproducible mechanical evidence to Symthaea, Mycelix, and Sol-Atlas.

## Why this is the next step

Symtropy already has an analytical free-fall convergence example, an orientation-aware collision validation example, invariant snapshots/drift diagnostics, and Criterion performance benchmarks. These are useful foundations. The next leverage point is to turn individual examples into an explicit, layered contract for what “mechanically valid” means, which cases are supported, and what evidence must accompany a result.

This cross-stack roadmap **complements rather than replaces** the crate-level [Research Validation Protocol](../../crates/core/symtropy-physics/RESEARCH_VALIDATION.md) and [Orientation-Aware Collision Validation](../../crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md). Keep those documents canonical for the physics crate’s current validity envelope and solver-specific acceptance criteria. The workstreams below focus on connecting that physics evidence to maintenance decisions, provenance, asset context, and lifecycle outcomes across the four systems.

The engine-wide physics corpus should compose with the existing [PHYS-VALID-06 qualification-corpus issue](https://github.com/Luminous-Dynamics/symtropy/issues/460), not create a competing scenario schema or duplicate solver qualification. The two implementation proposals linked from this research program are intentionally narrow additions to its analytical translational lane; the broader cross-system work here is about using qualified physical evidence in component, maintenance, provenance, and asset decisions.

This program deliberately separates four claims that are often conflated:

1. **Implementation correctness:** the code follows the stated algorithm and does not violate internal invariants.
2. **Numerical verification:** the solution converges toward an analytical or independently computed reference under controlled assumptions.
3. **Physical validation:** predictions agree with measurements for a stated physical system and operating domain.
4. **Engineering fitness:** the evidence is sufficient for a particular design or operational decision, including uncertainty and consequences of failure.

Passing one layer is not evidence that the later layers have passed. In particular, a fast rigid-body result is not a stress, fatigue-life, fracture, thermal, lubrication, or safety certification.

## Paradigm shifts

### 1. From “physics engine tests” to a mechanical validity envelope

Every feature should declare the models and operating conditions it actually supports: spatial dimension, body and joint types, contact assumptions, integrator, step-size range, materials (if applicable), loads, numerical tolerances, and known limitations. Out-of-envelope requests should return an explicit unsupported/unknown state rather than silently extrapolating.

### 2. From one reference run to convergence and differential evidence

A single output that looks plausible is weak evidence. Analytical cases should be evaluated across step sizes; numerical convergence should be measured and the expected method order documented. Where no closed-form answer exists, use independent solvers or experimental measurements, explain differences in solver assumptions, and keep the reference version pinned.

Differential agreement is not itself proof: two implementations can share the same modeling error. Preserve at least some analytic or experimental anchors.

### 3. From benchmark scores to falsifiable engineering contracts

Every benchmark declares its inputs, invariants, reference values or intervals, tolerances, known sensitivities, and explicit failure behavior. Add negative controls—e.g. wrong units, invalid material data, missing loads, tampered input digests, or deliberately perturbed assumptions—that must be rejected or produce a detectable change. A benchmark that cannot fail is a demo, not a gate.

### 4. From isolated simulation to a closed evidence loop

Use a common, versioned mechanical-analysis record across the stack. Symtropy supplies the modeled behavior and diagnostics; Symthaea proposes hypotheses and experiments against that record; Mycelix preserves authorship, signatures, revision lineage, and approval; Sol-Atlas attaches the asset to the facility, location, utilities, access, dependencies, and operational criticality.

Cryptographic provenance can prove which data and executable were attested to; it cannot prove the data are physically true. Engineering sign-off remains a distinct responsibility.

### 5. From predictive maintenance accuracy to decision quality

Fault classifiers must be evaluated on held-out machines, runs, and operating conditions—not just random windows from the same recording. Report false alarms per operating hour, missed failures, usable warning lead time, calibration, and performance under changes in load and speed. For repair recommendations, measure avoided downtime/cost against actual maintenance outcomes and a declared baseline.

## Proposed benchmark ladder

The ladder is incremental. It does not claim the current engine supports every case in the table.

| Tier | Benchmark family | Reference and measures | Gate / interpretation |
|---|---|---|---|
| W0 | State and dimensional invariants | Finite values, proper rotations, dimensional consistency, declared units, stable handles/replay | Zero silent unit substitution; invalid or missing required inputs fail closed. Record invariant residuals and all non-finite states. |
| W1 | Analytical free motion | Existing free-fall example; add constant-velocity and constant-acceleration cases where supported | Report position/velocity error at matched physical times across time steps. Check the expected convergence trend for the selected integrator; do not impose one universal percentage on every case. |
| W2 | Isolated collision and contact | Orientation-aware GJK/EPA cases; separated/touching/penetrating primitives; normal and tangential contact | Report classification accuracy, penetration depth, impulse/residual behavior, finite-state health, and sensitivity to orientation/scale. Contact assumptions and restitution/friction settings must be explicit. |
| W3 | Multi-body mechanisms | Begin with simple link/joint systems once the corresponding joint constraints are implemented and independently checked | Compare position, velocity, reaction-force trends and constraint residuals against analytic cases or a pinned independent multibody reference. Require step-size and constraint-tolerance sweeps. |
| W4 | Structural / thermal / fatigue interfaces | Separate validated FEA or domain-specific solvers; applicable NAFEMS/ASME V&V references | Require mesh/time-step convergence, boundary-condition disclosure, error measures, and verification/validation evidence. Do not infer structural adequacy from rigid-body simulation. |
| W5 | Asset health and prognosis | Public bearing/run-to-failure datasets, beginning with NASA IMS bearing data | Split by full run/machine and preserve chronology. Report PR-AUC and false alarms, lead time, calibration, confidence intervals, and domain-shift results against simple baselines. |
| W6 | Lifecycle / whole-system decisions | Asset graph, energy/resources, repair time, spares, downtime, end-of-life recovery | Compare total lifecycle outcomes against a documented baseline. Report trade-offs, uncertainty, and the evidence gaps that could reverse the decision. |

### How to set numeric thresholds responsibly

Do not choose a single magic tolerance for every mechanism. For each case:

1. Record a baseline on the current default branch and pin the compiler, target, hardware, solver settings, and commit.
2. Establish analytical or experimental reference uncertainty and perform step-size / tolerance sweeps.
3. Set case-specific absolute and relative error thresholds from that evidence. Treat singularities, discontinuities and contact transitions separately.
4. Add regression thresholds only after the baseline is stable; require reviewed threshold changes.
5. Keep physical acceptance criteria distinct from performance goals.

For well-conditioned analytic cases, a target such as <=1% relative error may be a reasonable *initial candidate*, but only after the quantity, reference uncertainty and convergence study support it. Near-zero quantities should use an absolute tolerance or a mixed tolerance. The test must not pass only because the denominator is tiny or undefined.

For a nominal 90% predictive interval, evaluate empirical coverage on held-out operating regimes and report the calibration curve. A 85–95% coverage band may be a useful initial research target, not a universal reliability guarantee. Also measure interval width: arbitrarily wide intervals can attain coverage without being useful.

## Required result contract

Every released benchmark run should emit a machine-readable record (JSON plus CSV where tabular time series helps) containing at least:

- schema_version and stable case_id
- repository, commit SHA, build profile, toolchain, target, and relevant feature flags
- model/geometry revision, units, materials and source identifiers when relevant
- integrator/solver, step size, solver tolerances, initial conditions, boundary conditions, loads and contact assumptions
- reference source/version, expected value or interval, absolute/relative tolerance, and reference uncertainty
- observed values, errors, invariant/constraint residuals, warnings, and explicit pass/fail/unsupported status
- deterministic seed or replay input, raw-input digest, and output digest
- hardware/software environment for timing results
- known limitations and reviewer/approval status if the result is used in an engineering decision

Do not place personal or proprietary supplier data in a public benchmark artifact. Use synthetic cases or appropriately licensed, de-identified sources.

A digest establishes identity/integrity relative to the digested bytes, not the truth or calibration of their contents. Signatures attest to an actor's statement, not to physical correctness.

## Cross-repository division of responsibility

### Symtropy — numerical and physical behavior

- Expand the existing analytical and orientation-aware validation examples into a named benchmark ladder.
- Add case-specific convergence checks and negative controls before widening supported mechanisms.
- Keep Criterion microbenchmarks separate from correctness/validation gates; report both throughput and invariant health.
- Enforce dimensional validity in energy accounting. Contact impulse has units of N·s, not joules; a dimensionless multiple of friction-impulse magnitude must never be promoted to physical heat. Compose with [THERMAL-LOSS-001 / world-contact friction-loss integration](https://github.com/Luminous-Dynamics/symtropy/issues/809): promote friction loss to thermal transfer only when the applicable mechanical-energy delta is measured, the inertia/contact regime is qualified, and the energy ledger reconciles. Preserve explicit unqualified/unsupported states for contacts where this is not yet defensible.
- Publish known unsupported models plainly. Rigid-body contact does not replace FEA, fatigue, fluid, thermal, wear, or fracture models.

### Symthaea — investigation and experiment selection

- For component and bearing semantics, compose the existing [IND-COMP-000 component qualification boundary](https://github.com/Luminous-Dynamics/symthaea/issues/5965) and [CIV-ENG-BEARING-001 integrated bearing blocker campaign](https://github.com/Luminous-Dynamics/symthaea/issues/6238), including its frozen synthetic corpus plan ([#6240](https://github.com/Luminous-Dynamics/symthaea/issues/6240)); do not create a competing bearing ontology or reimplement its disposition logic.
- Consume a typed, versioned analysis record rather than free-form solver output.
- Maintain competing diagnostic hypotheses, required observations, and evidence for/against each hypothesis.
- Select the next measurement by expected information gain balanced against cost and safety; expose assumptions and uncertainty.
- Never promote a generated explanation to a finding without an executable check, reference, observation, or qualified human review.

### Mycelix — provenance, permissions and review

- Compose lifecycle links with [CIRC-MFG-001](https://github.com/Luminous-Dynamics/mycelix/issues/3095), which already defines manufacturing provenance and circularity connections; do not create a second lifecycle database or event graph.
- Store or reference signed attestations for revisions, test/calibration records, supplier certificates, solver outputs, and approvals.
- Preserve revocation/supersession and disagreements between reviewers; a newer record must not erase the audit trail.
- Separate signatures/authorization from claims of validity. Make confidential maintenance/production records permissioned and minimize unnecessary disclosure.
- Start with a schema plus deterministic validation tests before requiring multi-agent deployment for the first engineering pilot.

### Sol-Atlas — asset context and operational impact

- Reuse the existing [pump/water service trajectory work](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/144) and [evidence-bounded asset assessment export](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/162) for operational impact and asset packets; align repairability measures with the [repair-first, reversible construction-systems benchmark](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/149) where relevant. Do not introduce a second asset identity system.
- Link each physical asset and component to stable identifiers, location, facility relationships, access constraints, and critical dependencies (power, water, cooling, transport, spares).
- Make service capacity and recovery time explicit; distinguish a failed component from a degraded service and from total asset unavailability.
- Propagate mechanical fault scenarios to operational impacts with documented assumptions and provenance.

## Immediate implementation sequence

**P0 — Validation inventory and frozen baseline**
- Inventory existing validation examples, tests, diagnostics, and Criterion benchmarks.
- Freeze baseline outputs with commit/toolchain metadata.
- Identify analytical cases that can be implemented without new solver capabilities.

**P0 — Analytical regression gates**
- Add deterministic tests for free motion and invariant health using case-specific tolerances.
- Make convergence across time-step sizes a release artifact.
- Add negative controls for bad time-step inputs, non-finite inputs, mismatched units at any typed boundary, and incorrect expectations where the API allows meaningful checks. Do not add a test for behavior the API does not actually specify.

**P1 — Contact and multi-body reference set**
- Extend isolated contact cases and add simple constrained mechanisms only after confirming supported joint semantics.
- Compare against a pinned independent solver and document model differences, rather than presenting an unqualified “matches engine X” claim.

**P1 — Bearing prognostics pilot**
- Use NASA IMS bearings as a first public benchmark after reviewing data terms and structure.
- Split by run/machine and time; block leakage from overlapping windows, future-derived labels and preprocessing fitted on the full dataset.
- Compare against trivial and conventional signal-feature baselines before evaluating Symthaea-assisted diagnostics.

**P2 — Shared evidence schema and asset integration**
- Define a small, typed Rust schema with semantic versioning and canonical serialization.
- Add deterministic validation, digest checks, compatibility fixtures, and explicit unsupported/missing states.
- Integrate downstream with Symthaea, Mycelix and Sol-Atlas only after the record contract is reviewed.

## CI and evidence policy

- Small deterministic W0/W1 regressions should run quickly on every relevant change.
- Longer differential, dataset and stress suites can run separately, but their status must be explicit.
- queued, in_progress, absent checks, and timed-out runners are **not passes**. Record local test commands and outputs, pinned to the exact tested commit.
- A change to a baseline, tolerance, dataset split or expected result requires a visible rationale and a negative-control review.
- Release evidence must identify exactly which commit it applies to. Evidence from a previous HEAD does not certify the current HEAD.

## Initial exit criteria

The first milestone is complete when:

1. The current analytical and collision examples are inventoried and their exact run commands are documented.
2. At least two deterministic analytic cases have checked expectations and a convergence report.
3. Every report records commit/toolchain/settings, case-specific tolerances and a machine-readable pass/fail result.
4. At least one deliberate invalid-input or perturbed-assumption control demonstrates that the harness can fail.
5. The workflow separates correctness, numerical verification, physical validation, performance, and engineering approval.

These criteria improve the quality of claims now, without pretending that a full industrial mechanics suite can be built in one PR.

## External references

- [Symtropy physics crate and current validation examples](https://github.com/Luminous-Dynamics/symtropy/tree/main/crates/core/symtropy-physics)
- [Project Chrono validation studies](https://www.projectchrono.org/validation/) — useful independent multibody, vehicle, and FEA comparison material; pin a particular reference case and assumptions before comparing.
- [NASA Prognostics Center of Excellence data repository](https://www.nasa.gov/intelligent-systems-division/discovery-and-systems-health/pcoe/pcoe-data-set-repository/) and [NASA IMS bearings dataset metadata](https://data.nasa.gov/dataset/ims-bearings) — public bearing prognostics starting points; confirm access terms and data granularity before creating an evaluation split.
- [NIST Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing) — STEP, QIF, MTConnect, conformance testing, and product/quality data traceability.
- [ASME V&V 10: Verification and Validation in Computational Solid Mechanics](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics) — relevant framework for structural analysis credibility; consult the current edition and applicable domain standards.
