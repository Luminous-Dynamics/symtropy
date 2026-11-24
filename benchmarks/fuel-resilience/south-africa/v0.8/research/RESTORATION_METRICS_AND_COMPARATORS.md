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

**Energy is not complete service quality.** The v0.8 Wh boundary cannot prove minimum instantaneous kW, voltage/frequency/waveform compliance or the quality of a downstream process service. Report independent engineering quality checks as separate predicates and fail closed when a required quality predicate is absent.

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

## 8. Composition with the Power Fabric restoration benchmark

Do not duplicate the generic infrastructure already being developed in Symtropy #6320 and its child contracts. SAFRB contributes the domain-specific input qualification and service outcome:

| Layer | Existing owner | SAFRB contribution |
|---|---|---|
| Frozen case, topology, observation profile | #6346 / PR #6347 | Tokenized South African port/fuel-chain profile, asset/service boundaries, fuel/access evidence references. |
| Planner proposal and actions | #6348 / PR #6350 | Fuel logistics and service alternatives are represented as candidate proposals, not authorization. |
| Information requests/results | #6369 / PR #6370; #6397 / PR #6401 | Symthaea EVSI may request cargo-status, assay, tank-gauge, route, generator or service-meter observations; request and result remain separate and time-indexed. |
| Run receipt and trajectory | #6352 / PR #6366 | Planned adapter will bind v0.8 service-floor trajectory and metrics; current v0.8 returns an in-memory Rust result and CSV, not a content-addressed artifact. |
| Artifact and claim verification | #6402 / PR #6405; #6419 | Independent byte, signature, parent-link and role verifier qualifies the complete chain. |
| Authority and access | Mycelix | Verifies issuer, rights, provider offer, reservation and commitment receipts; it does not make physical truth self-authenticating. |
| Spatial context | Sol-Atlas | Supplies governed asset/site/route geometry and affected service cohorts. |

Current v0.8 boundary: the evaluator returns a deterministic Rust result struct and the example binary prints CSV. It does not yet serialize canonical JSON, compute content digests, sign an output bundle or populate the generic run receipt. Build that adapter only against the accepted contract revisions, then require the independent artifact/claim verifier to qualify the resulting chain.

This is a proposed interop mapping, not a claim that the draft contracts have already been merged or that the end-to-end pipeline is implemented. No generic action, restoration or signature schema should be forked inside SAFRB.

## 9. External references

- IEEE 1366 distribution reliability-index family; use the current edition and licensed standard text for formal calculations.
- U.S. EIA Electric Power Annual: public SAIDI, SAIFI and CAIDI definitions and reporting notes: https://www.eia.gov/electricity/Annual/table.php?t=epa_11_01.html
- NERC GADS Cause Code Reports, including 2025 data updated 21 October 2026: https://www.nerc.com/programs/reliability-assessment--performance-analysis/generating-availability-data-system/gads-conventional/gads-cause-code-reports
- Review and application of a benchmark model for power-system restoration studies, Renewable and Sustainable Energy Reviews, May 2026: https://doi.org/10.1016/j.rser.2026.116787
- Nature Energy discussion of outage analysis, monitoring, mitigation and restoration, 2026: https://www.nature.com/articles/s44287-026-00298-3
- PyPSA stochastic optimization and CVaR: https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- NIST AI Risk Management Framework 1.0: https://www.nist.gov/publications/artificial-intelligence-risk-management-framework-ai-rmf-10


## 10. Frontier comparison and dataset discipline

The existing Power Fabric comparator programme #6320 has already identified several mandatory external baselines. SAFRB should reuse that reproduction work and keep benchmark tiers distinct:

- **Feeder restoration / degraded observation:** reproduce LARA (Findings of EACL 2026) and FLARE (arXiv:2610.23251) on their exact case sets, observation profiles, action budgets and reward definitions before attempting extensions. FLARE's public data release must be confirmed before claiming a reproduction.
- **Electrical feasibility / whole-process restoration:** compare against the 2026 distributionally robust IEEE-118 formulation only with its exact uncertainty model and independent AC-OPF checks at every proposed time step. A topology-only or DC-only result must not be described as AC feasible.
- **Restoration benchmark systems:** use the public benchmark model/review dataset (Zenodo DOI 10.5281/zenodo.17340051) as a separately versioned physical-restoration lane; record its model fidelity and scenario conversion receipt.
- **Fuel logistics / service delivery:** freeze the South African case and pair every planner with identical cargo, tank, berth, pipeline, access, generation and demand evidence. Compare operational outcomes, not software-stack branding.
- **Assurance overlay:** compare the same optimizer with and without SAFRB's evidence/service verifier. Count how many false accepts, unsupported stock promotions, infeasible schedules, hidden tail shortfalls and restoration false claims are prevented, and measure false rejects/latency as well.

These are not one leaderboard. LARA, FLARE, IEEE-118 AC restoration, fuel-network flow and industrial process simulation have different physics, action spaces, observation semantics and objective functions. Publish separate tables and only aggregate after an explicit common service contract is defined.

## 11. Research hypotheses beyond the current prototype

1. **Service-floor-first optimization:** hard constraints for critical services, with cost, emissions and noncritical curtailment optimized only after those floors are protected.
2. **Restoration-as-a-trajectory:** evaluate service at each time slice and require sustained recovery; a single recovered interval is not a restoration claim.
3. **Evidence-budgeted decision-making:** value measurements by expected reduction in decision loss, but include diagnostic latency, shared crews/tools and dependence between sources.
4. **Common-cause-aware resilience:** model correlated supplier, port, pipeline, grid, weather and communications disruptions explicitly rather than multiply independent event probabilities.
5. **Transition and recovery quality:** distinguish temporary service compensation (diesel generator, mobile power, battery) from restoration of the original service mechanism, and measure both.
6. **Build-back-better option value:** evaluate repair, temporary bypass, replacement, storage, grid reinforcement and alternative transport/energy pathways over a stated horizon, including staged financing/commissioning evidence.

The v0.8 evaluator only measures time-bounded service-floor outcomes. It does not yet model grid dynamics, black-start, protection, transformer inrush, cold-load pickup, pipeline hydraulics or restoration control sequences. Those must be independently validated by their engineering model owners.
