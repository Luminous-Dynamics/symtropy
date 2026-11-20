# Orbital Service Reconfiguration Benchmark v0.1

**Status:** deterministic synthetic qualification-aware capacity-planning reference; it does not measure actual accepted orbital service.  
**Owner:** [ORBITAL-SERVICE-RECONFIGURATION-001 #6375](https://github.com/Luminous-Dynamics/symtropy/issues/6375).  
**Stack:** nominal compatibility #6336 → robust compatibility #6372 → service reconfiguration.  
**Actual receiver-accepted service evidence:** separate composition/receipt owners #6221 and #6384.

## Paradigm shift

Redundancy is not provider count. It is the ability to preserve a **qualified and available capacity plan** after deterministic disruption by switching to an alternative whose exact configuration remains robustly qualified, independent of failed domains, available within the mission window, and ready after handoff/requalification.

This benchmark evaluates:
- planned demand and candidate service capacity;
- provider qualification and exact lifecycle/configuration effectivity;
- dependency and supplier common-cause exclusions;
- handoff and requalification timing;
- spare/propellant readiness;
- prior-outcome reconciliation and complete service-kind matching.

**Important semantic boundary:** `qualified_capacity_units` is a planning estimate derived from the lesser of demand and a qualified provider's declared per-tick capacity. It is not a measured transfer, receiver-accepted mass, completed service event, or proof that a spacecraft received anything. The current implementation deliberately calls the result a planned-capacity floor, not service acceptance. Actual receiver-accepted quantities must come from the byte-exact source receipt composition in #6221 and the receipt-admission owner #6384.

## State and decision rules

- Prior Unknown operation outcome blocks a new mutation/reconfiguration until reconciled.
- Exact provider and client profile/configuration/lifecycle identity is required.
- Robust qualification is checked at planned activation, even when no demand occurs at that exact tick, and again on demand-bearing ticks.
- A fallback sharing an active failed dependency is rejected.
- Supplier identity itself is an implicit shared failure domain; different provider names or component graphs do not make the same supplier independent.
- Handoff and requalification are sequential in v0.1. Their durations are added conservatively. Overlap requires an independently qualified transition-dependency contract; the benchmark does not assume overlap.
- Provider availability, exact service-kind match, spare and propellant readiness are hard gates.
- Deterministic fault sets have no implied probabilities.
- No spacecraft, valve, fluid or propulsion command is issued; `authority_granted` is always false.

## Frozen scenario corpus

The 28-case corpus covers nominal operation, robustly qualified independent failover, shared power/depot/sub-tier/supplier failures, robust-negative and expired qualification, client/configuration mismatch, prior-outcome Unknown versus reconciled, incomplete sequential handoff/requalification, missing spare/propellant, unavailable provider, unknown or missing fault domains, no-resupply capacity shortfall, demand surge, duplicate allocations, wrong service scope, and a mechanically qualified provider that does not support the required complete prepared-fluid service.

## Independent replay and matched nominal-only baseline

A separately implemented JavaScript reference replay matched all 28 expected scenario dispositions:

| Planned-capacity disposition | Cases |
|---|---:|
| Planned-capacity floor met (synthetic) | 3 |
| Rejected plan | 15 |
| Indeterminate | 6 |
| Planned-capacity shortfall | 4 |
| Expected-result mismatches | 0 |

A deliberately weak nominal-only scheduled fallback baseline uses nominal profile/ICD/frame fit and declared provider availability/fault membership, but ignores robust uncertainty/effectivity, exact lifecycle/configuration binding, service-kind completeness, resource readiness, sequential transition readiness, prior-outcome reconciliation, supplier identity as a common cause, and missing failure-domain graphs.

| Evaluator | Planned-capacity floor claims | False capacity claims |
|---|---:|---:|
| Nominal-only scheduled fallback | 17 / 28 | 14 |
| Evidence-qualified capacity reference | 3 / 28 | 0 |

A false capacity claim means the nominal-only baseline says planned capacity meets the declared floor while the frozen reference says Rejected, Indeterminate or PlannedCapacityShortfall. These are adversarial synthetic fixture results, not operational availability rates or comparisons against a production flight planner.

## Metrics

Report separately:
- planned demand and declared provider capacity;
- qualified capacity available over the mission horizon;
- planned capacity shortfall by tick;
- outage/capacity-unavailable ticks and first restored-capacity tick;
- robust qualification pass/reject/indeterminate outcomes;
- qualification age and requalification lead-time burden;
- capacity after deterministic fault sets;
- fault-domain and implicit supplier overlap;
- required spares, propellant, tooling and logistics unavailable;
- handoff and requalification time;
- nominal-only false capacity claims.

Do not report these values as receiver-accepted mass, completed transfer, real mission availability, or actual service continuity. Actual accepted service is reported only from the separately admitted receiver receipts.

No weighted scalar may hide a hard capacity-plan failure. No deterministic fault corpus may be converted into invented outage probabilities.

## Competitive baseline protocol

For identical provider sets, demand traces, fault sets, transition windows and qualification inputs, compare:
1. Static N-1 provider count / nominal capacity.
2. Nominal-only greedy fallback.
3. Supplier/fault-domain-aware fallback without robust UQ/effectivity.
4. Robust qualification-aware planned-capacity fallback (this benchmark).
5. Exact small-instance enumeration oracle.

Use equal inputs and disclose compute budgets. Compare exact feasible plans on held-out small cases. Do not claim better-than-SOTA without a matched external algorithm, identical constraints, held-out cases and reproducible results.

## Integration boundaries

- #6329 / #6336: nominal interface conformance.
- #6351 / #6372: robust uncertainty/effectivity and pair-scoped dependence.
- #1749: common-cause fault sets.
- #1750: supplier concentration and qualified alternatives.
- #3254: no-resupply continuity.
- #6221: delivery → capture → receiver-accepted fluid composition.
- #6384: exact-byte receipt admission and time-indexed receiver-accepted service.
- #6256 / #6303: future uncrewed ground test cell.

This crate consumes the robust compatibility evaluator. It does not create another interface, uncertainty, fluid, capture, reliability or repair-physics solver.

## Claim ceiling

A passing v0.1 establishes only deterministic synthetic qualification-aware capacity-planning semantics over the frozen corpus. It does not establish receiver-accepted service, statistical availability, real common-cause independence, physical mating, seal integrity, calibrated measurement, flight qualification, mission reliability, operational autonomy or commercial superiority.
