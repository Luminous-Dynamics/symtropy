# ECLSS Supportability Benchmark v0.1

**Status:** research protocol / synthetic-first  
**Owner:** Symtropy; domain adapter under [ECLSS-SUPPORTABILITY-001 #4298](https://github.com/Luminous-Dynamics/symtropy/issues/4298)  
**Claim ceiling:** synthetic supportability contract and published-reference comparison only

## 1. Research thesis

A life-support subsystem is not successful merely because it reaches a high recovery fraction at nominal conditions. The engineering object is a time-dependent service system whose usable outputs depend on process quality, storage, distribution, energy, heat rejection, consumables, component life, inspection, maintenance, spares, crew/robot time and the ability to requalify after repair.

The proposed benchmark evaluates **qualified service delivered over a declared mission horizon**, subject to hard service floors and resource limits, while accounting for the full sustainment burden needed to keep that service available.

This reframes the objective from isolated subsystem efficiency to sustainable, evidence-backed service. It does not replace specialized ECLSS, water chemistry, atmosphere, crop, thermal, power, biological or reliability models.

## 2. Current state of the field and comparison boundary

### 2.1 ISS water recovery

NASA reported in June 2023 that the ISS ECLSS water-recovery system demonstrated 98% total water recovery with the Brine Processor Assembly. The same report describes wastewater collection, urine distillation, brine treatment, humidity recovery, filtration, catalytic oxidation, purity sensing, reprocessing of unacceptable water and iodine dosing. Thus the 98% figure is not a standalone potable-service guarantee; output acceptance and the residual/brine path matter.

Primary source: https://www.nasa.gov/missions/station/iss-research/nasa-achieves-water-recovery-milestone-on-international-space-station/

### 2.2 CO2-to-oxygen recovery

NASA's SpaceCraft Oxygen Recovery page gives the ISS Sabatier carbon-dioxide reduction assembly an estimated average oxygen recovery of about 47% (described elsewhere on the same page as approximately 50%). NASA's SCOR programme seeks greater than 75%, with a 100% stretch goal. These are development goals, not achieved flight performance. This oxygen-from-CO2 boundary must remain distinct from water recovery and from oxygen generated through water electrolysis.

Primary source: https://www.nasa.gov/spacecraft-oxygen-recovery-scor/

### 2.3 Reliability and supportability

The 2025 ICES paper *Advancing ECLSS Reliability Modeling* states that the last release of ECLSS reliability data was in 2004 and that this leaves a gap for modern systems. The paper describes incorporating recent ISS component replacement rates, shelf life and throughput limits into reliability/sparing analysis. Its public abstract establishes the research direction; it does not expose a complete machine-readable event dataset in the abstract. We must not invent replacement distributions, MTBF, MTTR or mission probabilities from it.

Source: https://ntrs.nasa.gov/citations/20250003955

NASA's 2026 ICES paper describes using the ISS as a testbed to mature exploration-class ECLSS hardware and integration, with performance and reliability evidence intended to support deep-space missions.

Source: https://ntrs.nasa.gov/citations/20260003280

### 2.4 Regenerative life support

ESA's MELiSSA concept seeks to recover food, water and oxygen from organic waste, CO2 and minerals through coupled biological and physicochemical processes. This is an important architectural comparator, but the concept page is not a single closed-loop performance dataset. Compare it at the evidence level actually available; do not infer total closure, crew-equivalent capacity or operational reliability from a conceptual description.

Source: https://www.esa.int/Enabling_Support/Space_Engineering_Technology/Melissa/Closed_Loop_Concept

### 2.5 Integrated lunar infrastructure

NASA's current Moon-base description treats enduring presence as a coupled architecture: dependable power, habitation/life support, cargo transportation, communications/navigation, mobility, and reliable logistics must operate together under radiation, temperature and lunar-dust hazards. This supports a system-level benchmark, but does not mean those future systems are already deployed or qualified.

Source: https://www.nasa.gov/moon-base-frequently-asked-questions/

## 3. Paradigm shift

### From recovery fraction to accepted service

For every stream, distinguish:
1. feed entering a declared process boundary;
2. product leaving the process;
3. quality acceptance by the downstream consumer;
4. storage/distribution losses and access;
5. residual, waste, brine, purge and contaminant flows;
6. make-up required from stored or resupplied material.

An output outside the consumer's quality envelope contributes zero accepted service until reprocessed or independently accepted. A high gross recovery percentage cannot compensate for failed quality.

### From nominal efficiency to sustainment economics

Overlay a second, coupled ledger:

fault → detection → diagnosis → degraded mode → isolation → maintenance → parts/consumables → repair → requalification → restored service.

Every task consumes time, qualified labor or robot capability, tools, energy, consumables and potentially service reserve. The part must match the installed configuration, and a repair report is not evidence of restored service.

### From closed-loop rhetoric to finite-horizon closure

A loop is not closed merely because a product is recycled once. Evaluate residual losses, contaminant accumulation, quality decay, batch rejection, biological instability, maintenance-induced downtime, and every required make-up stream across repeated cycles. Keep each element/material balance separate.

### From one aggregate score to constrained Pareto evidence

Report a vector of service, quality, resupply, energy, heat, maintenance, crew time, reliability evidence and uncertainty. A critical floor breach is a gate—not a cost that can be offset by better efficiency elsewhere.

## 4. System boundaries and composition

This adapter consumes existing project owners:

- #3952 owns the planetary habitat / regenerative food loop and preserve constraints.
- #3957 owns full-facility energy, thermal, water and accepted-food accounting.
- #3254 owns finite logistics and irrecoverable/no-resupply disruption semantics.
- #1625 owns evidence-qualified, time-scoped availability and repair/requalification receipts.
- #1752 owns paired policy evaluation under common exogenous events and information budgets.
- #1803, #1840 and #1637 own their respective resource-product, energy-service and nutrient/resource accounting semantics.

The adapter binds these records into an ECLSS sustainment campaign. It must not add another crop model, thermal solver, water-treatment solver, resource ledger, reliability engine or actuation authority.

### Subsystem profile

A profile identifies:
- subsystem, component, installed configuration and software revision;
- function and critical-service consumer;
- input, output and byproduct streams with units, quality grades and process boundaries;
- throughput, turndown and environmental validity envelopes;
- electrical and thermal demand, including temperature-grade limits and heat rejection;
- consumables, batch identities, quantities, shelf life and storage conditions;
- maintainable items, inspection/cleaning/calibration tasks and replacement rules;
- spare identity/revision, interchangeability evidence, stock and lead time;
- required tools, crew/robot skills, access, task duration and resource demand;
- fault modes, detection coverage, isolation and degraded-mode limits;
- repair attempt, post-repair measurements and exact-configuration requalification;
- sensors/assays, calibration, uncertainty, timebase and raw-data digest;
- source, evidence tier, validity domain, known limitations and claim ceiling.

Unknown lifetime, replacement rate, failure rate or maintenance duration remains unknown. Do not substitute a convenient distribution or zero.

## 5. Benchmark design

### 5.1 Frozen scenario registry

The initial registry contains 30 typed synthetic cases, ECLSS-001 through ECLSS-030. The machine-readable scenario file is at:

docs/research/eclss/fixtures/eclss-supportability-synthetic-registry-v0_1.json

Schema:

docs/research/eclss/eclss_supportability_case.schema.json

The cases cover water recovery/quality, residual streams, oxygen recovery, CO2 removal, contaminants, common-cause dependencies, throughput, power/thermal limits, spare compatibility and shelf life, maintenance contention, repair/requalification, biological loop interruption, communication delay, out-of-domain evidence and tamper/metamorphic controls.

These cases are synthetic and deterministic. Their purpose is to test accounting, evidence and decision semantics before any empirical calibration.

### 5.2 Known-answer ledger check (synthetic only)

A deliberately small arithmetic fixture makes the distinction between recovery and resupply explicit:

- demand: 100 abstract water units per day;
- accepted recovery: 98% of the process-boundary input;
- daily make-up: 2 units;
- horizon: 30 days;
- initial qualified make-up inventory: 40 units;
- no resupply during the horizon.

The ledger therefore requires 60 units of make-up, has 40 units available, and ends with a 20-unit unserved deficit if no other qualified source exists. Recovery remains 98% in this toy model while the service floor eventually fails. This is an arithmetic counterexample, not an ISS reproduction, potable-water model or claim about an actual habitat.

### 5.3 Common event and information budgets

Every candidate architecture/policy pair receives the same:
- initial inventory and subsystem condition;
- crew demand and service-floor version;
- exogenous fault and environmental event trace;
- power, thermal and logistics envelope;
- observation availability, delay and sensor quality;
- mission horizon and resource accounting basis.

A policy does not receive future fault information. Exogenous events cannot be rewritten by a repair or by an intervention. If a policy receives additional measurements, report this as a different information arm and charge the full resource/time cost.

## 6. Baselines

Use matched cases and clearly separate empirical, simulated and conceptual evidence:

1. Open-loop supply with explicit consumable mass and delivery windows.
2. ISS-heritage subsystem references, only for public values with reproducible process boundaries.
3. Reactive maintenance after a qualified observed threshold.
4. Scheduled preventive maintenance.
5. Service-floor/reserve-first maintenance scheduling.
6. Adaptive maintenance or measurement selection only where likelihoods, decision utility and observation access are justified.
7. No-failure and perfect-information runs as diagnostic bounds, never peer competitors.

Where the strongest industry reference is unavailable as executable code/data, state the limitation and compare only the published quantities that are actually disclosed. Do not claim a software or architecture victory from unlike boundaries.

## 7. Metrics

Report independently by stream, service and interval:

- demand, accepted delivery, deficit and time below each authorized service floor;
- product-quality accept/reject/reprocess totals and evidence;
- gross recovery, net make-up, purge, residual and waste by material/element boundary;
- resupply mass and volume per crew-day and crew-year by grade;
- electrical energy, thermal energy by temperature grade, heat rejection and parasitics;
- throughput, availability, degraded-mode interval and storage reserve;
- time to detection, diagnosis, isolation, repair, requalification and stable recovery;
- maintenance count, crew-hours, robot-hours, tools, consumables and spare mass;
- stockout, expiry, storage excursion and interchangeability;
- contaminant/residual concentration trajectory and accumulation;
- first service loss, recurrence, and service-loss regret;
- fraction of intervals with current, calibrated and in-domain evidence.

MTBF/MTTR, probability of survival and Monte Carlo reliability are admissible only with justified exposure/event data, censoring treatment, dependence assumptions, uncertainty and model validation.

## 8. Proposed benchmark gates

P0 — schema/fixture validity, deterministic replay and independent arithmetic oracle.

P1 — every valid synthetic case matches an independent balance/decision evaluator; all negative controls fail closed.

P2 — reproduce public ISS water and oxygen reference statements without mixing process boundaries or converting targets into results.

P3 — admit the 2025 reliability paper's component replacement, shelf-life and throughput records only to the degree that methods/data are accessible and their assumptions can be reconstructed.

P4 — conduct a metered uncrewed analogue with independent quality assays, seeded component faults, maintenance logs, parts/shelf-life tracking and repeated cycles.

P5 — separately qualify integrated hardware and environmental/flight use. No promotion is automatic.

Hard gates:
- no critical service credit from an output that fails its consumer quality contract;
- no negative inventory or unexplained mass/element disappearance;
- no unqualified substitute, repair or post-repair service;
- no automatic recovery when resupply, parts, staff or tools are absent;
- no scalar score may mask a critical floor violation;
- unknown or out-of-domain evidence remains unresolved.

A candidate research goal is ≥20% lower service-loss regret or sustainment burden than the strongest transparent baseline on a frozen held-out suite, with no service-floor regression. This target is deferred until public baselines and analogue measurements establish a stable comparison. It is not an achieved result.

## 9. Promotion ladder and claim ceiling

| Evidence tier | Admissible claim |
|---|---|
| Synthetic | Exact accounting, state transition, negative-control and decision-semantics behavior on frozen known-answer cases |
| Published reference | Reproduction of disclosed source values with boundary, date, method and uncertainty retained |
| Ground analogue | Measured performance for the exact tested apparatus, environment, sensors, calibration and operating envelope |
| Integrated test | Coupled-system behavior only for the integrated configuration and event envelope actually tested |
| Flight | Flight performance only for the configuration, duty cycle and environmental conditions directly evidenced |

V0.1 is synthetic-only. It does not qualify ECLSS hardware, crew exposure, potable water, oxygen supply, medical safety, biological stability, a lunar/Martian site or a flight system. NASA/ESA sources are technical references, not endorsements or certifications.

## 10. Source register

- NASA, *NASA Achieves Water Recovery Milestone on International Space Station*, 20 June 2023; page last updated 2 March 2026: https://www.nasa.gov/missions/station/iss-research/nasa-achieves-water-recovery-milestone-on-international-space-station/
- NASA, *SpaceCraft Oxygen Recovery (SCOR)*: https://www.nasa.gov/spacecraft-oxygen-recovery-scor/
- Chen, Rivas and Rangel, *Advancing ECLSS Reliability Modeling: Integrating ISS Data for Sustainable Long-Duration Mission Planning*, ICES-2025-127: https://ntrs.nasa.gov/citations/20250003955
- Brown et al., *International Space Station as a Testbed for Exploration ECLSS: 2026 Status*, ICES-2026-379: https://ntrs.nasa.gov/citations/20260003280
- ESA, *MELiSSA Closed Loop Concept*: https://www.esa.int/Enabling_Support/Space_Engineering_Technology/Melissa/Closed_Loop_Concept
- NASA, *Moon Base: Frequently Asked Questions*: https://www.nasa.gov/moon-base-frequently-asked-questions/
- NASA, *Reliability and Maintainability*: https://sma.nasa.gov/sma-disciplines/reliability-and-maintainability

