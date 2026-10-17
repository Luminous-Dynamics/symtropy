# Physics Falsification-First Qualification Plan

**Status:** Proposed benchmark protocol; targets are not measured results.  
**Date:** 2026-10-17  
**Owners:** Symtropy (deterministic execution and numerical diagnostics); Symthaea (hypotheses, experiment selection, uncertainty and model credibility).

## Thesis: qualify claims, not just code

The paradigm shift is to treat every physics result as a bounded scientific claim with a testable falsifier—not as a plausible trajectory, a green unit test, or a conservation scalar.

    claim + assumptions + intended use
            ↓
    exact model/source/configuration identity
            ↓
    independent analytical / numerical / empirical comparison
            ↓
    hostile negative controls + uncertainty + applicability limits
            ↓
    versioned, reproducible evidence record

Keep these evidence classes distinct:

1. **Mathematical validity** — the stated equation or invariant follows from its premises.
2. **Implementation fidelity** — the exact code implements that statement within a declared numeric/error envelope.
3. **Numerical-solution verification** — timestep, discretization, solver settings and convergence are adequate for the observable.
4. **Calibration** — parameters are fitted to identified data.
5. **Physical validation** — predictions are compared with independent observations for the intended domain.
6. **Model credibility** — the evidence is adequate for one declared use, with remaining uncertainty/discrepancy explicit.

No class promotes itself into the next one. A theorem can be correct while its code is wrong; a code path can replay deterministically while its physics is wrong; a calibrated model can fail on held-out data.

## Paradigm shifts

### 1. From scalar conservation scores to named accounting boundaries

A balance is meaningful only when the terms share semantics and the accounted system is explicit. Distinguish reservoir storage changes, external inputs, work, transfers, dissipation, regeneration and unmodeled sinks. A numerical residual between two incomplete counters is a diagnostic—not a first-law proof.

For the consciousness physics path, operational capability budgets and impulse-derived heuristics must not silently become joules. In particular, regeneration must never be recorded as negative dissipation. Record only the amount actually accepted by the reservoir, separately from the mismatch diagnostic. Physical energy claims require the core typed-transfer boundary and complete reservoir reconciliation.

### 2. From “tests pass” to falsification power

Keep known-answer tests, seeded property tests, mutation tests, and independent differential oracles. The verifier should demonstrate that it catches wrong signs, omitted terms, invalid dimensions, changed thresholds, stale configuration, and unsupported physics—not merely that it approves a good example.

The independent oracle must not call the production reducer to calculate its own expected answer. Retain failing fixtures as permanent regression cases.

### 3. From one benchmark scene to a coverage envelope

Each model profile must say which physics it represents, which it approximates, which it does not represent, and the exact load / geometry / material / timestep envelope evaluated. Transfer to a new use is a new obligation. Do not infer aerodynamics, fluid dynamics, structural mechanics, acoustics or thermodynamics from a rigid-body collision engine.

### 4. From post-hoc tuning to preregistered holdouts

Freeze model revision, parameters, acceptance observables and tolerances before exposing confirmatory data. Search/training trajectories and reference-catalog recognition are not holdout validation. If confirmation data is used to tune the model, reclassify it as calibration/update evidence and require a new untouched holdout for the new model generation.

### 5. From highest-fidelity-first to value-of-information experiments

Symthaea should propose the next experiment by expected decision-relevant information per unit cost, with a simple baseline and uncertainty-aware stopping rule. Use low-cost analytical solutions and reduced models to reject bad hypotheses early; run expensive specialist solvers or physical tests only where they could change a decision. Keep the multi-fidelity methods, shared assumptions, and data dependencies explicit—two solvers agreeing because they share a model are not independent validation.

### 6. From “confidence” to decomposed uncertainty and discrepancy

Keep measurement, input, parameter, numerical, stochastic, surrogate and model-form uncertainty separate where relevant. Record correlation/common-mode dependencies. An unquantified uncertainty is unknown, not zero. Never turn an aggregate score into a pass if a required observable or safety hard constraint fails.

## Benchmark ladder

All thresholds below are **proposed acceptance targets**, not current results. Start with a small benchmark family and tune no tolerance after seeing confirmatory outputs.

| Gate | Benchmark / metric | Initial target |
|---|---|---|
| Provenance | Exact source/tree, configuration, profile, toolchain, inputs, seeds and output digest | 100% of qualified runs have complete required identity; changed claim-relevant input invalidates old evidence |
| Claim scope | Every result's equations, units, frames, domain, approximations and claim ceiling | 100% explicit; unsupported quantity cannot receive a qualified numeric result |
| Invalid-input containment | NaN, infinity, overflow, negative flow, invalid units, stale generation, malformed config | 100% of seeded invalid cases rejected or returned with explicit non-qualified disposition; zero false passes |
| Analytic known answers | Free motion, constant acceleration, harmonic oscillator where supported, pendulum/reference ODE, ideal contact/restitution cases within their assumptions | Error is measured against an independent analytical oracle; publish absolute and normalized error, not pass rate alone |
| Numerical convergence | Halve time step / refine discretization on smooth supported cases | Measured order agrees with the method's declared order within a preregistered tolerance; no blanket order claim for discontinuous contact events |
| Invariants | Applicable momentum, angular momentum, and energy/other balances for each explicitly closed model | 100% accounting closure at the declared algebraic transition level; numerical drift bounded by a scenario- and integrator-specific error budget |
| Differential tests | Compare optimized broadphase/contact/coupling result against a simpler independent reference | Zero missed required pairs/events and zero unexplained semantic mismatches over the frozen corpus |
| Replay | Same pinned build/profile/seed/input tape | Bitwise identity only where explicitly promised; otherwise exact-runtime reproducibility plus separately declared cross-platform tolerances |
| Mutation testing | Flip signs, remove terms, alter threshold, bypass validation, corrupt mapping, reuse stale output | Initial target: ≥90% kill rate for the curated in-scope physics mutations; publish survivors and denominator |
| Discovery quality | Cold invariant/equation discovery with holdout trajectories and negative controls | Zero known false controls accepted as discoveries; report precision/recall on a frozen labeled corpus and worst/median holdout residual |
| Uncertainty calibration | Predictive intervals on untouched measurements / scenarios | For a nominal 90% interval, target empirical coverage compatible with 90% within binomial uncertainty, plus interval width and proper scoring; do not report coverage without sample size |
| Multi-fidelity value | Same evaluation budget vs grid/random or domain-standard baseline | Report top-K hit rate, feasible yield, rank correlation, wall cost and decision regret on holdout cases; claim improvement only if the predeclared primary metric improves without hard-constraint regressions |
| Performance | p50/p95/p99 latency, throughput, allocations/memory, solver cost by workload regime | Establish baseline first; target ≥10% improvement on a preregistered primary workload with correctness and error unchanged, else record no win |
| Physical validation | Held-out measurement residual, measurement uncertainty, conditions and article/configuration match | Acceptance tolerances sourced from intended use / metrology / domain standards before comparison; no universal tolerance |

### Initial analytical and adversarial corpus

Use only cases supported by the selected implementation profile:

- Free particle and constant acceleration with closed-form trajectories.
- Harmonic oscillator and simple pendulum where a matching integration/model is implemented.
- Elastic collision/restitution and contact-boundary cases using declared masses, geometry and restitution semantics.
- Isolated-body, two-body and small system cases for momentum checks where no external impulse acts.
- Closed and open energy-system fixtures; deliberately add/remove a declared source or sink to verify the bookkeeping response.
- Duplicated, permuted and reordered insertion inputs to test determinism and order-invariance where promised.
- NaN/Inf, near-overflow, sub-ULP transfer, invalid unit/frame, stale config, missing parameter, out-of-domain request and solver non-convergence.
- Physics mutations: reverse a transfer sign, omit reservoir storage, change a collision impulse factor, corrupt a contact filter, skip a stale-input check, or reuse an old result after source/configuration changes.
- Discovery controls: random/shuffled trajectories, known non-invariants, single-orbit accidental invariants, template leakage, and data used for both tuning and confirmation.

The test must state whether a result is a known-answer pass, bounded numerical comparison, mutation detection, synthetic-only observation, or real physical validation.

## Ownership and implementation sequence

Reuse existing owners rather than adding a competing generic physics or evidence system:

- **Symtropy ENERGY AUTHORITY CONTRACT / PR #45:** separate physical-energy, operational-budget and heuristic channels.
- **Symtropy PR #1682:** proof-carrying invariant contracts and bridge from assumptions to exact implementation.
- **Symtropy issue #1693:** typed energy flows and complete-system boundaries.
- **Symtropy issue #460:** engine-wide rigid-body, contacts, CCD, joints, lifecycle, replay and numerical-health qualification corpus.
- **Symthaea issue #6861:** distinguish numerical Einstein-residual checking from formal geometric proof.
- **Symthaea issue #6862:** cold discovery, holdouts, symbolic/formal verification and novelty controls.
- **Symthaea issue #6122 and #3697:** model credibility and lifecycle VVUQ; preserve intended-use and evidence limits.
- **Symthaea issue #6173:** semantic coupling, conservation, uncertainty transfer and currentness.

Suggested execution:

1. Close the semantic defect in the regeneration/dissipation path; keep local diagnostics explicitly below the conservation-proof claim ceiling.
2. Freeze a compact machine-readable analytical corpus and independent known-answer oracle.
3. Add mutation controls to validate the validators.
4. Qualify the simplest supported mechanical cases and timestep-convergence curves before optimizing.
5. Add the Symthaea cold-discovery/holdout route and model-credibility projection.
6. Connect external reference solvers and measurements only through profile-specific unit/frame/model contracts.
7. Expand the corpus by physics domain; do not promote unsupported domains by analogy.

## Evidence and gate rules

Every benchmark record should include: schema/profile version, exact commit/tree and changed source blobs, compiler/toolchain and relevant dependencies, invocation/configuration, seed/input data digests, initial and final state digests where meaningful, expected-vs-observed outputs, metric values and tolerance provenance, warnings/non-convergence, and independent verifier status.

- QUEUED, SKIPPED, UNAVAILABLE, partial, stale-head or measurement-only results are not PASS.
- A test authored but not run is not a measured result.
- A deterministic trajectory is not physical validation.
- Agreement among multiple models is not independent validation when their assumptions/data are shared.
- Preserve failed and out-of-domain cases in the record.
- The exact tested commit is the only commit to which a test receipt applies.

## External research anchors

- NASA, [NASA-STD-7009B — Standard for Models and Simulations](https://standards.nasa.gov/standard/NASA/NASA-STD-7009), active standard record; its lifecycle framing emphasizes program-approved acceptance criteria and model credibility.
- NASA, [NASA-HDBK-7009B — implementation guide](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009), published 2026-02-03, companion practices for development and use of models and simulations.
- ASME, [VVUQ standards hub](https://www.asme.org/codes-standards/vvuq-standards), covering terminology, verification, validation and uncertainty quantification.
- NIST, [NIST AMS 100-85 (2026), on harmonizing CMS uncertainty methods with metrology](https://doi.org/10.6028/NIST.AMS.100-85), including risk-based credibility, uncertainty propagation and model/decision contexts.
- NIST, [Credibility Consideration for Digital Twins in Manufacturing](https://www.nist.gov/publications/credibility-consideration-digital-twins-manufacturing), describing VVUQ through the digital-twin lifecycle.

These references inform the process; this protocol does not claim standards compliance, certification, or approval by NASA, ASME, or NIST.

## Definition of done

The physics program becomes meaningfully stronger when a third party can reproduce a bounded claim, see exactly why it is supported, reproduce its counterexamples, identify where it stops applying, and verify that the code did not overstate its evidence. The north-star metric is **zero known false-positive physics qualifications in the frozen adversarial corpus**, with coverage and all misses reported—not the number of “green” tests or the size of the feature list.
