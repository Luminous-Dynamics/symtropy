# Protective Systems Research Update — 2026-11-14

Status: research synthesis and benchmark-design addendum. This document does not report model execution, physical experiments, product performance, certification, or regulatory approval.

## Why the benchmark needs two additional dimensions

The v0.1 protocol already treats complete protective configurations, compound hazards, evidence scope and lifecycle provenance as first-class. Two deeper capabilities should now be tested explicitly:

1. **Degradation observability:** can the system detect, localize and communicate loss of protection, including sensor faults and unobserved regions, and does repair trigger only the requalification actually supported by new evidence?
2. **Decision-value experimentation:** does the system choose experiments that change engineering decisions and reduce decision regret under a fixed total budget, including experiments that disprove a favored candidate?

These have dedicated follow-on owners:
- Symtropy #5267 — PROTECT-DAMAGE-OBSERVABILITY-001.
- Symtropy #5268 — PROTECT-VOI-001.

## Literature and industry implications

### Multi-fidelity discovery is conditional, not automatically superior

Sabanza-Gil et al., *Nature Computational Science* (2025), benchmarked multi-fidelity Bayesian optimization on synthetic problems and three real discovery problems. Their work emphasizes that the usefulness of a cheaper approximate source depends on its informativeness and cost. Therefore, the protective-design benchmark must include deceptive or uninformative low-fidelity proxies, ranking reversals, source-cost accounting and comparisons against single-fidelity baselines.

Reference: https://doi.org/10.1038/s43588-025-00822-9

### Active-learning workflows need adversarial evaluation

A 2026 *Digital Discovery* perspective critically examines hidden assumptions, biases and methodological choices in materials-science active-learning workflows. The practical implication is to publish acquisition-policy ablations, leakage controls, negative controls and held-out problem families; do not treat a sophisticated acquisition function as evidence of better discovery.

Reference: https://doi.org/10.1039/d6dd00081a

### Building resilience needs time-dependent, locally grounded measures

A 2026 scoping review in *Energy and Buildings* maps 176 studies and 29 resilience metrics. It reports that only 25% provide operational definitions, that multi-hazard/longitudinal and district-scale work is limited, and that Global South evidence is underrepresented. This reinforces three requirements:
- define the service/habitability function and denominator before scoring;
- evaluate trajectories and recovery, not just a peak or single snapshot;
- report local climate, infrastructure, affordability and equity strata without transferring effect sizes from other regions.

Reference: https://doi.org/10.1016/j.enbuild.2026.118111

### Vehicle comparisons must bind protocol and rating versions

IIHS publishes test protocols and rating guidelines separately. Its public index lists, for example, a May 2025 Moderate Overlap 2.0 test protocol and a February 2026 rating-guideline version. Store both identifiers and versions, plus vehicle category/model year and the exact rating basis. A generic “IIHS 2026” label is not a reproducible benchmark input.

Reference: https://www.iihs.org/ratings/about-our-tests/test-protocols-and-technical-information

## New paradigm: protection as an observable, recoverable service

The next frontier is not only peak resistance. It is the ability to sustain a defined protective function over time under wear, damage, environmental exposure, repair, component substitution and infrastructure disruption.

For each application, report separately:
- initial performance under the applicable method;
- degradation/damage state and observability coverage;
- probability and latency of detection, including false alarms and missed detections;
- time out of service and repair burden;
- which evidence/claims were invalidated by a change;
- which claims were requalified, by what evidence, and for what configuration;
- service continuity and cost over the declared life-cycle boundary.

A monitoring system must not infer integrity from silence. An unobserved region remains unknown. A repair record is not a test result. A digital signature authenticates an assertion's origin/integrity; it does not establish the physical truth of the assertion.

## Competitive benchmark protocol

Compare Luminous methods against credible conventional and published baselines under the same:
- frozen inputs, requirements, constraints and held-out partitions;
- experiment, compute, inspection and confirmation budgets;
- exact candidate/configuration identity;
- measurement and human-review burden;
- failed-run, retry, downtime and rework accounting.

For discovery, include DOE, random/stratified sampling, NSGA-II or a suitable multi-objective evolutionary method, single-fidelity BO, multi-fidelity BO, uncertainty sampling and hypothesis-discrimination policies.

For monitoring, include threshold/rule baselines, appropriate signal-processing features, calibrated statistical detectors and model-based residual methods where their assumptions are met.

For housing, compare the same functional unit and climate/outage trace, and report thermal service, air quality, moisture, water/power continuity, egress/accessibility, maintenance, cost and embodied impact separately.

Do not claim SOTA from a vendor feature checklist, an unmatched dataset, a single aggregate score, synthetic fixtures or a simulation-only result.

## Targets and claim discipline

- B0: 100% expected decisions on the frozen finite known-answer suite; zero false-PASS on seeded hard-gate violations; all valid positive controls accepted; reject-all and accept-all controls fail.
- B1: reproduce selected published baselines within the declared scope; report calibration, uncertainty, OOD abstention and worst-stratum results.
- B2: show lower decision regret or a better feasible Pareto front under matched fully burdened budgets, with paired repeated trials and uncertainty intervals.
- B3: independently qualify the exact complete physical configuration using competent methods and facilities.

A stretch research objective for B2 is a material reduction in fully burdened cost-to-decision (for example, 20%) at equivalent verified feasible quality. This is a hypothesis to test, not a current result or acceptance threshold; freeze it only after baseline variance and practical significance are established.

Never trade away hard safety floors for a scalar score. Publish negative results and policy ablations. Keep coupon, component, subsystem, complete-product, field and certification claims distinct.

## Immediate runner-independent work

1. Implement an independent Rust B0 fixture verifier with explicit dispositions and deterministic output.
2. Add mutation/property tests for stale standards, wrong scope, changed configuration, missing provenance, unknown values and signature-only evidence.
3. Implement a finite exact-oracle VOI microbenchmark before connecting expensive models.
4. Add a damage/repair state-transition oracle that tests claim invalidation and selective requalification.
5. Freeze a housing outage/habitability trace adapter that composes existing VECTOR-HABITAT and SPATIAL-RESILIENCE owners rather than creating another building-physics solver.

No CI status is inferred from source-level or local structural checks. No physical work, procurement, destructive testing or actuation is authorized by this research addendum.
