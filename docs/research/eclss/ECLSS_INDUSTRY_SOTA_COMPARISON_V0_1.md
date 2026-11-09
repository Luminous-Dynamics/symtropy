# ECLSS Industry / SOTA Comparison Protocol v0.1

**Research cutoff:** 9 November 2026  
**Parent:** [ECLSS-SUPPORTABILITY-001 #4298](https://github.com/Luminous-Dynamics/symtropy/issues/4298)  
**Implementation:** [draft PR #4301](https://github.com/Luminous-Dynamics/symtropy/pull/4301)  
**Status:** comparator specification; no algorithmic superiority claim

## 1. The competitive baseline is stronger than a simple rule-based maintainer

This project must not claim novelty for any of the following in isolation: stochastic component failures, mission-timeline repair simulation, spare-parts accounting, condition-based maintenance, joint maintenance/spares optimization, or chance constraints. Existing methods already cover these.

| Baseline | What the literature/tool covers | What this benchmark must compare fairly | Evidence / limitation |
|---|---|---|---|
| NASA maintenance-management requirements / condition monitoring | NASA's human-spaceflight hardware maintenance guidance requires maintenance procedure/status records, critical-step status, restored-function evidence, replacement-unit history and procedure-execution records; it argues condition monitoring can reduce ineffective interval-based PM, while opening/servicing introduces human-error and hardware-damage risk | Use as a direct process/assurance checklist: measure condition-data coverage and trust, false deferrals, premature PM, procedure adherence, task steps, post-maintenance verification, repeat-fault rate and maintenance-induced damage. Compare interval-based, condition-based and hybrid triggers under identical sensor quality and authority limits | Guidance/requirements are not a benchmark dataset and do not prescribe a universal ECLSS maintenance algorithm. Condition-based replacement must remain inside approved life/inspection limits; do not extrapolate beyond configuration-specific qualification |
| NASA ECLSS ground-test / Earth-Independent Operations programme (FY2026 planning) | NASA describes an integrated ECLSS ground-test system for component testing/autonomous-control demonstration and an EIO architecture combining large-data analytics with rule-based system models for diagnostics/situational awareness with reduced real-time Earth support | Compare rule-based diagnostics, data-driven anomaly detection and hybrid diagnosis on identical seeded faults, sensor dropouts/drift, telemetry latency, maintenance logs and ground-truth fault labels; measure false diagnosis, detection delay, calibration, crew workload and safe abstention | FY2026 programme/planning status, not demonstrated end-to-end autonomous life-support capability. Do not imply NASA has deployed or qualified such autonomy for a crewed mission |
| NASA Integrated ECLSS Ground Testbed (ICES-2025 programme) | NASA's public ICES-2025 abstract says ISS cannot be used to fully understand reliability because crew lives depend on the operational systems; the ground testbed is being developed to investigate reliability, spares/mass, performance and Earth-Independent Operations | Treat the testbed as the high-value physical validation venue: match subsystem configuration, fault injections, crew/robot task, parts, service floors and resource boundaries; compare measured service and maintenance receipts to simulation predictions | Public NTRS record is an abstract/status description, not a released full telemetry/event dataset or measured benchmark result. Do not claim testbed access, completion or system performance from the abstract |
| NASA Langley EMAT | Mission-timeline simulation; stochastic component failures; repair actions; spare-part demand; mission and system configuration inputs | Failure/repair/spare trajectory, task assumptions, life-limited items, mass/volume, mission service loss and resource burden | NASA's EMAT paper documents the tool and input structure; public paper is not a current, executable, independently reproducible full ECLSS data package |
| ELISSA / RELISSA | ECLSS failure simulation, backup activation and reliability trends; RELISSA links reliability simulation results to ESM mass, volume, power and heat | Same component graph and reliability assumptions; backup/dependency closure; reliability curves; ESM vector; model-run budget | ICES-2025-127 describes RELISSA as an established comparator; it notes no integrated crew-time accounting in the described RELISSA approach |
| ESA MELiSSA / Barcelona Pilot Plant / Concordia greywater | Long-term ground integration of biological compartments, mechanistic process models and control; ARTEMISS 2024 demonstrated CO2-to-O2 conversion kinetics in orbit; nine flight experiments reported; Concordia greywater system reports >90% greywater recovery and 11 years autonomous operation | Compare only matched process boundaries: CO2 conversion, oxygen production, bioreactor stability, accepted edible output, greywater quality/recovery, process control and component maintenance. Keep pilot-plant compartments, ARTEMISS flight experiment and Concordia greywater system as separate evidence arms | MELiSSA pilot-plant continuous operation is ground/pilot evidence; ARTEMISS validates a narrow in-orbit conversion result, not a complete regenerative habitat; Concordia is a greywater subsystem, not total ECLSS closure. Programme-reported figures must retain their source and boundary |
| SmartHab stochastic MILP (2024) | Telemetry-informed, chance-constrained stochastic mixed-integer optimization jointly selects maintenance and spare provisioning for deep-space habitats | Same mission horizon, telemetry, failure data, spare catalog, service demand, risk limit and resource budget; compare service probability, stockouts, cost and solve time | Rozas, Basciftci and Gebraeel, *Acta Astronautica* 214 (Jan 2024), DOI 10.1016/j.actaastro.2023.10.028; evaluated through simulation studies, not a flight demonstration |
| Distributionally robust CBM + spares MILP (2026) | Uses remaining-life distributions from prognostics; discrepancy-based ambiguity sets and distributionally robust chance constraints; exact MILP reformulation for its stated formulation; wind-turbine simulation case | Test sensitivity to biased/noisy RUL distributions, multiple non-identical components, spare diversity, opportunistic maintenance, and the same ambiguity set / compute budget | Rozas, Xie, Gebraeel and Robinson, *European Journal of Operational Research* 328 (2026), DOI 10.1016/j.ejor.2025.06.025; wind-turbine case study, not spaceflight/ECLSS validation |
| COMPASS stochastic O&M (2026, photovoltaic fleet) | Two-stage stochastic MIP integrates sensor-driven condition maintenance, crew routing, spare inventory/lead times and production planning; 30-day receding-horizon operation; reported experiments on 300 assets across 25 sites | Compare schedule cost, asset downtime, spare inventory, crew travel/availability and service/production under identical task, lead-time and data assumptions. Adapt the planning layer's problem structure—not the PV process model—to ECLSS cases | Strong industrial logistics comparator outside ECLSS; PV generation/price/maintenance distributions and 300-asset fleet are not equivalent to human-rated life support, and the article's reported results are not ECLSS validation |
| Deterministic and robust finite-scenario references | Transparent rules, exact small-instance policies and finite model-set robustness | Preserve non-probabilistic model uncertainty; compare maximin/minimax-regret or predeclared model-wise criteria without fabricating model probabilities | Compose Symtropy #1752, #4057 and #3171; do not create a second policy optimizer |

Primary references:

- NASA Langley, *Design and Application of the Exploration Maintainability Analysis Tool (EMAT)*: https://ntrs.nasa.gov/citations/20120015419
- Chen, Rivas and Rangel, ICES-2025-127: https://ntrs.nasa.gov/citations/20250003955
- NASA Human Spaceflight Hardware Maintenance / Condition Monitoring guidance: https://sma.nasa.gov/sma-disciplines/hardware-and-equipment
- NASA FY2026 Technical Supplement (Integrated ECLSS Ground Test System / Earth-Independent Operations programme): https://www.nasa.gov/wp-content/uploads/2025/09/fy2026-nasa-technical-supplement.pdf
- Rozas, Basciftci and Gebraeel, *Data-driven joint optimization of maintenance and spare parts provisioning for deep space habitats: A stochastic programming approach*, Acta Astronautica 214 (2024), 167–181: https://doi.org/10.1016/j.actaastro.2023.10.028
- Rozas, Xie, Gebraeel and Robinson, *Data-driven joint optimization of maintenance and spare parts provisioning: A distributionally robust approach*, European Journal of Operational Research 328 (2026), 122–136: https://doi.org/10.1016/j.ejor.2025.06.025
- Sahin, Zhao, Qiu and Yildirim, *A stochastic optimization framework for condition-based maintenance, crew management, and spare parts logistics in solar energy systems*, Renewable Energy (15 August 2026), DOI 10.1016/j.renene.2026.125878: https://www.sciencedirect.com/science/article/pii/S0960148126007044
- ICES-2026-379, NASA ISS Exploration ECLSS testbed status: https://ntrs.nasa.gov/citations/20260003280

### Ground testbed as the decisive physical comparator

NASA's ICES-2025 ground-testbed abstract explicitly states that the ISS cannot be used to fully investigate ECLSS reliability because the crew depends on those operational systems. NASA is developing an integrated ground testbed to investigate reliability, spare mass, performance and Earth-Independent Operations. This aligns with the benchmark's staged path: use published ISS records to define reproducible cases, then compare against controlled fault/maintenance campaigns in a ground testbed where failures can be safely injected and components repeatedly serviced.

Public source: https://ntrs.nasa.gov/citations/20240014460

The public NTRS record is an abstract/status description, not a machine-readable telemetry dataset. Treat testbed capability and intended research as programme context until full configurations, procedures, measurements and results are accessible. The benchmark does not claim that Symtropy has access to NASA hardware or that NASA has endorsed this protocol.

## 2. What is actually novel and testable here

The proposed contribution is an **evidence-qualified service-and-sustainment evaluation layer** that composes the above methods with a richer, explicit system boundary. It is not a replacement for EMAT, RELISSA or a MILP optimizer.

1. **Accepted service, not component uptime alone.** A functioning pump or a high recovery fraction earns credit only when its output meets the receiving consumer's quality and quantity contract at the required time.
2. **Finite, configuration-qualified spares.** A stocked ORU is not available if its internal configuration, revision, interfaces, shelf condition or functional equivalence is wrong. NASA's 2026 FBCO2 Air Save Pump case is a concrete adversarial example.
3. **Maintenance as a scheduled resource-constrained operation.** MTTR, hands-on R&R, gather/stow, diagnosis, access, isolation, requalification, crew/robot skill and competing tasks remain different quantities. Opportunistic work can save downtime but must be evaluated with actual task precedence and system isolation constraints.
4. **Multiple competing life limits.** Stochastic failure, preventive calendar replacement, throughput limits, shelf life, sensor drift and contamination/fouling cannot be collapsed into one MTBF or one scalar "life" field.
5. **Post-repair evidence.** A repair claim or restored nominal flow is not service restoration. Exact configuration, post-repair acceptance, sensor/calibration state and dependent-service recomputation must pass.
6. **Model uncertainty without false precision.** Use stochastic or distributionally robust optimization when the required prognostic and calibration data justify it. Where they do not, retain a finite defensible model set, interval bounds or explicit unresolved state and evaluate worst-case / regret criteria. Do not manufacture probabilities from sparse maintenance narratives.
7. **Evidence-domain applicability.** ISS microgravity demonstration results cannot silently qualify partial-gravity lunar/Martian surface operation or a different cabin pressure. A technology demo cannot silently become a critical certified service.

This is a compositional interoperability and assurance contribution. It succeeds only if it can reproduce existing baseline outcomes on matched scopes, reveal their known limitations and provide more complete service/sustainment outcomes without changing their assumptions invisibly.

### Proof-carrying operations as an assurance contribution

The benchmark's proposed differentiator is not a new optimizer. It is to wrap schedule proposals (including EMAT/RELISSA/SmartHab/DRCC outputs) in an independently checkable **maintenance-plan envelope**: exact asset/configuration, procedure, isolation, staff/tool qualification, parts compatibility and reservation, service-floor margin, resource vector, authorization scope, and post-repair requalification. The plan must be rejectable or deferred when evidence is missing or stale. Execution receipts and after-tests are separate objects from plans. This complements existing reliability and optimization methods; it is not a claim that their tools lack all assurance features.

## 3. Fair comparison with regenerative biological life support

MELiSSA must be treated as a genuine technical comparator, but its different evidence layers must not be merged:

- **Barcelona Pilot Plant:** connected biological and cultivation compartments with continuous ground operation and model-based process control. Compare biological process dynamics, elemental closure, product quality, stability and long-duration operation only against matched compartments and boundaries.
- **ARTEMISS 2024:** a flight experiment that demonstrated CO2-to-O2 conversion kinetics. It is a narrow orbital process result, not a crew-equivalent ECLSS or complete food/water/air loop.
- **Nine reported flight experiments:** a programme-level count of precursor experiments; it is not nine complete regenerative habitats.
- **Concordia greywater:** the MELiSSA Foundation reports greater than 90% greywater recovery and eleven years of autonomous operation with very little maintenance. This is a remote-site greywater reference only. Do not compare it directly with total water recovery, potable output, oxygen recovery or a complete human ECLSS.

Sources:
- Pilot plant: https://webs.uab.cat/melissapilotplant/en/the-melissa-pilot-plant/
- Peer-reviewed compartment integration: https://www.frontiersin.org/journals/astronomy-and-space-sciences/articles/10.3389/fspas.2021.750616/full
- Ground demonstration and Concordia: https://melissafoundation.org/the-melissa-program/ground-demonstration/
- Flight experiments and ARTEMISS: https://melissafoundation.org/the-melissa-program/flight-experiments/
- MELiSSA modeling/control: https://melissafoundation.org/the-melissa-program/research-development/modelling-control/

A fair Symtropy contribution is **not** to claim superior biological process control, microbial ecology, or loop closure. The complementary research target is to compose existing biological/physicochemical process models with source-bounded evidence, material/component lineage, service-quality contracts, maintenance and requalification, finite spares/consumables, common resource ledgers and independent replay. Biological safety and ecological structure remain with the domain owners (#2244 and #2206); this ECLSS adapter consumes their qualified outputs and keeps structural biological uncertainty explicit.

### Source data semantic safeguard

The benchmark source register now keeps component R&R and Table 1 crew task times as separate data classes. In particular, Table 5 R&R values are not silently treated as Table 1 crew-hours, and the UPA Distillation Assembly's 7 h operational MTTR is not conflated with its separate 3 h Table 1 crew R&R task. This is necessary for fair comparison with EMAT/RELISSA/SmartHab/DRCC models that distinguish repair duration from crew task time.

## 4. Mandatory paired-comparison protocol

Every method receives the same frozen:
- component graph, installed configuration and functional dependencies;
- initial component ages/condition, consumables, qualified spares and storage state;
- service demand, product-quality thresholds and critical service floors;
- hazard timeline, environment, common-cause events and no-resupply interval;
- sensors, calibration, observation latency, missingness and evidence budget;
- crew/robot availability, skill constraints, tools, task access and maintenance windows;
- mass, volume, power, heat-rejection, consumable, crew-time and monetary budgets;
- planning horizon, risk/ambiguity assumptions and wall-clock/solver budget.

Different information, different probabilities, different risk tolerance, different solver budgets or different repair authority define distinct experimental arms. They cannot be hidden inside a single leaderboard row.

Baselines must include:
1. no repair / run-to-failure diagnostic;
2. fixed preventive replacement;
3. reactive threshold repair;
4. rule-based service-floor/reserve-first maintenance;
5. EMAT / RELISSA or a faithful documented reproduction, when executable implementation and matched data are available;
6. 2024 SmartHab stochastic MILP reproduction on its supported assumptions;
7. distributionally robust CBM/spares method on an explicitly declared ambiguity set when its RUL and sensor assumptions are available;
8. finite-model robust policy from the existing Symtropy owner;
9. perfect-information / no-failure oracle bounds, clearly labelled unattainable diagnostics only.

If an external tool is inaccessible or cannot accept identical inputs, report a **feature / assumption coverage comparison**, not a runtime or performance ranking.

## 5. Metric vector

Report per scenario and aggregated only with visible uncertainty:

**Service:** accepted delivery per stream/interval; unmet demand; critical-floor breach count, duration and severity; first service loss; degraded-mode time; restoration and recurrence.

**Reliability and evidence:** failures by type; reliability/availability only under justified probability models; calibration/coverage of remaining-life distributions; sensor false-negative and false-positive controls; out-of-domain and unresolved rates; model disagreement.

**Sustainment:** spare mass/volume and stockouts; qualified/interchangeable fraction; resupply demand; consumable expiry; maintenance events; diagnosis, hands-on R&R, logistics/gather/stow, isolation and requalification times separately; crew/robot hours; spare and tool burden.

**Physical resources:** mass/element balance; electrical energy; heat by temperature grade; heat rejection; material/chemical residuals; power/thermal service curtailment.

**Optimization:** total and lower-tail service loss; expected value only when probabilities are justified; worst-case service loss over the admitted uncertainty set; minimax regret; resource/Pareto frontier; solve time, memory, optimality gap and timeout/abstention.

Do not report a scalar leaderboard score that can trade a protected service-floor failure for lower spares mass or crew time.

## 6. Proposed benchmark targets — hypotheses, not results

- **Hard controls:** every frozen critical negative control must reject false availability, unqualified output, wrong-revision spare, missing repair test, stale sensor and unsupported environment. Zero false accepts on the frozen deterministic corpus is a software acceptance criterion, not a statistical safety claim.
- **Conservation:** exact or declared-tolerance closure for each independent mass/element/resource ledger; every residual and rejected output accounted for.
- **Reproducibility:** deterministic exact-head replay and source/input/output digests for the reference fixtures.
- **Decision quality:** after a common baseline is established, target at least 20% reduction in held-out worst-case service-loss regret (for finite model sets) or a predeclared lower-tail service-loss measure (for calibrated probability models) relative to the strongest transparent baseline, with no service-floor regression.
- **Sustainment efficiency:** report Pareto improvements in spare mass/volume, crew-hours and maintenance downtime; do not set universal percentages before the comparator set and ground data are stable.
- **Distribution shift:** test held-out component families, usage/duty-cycle shifts, sensor drift, maintenance-time shifts, supplier/part changes and environment/applicability changes. A policy must either remain within predeclared bounds or explicitly escalate/abstain.

A claimed SOTA improvement requires an executable baseline, identical frozen inputs, predeclared metrics, independent outcome recomputation and a complete accounting of constraints/compute. Source comparison alone does not establish superiority.

## 7. Ablations and falsification

Required ablations:
- remove product-quality gating;
- collapse distinct life limits into MTBF;
- omit crew logistics/gather/stow;
- permit unqualified substitute spares;
- allow repair completion without requalification;
- use a single nominal RUL distribution versus a declared ambiguity set;
- remove common-cause dependencies;
- give a policy future-event or privileged observation leakage as a negative control;
- remove time/energy/crew/spare budgets from the optimizer;
- compare point estimates versus interval/model-set uncertainty.

The benchmark should make a plausible policy fail for explainable reasons when one of these controls is removed. This is stronger evidence than a high average score on only nominal cases.

### Condition-policy evaluation and maintenance-induced risk

The maintenance-management comparison is incomplete unless the benchmark treats maintenance itself as a potentially harmful intervention. The paired-policy evaluation should compare approved interval-based, condition-based and hybrid maintenance on one frozen event/observation trace. Seed false-positive alarms, sensor drift, blocked sensors, common-mode bias, delayed/missing telemetry and out-of-domain evidence. Also seed intervention-induced damage, contamination, leakage, connector/valve damage or configuration error so unnecessary maintenance is not assumed harmless.

Score net accepted service, missed/late maintenance, false deferral, unnecessary PM, maintenance-induced damage, service interruption, spares, crew/tool time and post-repair acceptance. Keep the #1752 paired-policy engine as the optimizer/evaluator; this ECLSS benchmark supplies domain fixtures and metric boundaries. A condition-monitoring model cannot extend an approved life limit or authorize a task solely from its prediction.

## 8. Proof-carrying plan admissibility as a benchmark layer

The Rust validator now includes a **13-case proof-carrying-plan corpus** with one admissible plan and twelve negative controls. It checks configuration-matched spares, shelf-life, isolation, service floors, worker skills/availability, tool conflicts, spare reservations, authorization scope/expiry, procedure approval, task windows, dependencies and post-repair tests. This is an independent evidence-contract layer around any schedule generated by EMAT, RELISSA, SmartHab, a DRCC MILP or a rule-based planner; it is not a replacement optimizer and cannot authorize execution. Every comparator should be evaluated both on schedule quality and on whether its proposed tasks are independently admissible.

## 9. Execution sequence

1. Preserve the selected public ICES-2025 component rows as source records, not a statistically complete raw maintenance dataset.
2. Use the 2026 ISS operational cases as scenario narratives and configuration / supportability controls, not invented event-rate samples.
3. Keep AOGA programme schedule variance unresolved until a primary source establishes which milestone is actually achieved (mod-kit delivery, flight hardware completion, launch, installation or demonstration are different events).
4. Add exact small maintenance-scheduling and spares cases to the paired policy owner #1752; reuse #4057 / #3171 for finite-model and shift logic.
5. Seek executable/public implementations and matched input formats for EMAT/RELISSA and the 2024/2026 optimization formulations before any performance ranking.
6. Only then add a metered uncrewed ground ECLSS analogue with independently assayed outputs, recorded maintenance actions and seeded, bounded failure tests.

**Claim ceiling:** this protocol defines a fair comparison plan and evidence boundary. It does not establish that Symtropy outperforms EMAT, RELISSA, SmartHab optimization, the DRCC method, NASA, ESA or any operational ECLSS.
