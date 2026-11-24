# SAFRB v0.8 — restoration metrics, industry comparators and research gates

**Research cut:** 24 November 2026  
**Status:** benchmark design; no field comparison or superiority result claimed

## 1. Why a restoration lane is needed

v0.6 checks a supplied multi-carrier energy schedule at a service boundary. v0.7 checks whether a proposed flow fits within evidence-bounded physical capacity and access constraints. Neither version alone expresses whether a critical service stayed above its minimum throughout a disruption or whether apparent recovery persisted long enough to count as restoration.

The new evaluation object is a service trajectory:

**(boundary, service, time window, critical floor, target, delivered interval, evidence)**

This is an assurance output, not an optimizer. It should be compared separately from planning quality.

## 2. Map established reliability metrics without misusing them

| Comparator | What it measures | SAFRB treatment |
|---|---|---|
| IEEE 1366 SAIDI / SAIFI / CAIDI | Customer-weighted sustained interruption frequency and duration, and average restoration duration. | Report when a valid customer-level denominator and interruption event data exist; do not infer them from aggregate Wh. |
| ASIFI / ASIDI | Load-weighted interruption frequency and duration, useful for systems dominated by large industrial/commercial loads. | Suitable adjacent measures for port, mine, hospital and industrial load boundaries when connected-load denominators are known. |
| EENS / AENS / EUE | Expected or realized energy not supplied / unserved over a defined horizon and system boundary. | Report Wh/kWh shortfall and tail distribution; keep critical-floor shortfall separate from full-target shortfall. |
| LOLE / LOLP | Probabilistic adequacy: expected duration/frequency of load loss under defined system and scenario assumptions. | Use only after scenario probabilities and calibration are defensible. |
| NERC GADS outage/derate event data | Generation event causes, event counts, lost MW and MWh. | Use as a generation outage/event vocabulary and potential benchmark source; do not assume it covers South African fleet behavior. |
| Restoration benchmark studies | Dynamic restoration sequences, black-start, topology constraints and operating limits. | Compare trajectory and restoration feasibility only when model boundaries and dynamics are matched. |

SAIDI and SAIFI alone are inadequate for SAFRB's industrial use case: they can hide the magnitude of critical energy shortfall, service class and uncertainty. EENS alone can hide the timing and duration of a service failure. Publish the family together rather than collapsing resilience into one score.

## 3. SAFRB service-continuity measures

For each service boundary and time window, retain both bounds:

- **critical_shortfall_lower = max(critical_floor - delivered_upper, 0)**
- **critical_shortfall_upper = max(critical_floor - delivered_lower, 0)**
- **target_shortfall_lower = max(target - delivered_upper, 0)**
- **target_shortfall_upper = max(target - delivered_lower, 0)**

Report at minimum:

1. Confirmed critical-shortfall Wh and maximum bounded shortfall Wh.
2. Guaranteed service-hours / observed horizon-hours.
3. Count and duration of confirmed, uncertain and guaranteed windows.
4. Time to first restoration candidate.
5. Time to confirmed sustained restoration.
6. Number of restoration failures / right-censored episodes.
7. Service recovery curve: fraction of critical service restored over time.
8. Service-zone and service-class worst case, p95 and p99.
9. Cost, emissions and resource use as separate outcome dimensions.
10. Evidence coverage: measured vs reported vs derived vs scenario, with explicit unknowns.

Never call a non-guaranteed interval an observed physical outage when the evidence bounds overlap the threshold. Preserve an indeterminate category.

## 4. Competitive benchmark arms

All arms must receive the same frozen input state, time steps, service boundary, critical floors, topology, outage set, demand, evidence eligibility, uncertainty intervals, and cost/priority policy.

- **R0 — Status quo:** no adaptation / existing operator plan.
- **R1 — Rule-based restoration:** documented human/operator restoration sequence or transparent deterministic priority rules.
- **R2 — Deterministic LP/MILP:** HiGHS or Gurobi with the same constraints and time limit.
- **R3 — PyPSA deterministic:** same multi-carrier and storage assumptions where the model boundary fits.
- **R4 — PyPSA stochastic/CVaR:** scenario recourse and risk penalty where probability assumptions are defensible.
- **R5 — Robust/distributionally robust:** declared ambiguity set and worst-case objective.
- **R6 — SAFRB assurance wrapper:** optimizer output is accepted only if source eligibility, physical/accounting invariants, access limits and service trajectory pass an independent check.
- **R7 — Human operator/planner:** same information cut-off and decision budget, under an approved anonymised evaluation protocol.

Do not compare the v0.8 evaluator against an optimizer as though they perform the same function. The evaluator judges trajectories; optimizers propose schedules. A fair combined comparison is optimizer + independent assurance wrapper versus the same optimizer without that wrapper.

## 5. Proposed research gates

These are hypotheses to pre-register, not achieved results.

| Area | Proposed gate |
|---|---|
| Structural correctness | Zero accepted gaps, overlaps, duplicate window/period IDs, invalid units, timestamp overflow or broken evidence windows in the negative suite. |
| False service assurance | Zero cases where the evaluator reports continuity while a qualified lower delivery bound falls below the critical floor. |
| Uncertainty honesty | Every interval overlapping the critical floor must produce an indeterminate continuity disposition, never a pass. |
| Restoration integrity | Zero restoration confirmations before the configured sustained-recovery duration is met. |
| Horizon handling | 100% of unconfirmed recovery episodes explicitly right-censored; never impute recovery outside the observed horizon. |
| Replay | Bit-identical receipt for identical canonical inputs, code version and configuration. |
| Calibration | Nominal 90% service-delivery intervals achieve 85–95% empirical coverage on a held-out, operator-approved historical set, with subgroup and horizon breakdowns. |
| Planning value | At equal cost and identical hard constraints, target at least 10% lower CVaR95 of critical-service unserved energy than the strongest deterministic baseline across a preregistered disruption suite. |
| Restoration value | Target at least 10% lower p95 time-to-sustained-restoration than the matched operator/rule baseline, without worsening the worst service zone or service class. |
| Economics and environment | Publish total cost, emissions, curtailment, and lifecycle assumptions separately; no single composite score may conceal a critical-service regression. |

The 85–95% calibration range is a pilot tolerance for a nominal 90% interval, not permission to use uncalibrated uncertainty bounds. Use proper scoring rules (weighted interval score / pinball loss) alongside coverage so excessively wide intervals do not win.

## 6. Scenario corpus

Build a frozen, versioned event suite with single and compound disruptions:

- supplier / country-of-origin interruption and delayed cargo;
- berth closure, vessel queue and discharge-arm failure;
- tank contamination, quarantine, unusable ullage and meter drift;
- pipeline segment outage, pump failure and common power-feed loss;
- grid curtailment, islanding, black-start limitation and generator failure;
- generator derating, fuel-quality incompatibility and maintenance lockout;
- battery state-of-charge depletion, inverter limit and recharge constraint;
- communications loss, timestamp drift, duplicate/replayed events and credential expiry;
- simultaneous port, logistics and grid disruption;
- staged domestic production restoration with financing, commissioning, feedstock and ramp-up gates.

Use explicit correlation groups for common-cause disruptions. Avoid multiplying independent probabilities for events with shared causes.

## 7. How to beat the field honestly

The differentiator is not a faster LP solver, a more decorative digital twin, or an LLM explanation. It is a qualified end-to-end chain:

1. **Evidence before optimization:** reject stale, reported-only or ineligible inputs before they enter the feasible set.
2. **Service, not commodity:** optimize/measure delivered critical service rather than tonnes or litres alone.
3. **Rights plus physics:** enforce the intersection of physical feasibility and externally authorized access.
4. **Time-path, not snapshot:** prove continuity, interruption depth and sustained recovery across an explicit horizon.
5. **Counterfactual resilience:** show what would have happened under alternative supplier, route, carrier, storage and restoration policies.
6. **Active measurement:** use Symthaea to identify the next observation with the highest decision value, including the test that could falsify the leading explanation.
7. **Independent replay:** publish frozen inputs, provenance, code/configuration digests, failed cases and an independent verifier result.
8. **Human authority:** keep execution separate from proposal and qualification; initial deployments remain read-only.

## 8. External references

- IEEE 1366 distribution reliability-index family; use the current edition and licensed standard text for formal calculations.
- U.S. EIA Electric Power Annual: public SAIDI, SAIFI and CAIDI definitions and reporting notes: https://www.eia.gov/electricity/Annual/table.php?t=epa_11_01.html
- NERC GADS Cause Code Reports, including 2025 data updated 21 October 2026: https://www.nerc.com/programs/reliability-assessment--performance-analysis/generating-availability-data-system/gads-conventional/gads-cause-code-reports
- Review and application of a benchmark model for power-system restoration studies, Renewable and Sustainable Energy Reviews, May 2026: https://doi.org/10.1016/j.rser.2026.116787
- Nature Energy discussion of outage analysis, monitoring, mitigation and restoration, 2026: https://www.nature.com/articles/s44287-026-00298-3
- PyPSA stochastic optimization and CVaR: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- NIST AI Risk Management Framework 1.0: https://www.nist.gov/publications/artificial-intelligence-risk-management-framework-ai-rmf-10
