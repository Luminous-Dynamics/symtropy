# Port Nexus Multi-Domain Event Trigger Benchmark v0.1

**Protocol:** PORT-NEXUS-MULTIRATE-EVENT-001  
**Snapshot:** 19 November 2026  
**Status:** read-only trigger monitor and synthetic benchmark; not an optimizer or controller.

## Decision

A port-city compute campus has several time-varying service obligations, not one load curve:
- vessels, berth occupancy, quay cranes, yard equipment and shore power;
- AI jobs with release times, deadlines and useful-work obligations;
- liquid cooling, seawater intake and heat-rejection margin;
- chilled-water / thermal storage with finite state, rate limits and losses;
- renewables, PCC/feeder import envelopes and electricity procurement;
- evidence freshness and decision-time availability.

Re-optimizing every 15 minutes is simple but can waste solver budget and create plan churn. Re-optimizing only on power deviations misses logistics disruptions, compute deadlines or cooling-state changes. The new monitor asks **when the existing owner optimizers need to be called again**, based on typed signals and declared constraints.

## New industry evidence and what it changes

### Port operations: event-triggered logistics + energy

Li et al., *Microgrid energy optimal scheduling in ports incorporating the bidirectional event trigger mechanism*, Journal of Cleaner Production 568 (15 June 2026), DOI [10.1016/j.jclepro.2026.148666](https://doi.org/10.1016/j.jclepro.2026.148666).

Their Tianjin Dongjiang case uses an hourly day-ahead MILP for berth/quay-crane assignment and energy dispatch. During operations it updates at 15-minute intervals, triggers a 1-hour rolling re-optimization based on both logistics-side and energy-side deviations, and reports case-specific reductions in operating cost (18.64% vs MPC; 22.44% vs a logistics-only event trigger) and online solve time (88.90% and 77.76%). These are results of that study's model and case data, not a transferable Luminous result.

**Our extension:** preserve the bidirectional logistics/energy idea, then add compute, cooling, thermal-store and evidence-freshness domains. Do not use an aggregate weighted score that can hide a hard PCC, deadline or thermal boundary breach.

Source: [ScienceDirect — BETM port microgrid scheduling](https://www.sciencedirect.com/science/article/pii/S0959652626012072).

### Port VPP: heterogeneous demand and intraday uncertainty

Zhang et al., *Scenario-driven day-ahead–intraday coordination of seaport virtual power plants under heterogeneous load uncertainty*, Sustainable Energy Research 13, article 38 (11 September 2026), DOI [10.1186/s40807-026-00269-0](https://doi.org/10.1186/s40807-026-00269-0).

The paper models ship shore-power demand and battery-swap load as stochastic scenarios, derives crane load from terminal operation, and combines day-ahead dispatch with intraday rolling correction. Its case reports storage-related operating-cost reduction of 9.79%, maximum electricity-purchase deviation reduction of 52.11%, and execution-stage cost reduction of 18.4% for the proposed coordination. Renewable generation is supplied as a forecast sequence rather than independently stochastic in the paper's main case; a matched reproduction must preserve that boundary.

**Our extension:** scenario generation must include coupled vessel arrivals, berth occupancy, crane workload, storage state, compute queue/deadline slack and cooling mode. Measure the uncertainty and covariance that survive into the trigger decisions.

Source: [Springer Nature — seaport VPP coordination](https://link.springer.com/article/10.1186/s40807-026-00269-0).

### Liquid-cooled data centres: compute is not the only flexible load

*Exploring the Demand Response Potential of Liquid-Cooled Data Centers*, Applied Energy 412 (1 June 2026), DOI [10.1016/j.apenergy.2026.127679](https://doi.org/10.1016/j.apenergy.2026.127679), develops an electro-thermal coupled model and coordinated demand-response approach for server and liquid-cooling systems. This is a warning against measuring only the server workload: pumps, chillers, thermal inertia and cooling constraints have their own dynamics and service limits.

Source: [Applied Energy — liquid-cooled data-centre demand response](https://www.sciencedirect.com/science/article/pii/S0306261926003314).

### Data-centre TES + water-side economiser

*AI-based model predictive control of chiller plant with thermal energy storage combining a water-side economizer system in data centers*, Energy and Buildings (15 August 2026), DOI [10.1016/j.enbuild.2026.117668], reports up to 11% lower chiller energy, 5% lower whole-cooling-system energy and 4% lower cost in its modeled case. Reproduce the precise system boundary before comparing.

DOE FEMP's data-centre M&V addendum also notes that thermal storage can increase annual chilled-water-plant energy while shifting demand away from high-cost periods and adding resilience. Thus:
- peak reduction is not the same as energy reduction;
- tariff savings are not the same as emissions savings;
- thermal storage is not a lossless battery;
- compute flexibility and cooling flexibility must be reported separately and jointly.

Sources: [Energy and Buildings study](https://www.sciencedirect.com/science/article/abs/pii/S0378778826007280); [DOE FEMP Data Center M&V Addendum](https://www.energy.gov/sites/default/files/2025-09/femp_mv_5_0_data-center-addendum_092025.pdf).

## Paradigm shift: three nested operating clocks

1. **Planning clock (day-ahead to multi-day):** vessel arrivals, berth allocation, crane/yard plans, compute reservation, energy procurement, renewable forecasts, cooling mode and thermal-store reserve.
2. **Coordination clock (15-minute snapshots in v0.1):** compare realized logistics, PCC/feeder, compute, cooling, storage and renewable signals against the frozen plan. Re-plan only when a defined soft or hard event warrants it.
3. **Protection / local-control clock:** electrical protection, UPS, generator controls, cooling interlocks and equipment safety remain deterministic local functions. They do not wait for this monitor, Symthaea, Mycelix, internet connectivity or a solver.

The monitor is read-only. It requests a fresh plan from an existing optimizer; it does not produce or apply one.

## V0.1 monitor semantics

- Signal definitions bind ID, domain, unit, baseline, absolute/relative soft threshold, criticality, maximum age and optional upper/lower hard bound.
- A measurement binds value, observation time, first-available time, evidence reference and evidence quality.
- Future-available observations are rejected to prevent look-ahead.
- Critical missing, stale or unknown evidence yields DATA_INCOMPLETE plus evidence-refresh-required, never NO_TRIGGER.
- Hard bounds and stale-plan limits bypass normal cooldown and soft hysteresis.
- A repeated persistent hard event does not generate an identical request every sample; a changed hard-reason signature does.
- Soft triggers use cooldown and a lower reset threshold (hysteresis) to avoid event chatter.
- Snapshot sequence and ID are pinned; exact duplicate replay is idempotent, conflicting content at the same sequence is invalid.
- Reason codes and receipts are canonicalized independent of input order.

## Comparator policies

Run each identical frozen scenario against:
1. fixed 15-minute re-plan;
2. fixed 60-minute re-plan;
3. energy-only / one-sided trigger;
4. logistics + energy bidirectional trigger (BETM-style);
5. multi-domain trigger including compute and cooling/thermal-store;
6. multi-domain trigger including evidence freshness and hard-boundary bypass.

## Metrics

Never report only a lower solver-call count. Report:
- re-plan invocations per 24h and reduction vs fixed 15-minute baseline;
- missed hard events and false-positive requests;
- event-detection delay and event-to-request latency;
- plan age by domain and maximum stale-plan interval;
- plan churn, berth/assignment changes and displaced compute jobs;
- shore-power delivered/unserved service and terminal throughput;
- compute useful work, on-time completion, queue growth and p95/p99 latency;
- PCC/feeder violations, peak, ramp and deferred-work rebound;
- cooling electricity, cooling margin, thermal-store reserve exhaustion and thermal excursions;
- renewable curtailment, cost and carbon on matched boundaries;
- stale/missing evidence coverage, signal freshness and decision-time leakage;
- monitor runtime, memory and deterministic receipt equality.

## Proposed gates

These are research hypotheses, not achieved results:
- zero missed hard-boundary events in the declared synthetic corpus;
- zero future-data leakage in negative controls;
- missing/stale critical evidence never classified NO_TRIGGER;
- no increase in service misses or hard constraint violations compared with fixed 15-minute re-planning;
- target at least 50% fewer re-plan requests than fixed 15-minute re-planning, conditional on equal hard-gate and service outcomes;
- report objective regret and plan churn alongside invocation reduction;
- replay 100 input permutations with byte-identical canonical receipts.

## Ownership

- #4582 remains canonical for port/berth/shore-power schedule verification and W/Wh/readiness semantics.
- #6211 remains owner for compute workload identity, service obligations and rebound admission.
- #2683 / #1916 remain owners for thermal-store state and multi-period thermal integration.
- #6228 owns ocean cooling scenario and 0D thermal accounting work.
- #6209 owns decision-time marine forecast replay.
- Symthaea proposes candidate plans and value-of-information requests; it does not verify its own outputs.
- Sol-Atlas supplies source-qualified spatial and marine context.
- Mycelix owns participant authority, provenance, approval and outcome evidence.

## Claim ceiling

A PASS establishes only deterministic trigger behavior over caller-supplied time-qualified signal definitions and snapshots. It does not establish physical truth, optimize schedules, validate forecasts, prove grid dynamics, provide protection, qualify thermal storage or authorize dispatch.
