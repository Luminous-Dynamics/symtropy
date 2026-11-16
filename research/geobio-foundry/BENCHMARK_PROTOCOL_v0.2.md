# GeoBioBench Evaluation Protocol v0.2

**Date:** 2026-11-16  
**Status:** Proposed protocol for external review; no benchmark results claimed

## 1. Purpose

GeoBioBench evaluates whether a site-to-capability workflow can produce a resource-aware, risk-aware, independently reproducible engineering decision. It is not a single leaderboard score. A system that maps geology well but falsely qualifies a pressure boundary has not passed the integrated task.

## 2. Evidence tiers are separate

| Tier | Evidence | Permitted claim |
|---|---|---|
| S0 | Deterministic synthetic scenarios | Software logic and controlled fault coverage |
| S1 | Analytical or independently implemented reference solution | Numerical correctness under stated assumptions |
| S2 | Bench/laboratory test | Physical validation within the tested conditions |
| S3 | Terrestrial field analogue | Validation in the declared analogue environment |
| S4 | Orbital/spaceflight experiment | Validation under the tested spaceflight conditions |
| S5 | Destination-surface operation | Validation under the actual destination conditions |

Scores must not be pooled across tiers to imply a higher tier. S0/S1 success is not physical qualification; S2/S3 success is not flight qualification.

## 3. Benchmark definitions and proposed gates

All numerical targets below are proposed programme gates for frozen test envelopes. They are not universal safety guarantees, published industry standards or achieved GeoBio results.

| Benchmark | Primary measurements | Proposed gate |
|---|---|---|
| GeoSurveyBench | Surface/volume reconstruction error by spatial scale, coverage, interval width, calibration by geology and sensor modality | On held-out sites, nominal 90% prediction intervals should achieve 85–95% empirical coverage; masked regions must remain explicitly unobserved rather than be confidently filled |
| GeoStabilityBench | Failure-probability calibration, deformation error, false-safe and false-alarm counts, model discrepancy | Zero false-safe classifications in the finite high-consequence challenge set; Brier score and calibration must be compared with the strongest reproducible FEM/reliability baseline |
| PressureBench | Leak-rate detection threshold, detection probability, false alarms, isolation latency, pressure decay, compartment mass balance, cycle history | Detect every seeded critical leak above the calibrated detection limit in the declared fault matrix; no critical isolation failure in that finite matrix |
| ISRUProcessBench | kg on-spec product/kg feed, kg product/kWh, purity, throughput, thermal rejection, consumables, uptime, residue/byproduct quality | Reproduce a published or independently measured baseline first; only claim improvement when the primary resource metric improves beyond uncertainty at matched feedstock, purity, environment, and system boundary |
| BioAsteroidRepBench | Element-specific leachate concentration and recovered product, biological/abiotic effect size, confidence interval, medium burden, replicate variance | Reproduce the published protocol and controls before changing strain, medium or feedstock; report effect uncertainty and independent batch variance rather than targeting a headline fold-change |
| BioProcessBench | Yield, productivity, purity, downstream recovery, contamination, batch variability, sensor drift, downtime | Independently repeated batches with full upstream/downstream mass and energy accounting; no unmeasured process stream treated as negligible |
| MultiScaleBioBench | Spatial/temporal gradients, residence-time distribution, local sensor histories, physiology proxy and scale-up prediction error | Beat a well-mixed baseline on held-out operating histories without regressing product quality or stability |
| BioLoopBench | Elemental and water inventories, recovery, purge, make-up, detection limits, uncertainty | A loop may be called balanced only when every relevant stream is quantified and the residual is consistent with propagated measurement uncertainty |
| ContactSequenceBench | Approach/contact impulse, attitude disturbance, anchor/stabilize success, sample recovery, disengagement, retreat margin and final state | No loss of controllability or unrecoverable robot state in the frozen challenge suite; task-chain success and uncertainty reported across the declared stochastic distribution |
| FailureCascadeBench | Fault-detection time, service continuity, unsafe-state duration, isolation/recovery time | Zero hard safety-invariant violations in the declared finite fault suite; safe shutdown or abstention counts as correct behavior |
| ValueOfInformationBench | Expected decision regret, posterior uncertainty, measurement mass/energy/time, decision stability | At an equal observation budget, target at least 20% less decision regret than a declared passive/uniform-sampling baseline; preregister utility and cost |
| EvidenceAdversaryBench | False acceptance/rejection, stale/falsified/replayed claim detection, provenance preservation, authorization state | Reject all known invalid claims in the frozen adversarial suite; zero false acceptance in that suite |
| CrossSiteTransferBench | Out-of-domain detection, calibration shift, error by geology/mineralogy and site | Hold out entire sites and mineralogical batches; no random point split leakage; system must abstain when beyond its validated domain |
| EconomyBench | Lifecycle cost and imported mass per delivered functional unit, energy, water, spares, maintenance, downtime, waste and uncertainty | No superiority claim from a single-point TEA; comparison must show assumptions, sensitivity, uncertainty intervals and matched product specifications |

## 4. Baselines to use

| GeoBio problem | Minimum comparable baseline |
|---|---|
| Geological mapping | Classical photogrammetry/LiDAR/GPR processing, point-cloud registration, conventional Bayesian mapping, and NASA CADRE mapping concepts where comparable |
| Subsurface stability | Analytical rock mechanics where applicable, conventional FEM, reliability/probabilistic analysis, and published real-scene lava-tube methods |
| Excavation and construction | Conventional excavation planning, NASA IPEx design objective, ARMADAS modular robotic assembly concepts |
| Regolith processing | NASA MRE, CaRD and Blue Alchemist reported prototype results; matched abiotic processes and laboratory energy/purity measurements |
| Bioleaching | Matched abiotic leach, organism-only and medium controls, published BioAsteroid protocol where reproducible |
| Bioreactor control | Well-mixed model, conventional PID/MPC, multiscale CFD/physiology model where data permit |
| Asteroid contact | Mission-heritage TAG/short-contact and rapid-retreat concepts, independent contact-dynamics model, integrated task-sequence model |
| Resource allocation | Greedy, linear programming and robust optimization under uncertainty |
| Fault recovery | Deterministic state machine, conventional fault detection, safety-shielded planner |
| Evidence integrity | Cryptographic provenance checks plus deliberately stale, forged, replayed, correlated and unauthorized negative controls |

A baseline must use the same data, test envelope, product specification, resource boundary and available information. If a reference implementation cannot be reproduced, document that limitation and avoid claiming superiority over it.

## 5. Experimental design rules

1. Publish interfaces, units, scoring code and acceptance criteria before blind evaluation.
2. Split by geological site, mineralogical batch, reactor scale, operating history and mission sequence—not random rows from the same source.
3. Freeze blind labels, input hashes, model versions, random seeds and scenario configurations.
4. Preregister primary endpoints, secondary endpoints, sample size, exclusions, stopping criteria, uncertainty method and multiplicity correction.
5. Include ablations: terrain-only, physics-only, process accounting, Symthaea planning, Sol-Atlas evidence, Mycelix provenance/authorization and independent instruments.
6. Report median, tails, confidence intervals, calibration, abstention and catastrophic failures.
7. Include negative controls: missing units, non-finite values, missing assays, unmodelled fractures, sensor bias, stale calibration, process mass imbalance, pressure leaks, contamination flags and replayed evidence.
8. Publish raw observations where permitted, derived products, code version, environment, hashes and a reproduction path.
9. Keep simulated, laboratory, field-analogue, orbital and surface results on separate scoreboards.
10. Do not change a benchmark after seeing blind results without versioning it and preserving the previous score.

## 6. Evidence and claim dispositions

Every run must end in exactly one declared disposition:
- **PASS:** declared criteria met within the specified test envelope.
- **FAIL:** one or more criteria violated.
- **UNQUANTIFIED:** required measurement, uncertainty or provenance is missing.
- **ABSTAIN:** the system correctly refused to make a claim or execute a proposed action.
- **INVALID:** the test fixture or instrumentation violated its own protocol.

Do not convert UNQUANTIFIED or INVALID into PASS. ABSTAIN is a successful safety outcome when evidence is insufficient or the case is out of domain.

## 7. Superiority-claim rule

GeoBio may claim superiority over a baseline only if:
- the comparator is the strongest reproducible baseline for the test tier;
- candidate and baseline receive equivalent data and resource boundaries;
- the primary metric and statistical test were preregistered;
- improvement exceeds measurement uncertainty and is statistically supported;
- no safety, purity, availability, reliability or evidence metric regresses beyond a declared margin;
- an independent reviewer reproduces the result;
- evidence tier and limits are explicit.

## 8. Initial release gate

GeoBioBench v0.2 is ready for external review when:
- synthetic datasets exist for software-only benchmarks;
- analytical reference cases exist for mass/elemental balance and pressure compartment mass balance;
- scoring code has unit tests and negative controls;
- blind inputs and scoring hashes are frozen;
- an independent reviewer can reproduce at least one scorecard;
- all proposed thresholds are clearly distinguished from measured achievements.
