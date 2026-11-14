# Planetary Defence: NEO Detection, Risk, Deflection and Verification

**Status:** research and benchmark charter; not an operational planetary-defence system  
**Baseline date:** 2026-11-08  
**Parent work:** SPACE-CAPABILITY-LEDGER-001 (#1972), BENCH-SPACE-001 (#1737), and the orbital-access / launch-chain workstreams  
**Claim ceiling:** no flight qualification, no operational impact-warning authority, and no demonstrated end-to-end asteroid-deflection capability is claimed here.

## 1. Executive position

Humanity has demonstrated that a spacecraft can measurably change an asteroid's motion. NASA's DART impact on Dimorphos in 2022 shortened the moonlet's mutual orbital period by about 33 minutes; published analyses infer an along-track velocity change of about 2.7 mm/s. Ejecta contributed material momentum transfer beyond the spacecraft's direct incident momentum.

That is an important proof of principle, not a complete planetary-defence capability. We have not yet demonstrated an operational campaign that discovers a previously unknown threatening asteroid, determines its physical and dynamical state, selects and launches a mitigation mission, and independently verifies that the full post-mitigation uncertainty distribution no longer intersects Earth.

The objective is not asteroid destruction. It is **reliable reduction of Earth-impact probability, verified by independent observations, while preserving civil-protection options**.

This programme should build on established public systems and flight evidence rather than claim to replace NASA CNEOS, JPL Horizons, the Minor Planet Center, ESA NEOCC, DART or Hera.

## 2. Current external baseline (2026-11-08)

### Detection and observation

- The Minor Planet Center (MPC) is the clearinghouse for minor-planet and comet astrometry. ADES is its preferred astrometric data-exchange format. Observation quality, timing, observatory identity, uncertainty and reduction metadata must be preserved.
- JPL CNEOS operates Sentry for long-term impact monitoring and Scout for short-arc follow-up of new candidate objects. It publishes orbit solutions, close approaches and impact-probability assessments.
- JPL Horizons provides high-precision solar-system and small-body ephemerides and can produce asteroid/comet SPK files for SPICE workflows.
- Rubin Observatory began issuing scientific alerts in February 2026; the initial release included 800,000 alerts in one night, with a planned system capacity of up to millions per night. Alert volume is not equivalent to validated NEO discoveries: candidate classification, follow-up, orbit linkage and confirmation remain necessary.
- NASA's NEO Surveyor remains a future mission as of this baseline. NASA/JPL lists launch no earlier than September 2027 and a five-year baseline survey intended to find at least two-thirds of NEOs larger than 140 m. Do not describe it as already operational.

### Deflection and characterization

- DART struck Dimorphos on 2022-09-26. The observed mutual-orbit period change was approximately −33.0 ± 1.0 minutes (3σ); published analysis inferred an along-track velocity change of 2.70 ± 0.10 mm/s (1σ). These are historical benchmark measurements.
- DART did not establish a universal momentum-transfer coefficient. The response depends on target mass, density, porosity, cohesion, shape, spin, impact geometry and ejecta physics; uncertainty in target mass materially affects inferred parameters.
- ESA's Hera spacecraft arrived at the Didymos–Dimorphos system on 2026-11-04 (JST) and entered its Early Characterization Phase. JAXA's 2026-11-06 TIRI release reports thermal images acquired on 2026-10-28/29 and 2026-10-31; the 2026-10-31 frame was taken from 57 km before arrival. The release date is not the observation date. JAXA states that Didymos surface-temperature analysis is still underway; thermal time series may later constrain thermal inertia and surface physical state, but no calibrated derived estimate is asserted here. The mission plan schedules Detailed Characterization from 2026-12-02, CubeSat deployment from 2027-01-13, close observation of the DART crater from 2027-02-03, and a closer experimental phase from 2027-04-28. The ESA/Hera arrival event is expected on 2026-11-25. Do not treat pre-arrival thermal images or early visible-light imagery as final mass, internal-structure or crater-physics results.

### Impact-monitoring algorithm baseline

CNEOS Sentry is not merely a forward Monte Carlo propagator. Its public method samples the orbital uncertainty region with virtual asteroids, screens close approaches over the monitoring horizon, and applies an impact-conditioned orbit-determination / importance-sampling step to find rare virtual impactors. This means a Symtropy implementation must be benchmarked against rare-event search completeness and probability calibration, not only nominal trajectory propagation.

Sentry-II research improves the treatment of non-Gaussian uncertainty and nongravitational effects. It is an important published research baseline, but this charter does not claim that every Sentry-II research method is deployed in the operational Sentry service. Preserve the distinction between operational Sentry, the Sentry-II paper and a Symtropy implementation.

Primary references:
- CNEOS Sentry methodology: https://cneos.jpl.nasa.gov/sentry/intro.html
- Sentry-II impact-monitoring method: https://cneos.jpl.nasa.gov/doc/roa_impact_paper.html

### Rust astrodynamics interoperability baseline

We should reuse specialist infrastructure where it demonstrably covers our requirements rather than reflexively write a new orbit engine:

- **ANISE 0.10.6 + hifitime 4.3.1:** candidate frame, ephemeris and time-scale infrastructure. ANISE is SPICE-compatible and documents lunar flight heritage; its package license is MPL-2.0. hifitime 4.3.1 is also MPL-2.0. Kernel identity, coverage, source, time conversion and frame transform behavior must be pinned and verified.
- **Nyx 2.6.x:** candidate high-fidelity propagation, estimation and mission-design backend. It is AGPL-3.0-or-later; review the exact feature/dependency/license profile before integration.
- **Brahe 1.8.x:** MIT-licensed Rust astrodynamics candidate with force models, numerical integrators and variational equations; project maturity and NEO-specific validity must be assessed rather than inferred from satellite feature coverage.
- **JPL Horizons / SBDB / CNEOS:** external reference products and operational comparison points, not libraries. Compare only after matching origin, axes, frame realization, epoch, time scale, observer, aberration, force model and ephemeris versions.

The active comparison and integration task is Symtropy [ASTRODYNAMICS-INTEROP-001 #4217](https://github.com/Luminous-Dynamics/symtropy/issues/4217). The decision gate is a backend-neutral canonical contract and blind NEO-specific comparison before choosing a default backend.

References:
- ANISE: https://docs.rs/anise/latest/anise/ and https://github.com/nyx-space/anise
- hifitime: https://docs.rs/hifitime/latest/hifitime/
- Nyx: https://github.com/nyx-space/nyx
- Brahe: https://docs.brahe.space/latest/ and https://arxiv.org/abs/2601.06452
- Horizons: https://ssd.jpl.nasa.gov/horizons/manual.html
- SBDB API: https://ssd-api.jpl.nasa.gov/doc/sbdb.html

### Reuse the existing comparative benchmark methodology

Brahe's current public benchmark suite covers 32 tasks across eight general astrodynamics modules and compares against OreKit 13.1.5, GMAT R2026a, Basilisk and Nyx/ANISE where equivalent public APIs exist. Its published protocol separates performance (200 iterations on fixed inputs) from accuracy (a sweep of 200 independent initial conditions), and reports p50/p95/p99/maximum error distributions rather than a single mean. The docs explicitly warn that comparisons must be apples-to-apples: different frames, gravity models, API setup costs and unsupported operations can create misleading comparisons.

Symtropy should reuse this protocol and task taxonomy where applicable. Do not duplicate the generic 32-task suite. Add a domain-specific NEO extension for heliocentric state propagation, observation residuals, orbit-determination consistency, encounter/B-plane geometry, virtual-impactor discovery, rare-event probability estimation, posterior calibration and time-sliced historical impact replay. Use OreKit/Brahe/Nyx/ANISE as implementation comparators, SPICE/Horizons as convention-matched ephemeris references, and CNEOS outputs as operational impact-monitoring comparisons. No single software package is ground truth for every NEO quantity; independent reference products and physical event truth must remain distinct.

Reference methodology: https://docs.brahe.space/latest/about/benchmarks.html

### What this baseline means

We can build an interoperable, reproducible research system now. We cannot honestly claim to outperform authoritative orbit determination or impact-monitoring systems before blind, held-out comparison against their products and independent truth data.

## 3. The critical domain separation

The current Mycelix Space repository is archived/read-only. Its former Earth-orbiting path uses TLE/SGP4 and CCSDS conjunction concepts. That work is useful historical context, but asteroid dynamics must not be implemented as an extension of SGP4 or by reinterpreting TLEs.

Create a separately typed **heliocentric/barycentric small-body domain**:

| Concern | Earth-orbit SSA | NEO planetary defence |
|---|---|---|
| State origin | Earth-centred | Heliocentric or solar-system barycentric |
| Typical input | TLE/OMM and tracking data | MPC astrometry, radar, occultations, spacecraft data |
| Propagation | SGP4/SDP4 for TLE products | Numerical N-body dynamics and validated ephemerides |
| Geometry | Earth-centred conjunction plane | Earth encounter plane, virtual impactors and keyholes |
| Uncertainty | Short/medium-horizon orbit covariance | Long-arc posterior; often non-Gaussian or multimodal |
| Time/frame | Explicit TEME/ITRF/ECI handling | ICRF/J2000 or declared realization; UTC/TT/TDB conversions |
| Primary risk | Object-object collision | Earth impact probability and impact consequences |

A type must carry its origin, axes, frame realization, epoch, time scale, units, force model and provenance. “ICRF” alone is not sufficient metadata when frame realization, epoch and kernel version matter.

## 4. Proposed four-system architecture

### Symtropy — validated numerical and physical models

Symtropy may host reusable mathematics, dynamics interfaces, optimization and digital-twin components. Its present rigid-body/PBD engine is not an asteroid orbit-determination solver and is not a validated shock-hydrodynamics, fragmentation or granular-ejecta code.

Recommended separation:

1. A specialist astrodynamics component for heliocentric state propagation, variational equations, uncertainty ensembles and encounter geometry. Prefer a clearly bounded new crate (for example, symtropy-astro) over adding solar-system semantics to the generic rigid-body core.
2. A separately validated small-body response model for impact, ejecta and structural response. Benchmark it against published DART observations, Hera characterization when available, laboratory impact data and specialist hydrocode references.
3. A mission-systems layer for propulsion budgets, navigation, power, thermal, communications, launch windows, reliability and manufacturing constraints. Reuse specialist solvers where they are better validated than a new implementation.

### Symthaea — scientific workflow intelligence

Use Symthaea to:
- diagnose orbit-fit residual patterns and propose testable causes;
- rank follow-up observations by expected reduction in downstream decision uncertainty;
- identify disagreement between independent orbit solutions and model assumptions;
- search experiment designs and sensitivity spaces;
- explain evidence, uncertainty and claim boundaries to engineers and decision-makers.

Do not use Symthaea as the numerical authority for an orbit, impact probability, deflection command or flight decision. Its recommendations must be reproducible, externally checked and subordinate to deterministic specialist software and human approval.

### Mycelix — provenance, collaboration and evidence

Use Mycelix to preserve:
- raw observation identifiers and immutable content digests;
- observatory, instrument, reduction pipeline and calibration metadata;
- source revision, retrieval time, license and transformations;
- orbit-solution versions and disagreements;
- simulation inputs, software commits, toolchain and ephemeris/kernel versions;
- review, sign-off, correction and supersession history.

Consensus or reputation must never be used as a substitute for physical validity. Preserve dissenting orbit solutions and their assumptions; never average incompatible solutions merely to create a consensus number.

### Sol-Atlas — decision visualization and infrastructure context

Visualize:
- heliocentric trajectories and Earth encounter geometry;
- uncertainty corridors and virtual-impactor/keyhole structure;
- observation windows and the expected value of candidate follow-up campaigns;
- alternative mitigation timelines and residual-risk distributions;
- launch-site, tracking, communications, industrial and civil-protection dependencies.

Every visualization must carry epoch, frame, units, model/version, source digest and uncertainty. A thin line representing only the nominal trajectory is not an acceptable hazard visualization.

## 5. Existing cross-repository work to compose with

The following active issues already own important shared primitives. Planetary defence should integrate with them rather than create parallel implementations:

- **Symthaea #8192 — HAZARD-VOI-001:** cost-aware experiment and observation selection for natural-hazard uncertainty. Reuse its decision-sensitive value-of-information framing for telescope follow-up, reconnaissance and model-discrimination experiments.
- **Symthaea #11112 — NEO-OBSERVATION-VOI-001:** specialize that value-of-information work for NEO observation scheduling, correlated observatories, multi-modal posterior discrimination and explicit abstention.
- **Symthaea PR #11165 — finite NEO observation VOI oracle:** a synthetic exact Rust verifier shows why decision value must outrank entropy gain, preserves shared calibration dependence, and now requires abstention when source dependence is unknown and no joint likelihood is supplied. It is a finite decision-theory control, not a real NEO observation model.
- **Mycelix #455 — qualified orbital-state profiles:** use its explicit state/frame/time/dynamics/uncertainty/provenance contract as the cross-domain reference; extend with heliocentric system identity instead of accepting a bare ECI/JD state.
- **Mycelix #5952 — NEO-EVIDENCE-001:** preserve ADES astrometry, orbit-solution provenance, observatory/source correlations, correction history and immutable content digests without duplicating raw instrument telemetry in the DHT.
- **Mycelix PR #5971 — source-correlation evidence profile v0.1:** records dependent / estimated / evidenced-independent / unknown source relations; requires a common-cause or evidence basis and prevents unknown dependence from being silently treated as independent. This is provenance, not physical correlation estimation.
- **Mycelix #413 — common spatial truth substrate:** reuse typed observation/estimate/prediction separation, covariance, reference systems, source independence, trusted time and provenance. Preserve its rule that raw telemetry does not belong in the Holochain DHT.
- **Mycelix #4020 — BootstrapMilestone provenance:** reuse content-addressed evidence references, state transitions, correction/retraction history and authority-basis separation for manufacturing/mission milestones.
- **Sol-Atlas #76 — ephemeris-backed hierarchical solar-system catalogue:** use the renderer-neutral object catalogue, JPL Horizons adapter, explicit epoch/frame/time metadata and bounded body selection. Planetary-defence layers should add uncertainty and risk semantics, not a second catalogue/ephemeris cache.
- **Sol-Atlas #640 — NEO-RISK-VIS-001:** consume qualified trajectories and posterior products to render encounter geometry, multi-modal uncertainty and source/evidence metadata; do not calculate official impact probabilities in the renderer.
- **Symtropy #1972 / #1737:** retain the historical mission capability ledger and independent verifier as the parent evidence and test infrastructure.
- **Symtropy #4217 — ASTRODYNAMICS-INTEROP-001:** compare ANISE/hifitime, Nyx and Brahe against frozen SPICE/Horizons/CNEOS references before selecting or implementing a NEO dynamics backend.
- **Symtropy PR #4219:** adds a frozen DART/Dimorphos reference record and standalone Rust data-contract verifier. Seven Rust tests are authored but not yet reported as run; this is not an orbit or impact-physics simulation.
- **Symtropy #4264 — HERA-DATA-CLOSURE-001:** independently validate DART response models against Hera observations as calibrated products are released. Keep observations, derived estimates, model outputs and held-out validation separate; do not infer final mass, internal structure or crater physics from first-light/thermal imagery.
- **Symtropy #4221 / PR #4224:** adds a historical 2008 TC3 terminal-event reconstruction summary. It is explicitly summary-only until original time-sliced astrometry and pre-impact forecast snapshots are frozen.
- **MPC acquisition route for #4221:** use the official Observations API to preserve ADES XML/ADES_DF plus legacy OBS80, and use WAMO to resolve tracklet/station or observation identifiers to submission status and timestamp-bearing submission-block IDs where available. This is a documented acquisition path, not evidence that raw 2008 TC3 records have been ingested. A current MPC response is not a historical as-of snapshot; missing submission availability must stay unknown and blocks time-sliced replay.
- **Symthaea #11112 / Mycelix #5952 / Sol-Atlas #640:** supply decision-value observation planning, astrometric provenance and source-bound uncertainty visualization.

These interfaces are dependencies and integration targets, not evidence that the planetary-defence system is already implemented.

## 6. Paradigm shifts

### 6.1 From discovery counts to completeness and recoverability

A telescope seeing many moving objects is not the same as a system knowing which objects could hit Earth. Measure survey completeness by size, albedo, orbit, solar elongation, sky coverage and observation cadence. Track the full path from transient alert → candidate linkage → confirmed object → orbit-quality threshold → risk assessment → continued recoverability.

Optimize telescope scheduling for **expected reduction in decision loss**, not only apparent magnitude or discovery count.

### 6.2 From one best-fit orbit to a calibrated posterior

The best-fit orbit is not the risk distribution. Preserve:
- correlated astrometric errors and observatory/star-catalog biases;
- alternative orbit-linkage hypotheses;
- non-Gaussian and multimodal solution structure;
- systematic model uncertainty and nongravitational effects such as Yarkovsky where material;
- covariance and sampled virtual-asteroid ensembles with explicit weights.

A Gaussian covariance is useful only where its approximation has been validated. Do not compress a multi-modal impact solution into a single mean and covariance ellipse.

### 6.3 From nominal miss distance to robust risk reduction

A mitigation is not successful because the nominal trajectory misses Earth. Evaluate the posterior encounter distribution, possible gravitational keyholes, model uncertainty and residual impact probability. The required acceptable risk threshold is an authority/governance decision, not a number Symtropy invents.

The linearized planning model can be expressed as:

δx(t) = Φ(t,t₀) δx₀ + ∫[t₀,t] Φ(t,τ) B(τ) δu(τ) dτ

where Φ is the state-transition matrix, δx₀ is initial-state perturbation, and δu represents a modeled intervention. This is a local sensitivity model, not a substitute for nonlinear propagation, posterior sampling or validated impact physics.

### 6.4 From one-shot missions to adaptive campaigns

A credible campaign can include:
1. rapid follow-up and orbit refinement;
2. reconnaissance to measure mass, shape, spin, composition and surface/structural characteristics;
3. mitigation selection conditioned on the characterized object and warning time;
4. one or more mitigation opportunities where justified;
5. independent post-action orbit determination and risk reassessment.

Not every threat will permit every phase. Short warning times, high relative velocities, high-inclination orbits, comets and large objects may sharply constrain available options. Civil protection remains necessary.

### 6.5 From ideal CAD to manufacturing-aware mission assurance

Interceptors and spacecraft are not just trajectories. Qualification must connect design, material batch, manufacturing process, nondestructive inspection, environmental testing, acceptance evidence, operations and repairability. Unknown material properties or unqualified parts must remain explicit risk terms.

### 6.6 From proprietary point solutions to an interoperable public evidence fabric

Use public standards and authoritative reference products. Exchange data through documented interfaces; preserve provenance and disagreement; make benchmark inputs and outputs reproducible. The differentiator should be integration quality, calibrated uncertainty and trustworthy evidence—not a claim that a new software stack is automatically more accurate than specialist agencies.

## 7. Mitigation concepts to compare

This is a high-level civil planetary-defence comparison, not a weapon-design specification.

| Concept | Best-fit conditions | Main uncertainty / limitation | Evidence status |
|---|---|---|---|
| Kinetic impactor | Sufficient warning time; reachable target; characterized enough physical response | Momentum transfer depends on mass, porosity, cohesion, spin, shape and ejecta | Demonstrated on Dimorphos by DART |
| Gravity tractor / slow-push concepts | Long warning time; small persistent perturbation is useful; rendezvous and station-keeping feasible | Very low acceleration; long lead time and high operational endurance | Proposed / not demonstrated as a planetary-defence mission |
| Other controlled low-thrust concepts | Long lead time and feasible close-proximity navigation | Plume/interaction physics, control, power, target coupling and duration must be validated | Research concepts; no operational defence demonstration |
| Civil protection | When mitigation is infeasible, late, incomplete or uncertain | Depends on impact location/time forecasts, warning, resilient infrastructure and evacuation readiness | Established emergency-management domain |
| Nuclear standoff options | Contingency concept considered only by authorized national/international authorities | Severe legal, governance, physical and fragmentation uncertainties | Not demonstrated; not part of initial implementation |

Do not recommend fragmentation or disruption from one nominal simulation. A fragmented object can remain dangerous, and the resulting distribution must be modeled and independently reviewed. Initial engineering scope should focus on detection, characterization, uncertainty reduction, kinetic-impact evidence replay and civil-protection interfaces.

## 8. Benchmark suite and acceptance targets

Targets below are proposed research gates, not current system capabilities or regulatory standards.

### A. Data and astrometry

- 100% parse/round-trip correctness for the selected ADES fixtures.
- Preserve original observation data and all reductions; no silent unit, time-scale or coordinate conversion.
- Report residuals by observatory, instrument, catalogue, night, sky geometry and epoch.
- Normalized residuals must be statistically calibrated on held-out data; do not set an arbitrary universal arcsecond target independent of instrument and observing conditions.
- Inject clock offsets, catalogue biases, duplicate observations, outliers and missing metadata; each must be detected, modeled or rejected explicitly.

### B. Orbit determination and propagation

- Reproduce two-body analytic cases to a declared numerical tolerance.
- Add N-body test cases with pinned planetary ephemerides and explicit force-model settings.
- Cross-compare state and ephemeris outputs with JPL Horizons/SPICE at held-out epochs after harmonizing origin, frame, time scale, units, observer and ephemeris version.
- Report position/velocity residuals, energy/angular-momentum behavior where applicable, integrator convergence order, step-size sensitivity and reproducibility.
- Compare at least two independent propagation/orbit-determination implementations; agreement between two runs of the same code is not independent validation.

### C. Uncertainty and risk

- Coverage calibration: measured empirical coverage of nominal 1σ/2σ/3σ regions on synthetic truth and held-out historical cases.
- Proper scoring rules: log score and Brier score for calibrated impact-risk forecasts where truth labels and forecast windows are well-defined.
- Reliability diagrams, false-positive burden, false-negative analysis and alert lead time.
- Multimodal posterior retention and keyhole recovery must be tested explicitly.
- Negative controls: wrong time scale, wrong frame, stale kernel, biased observatory, duplicate-night measurements, omitted nongravitational term and collapsed multimodal solution.

### D. DART / Hera physical-response replay

- Reproduce the published DART mutual-orbit change of −33.0 ± 1.0 minutes (3σ) and along-track velocity change of 2.70 ± 0.10 mm/s (1σ), with declared assumptions and independent reference inputs.
- Separate direct spacecraft momentum from ejecta momentum; quantify sensitivity to target mass and density.
- Do not claim to reproduce the crater, mass distribution or interior structure until the corresponding data and independent reference values are available.
- Add Hera-derived constraints only after publication; freeze raw data, processing versions and uncertainty metadata.

### E. Mitigation decision quality

- Compare alternatives across warning-time, size, orbit geometry, physical-class and launch-readiness strata.
- Optimize reduction in posterior impact probability and authority-defined residual-risk bounds, not a scalar nominal miss distance.
- Report sensitivity to target mass, density, porosity, cohesion, shape, spin, impact location, momentum-transfer uncertainty and orbit posterior.
- Include launch delay, navigation error, missed intercept, incomplete momentum transfer, failed reconnaissance and delayed verification scenarios.
- Keep civil protection and no-mitigation baselines in the comparison.

### F. Space engineering beyond planetary defence

The umbrella programme should also benchmark:
- chemical propulsion: measured thrust, specific impulse, mixture ratio, chamber/nozzle model correlation, thermal margins, start/restart and lifecycle reliability;
- electric propulsion: thrust, specific impulse, input power, efficiency, lifetime/endurance and plume interactions;
- spacecraft: mass/power/thermal/communications budgets, attitude and orbit control, fault detection and safe recovery;
- structures/materials: strength, fatigue, fracture, cryogenic compatibility, radiation/thermal cycling and process variability;
- manufacturing: dimensional accuracy, porosity/defect detection, inspection coverage, batch repeatability and qualification time;
- mission systems: delivered payload at the required terminal state, schedule reliability, lifecycle cost and evidence completeness.

Use flown engine, spacecraft and materials data as baselines. Do not compare a laboratory thruster test to a flight-proven propulsion system without matching operating point, lifetime, environment and maturity.

### G. Evidence and reproducibility

Every benchmark receipt must include:
- immutable input and dataset digests;
- source URLs, license/attribution, retrieval time and source revision;
- code commit, dependency lock and toolchain;
- coordinate/time conventions and ephemeris/kernel versions;
- model assumptions, configuration, output digests and uncertainty;
- test status and independent-review record.

A queued test is not a pass. A model result without source and configuration is not evidence. Synthetic fixtures must never be represented as measured observations or historical mission reproduction.

## 9. Initial work packages and ordering

### P0 — Domain and evidence contracts

- Define separate NEO object, astrometry, orbit-solution and impact-assessment types.
- Freeze schemas for source/epoch/time/frame/units/covariance/provenance.
- Record authoritative external product versions and licenses.
- Add claim ceilings to all result classes: synthetic, reference-reproduced, independently verified, hardware-tested, flight-demonstrated, operationally qualified.

### P1 — Numerical foundations

- Unit-safe heliocentric/barycentric state types.
- Time-scale and reference-frame conversion tests.
- Two-body analytic regression fixtures.
- N-body integration with pinned ephemeris inputs and convergence tests.
- State-transition and ensemble uncertainty propagation.
- Independent cross-check adapter for Horizons/SPICE.

### P2 — Observation and orbit assessment

- ADES parser and validation.
- Observation residual analysis and systematic-bias diagnostics.
- Independent orbit-solution comparison.
- Virtual-asteroid ensemble and encounter-plane probability calculation.
- Reproduction of selected public CNEOS historical cases.

### P3 — DART historical replay

- Frozen DART pre-impact and post-impact orbital data.
- Mutual-orbit model and independent period-change calculation.
- Momentum-transfer/ejecta sensitivity analysis.
- Clear distinction between model fit, parameter inference and physical validation.
- Hera data ingestion once released and attributable.

### P4 — Deflection and mission trade studies

- Warning-time and target-class scenario matrix.
- Kinetic-impactor mission feasibility and uncertainty propagation.
- Slow-push alternatives as separately typed, evidence-tiered options.
- Reconnaissance value-of-information studies.
- Independent post-mitigation verification contract.

### P5 — Industrial and civic integration

- Interceptor manufacturing and qualification traceability.
- Launch, tracking, communications and mission-operations resilience.
- International observation-sharing and independent review.
- Impact forecasting interfaces for emergency management and civil protection.
- Sol-Atlas risk and observation visualization.

## 10. Stop conditions and authority boundaries

- No autonomous mission command or deflection execution from Symthaea, Symtropy, Sol-Atlas or Mycelix.
- No public hazard declaration from an unreviewed model output.
- No single trust score, AI explanation or aggregate readiness score can overrule physical evidence.
- No mitigation success claim until independent observations confirm the post-action orbit and uncertainty envelope.
- No destructive/disruptive concept is advanced from a single nominal run or without authorized governance, specialist review and a full consequence model.
- Any material disagreement between independent orbit solutions must remain visible and block automated certainty claims.
- Flight/operational certification remains with the responsible agencies and authorities.

## 11. Research references

### Operational and observation systems
- JPL CNEOS / Sentry / Scout: https://cneos.jpl.nasa.gov/about/cneos.html
- JPL Horizons: https://ssd.jpl.nasa.gov/horizons/
- JPL small-body orbits and ephemerides: https://ssd.jpl.nasa.gov/orbits.html
- MPC astrometry reporting and ADES guidance: https://docs.minorplanetcenter.net/mpc-ops-docs/astrometry/reporting-observations/
- MPC Observations API (ADES XML/ADES_DF/OBS80; ADES 2017/2022): https://docs.minorplanetcenter.net/mpc-ops-docs/apis/get-obs/
- MPC WAMO API (submission identifiers, status and block lineage): https://docs.minorplanetcenter.net/mpc-ops-docs/apis/wamo/
- MPC Observations API tutorial: https://docs.minorplanetcenter.net/tutorials/notebooks/mpc_tutorial_api_observations/
- MPC WAMO API tutorial: https://docs.minorplanetcenter.net/tutorials/notebooks/mpc_tutorial_api_wamo/
- Rubin Observatory first scientific alerts (2026-02-25): https://rubinobs.org/news/first-alerts
- NASA/JPL NEO Surveyor (launch no earlier than September 2027): https://www.jpl.nasa.gov/missions/near-earth-object-surveyor/

### DART and Hera
- Thomas et al., “Orbital period change of Dimorphos due to the DART kinetic impact,” Nature (2023): https://www.nature.com/articles/s41586-023-05805-2
- Cheng et al., “Momentum transfer from the DART mission kinetic impact on asteroid Dimorphos,” Nature (2023): https://www.nature.com/articles/s41586-023-05878-z
- ESA Hera mission: https://www.esa.int/Space_Safety/Hera
- JAXA/ISAS TIRI initial thermal images (released 2026-11-06; includes 2026-10-28/29 and 2026-10-31 acquisition timestamps): https://www.isas.jaxa.jp/topics/004428.html
- JAXA/Hera-Japan November 2026 mission updates and arrival confirmation (2026-11-04 JST): https://hera.isas.jaxa.jp/wp-hera/en/date/2026/11/
- Hera observation/operations plan (2026-11-02; ECP, DCP, CubeSat, crater close-observation and experimental phase schedule): https://hera.isas.jaxa.jp/wp-hera/en/4827/
- ESA first separate resolution of Didymos and Dimorphos (2026-10-27): https://www.esa.int/Space_Safety/Hera/First_light_Hera_resolves_two_asteroids_instead_of_one

### Data-exchange and evidence standards
- CCSDS Orbit Data Messages, CCSDS 502.0-B-3 (with June 2026 technical corrigendum): https://ccsds.org/publications/allpubs/entry/3073/
- ISO 26900:2024 Orbit Data Messages: https://www.iso.org/standard/85529.html

---

**First measurable objective:** establish a heliocentric NEO data/physics contract and a reproducible DART historical replay before attempting novel deflection optimization. That gives us a hard scientific anchor, a useful contribution to the existing space programme, and a route to improve without claiming to exceed SOTA prematurely.
