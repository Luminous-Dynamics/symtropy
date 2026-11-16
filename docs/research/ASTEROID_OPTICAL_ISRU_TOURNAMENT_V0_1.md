# Asteroid Optical ISRU — Matched Tournament Contract v0.1

**Status:** research contract; no physical performance claim  
**Tracks:** [ASTEROID-OPTICAL-ISRU-001](https://github.com/Luminous-Dynamics/symtropy/issues/5516) and [ASTEROID-OPTICAL-ISRU-002](https://github.com/Luminous-Dynamics/symtropy/issues/5546)  
**Related application:** Lunar Photon Foundry

## 1. Purpose

This protocol compares optical and non-optical resource-processing architectures without allowing them to win by moving losses outside the measurement boundary.

The benchmark asks two different questions:

1. **Process question:** given the same feedstock batch, environment, receiver/chamber and accepted-product contract, which process produces the most independently accepted product?
2. **Systems question:** which complete deployed architecture produces the most accepted product and service for its full mass, energy, maintenance, storage, thermal-rejection and reliability burden?

These questions must never be collapsed. A process-level win does not establish a whole-system win.

This is a benchmark/evaluator contract. It is not a validated thermal, chemistry, fracture, ablation, particle-capture or orbital-dynamics solver.

## 2. Design principles

- Optimize **accepted, captured, purified and independently assayed product**, not peak temperature, emitted laser watts, removed mass or gas-sensor signal.
- Keep primary energy, electricity, optical energy, absorbed thermal energy, useful process heat and chemical energy as distinct streams with explicit meter planes.
- Preserve unknown values as unknown. Do not replace absent measurements with zero.
- Capture is part of the process, not a downstream accounting convenience.
- Treat sample morphology, porosity, volatile phase, thermal history and batch identity as causal inputs, not nuisance labels.
- Use uncertainty-aware multi-objective comparison; no opaque universal weighted score.
- Separate source-reported experiments, source-derived models, Luminous simulations, synthetic fixtures and proposed targets.
- A deterministic receipt proves replay and declared arithmetic only, not physical truth.

## 3. Tournament tracks

### Track A — matched process isolation

Freeze the feedstock batch, subsample selection, sample morphology, initial thermal state, environment, receiver/chamber, campaign duration, product specification and process-energy measurement plane.

Compare, where physically applicable:

| Lane | Process family | Primary question |
|---|---|---|
| A1 | Direct solar concentration | What does the absorbed solar-flux field do to this material? |
| A2 | Laser irradiation | Does controllable, localized flux improve useful release and capture? |
| A3 | Indirect solar-thermal receiver | Does controlled heat transfer improve selectivity, capture or fouling? |
| A4 | Resistive/electrical heating | What is the matched process baseline? |
| A5 | Hybrid optical + thermal buffer | Does buffering improve accepted output over a fixed campaign? |
| A6 | Chemistry/electrochemistry | Applicable lunar comparator; keep the process family and feedstock scope explicit. |

The process-isolation track must state whether it matches **incident energy**, **absorbed energy**, **temperature history**, **spatial flux profile**, or a subset. These are different experimental controls. If a control is not matched, the comparison is not a pure process comparison.

### Track B — whole-system architecture

Compare the complete system for the same mission service:

| Lane | Complete architecture | Burdens that must be included |
|---|---|---|
| B0 | Defer / no-build / import | Earth supply mass, launch, schedule and service dependence where comparable |
| B1 | Local solar concentrator | Collector, tracking, deployment, receiver, dust/fouling, thermal rejection |
| B2 | Nearby solar concentrator/free-flyer | B1 plus standoff range, finite solar disc, attitude, occlusion, body-relative station keeping |
| B3 | Solar-electric laser relay | Collector, electrical conversion, laser, beam quality, optics, pointing, receiver, cooling, interlocks |
| B4 | Indirect solar-thermal plant | Collector, transfer medium, insulation, receiver, pumps/flow, thermal storage and losses |
| B5 | Resistive/electrical plant | Source generation, conversion, cable/bus, heaters, storage and heat rejection |
| B6 | Hybrid autonomous plant | Full burden of each subsystem plus control, redundancy, maintenance and spares |

For lunar deployments include surface photovoltaics + storage, surface distribution and compact fission as appropriate system alternatives. For asteroids, include body rotation/tumbling, irregular gravity, local illumination, proximity navigation, station keeping and capture geometry. Do not reuse a geostationary-orbit abstraction for small bodies.

## 4. Frozen campaign identity

A run is comparable only when all identity fields match:

- target body, coordinate frame, site, epoch and illumination/visibility window;
- feedstock lot and batch digest, sampling protocol, representativeness, assay, mineralogy, morphology, grain/fragment distribution, porosity, density and thermal history;
- pressure, initial temperature, gravity regime and environmental boundary conditions;
- receiver, containment and process-article identity/revision;
- product contract: material/species, physical state, purity, contamination limit, pressure/temperature, required delivery state and assay protocol;
- campaign duration, service demand, duty cycle and availability requirement;
- calibration records, uncertainty and source/evidence class.

A matching asteroid taxonomic class is not a substitute for a matching batch assay. A matching nominal laser power is not a substitute for a matching beam profile or absorbed energy.

## 5. Material-response and capture chain

Track both solid and gas paths independently:

`feedstock inventory → liberated/ejected → captured raw stream → separated/purified → independently assayed → accepted product`

For each stream record input inventory, released mass, captured mass, escaped mass, redeposition, process residue, purification losses, contamination and unassigned residual. Report the mass-balance residual and tolerance.

For solids, record fragment-size and velocity distributions, capture geometry, rebound/re-impact, escape and collection losses.

For gases/volatiles, record species-resolved flow, containment pressure/leakage, condensation/cold-trap capture, cross-contamination, purification recovery and product storage state.

Enrichment is not refining. A detected species is not a purified product. Captured raw mass is not accepted product. Unknown streams block closure and product credit.

## 6. Energy and exergy ledger

Record separate meter planes, at minimum:

1. incident solar energy at primary collector;
2. collector optical output;
3. electrical input and output for conversion stages;
4. laser optical output at transmitter aperture;
5. optical energy incident on receiver;
6. receiver-absorbed energy;
7. useful thermal/process energy;
8. chemical energy consumed or produced;
9. auxiliary, storage, standby, tracking and thermal-rejection loads.

Useful ratios include:

- optical path: receiver-incident optical energy / transmitter-aperture optical energy;
- receiver absorption: absorbed energy / receiver-incident energy;
- process conversion: useful process energy / absorbed energy;
- accepted product yield at a declared source boundary: accepted product kg / input kWh;
- receiver-specific yield: accepted product kg / receiver-incident kWh.

A ratio is comparable only where both meter-plane identity and energy form match. Do not compare solar-incident kWh with transmitter DC kWh or receiver thermal kWh as a single homogeneous scalar. A whole-system contest must retain the complete energy vector and include conversion losses and auxiliary energy.

## 7. Primary metrics

Report the metric vector, not just a winner:

| Metric | Definition | Comparison rule |
|---|---|---|
| Accepted-product yield | Independently accepted kg / declared source-boundary kWh | Same meter plane and energy form |
| Receiver yield | Accepted kg / receiver-incident kWh | Same receiver boundary |
| Throughput | Accepted kg / campaign day | Same campaign duration |
| Capture fraction | Captured raw mass / released mass | Separate gas and solid streams |
| Product recovery | Accepted product mass / captured raw stream mass | Same product contract |
| Infrastructure productivity | Accepted kg / (deployed system kg × campaign years) | Full lifecycle boundary |
| Availability | Service-available time / required service time | Same definition and duty cycle |
| Maintenance | Spares, replacement mass, interventions and downtime | Full operating period |
| Prediction quality | Held-out error, interval coverage, OOD refusal | Held-out batches, not random rows |
| Safety/stewardship | Hard-gate violations and fail-safe disposition | No compensation by productivity |

Keep cost and Earth-delivered mass separate unless the source data support a consistent boundary and unit. Do not publish a weighted overall score as the primary result.

### Heliocentric finite-Sun invariant

For the ideal circular-collector screen, separate local solar flux, image size, total collected power and focal-plane irradiance:

`F(r) = F_1 / r^2`

`d_image(r) = L \theta_1 / r`

`C(r) = (D / d_image(r))^2`

Therefore the ideal average irradiance over the geometric image is:

`E_image(r) = F(r) C(r) = F_1 (D / (L \theta_1))^2`

For fixed collector diameter `D` and focal distance `L`, heliocentric distance cancels from this ideal average focal-plane irradiance. This does **not** mean system throughput is invariant: the total collector power still falls as `1/r^2`, while the image diameter falls as `1/r`. The result assumes perfect collection, a uniform geometric image and no receiver/thermal losses. Real sunshape, aberration, spillage, reflectance, pointing, deformation and flux nonuniformity require an optical model and measurement.

The tournament must therefore report total intercepted power, focal-image diameter, spatial irradiance map, receiver-absorbed energy and accepted product separately. It must not infer process throughput or product/day from the ideal focal-plane irradiance alone. The analytic regression implementation is in dependent draft [PR #5552](https://github.com/Luminous-Dynamics/symtropy/pull/5552).

## 8. Uncertainty-aware Pareto comparison

For each maximized metric, preserve an interval `[L, U]`. Candidate A robustly dominates B only when:

`L(A) >= U(B)`

for every compared objective, with strict improvement on at least one. Reverse the inequality for minimized metrics. If uncertainty intervals overlap, boundaries differ or an objective is missing, report **INCOMPARABLE** or **NOT_COMPARABLE**.

The evaluator must also report why a comparison is unresolved. Missing data cannot be interpreted as a poor value or an optimistic zero.

## 9. Material-aware adaptive experiment design

Symthaea may propose which experiment to run next, but selection occurs inside a frozen safe experimental envelope and a predeclared budget.

Compare the adaptive policy against equal-budget baselines:

- fixed canonical test matrix;
- one-factor-at-a-time;
- space-filling design (Latin hypercube or Sobol sequence);
- random selection.

Reserve entire feedstock batches or material-morphology families for held-out testing before tuning. Do not split near-identical subsamples from one lot between training and validation.

Report:
- information gained per experiment and per unit experiment cost;
- prediction interval coverage and sharpness;
- calibration by material and operating regime;
- held-out prediction error;
- out-of-distribution detection and refusal quality;
- candidate-to-measurement disagreement and model revision history.

A model suggestion is not a measured result. A high expected information gain is not permission to violate a safety or material-handling constraint.

## 10. External reference set and claim boundaries

| Reference | What is supported | What must not be claimed |
|---|---|---|
| NASA optical-mining demonstration (8 kW class, 10 m concentrator) | Historical vacuum experiment using high-fidelity CI-type asteroid simulant without contact/downforce | Operational asteroid mine or unreported accepted-product throughput |
| Nectar carbonaceous-chondrite experiments (2025) | Irradiance-, morphology- and brittle/ductile-regime-dependent response; organic deposition can impair performance | Universal monotonic benefit from higher irradiance |
| Cryogenic-vacuum laser mining (2026) | C-/S-type simulant test at 300/600/900 W, 30 s, 10^-2 Pa and 253 K; Fe-Ni enrichment in ejected fines | Qualified refined metal or captured yield absent corresponding evidence |
| NASA CaRD (2026) | Integrated solar-carbothermal prototype; CO detection confirmed a reaction | A deployed lunar plant or independently accepted purified O2 |
| Lunar Resources MRE (ICES 2026) | NASA NTRS reports 25 kg lunar simulant and average 0.07 kg/h oxygen during a nine-hour vacuum demonstration | An asteroid process result or a directly matched solar-thermal comparison |
| Blue Alchemist / NASA TechPort | Integrated lunar-regolith manufacturing programme and its stated technology objectives | Independently measured delivery of every advertised commodity or a flight-operational plant |
| TransAstra SolarForge | Proposed solar-powered asteroid/debris processing architecture; Capture Bag ISS demonstration reported by developer | Demonstrated asteroid resource production |
| JPL-affiliated cislunar laser-beaming model (2026) | Modeled transmission, visibility, pointing and receiver trade space | Asteroid laser-mining experiment or flight-demonstrated power service |

Primary sources:
- NASA optical mining: https://www.nasa.gov/general/optical-mining-of-asteroids-moons-and-planets-to-enable-sustainable-human-exploration-and-space-industrialization/
- Nectar study, DOI: https://doi.org/10.1016/j.actaastro.2025.04.033
- 2026 laser study, DOI: https://doi.org/10.1016/j.cherd.2026.08.007
- NASA CaRD, NTRS 20260003603: https://ntrs.nasa.gov/citations/20260003603
- NASA Lunar Resources MRE, NTRS 20260006167: https://ntrs.nasa.gov/citations/20260006167
- Blue Alchemist / NASA TechPort: https://techport.nasa.gov/projects/146991
- TransAstra: https://www.transastra.com/
- Cislunar laser beaming, DOI: https://doi.org/10.1103/hry6-dl1g

Source-access and transcription limits must be recorded. If only an abstract or summary is accessible, reproduce only those reported values; do not digitize or invent missing experimental points.

## 11. Qualification ladder

| Level | Qualification | Exit condition |
|---|---|---|
| 0 | Contract/analytic | Units, identities, geometry and hand calculations independently checked |
| 1 | Published reproduction | Source-accessible experiment boundaries reproduced with exact caveats |
| 2 | Laboratory repeatability | Independent batch assay, calibrated meters, gas/solid capture and independent product assay |
| 3 | Integrated campaign | Energy, receiver, process, containment, purification, robotics and maintenance are jointly measured |
| 4 | Environmental demonstration | Relevant vacuum/thermal/gravity and deployment constraints physically tested |
| 5 | Flight/mission demonstration | Mission-specific hardware, operations, safety and product acceptance independently qualified |

Passing one level does not silently promote the next.

## 12. Baseline comparisons and target hypotheses

Do not claim superiority until the baseline is reproduced on the same campaign contract.

Candidate future research targets, not achieved results:

- 100% rejection of seeded unknown-as-zero, boundary substitution, mass double-counting, enrichment-as-product and non-independent-assay controls.
- 95% prediction-interval coverage on held-out conditions where 95% intervals are declared, with sample size, calibration procedure and uncertainty reported.
- At least 20% improvement on one predeclared Pareto objective versus the strongest matched baseline, without regression on hard constraints. This target is eligible only after repeatability and a preregistered protocol; it is not a universal requirement or a result claimed today.
- Zero accepted products without independent product-contract evidence.
- Zero superiority claims across unlike energy boundaries or unmatched feedstock/product contracts.

The first empirical target is to reproduce known results honestly, then demonstrate repeatable improvement on new held-out feedstock.

## 13. Ownership across Luminous Dynamics

- **Symtropy:** comparison contract, accounting, independent verifier, Pareto reporting and deterministic receipts.
- **Symthaea:** architecture proposals, material-aware experiment selection and constrained multi-objective optimization. Proposed outputs never replace measured inputs.
- **Sol-Atlas:** site, frame, epoch, terrain, illumination/visibility and protected-region context. No duplicated orbit or thermal/optics solver.
- **Mycelix:** feedstock/asset identity, custody, calibration-record provenance and revocation. Provenance binds evidence; it does not self-certify physics or assays.

Compose with existing owners: Symthaea #11756/#11786/#11796; Symtropy #1803/#1840/#3458/#5341; Sol-Atlas #746/#751; Mycelix #6103. Do not create parallel universal energy, ephemeris, orbit, optics, thermal, material-ledger, custody or visualization engines.

## 14. Claim ceiling

This protocol defines fair comparison and evaluator requirements. It does not establish physical accuracy, asteroid reserves, recoverable resources, economic viability, hardware readiness, safety approval, TRL, flight qualification or commercial production. Any later physical claim requires calibrated measurements, independent assay, reproducible tests and review by appropriately qualified specialists.
