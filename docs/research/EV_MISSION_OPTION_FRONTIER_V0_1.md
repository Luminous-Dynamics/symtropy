# EV mission-option frontier audit v0.1

## Purpose

This tool compares predeclared mission alternatives that have already been evaluated by the vehicle mission-physics screen in Symtropy #2084. It does not simulate a route, choose a route, tune a vehicle, change a charger schedule or generate a new battery model.

The question is:

**Which predeclared alternatives reliably deliver the same minimum useful service, while offering an evidence-supported trade-off between battery energy and elapsed time?**

The implementation is tracked by [EV-MISSION-OPTIONS-001 #4761](https://github.com/Luminous-Dynamics/symtropy/issues/4761).

## Compose existing owners

- Symtropy #2084: deterministic vehicle mission-physics report and the existing vehicle, charging-queue and SOC-dependent charge-curve evaluators.
- Symtropy #2112: reference vehicle/design portfolio.
- Symtropy #2114: useful-service verification and independent physical test programme.
- Symthaea #7803: generic decision-value / experiment-selection benchmark. This tool is an alternative comparison audit, not another decision selector.
- Symthaea #11589/#11591: time-indexed battery mission capability.
- Symthaea #11633/#11635: independent physical mission-outcome validation.

## Paradigm shift: same service first

Comparing energy consumption without a fixed service requirement invites false improvements. A load-shed option can appear excellent by using less energy while delivering fewer tonnes, trips or hectares.

Every option set therefore declares one service contract:
- contract ID and canonical SHA-256;
- exact mission functional unit, such as "tonne-km per shift";
- service quantity unit, such as "tonne-km";
- a positive minimum service quantity.

Each candidate declares lower/upper bounds on service delivered. The lower bound must meet the contract minimum to enter the frontier. A lower-energy candidate that misses the service floor remains visible with INSUFFICIENT_SERVICE, but is excluded from the frontier.

## Source mission reports and integrity

Each candidate binds to:
- one exact report reference and SHA-256;
- one exact scenario ID inside that report;
- one SHA-256 of the selected scenario-result object.

The audit recomputes the canonical SHA-256 over the embedded mission report and selected scenario result. Canonical JSON sorts object keys; arrays (such as chronological mission segments) retain their given order. Reusing one report/scenario result for multiple candidates is rejected.

The consumed report must be the #2084 vehicle_mission_physics output family (EV-PLATFORM-001, profile set EV-MISSION-PHYSICS-001). The selected result must carry the exact same mission functional unit as the service contract.

The audit independently rechecks mass, peak power, energy margin and unserved-energy hard-gate fields. A mission report claiming a pass while one of those hard-gate fields fails is rejected as contradictory input. A report status of blocked or indeterminate cannot be promoted.

## v0.1 objective boundaries

The first version intentionally compares only two minimizing objectives:

| Objective | Source | Unit | Boundary |
|---|---|---|---|
| Net battery energy demand | #2084 scenario result, with declared bounds | kWh | Battery-terminal net energy |
| Elapsed mission time | Travel + stationary segment + declared non-travel time, with declared bounds | h | Vehicle out-the-door elapsed time |

The metric uncertainty intervals must contain the nominal values calculated from the source report. Each bound also carries a declared basis (DECLARED_HARD_BOUND or SCENARIO_ENVELOPE), evidence class, reference and digest.

The bounds are **not automatically 90%/95% confidence intervals or probability distributions**. No probability is invented from interval endpoints. The robust comparison treats each declared interval as a deterministic bound or scenario envelope. A future version may add a statistical uncertainty model, but only with an explicit probability source and independent calibration.

### Explicit boundary limitation

v0.1 compares battery-terminal net energy only. It does not yet compare charger/site input energy, trolley/direct-supply energy, preheat energy from a site supply, grid emissions, cost or charging queue delay. Those quantities must not be hidden inside battery energy or compared across candidates until their own common system boundary and evidence contracts are defined.

In particular, a trolley or external preheat option cannot claim an energy win by moving energy outside the battery boundary. It must later report both battery-terminal energy and the relevant site/direct-supply energy as separate metrics.

## Conservative robust dominance

All v0.1 objectives are minimized. Candidate A robustly dominates candidate B only when:

- A's battery-energy upper bound is no greater than B's battery-energy lower bound;
- A's elapsed-time upper bound is no greater than B's elapsed-time lower bound;
- at least one of those inequalities is strict.

This is stronger than comparing nominal points. It means the worst declared value for A is no worse than the best declared value for B in every objective. Where intervals overlap, the options remain incomparable under current evidence. Overlap is not silently resolved by midpoint, weighted sum or assumed probability.

The audit outputs:
- every candidate and its disposition/reason;
- separate source evidence grades for the mission result, service quantity, battery-energy interval and elapsed-time interval;
- pairwise robust-dominance relationships;
- the complete robust non-dominated frontier;
- candidate counts for eligible, blocked, insufficient-service and indeterminate options.

The overall source grade is derived from all four evidence components. Any unknown component keeps the set MIXED_OR_INDETERMINATE; a mixture of synthetic and evidence-backed sources is labelled MIXED_SYNTHETIC_AND_EVIDENCE_BACKED. A mission model's evidence class alone cannot conceal weaker or missing objective-bound evidence.

There is no universal scalar score and no single winner. A human or a separately authorized preference/decision policy can select from the frontier according to declared priorities.

## Candidate dispositions

- ELIGIBLE_SYNTHETIC_SCREEN: source mission screen passes with synthetic inputs; only an algorithm/software comparison.
- ELIGIBLE_EVIDENCE_BACKED_SCREEN_NOT_VALIDATED: source status indicates evidence-backed screen; it is still not physical vehicle qualification.
- BLOCKED_BY_MISSION_SCREEN: the source mission screen blocks the candidate.
- INSUFFICIENT_SERVICE: the conservative delivered-service lower bound misses the shared minimum.
- INDETERMINATE: source mission or objective/service evidence is unknown or cannot support the decision.

Malformed digests, incorrect mission-result identity, unit/boundary mismatch, contradictory source gates, duplicated scenario reuse or uncertainty intervals that fail to contain the source nominal value reject the option set.

## Frozen synthetic tournament

Eight synthetic candidates exercise:
1. Baseline route.
2. Energy-efficient alternative route: lower energy, longer time.
3. Faster route: higher energy, shorter time.
4. Wasteful route: robustly dominated by baseline.
5. Slow route: robustly dominated by the eco route.
6. Load-shed / reduced-work option: lower energy but below the service floor.
7. Power/energy-blocked option.
8. Unknown/out-of-domain option.

Expected results:
- 8 candidates;
- 5 screen-eligible candidates, 1 mission-blocked, 1 insufficient-service and 1 indeterminate;
- 2 robust-dominance pairs;
- 3 frontier candidates: baseline, eco route and fast route;
- mixed/indeterminate evidence grade because one candidate carries unknown source evidence; no physical result is admitted.

All mission reports, option identities, work quantities, energy/time values, uncertainties and digests are synthetic. These fixtures test comparability, hash binding and frontier arithmetic only. They do not establish actual energy savings, range, reliability, service output or physical feasibility.

## Benchmark strategy

### Software / algorithmic gates

- 100% agreement with hand-computable robust-dominance cases.
- Zero load-shed options below the contract minimum admitted to the frontier.
- Zero blocked, indeterminate, mismatched-unit or digest-mismatched options promoted.
- 100/100 input-order permutations produce identical canonical corpus and report digests.
- Every excluded candidate retains a disposition and reason.
- Zero weighted-sum or hidden preference ranking.

### Physical promotion path

When real mission outputs exist, bind vehicle/pack/configuration, model/profile digests, calibration and service measurements. Then consume independently admitted physical outcomes through Symthaea #11635 and report:
- energy per useful-service unit;
- on-time and completed service;
- missed work and load-shed;
- charge queue/dwell and downtime;
- reserve/temperature/voltage/power violations;
- false-feasible/false-infeasible rates and uncertainty coverage.

The independent sample unit must be vehicle/pack/mission, not repeated intervals from one vehicle. Synthetic cases never contribute to a physical rate.

## Industry comparison

Fleet charging schedulers and vehicle-energy design screens are already established engineering capabilities. PR #2084 itself already contains:
- a discrete-event charging queue comparing FCFS and earliest-departure-deadline-first;
- shared charger-port/site power and vehicle acceptance limits;
- piecewise SOC-dependent charge acceptance;
- longitudinal mission physics with mass, grade, drag, drivetrain, auxiliary and work-tool energy;
- a 19-design reference portfolio and 95 release-blocking verification gates.

The differentiator proposed here is not "AI route optimization". It is a conservative, evidence-bound comparison of alternative mission strategies that preserves the useful-service contract and uncertainty-overlap semantics. It composes with the existing schedulers and decision-value benchmark rather than replacing them.

Commercial battery products already market SOH tests, fault prediction, adaptive charging, fleet battery digital twins and BESS analytics. A mission frontier alone does not beat those products. A credible differentiator would require matched real-world useful-service cases, independently calibrated energy/thermal models, real charging infrastructure constraints and the physical outcome validation from #11635. No competitive performance claim is made here.

## Safety and claim ceiling

This is a read-only comparator of precomputed mission results. It cannot choose or alter routes, set speeds, shed loads, change battery/BMS/charger settings, command vehicles or authorize field testing. It is not a route optimizer, calibrated digital twin, physical vehicle qualification, safety case, certification or deployment approval.
