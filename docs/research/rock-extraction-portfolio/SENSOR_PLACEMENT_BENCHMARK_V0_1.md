# Sensor placement and active diagnosis benchmark v0.1

**Status:** exact finite sensor-subset/Pareto optimizer using synthetic lifecycle costs and the #4005 synthetic response matrix. No actual installation recommendation or mine economics is produced.

## 1. Paradigm shift

The previous question was whether a proposed sensor layout can distinguish faults. The next question is which layout provides the required diagnosability at the lowest defensible lifecycle burden, and how robust that result is when a sensing principle becomes unavailable.

Do not minimize purchase price or sensor count in isolation. A layout with a cheap sensor that requires frequent recalibration, has long service outages or duplicates another modality may be a worse engineering choice.

This benchmark keeps four objectives separate:
- lifecycle cost;
- instrument count;
- maintenance effort;
- planned unavailability;
- worst-case fault-pair coverage under declared outages.

It reports a Pareto frontier, not a hidden weighted winner.

## 2. Research basis

- Diao et al. (2025), *Advanced Engineering Informatics* 65 Part B, 103228, DOI 10.1016/j.aei.2025.103228, propose sensor selection using qualitative physical models and criteria including diagnosis, risk reduction, observability, integration and cost. This benchmark makes the fault-pair and outage constraints explicit.
  https://doi.org/10.1016/j.aei.2025.103228
- Borg et al. (2026), *Measurement* 258A, 118992, DOI 10.1016/j.measurement.2025.118992, measure pneumatic fault monitoring across multiple system-complexity and fault-profile levels, supporting cross-configuration sensor evaluation.
  https://doi.org/10.1016/j.measurement.2025.118992
- Zhang et al. (2026), *Measurement* 272, 121117, DOI 10.1016/j.measurement.2026.121117, provide a current-only baseline in very dilute-phase conveying; its score cannot be transferred to coarse hard-rock flow.
  https://doi.org/10.1016/j.measurement.2026.121117
- Cheng & Huan (2025; revised 2026), *Optimal stopping for sequential Bayesian experimental design*, treats selection and stopping as coupled sequential decisions. The next active-diagnosis layer should use #3917 EVSI, not information gain alone.
  https://arxiv.org/abs/2509.21734

## 3. Exact v0.1 algorithm

The input references the exact #4005 observability case and supplies:
- candidate-to-channel mappings;
- lifecycle cost in one declared unit;
- maintenance hours/year;
- planned unavailable hours/year;
- cost-source evidence class and reference;
- maximum channel count and optional lifecycle budget;
- declared channel-outage and independent-principle-outage scenarios.

For N candidates, v0.1 evaluates every non-empty subset:

`2^N - 1`

It supports at most 20 candidates. Larger search spaces are rejected with SEARCH_SPACE_LIMIT. No implicit heuristic fallback is permitted.

Each subset is evaluated:
1. nominally;
2. after each explicitly declared channel outage;
3. after each explicitly declared common-principle outage.

For every required #3993 fault pair, the evaluator uses the same response-interval separation, evidence-role, sensor-fault and independence-group semantics. Critical pairs must remain IDENTIFIABLE under every required scenario when the profile requests this constraint.

The exact search is a reference oracle for future mixed-integer/constraint solvers. Any later solver must agree with it on small golden instances.

## 4. Pareto objectives

For each feasible layout, calculate:
- total lifecycle cost;
- selected channel count;
- sum of annual maintenance hours;
- sum of planned annual unavailability hours;
- worst-case number of required pairs still identifiable across nominal/outage scenarios.

A layout A dominates B only when A is no worse on every objective and strictly better on at least one. All nondominated layouts are returned with stable ordering. Equal-objective layouts may both remain on the frontier when their actual sensor identities differ.

No cross-unit comparison is made. Costs must use the exact declared cost unit. Maintenance hours remain a separate physical resource dimension.

## 5. Synthetic example

The fixture contains seven candidate channels:
- line pressure differential;
- filter pressure differential;
- airflow;
- blower current;
- gravimetric mass flow;
- electrostatic/capacitance flow estimator;
- independent instrument-health diagnostic.

It evaluates five declared principle-outage scenarios: pressure, airflow, gravimetric, electrical and electrostatic/capacitance. Synthetic lifecycle costs use **SYNTHETIC_COST_POINTS**, not USD, ZAR, EUR or a vendor quote. Annual maintenance/unavailability hours are also fabricated.

The #4005 observation envelopes are synthetic. The Pareto set, if any, is a software demonstration only. It must not be used to purchase or install physical equipment.

## 6. Benchmark gates

Software:
- exact all-subset enumeration for small N;
- all critical pairs identified nominally and in every required outage scenario for a feasible layout;
- exact lifecycle budget / channel limit enforcement;
- 100% rejection of unknown channels, duplicate candidates, invalid units, missing cost provenance, duplicate scenarios and invalid outage references;
- no Pareto result dominated on the declared objective vector;
- deterministic output under candidate-order permutation;
- explicit SEARCH_SPACE_LIMIT above 20 candidates;
- exact agreement with brute-force golden solutions.

Physical:
- actual vendor quote, installation, calibration, maintenance and replacement evidence;
- measured instrument availability and recalibration duration;
- independent common-mode failure review;
- qualified per-channel fault response envelopes from #3991/#4005;
- prospective material/path campaigns and outage analysis.

No numerical cost saving, uptime improvement or diagnosis uplift is claimed from the synthetic profile.

## 7. Active diagnosis beyond v0.1

The next increment should connect unresolved fault pairs to #3917's EVSI calculation.

Possible observations include passive, evidence-gathering actions:
- collect a longer synchronized sensor window;
- obtain an independent gravimetric catch-and-weigh sample;
- obtain a representative PSD/morphology sample;
- collect a filter/fines branch sample;
- compare an independently calibrated redundant sensor;
- inspect a designated physical sample.

The selected observation must have a calibrated likelihood model, exact material/path domain, full cost, decision utility and stopping rule. If no observation has positive NetEVSI above the stop threshold, return STOP / HUMAN_REVIEW rather than inventing a best action.

Any physical actuation, line clearing, pressure/airflow adjustment or machine intervention remains outside this optimizer and requires the responsible engineering/safety authorization.

## 8. Integration

- #3993/#4005: fault-pair observability and response evidence.
- #3987/#3991: measurement system / calibration domains.
- #3917: one-step EVSI and stopping.
- #3877/#3913: independently labelled event evidence.
- #3927: stage-resolved material flow/PSD.
- #3910: morphology.
- #3890: spatial residual/support evidence.
- #3853: strategy Pareto eligibility.
- #4006: this cost-constrained sensor layout/Pareto and future active-diagnosis programme.

## 9. Claim boundary

This is exact subset optimization over declared candidate channels and evidence. It does not select safe physical placements, verify common-mode independence, approve equipment, specify pressure/wiring/hazardous-area requirements or command a machine. Synthetic costs and signatures are never promoted to measured engineering evidence.
