# Orbital infrastructure and accelerator benchmark roadmap — November 2026

**Purpose:** set a defensible route from source-grounded analytical references to engineering-grade VVUQ.  
**Owners:** Symtropy ORS workstream #5387, momentum control-volume reference #5872, model discrimination #5496, astrodynamics interoperability #4217; Symthaea T0 reference PR #7353; Mycelix evidence envelope PR #4964; Sol-Atlas megastructure context PR #884.

## Executive position

Do not try to “beat SOTA” by recreating mature generic astrodynamics, FEM, plasma or accelerator packages. Use the strongest maintained packages as specialist backends and independent comparators. The differentiating result should be a rigorously qualified, cross-domain engineering workflow for orbital infrastructure: exact boundaries and frames, conservative multi-physics interfaces, model-family falsification, experiment selection under uncertainty, and evidence receipts that prevent a synthetic or stale result from being promoted into a physical claim.

The first novelty target is not a new solver. It is a **decision-grade, falsifiable benchmark for an ORS moving stream and its support architecture**, built from small analytical oracles outward.

## Industry reference points (what exists now)

### Astrodynamics: reuse the mature comparison protocol

Brahe's published comparison currently exercises 32 tasks across 8 modules (time, coordinates, attitude, frames, orbits, propagation, force models and access). It compares API-compatible task subsets against Orekit 13.1.5, NASA GMAT R2026a, Basilisk, and Nyx/ANISE. Its protocol separates timing from accuracy: 200 fixed-input timing iterations and accuracy sweeps over 200 independent initial conditions, reporting p50/p95/p99/maximum error distributions against Orekit. The project itself warns that hardware and setup differences affect performance numbers. Treat this as a useful comparison method, not a universal leaderboard. Source: https://docs.brahe.space/latest/about/benchmarks.html

**Decision:** Symtropy should not build a second general orbit propagator. Reuse ANISE/hifitime, Nyx, Brahe, Orekit, GMAT, SPICE/Horizons where their declared task and frame semantics fit. Keep backend identity, kernels, force model, time scale, frame path, solver settings and raw result attached to each comparison. Use the existing astrodynamics interop issue #4217.

### Model/simulation credibility: use formal VVUQ practices

NASA-STD-7009B is an active model-and-simulation standard. It establishes lifecycle practices and calls for acceptance criteria to be defined by the project and approved by the delegated technical authority. NASA-HDBK-7009B, dated 3 February 2026, is the implementation handbook. ASME's VVUQ portfolio distinguishes numerical verification, physical validation and uncertainty quantification; VVUQ 20.1-2024 provides multivariate validation metrics. NIST report AMS 100-85, published 5 October 2026, explicitly harmonizes ASME VVUQ approaches with metrology and presents risk-based credibility assessment.

Sources:
- https://standards.nasa.gov/node/263
- https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009
- https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty
- https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing

**Decision:** model credibility must scale with model influence and decision consequence. A tiny arithmetic reference can have a tight, narrow claim. A coupled support transient that influences architecture or safety decisions needs stronger independent verification, measurement correlation and uncertainty evidence. A zero residual by itself is not validation.

### Particle accelerators: benchmark beam quality and programme maturity separately

A 2025 Nature Communications FACET-II experiment demonstrated a plasma-wakefield transformer using a 10 GeV drive bunch and meter-scale, three-stage plasma. It produced trailing bunches above 20 GeV, with sub-percent energy spread, 2 mm·mrad normalized emittance, multi-kA peak current, transformer ratio above 2 and more than 10x brightness enhancement versus the drive bunch. The paper also says that simultaneously achieving the combination needed for hard-X-ray FELs remains a challenge. This is a credible subsystem benchmark, not an orbital-ring result. Source: https://www.nature.com/articles/s41467-025-65742-8

**New 2026 literature boundary:** a *Reviews of Modern Physics* review of beam-driven PWFA was accepted on 21 September 2026. It surveys accelerating fields up to roughly 100 GV/m and decades of progress, while emphasizing unresolved energy-efficiency and beam-quality challenges, including positron operation and multi-stage transport. Source: https://doi.org/10.1103/2lw2-szhs. This is an excellent literature map for the accelerator track, but it is not itself evidence that a collider based on PWFA is ready.

The 2025 Particle Data Group review also cautions against equating high gradient with a collider solution: it notes that projected luminosity for plasma-based multi-TeV colliders remains orders of magnitude below conventional collider projections, with positron focusing, stage-to-stage matching, beam quality, efficiency, stability and alignment still important challenges. Source: https://pdg.lbl.gov/2025/reviews/rpp2025-rev-accel-phys-colliders.pdf.

**Benchmark implication:** compare the full accelerator chain, not just gradient or energy gain. Freeze driver-to-witness transfer efficiency, witness energy spread and tails, normalized emittance growth, positron/electron symmetry, stage-matching losses, repetition rate, wall-plug efficiency, luminosity and availability. The accelerator can outperform a conventional system in gradient or compactness and still lose on luminosity, reliability or total facility cost.

For programme-scale maturity, CERN's Future Circular Collider feasibility study estimated about 15 billion CHF for FCC-ee with four experiments over roughly 12 years, with civil engineering around one third of the cost. CERN's published timeline places a member-state/partner decision around 2027–2028. Independent expert review in November 2025 found no technical showstopper based on the submitted study, while identifying further work such as environmental and socio-economic assessments. Sources:
- https://home.cern/science/accelerators/future-circular-collider/
- https://home.cern/cern-council-reviews-feasibility-study-next-generation-collider/

**Decision:** keep beam physics and ORS material-stream mechanics as distinct benchmark tracks. Compare accelerator concepts on beam energy, energy spread, normalized emittance, bunch charge, peak current, brightness, transformer ratio, efficiency, repetition rate, stability and uptime. Compare infrastructure programmes on requirements maturity, independent review, civil/geotechnical design, environmental assessment, stakeholder engagement, cost uncertainty, schedule risk, construction mass, commissioning and lifecycle operations. These are not interchangeable metrics.

## Paradigm shifts to pursue

1. **From scalar loads to open-system vector balances.** Every stream model declares a control volume, inertial frame, epoch, mass storage, inlet/outlet momentum flux and force terms. No unexplained “support load” balancing term.
2. **From one solver to independently checked specialist composition.** Use specialist orbit, field, structural, thermal and control backends only at explicit typed interfaces. An adapter must preserve each backend's validity domain and uncertainty.
3. **From fitting to falsification.** Reuse #5496: keep characterization, discovery, preregistered validation and sealed challenge data separate; require a declared minimum detectable effect and power before claiming model adequacy.
4. **From deterministic output to evidence-qualified decisions.** Each proposal, input, model, solver, output, independent check, measurement and claim must have immutable identities and an exact evidence class. Mycelix binds provenance; it does not manufacture physical truth.
5. **From raw throughput to useful delivered service.** For Earth-to-orbit and future terminal networks, optimize accepted payload mass at the required orbit/service and deadline, with construction mass, energy, availability, common-cause failures, environmental and rights constraints explicit.
6. **From one “best design” to a Pareto/robust design set.** Preserve cost, energy, mass, safety margin, uncertainty, environmental impact and time-to-service as separate objectives. Do not conceal tradeoffs in one composite score.

## Benchmark ladder and proposed targets

All values below are *proposed targets*, not achieved results. Tolerances must be tied to scale, uncertainty, mathematical oracle or intended use; avoid one arbitrary epsilon for every regime.

| Tier | Benchmark | Proposed acceptance target |
|---|---|---|
| T0-A | Analytical unit/orbit/control-volume references | 100% frozen known-answer cases; dimension and sign checks; exact source/constant identity; no hidden fitted values |
| T0-B | Frame and conservation metamorphics | 100% negative-control rejection; mass, momentum and frame-covariance residuals within predeclared scale-aware bounds; deterministic receipts |
| T1-A | Reduced-order moving-support transient | Independently assembled state-space/closed-form oracles; time-step/event convergence; energy and momentum ledgers separately close |
| T1-B | Model-family discrimination | For each materially detectable alternative: >=80% power at alpha <=5%; zero false-adequacy on sealed known-truth challenges; underpowered cases marked unresolved |
| T1-C | Sampled control / actuator | 100% command-to-applied-force traceability; no unbounded force, stale command or unmodelled saturation; independent impulse/work checks |
| T2 | Multi-support/rider segment | Independent equilibrium and perturbation modes; common-cause and support-loss scenarios; uncertainty and sensitivity; no unsupported force law |
| T3 | Coupled field/structure/thermal | Cross-solver agreement on matched cases and boundary terms; independent constitutive characterization; measurement correlation and full uncertainty budgets |
| P1 | Physical component correlation | Separate characterization and sealed validation runs; sensor calibration, synchronization, bandwidth, uncertainty and negative controls |
| P2 | Integrated test article | Pre-registered test plan; independent reviewer; failures/exclusions retained; evidence supports only the tested envelope |

### Exact ORS benchmark metrics

Report, never collapse into one score:

- dimensional mass residual (kg/s);
- momentum-flux and required-resultant vector errors (N);
- equal/opposite interface impulse residual (N·s);
- angular-momentum and energy residuals once the relevant balance terms are implemented;
- equilibrium force/moment residuals;
- state trajectory errors and mode/eigenvalue errors;
- time-step and mesh convergence order;
- model discrepancy and uncertainty-normalized residuals;
- minimum detectable effect, detection power, Type-I error and false-adequacy rate;
- actuator limits, delayed command and saturation statistics;
- failure detection, recovery and safe-state timing;
- runtime / memory only for matched build, host, model, tolerance, workload and output quality.

### Exact accelerator metrics

- final beam energy and energy gain;
- relative energy spread (including tails, not only RMS);
- normalized transverse emittance by plane;
- bunch charge and peak current;
- brightness and transformer ratio;
- wall-plug-to-beam efficiency and energy recovery, with boundaries specified;
- repetition rate, jitter, shot-to-shot reproducibility, uptime and component lifetime;
- radiation, activation, heat rejection, cryogenic load and shielding requirements;
- capex, lifecycle cost, civil engineering, commissioning schedule and environmental burden.

### Statistical and performance protocol

For stochastic/sampled comparisons, freeze seeds and workload definitions; use at least 200 independent initial conditions where that is statistically meaningful and report p50/p95/p99/max, not just averages. For rare-event safety claims, use justified importance sampling or adversarial challenge sets and report effective sample size / coverage; never pretend ordinary random sampling proves rare failure absence. Performance runs use release builds, pinned toolchains, identical hardware, matched accuracy and repeated measurements. A faster but less accurate or less faithful solver does not win.

## What “better than SOTA” should mean here

- **Do not claim to beat** Orekit/GMAT/Nyx/Brahe on generic astrodynamics without a matched benchmark and independent reference.
- **Aim to exceed current practice in integration quality:** one exact typed boundary across frames, mass/force ports, structural models, controller and evidence receipts; explicit unknowns and claim ceilings; no friendly-name dispatch or silent assumptions.
- **Aim to exceed current practice in model adequacy:** active model discrimination, sealed challenges and decision-aware experiment selection, not just calibration residuals.
- **Aim to exceed current practice in traceability:** exact-head outputs, corpus/configuration/source digests, byte-identical replay, negative controls and immutable independent-check records.
- **Aim to produce an ORS-specific benchmark others can reproduce:** analytical cases, model mutations, convergence sweeps and eventually instrumented component data, published with transparent failures and matched baselines.

There is no justified “we are SOTA” claim today. The near-term achievement is a rigorous, public, falsifiable reference suite and a clean evidence ladder. A physical orbital-ring feasibility claim remains far beyond the present software references.

## Immediate execution order

1. Close the exact-head Rust and receipt checks on PRs #5855 and #5874.
2. Add independent angular-momentum and mechanical-energy control-volume references as separate small crates/issues, reusing the same frame/epoch/receipt profile.
3. Reuse #5496 to build sealed model-family mutations against #5417; do not build a second experiment selector.
4. Reuse #4217's astrodynamics backends for geometry and orbit state; do not create a new propagator.
5. Define the smallest safe, low-energy two-cart apparatus from #5437 for spring/damper model correlation only.
6. Only after equilibrium, force law, control volume, uncertainty and correlation are independently supported should the team move into coupled magnetic/structural analysis.

No construction, flight-readiness, human-safety or orbital-ring feasibility claim is authorized by this roadmap.
