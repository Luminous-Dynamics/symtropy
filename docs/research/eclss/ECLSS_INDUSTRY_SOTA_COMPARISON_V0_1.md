# ECLSS Industry / SOTA Comparison Protocol v0.1

**Research cutoff:** 9 November 2026  
**Parent:** [ECLSS-SUPPORTABILITY-001 #4298](https://github.com/Luminous-Dynamics/symtropy/issues/4298)  
**Implementation:** [draft PR #4301](https://github.com/Luminous-Dynamics/symtropy/pull/4301)  
**Status:** comparator specification; no algorithmic superiority claim

## 1. The competitive baseline is stronger than a simple rule-based maintainer

This project must not claim novelty for any of the following in isolation: stochastic component failures, mission-timeline repair simulation, spare-parts accounting, condition-based maintenance, joint maintenance/spares optimization, or chance constraints. Existing methods already cover these.

| Baseline | What the literature/tool covers | What this benchmark must compare fairly | Evidence / limitation |
|---|---|---|---|
| NASA Langley EMAT | Mission-timeline simulation; stochastic component failures; repair actions; spare-part demand; mission and system configuration inputs | Failure/repair/spare trajectory, task assumptions, life-limited items, mass/volume, mission service loss and resource burden | NASA's EMAT paper documents the tool and input structure; public paper is not a current, executable, independently reproducible full ECLSS data package |
| ELISSA / RELISSA | ECLSS failure simulation, backup activation and reliability trends; RELISSA links reliability simulation results to ESM mass, volume, power and heat | Same component graph and reliability assumptions; backup/dependency closure; reliability curves; ESM vector; model-run budget | ICES-2025-127 describes RELISSA as an established comparator; it notes no integrated crew-time accounting in the described RELISSA approach |
| SmartHab stochastic MILP (2024) | Telemetry-informed, chance-constrained stochastic mixed-integer optimization jointly selects maintenance and spare provisioning for deep-space habitats | Same mission horizon, telemetry, failure data, spare catalog, service demand, risk limit and resource budget; compare service probability, stockouts, cost and solve time | Rozas, Basciftci and Gebraeel, *Acta Astronautica* 214 (Jan 2024), DOI 10.1016/j.actaastro.2023.10.028; evaluated through simulation studies, not a flight demonstration |
| Distributionally robust CBM + spares MILP (2026) | Uses remaining-life distributions from prognostics; discrepancy-based ambiguity sets and distributionally robust chance constraints; exact MILP reformulation for its stated formulation; wind-turbine simulation case | Test sensitivity to biased/noisy RUL distributions, multiple non-identical components, spare diversity, opportunistic maintenance, and the same ambiguity set / compute budget | Rozas, Xie, Gebraeel and Robinson, *European Journal of Operational Research* 328 (2026), DOI 10.1016/j.ejor.2025.06.025; wind-turbine case study, not spaceflight/ECLSS validation |
| Deterministic and robust finite-scenario references | Transparent rules, exact small-instance policies and finite model-set robustness | Preserve non-probabilistic model uncertainty; compare maximin/minimax-regret or predeclared model-wise criteria without fabricating model probabilities | Compose Symtropy #1752, #4057 and #3171; do not create a second policy optimizer |

Primary references:

- NASA Langley, *Design and Application of the Exploration Maintainability Analysis Tool (EMAT)*: https://ntrs.nasa.gov/citations/20120015419
- Chen, Rivas and Rangel, ICES-2025-127: https://ntrs.nasa.gov/citations/20250003955
- Rozas, Basciftci and Gebraeel, *Data-driven joint optimization of maintenance and spare parts provisioning for deep space habitats: A stochastic programming approach*, Acta Astronautica 214 (2024), 167–181: https://doi.org/10.1016/j.actaastro.2023.10.028
- Rozas, Xie, Gebraeel and Robinson, *Data-driven joint optimization of maintenance and spare parts provisioning: A distributionally robust approach*, European Journal of Operational Research 328 (2026), 122–136: https://doi.org/10.1016/j.ejor.2025.06.025
- ICES-2026-379, NASA ISS Exploration ECLSS testbed status: https://ntrs.nasa.gov/citations/20260003280

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

## 3. Mandatory paired-comparison protocol

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

## 4. Metric vector

Report per scenario and aggregated only with visible uncertainty:

**Service:** accepted delivery per stream/interval; unmet demand; critical-floor breach count, duration and severity; first service loss; degraded-mode time; restoration and recurrence.

**Reliability and evidence:** failures by type; reliability/availability only under justified probability models; calibration/coverage of remaining-life distributions; sensor false-negative and false-positive controls; out-of-domain and unresolved rates; model disagreement.

**Sustainment:** spare mass/volume and stockouts; qualified/interchangeable fraction; resupply demand; consumable expiry; maintenance events; diagnosis, hands-on R&R, logistics/gather/stow, isolation and requalification times separately; crew/robot hours; spare and tool burden.

**Physical resources:** mass/element balance; electrical energy; heat by temperature grade; heat rejection; material/chemical residuals; power/thermal service curtailment.

**Optimization:** total and lower-tail service loss; expected value only when probabilities are justified; worst-case service loss over the admitted uncertainty set; minimax regret; resource/Pareto frontier; solve time, memory, optimality gap and timeout/abstention.

Do not report a scalar leaderboard score that can trade a protected service-floor failure for lower spares mass or crew time.

## 5. Proposed benchmark targets — hypotheses, not results

- **Hard controls:** every frozen critical negative control must reject false availability, unqualified output, wrong-revision spare, missing repair test, stale sensor and unsupported environment. Zero false accepts on the frozen deterministic corpus is a software acceptance criterion, not a statistical safety claim.
- **Conservation:** exact or declared-tolerance closure for each independent mass/element/resource ledger; every residual and rejected output accounted for.
- **Reproducibility:** deterministic exact-head replay and source/input/output digests for the reference fixtures.
- **Decision quality:** after a common baseline is established, target at least 20% reduction in held-out worst-case service-loss regret (for finite model sets) or a predeclared lower-tail service-loss measure (for calibrated probability models) relative to the strongest transparent baseline, with no service-floor regression.
- **Sustainment efficiency:** report Pareto improvements in spare mass/volume, crew-hours and maintenance downtime; do not set universal percentages before the comparator set and ground data are stable.
- **Distribution shift:** test held-out component families, usage/duty-cycle shifts, sensor drift, maintenance-time shifts, supplier/part changes and environment/applicability changes. A policy must either remain within predeclared bounds or explicitly escalate/abstain.

A claimed SOTA improvement requires an executable baseline, identical frozen inputs, predeclared metrics, independent outcome recomputation and a complete accounting of constraints/compute. Source comparison alone does not establish superiority.

## 6. Ablations and falsification

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

## 7. Execution sequence

1. Preserve the selected public ICES-2025 component rows as source records, not a statistically complete raw maintenance dataset.
2. Use the 2026 ISS operational cases as scenario narratives and configuration / supportability controls, not invented event-rate samples.
3. Keep AOGA programme schedule variance unresolved until a primary source establishes which milestone is actually achieved (mod-kit delivery, flight hardware completion, launch, installation or demonstration are different events).
4. Add exact small maintenance-scheduling and spares cases to the paired policy owner #1752; reuse #4057 / #3171 for finite-model and shift logic.
5. Seek executable/public implementations and matched input formats for EMAT/RELISSA and the 2024/2026 optimization formulations before any performance ranking.
6. Only then add a metered uncrewed ground ECLSS analogue with independently assayed outputs, recorded maintenance actions and seeded, bounded failure tests.

**Claim ceiling:** this protocol defines a fair comparison plan and evidence boundary. It does not establish that Symtropy outperforms EMAT, RELISSA, SmartHab optimization, the DRCC method, NASA, ESA or any operational ECLSS.
