# Aviation Engineering and Aerial Transport: State of Practice and Benchmark Plan

**Reviewed:** 2026-10-20  
**Purpose:** guide Symthaea / Symtropy / Sol-Atlas / Mycelix research priorities; not a certification basis, operational procedure, or claim that any project is superior to industry tools.

## Executive finding

The defensible opportunity is **evidence-integrated assurance across subsystem boundaries**, not attempting to out-solve every mature aerodynamics, optimization, air-traffic or certification package.

Current tools already cover much of the specialist territory:
- OpenVSP is a parametric aircraft-geometry tool. Its 3.47.0 release announced swept-volume analysis (including motion-vector dispersion) and expanded surface-visibility analysis in January 2026.
- OpenMDAO is a mature multidisciplinary design-analysis and optimization framework, with high-fidelity model integration and analytic derivative tooling; its ecosystem includes Dymos for trajectory optimization and pyCycle for turbine-cycle analysis.
- EUROCONTROL's Trajectory Prediction Specification v2.0 provides requirements and guidance for modeling planned/tactical trajectories; medium-term conflict-detection systems derive positions at the same time and apply defined separation criteria.
- NASA's Formal Methods Research Program develops assurance methods for mission-critical and safety-critical avionics; its air-traffic work includes ACCoRD/state-based conflict detection and resolution. NASA's March 2026 Air Traffic Management and Safety overview emphasizes moving from tactical to strategic operations and preserving traceability from requirements through models, scenarios, analysis, tests, metrics and outcomes.
- NIST's Digital Thread for Manufacturing programme works on standards, conformance tests, model-based product definitions, inspection backflow and trustworthy product-data assets, with STEP, QIF and MTConnect among the standards in scope.
- FAA AC 120-27F, active as of this review, gives operators guidance for weight-and-balance control programmes. FAA aircraft certification is a multi-phase process from regulatory basis and design through build/test, design approval and production approval.

The first Symtropy pairwise trajectory kernel is a deliberately tiny research component, not a replacement for any of these systems. The first Symthaea loading kernel is likewise a subset of weight-and-balance arithmetic, not an approved programme.

## Comparison table: compare by scope, not by slogan

| Reference system / practice | What it is good at | What our stack should do |
|---|---|---|
| OpenVSP | Parametric aircraft geometry and geometry/visibility analyses | Consume versioned geometry/analysis artifacts and their limitations rather than reimplement a mature geometry tool. Add evidence links and configuration invalidation around them. |
| OpenMDAO / Dymos / pyCycle | Coupled multidisciplinary analysis/optimization, derivative-based design search and trajectory optimization | Compose specialized tools behind explicit input/output schemas. Benchmark coupling fidelity, robustness under model shift, traceability and reproducibility—not raw optimizer superiority without matched models. |
| EUROCONTROL trajectory prediction guidance | Structured planned/tactical trajectory behavior and conflict-prediction requirements | Use a documented reference model and independent fixtures; make time validity, uncertainty, frames and applicability explicit; test conservative conflict detection separately from planning efficiency. |
| NASA formal methods / ACCoRD | Formal analysis of conflict detection/resolution and safety-critical digital systems | Treat independent formal/oracle checks as first-class gates. Do not claim safety assurance from unit tests, simulations or an AI narrative. |
| FAA weight/balance and certification | Aircraft/operator-specific approved limits, design/conformity/production approval and operational obligations | Treat approved-source applicability and human/authority sign-off as external decision gates. Never generate generic aircraft limits. |
| NIST manufacturing digital thread | Product-definition standards, conformance tests, inspection data and secure traceability through manufacture | Bind configuration, material/process/measurement evidence and design-change impact into a reproducible lifecycle thread, and reuse standard exchange formats. |

## Measured prototype baseline — 2026-10-20

The isolated Symtropy branch `feat/airspace-trajectory-conflict-audit-001`, exact head `6d3f16623b69c5ae58beddca92983f611d331443`, passed [workflow 39422964230](https://github.com/Luminous-Dynamics/symtropy/actions/runs/39422964230). The dedicated job ran formatting, Clippy with warnings denied, unit/integration tests, and the release benchmark; Linux, Windows and macOS repository matrices also passed.

The finite correctness corpus passed: 9,225 closed-form synchronized-parallel cases; 72,600 exact integer-rational sloped-segment comparisons; exact-boundary checks; three metamorphic invariants; and a 96-trajectory candidate-recall check against brute-force pairwise audits. These are bounded synthetic tests, not a proof for arbitrary floating-point trajectories.

The hosted Linux runner emitted these measurements for a fixed paired-path fixture (11 release-mode repetitions per size):

| Trajectories | Candidates | All possible pairs | Candidate reduction | p50 candidate generation | p95 candidate generation |
|---:|---:|---:|---:|---:|---:|
| 10 | 5 | 45 | 88.8889% | 1 µs | 21 µs |
| 100 | 50 | 4,950 | 98.9899% | 27 µs | 39 µs |
| 1,000 | 500 | 499,500 | 99.8999% | 230 µs | 338 µs |
| 10,000 | 5,000 | 49,995,000 | 99.9900% | 3.017 ms | 4.123 ms |

**Interpretation limits:** the CI host was not pinned; the fixture pairs nearby crossing paths and spaces the groups apart; the timings cover broadphase generation only. They exclude the 5,000 pairwise narrow-phase audits in the 10k case, live feed ingestion, surveillance association, uncertainty calibration, vehicle feasibility, route resolution, conflict grouping, controller display, failover and end-to-end decision latency. The result supports the feasibility of this particular filter; it is not comparable to EUROCONTROL operational systems and is not evidence of state-of-the-art superiority.

For context, EUROCONTROL's MTCD specification describes distinct functions: detecting probable separation loss, detecting restricted-airspace penetration, and identifying encounters relevant to shared airspace use. This prototype presently does only pairwise geometric screening on supplied synthetic trajectories. The next fair benchmark must freeze the same trajectories/criteria, pin the host and toolchain, report broadphase plus narrow-phase p50/p95 and memory, and retain 100% candidate recall against brute force.

Separately, the FAA's UTM concept is a collaborative ecosystem covering flight planning, authorization, surveillance and conflict management; it is not merely a geometric intersection test. A Reuters report dated 13 October 2026 describes lawmakers' safety-evaluation, training and transparency concerns about a separate AI-based FAA flight-management pilot. That is a timely reminder that human authority and published evaluation evidence matter, not evidence that this prototype has comparable function or risk.

## South African source and approval context

For work intended to be relevant in Johannesburg/Gauteng, distinguish the FAA/EU research references above from the applicable South African authority and operational publications. SACAA's current [Airworthiness page](https://www.caa.co.za/industry-information/airworthiness/) publishes technical guidance for Design Organisation Approval, Part Manufacturing Approval, STCs, type acceptance/certification and experimental certificates. It describes Type Acceptance Certificates under South African Civil Aviation Regulations, identifies model/category/limitations, and explains the relationship between type acceptance, registration and certificate of airworthiness. This is a reason to model approval path, product/model/serial applicability and limits as typed evidence—not a generic boolean "approved".

For airspace context, SACAA's [Aeronautical Charts and Information page](https://www.caa.co.za/industry-information/aeronautical-charts/) exposes AIP, AIP supplements, NOTAM summaries, aerodrome/helistop directories, registered/approved aerodromes and AIRAC/AIP amendment dates. It specifically advises users to consult the latest NOTAMs and chart hand amendments for possible changes or suspension notices. Sol-Atlas should therefore retain publication/effective dates, validity intervals and update-state; an old chart or a mapped airfield is not an operational status assertion. Exact applicability, source rights and operational procedures must be checked against the current SACAA publication for the intended aircraft category and operation.

## The paradigm shift to pursue

### 1. From model output to an evidence-carrying decision

Every consequential claim should carry the exact configuration and source revision, units/reference frame, toolchain and executable identity where available, assumptions, model applicability, uncertainty, validator version, and evidence class (synthetic, simulated, measured, calibrated, independently reproduced, reviewed). A digest proves byte identity only if the bytes are available and recomputed; it does not prove truth, approval or applicability.

A changing part, trajectory, aircraft configuration, criteria revision, calibration or dataset must invalidate dependent results or explicitly mark them for review. Historical findings must remain auditable rather than disappearing under an overwriting "current" status.

### 2. From one solver to qualified heterogeneous solvers

Do not ask Symthaea to replace CFD, structures, CAD, trajectory optimization, airspace context, measurements and certification. It should select and coordinate specialist tools, verify contracts, compare outputs and actively search for contradictions. Symtropy owns scenario execution/replay; Sol-Atlas owns read-only spatial context; Mycelix owns access-controlled provenance and attestations; the applicable manufacturer/operator/authority remains the authority for airworthiness and operational decisions.

### 3. From nominal answers to uncertainty and abstention

A nominal feasible result is insufficient when credible uncertainty, stale data, model drift or an out-of-domain input can flip the conclusion. Report false-safe, false-alarm, unknown and no-coverage cases separately. A useful assistant should be rewarded for timely abstention when required evidence is missing, not punished for declining to guess.

### 4. From isolated component metrics to mission and lifecycle outcomes

For aircraft engineering, the useful output is not the lightest wing, highest static thrust, fastest optimizer or lowest modeled manufacturing cost in isolation. Compare mission-completed payload, reserve margins, whole-life maintenance and energy, conforming-part yield, inspection/repair effort, availability, noise/emissions, affordability and resilience under the same declared constraints. Keep hard requirements as constraints, not preferences that can be traded away by a weighted score.

### 5. From the AI claim to a reproducible physical thread

Begin aircraft manufacturing integration on a benign, non-flight-critical physical artifact, then advance one assurance step at a time. Record design -> manufacturing plan -> build -> metrology/inspection -> controlled test -> independent reproduction -> design fork -> impact analysis -> revalidation obligations. This aligns with Mycelix AeroCommons rather than creating another provenance graph or a parallel certification authority.

## Benchmark portfolio

### A. Pairwise trajectory screening — current Symtropy prototype
- Maintain two independent evidence lanes: a small analytical/closed-form oracle for tractable geometry and a separately implemented exhaustive finite-domain oracle for bounded waypoint/time grids.
- Require 100% agreement on the frozen oracle corpus and zero false-safe outcomes on all known synthetic conflict cases, including exact tangencies, vertical crossings, point overlaps and uncertainty-expanded conflicts.
- Include negative controls for partial/expired criteria, mismatched frame or vertical datum, malformed time ordering, non-finite coordinates and numeric overflow.
- Track false positives separately from false negatives, and include near-boundary adversarial cases to characterize floating-point sensitivity.
- Current implementation uses local Cartesian piecewise-linear paths and fixed scalar uncertainty bounds. It does not handle geodetic conversion, dynamic aircraft feasibility, wind, live tracks, surveillance fusion or operational separation criteria.
- Scale testing should begin at 10 / 100 / 1,000 trajectory segments and then measure any indexed fleet candidate generator separately from pairwise exact screening. Report segment pairs examined, p50/p95 latency, memory and candidate recall. Do not imply 10,000-trajectory fleet-scale performance from the current pairwise function.

### B. Loading master / weight-and-balance — current Symthaea prototype
- Require exhaustive bounded-grid agreement with a separately implemented mass/moment/CG oracle, including station aggregation, negative/invalid input, exact bounds, overlapping/missing envelope slices and finite arithmetic.
- Require explicit state/phase identity. The current code accepts Takeoff only and must reject Ramp, Taxi and Landing until approved-data semantics and phase-specific calculations exist.
- Add uncertainty propagation for mass/arm/CG only after the definitions and assumptions are reviewed. A nominal CG within an envelope is not enough if its credible interval crosses a limit.
- Verify exact aircraft configuration and approved source applicability as external authority/data gates; a digest string alone is insufficient.
- Keep cargo bay geometry, floor loads, restraint forces, dangerous-goods compatibility, performance/trim and maintenance/airworthiness in separately owned, evidence-bound modules.

### C. Aircraft propulsion / airframe co-design
- Reuse existing Symthaea owners: [TRANSPORT-FIXED-WING-001 #6249](https://github.com/Luminous-Dynamics/symthaea/issues/6249), [AIRCRAFT-ENERGY-001 #7646](https://github.com/Luminous-Dynamics/symthaea/issues/7646), [PROP-OPT-001 #7809](https://github.com/Luminous-Dynamics/symthaea/issues/7809), and [AIRFAMILY-001 #7745](https://github.com/Luminous-Dynamics/symthaea/issues/7745). Do not create another universal flight solver.
- Use mission-matched baselines and independent analysis packages. State which tool and fidelity produced every physical estimate. Begin with sensitivity/gradient/uncertainty checks before multi-objective optimizers; do not trust an optimum outside the component models' validity domains.
- Compare feasible payload-range, reserve energy/fuel, climb/field performance, structural mass, lifecycle cost, maintenance access and noise/emissions under one pinned mission, weather/environmental case and operating rules.
- Use OpenVSP as a geometry input/analysis candidate and OpenMDAO/Dymos-like workflows as MDO/trajectory-optimization comparators where license and integration boundaries permit. Verify tool versions and benchmark dataset rights before acquisition.

### D. Manufacturing and lifecycle
- Reuse [Mycelix AeroCommons #3638](https://github.com/Luminous-Dynamics/mycelix/issues/3638), [AeroCommons reproducibility challenge #3643](https://github.com/Luminous-Dynamics/mycelix/issues/3643), [MFG-QDRIFT-001 #5218](https://github.com/Luminous-Dynamics/mycelix/issues/5218), and NIST digital-thread/STEP/QIF work.
- Measure conforming yield, rework, inspection time, measurement uncertainty, traceability completeness, configuration-impact accuracy, evidence invalidation correctness, parts/tool availability and lifecycle cost.
- Run paired baseline-vs-candidate trials over the same frozen workload and cluster holdouts. Separate synthetic model wins from observed physical improvement.
- First physical pilot should be non-flight-critical. No aircraft component moves toward operational use based on simulation alone.

## Suggested staged target levels

| Level | Acceptance gate | Claim permitted |
|---|---|---|
| R0 — arithmetic / semantics | Hand-computed and analytic oracles pass all enumerated small cases; malformed/unknown inputs fail closed | Research kernel implements its stated bounded contract |
| R1 — independent reproduction | Separate verifier, frozen head/data/toolchain, deterministic report and negative controls all agree | Reproducible research result on the frozen scope |
| R2 — model validity | External data/model comparison, calibration diagnostics, applicability domains and uncertainty coverage measured on held-out cases | Qualified predictive claim for named domain and conditions |
| R3 — physical correlation | Instrument/calibration identity, controlled test fixture, predeclared metrics, independent observations and measurement uncertainty | Correlated model for that tested domain—not certification |
| R4 — regulated integration | System hazard analysis, independent safety/assurance assessment, authority-approved means of compliance, conformity and required certificates | Only the exact approval granted by the competent authority |

No level should be skipped by adding more AI, tests, attestations or impressive visualization.

## Immediate next actions

1. Keep the analytic oracle integration test separate from the pairwise algorithm; the current suite enumerates 9,225 synchronized parallel-path cases, exact-boundary tests, and three metamorphic invariants (argument-order reversal, collinear waypoint subdivision, uniform time-origin translation).
2. Run exact-head cargo test, clippy and rustfmt in a supported environment; current authoring environment has not executed these tests.
3. Add randomized/property tests and an independently written finite-grid oracle before performance optimization; metamorphic invariants are necessary but not a substitute for that oracle.
4. Measure numeric behavior near boundaries; document floating-point error and never make an unconditional zero-false-safe claim based only on finite synthetic unit tests.
5. Establish an indexed broadphase only after the pairwise geometry passes its oracle. Benchmark candidate recall, not just speed.
6. Link the loading manifest/profile revision to Mycelix AeroCommons evidence and Symtropy transport-load qualification.
7. Keep all aerial-planning inputs synthetic/offline until jurisdictional source rights, frame transformations, validity, completeness and operational applicability can be reviewed.

## Sources

- FAA, [AC 120-27F — Aircraft Weight and Balance Control](https://www.faa.gov/regulations_policies/advisory_circulars/index.cfm/go/document.information/documentID/1035868) — active as of this review.
- FAA, [Aircraft Weight & Balance Handbook](https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/FAA-H-8083-1B) — consult the current handbook and addendum.
- FAA, [How it Works: Aircraft Certification](https://www.faa.gov/newsroom/how-it-works-aircraft-certification) — 23 April 2026.
- FAA, [Production Approvals](https://www.faa.gov/aircraft/air_cert/production_approvals) — design/production authorization types.
- EUROCONTROL, [Specification for Trajectory Prediction, Edition 2.0](https://www.eurocontrol.int/publication/eurocontrol-specification-trajectory-prediction) — 3 March 2017.
- NASA, [Formal Methods Research Program](https://shemesh.larc.nasa.gov/) and [Formal Analysis of Air Traffic Management Systems](https://shemesh.larc.nasa.gov/fm/fm-atm.html).
- NASA, [Air Traffic Management and Safety Project](https://www.nasa.gov/directorates/armd/aosp/atms/about-atms/) — 5 March 2026.
- OpenVSP, [3.47.0 release notes](https://openvsp.org/blogs/announcements/2026/01/23/openvsp-3-47-0-released) — 23 January 2026.
- OpenMDAO, [official framework overview](https://openmdao.org/) and [documentation](https://openmdao.org/openmdao_docs/).
- NIST, [Digital Thread for Manufacturing](https://www.nist.gov/programs-projects/digital-thread-manufacturing).

Links and source versions establish research direction only. This comparison has not run these external tools on a matched benchmark corpus and does not assert an SOTA ranking.
