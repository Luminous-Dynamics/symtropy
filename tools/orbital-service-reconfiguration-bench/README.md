# Orbital Service Reconfiguration Benchmark v0.1

**Status:** deterministic synthetic reconfiguration reference; no physical or flight service is qualified.  
**Owner:** [ORBITAL-SERVICE-RECONFIGURATION-001 #6375](https://github.com/Luminous-Dynamics/symtropy/issues/6375).  
**Parent chain:** nominal compatibility #6336 → robust compatibility #6372 → service reconfiguration.

## Paradigm shift

Redundancy is not the number of nominal providers. It is the ability to preserve an accepted-service floor after a deterministic disruption by switching to an alternative whose exact configuration remains robustly qualified, independent of the failed domains, available within the mission window, and ready after handoff/requalification.

The adapter calls the robust compatibility evaluator from #6372 for each provider at the declared activation tick and again during each demand-bearing service tick. It does not reimplement interface or uncertainty semantics. Activation-time qualification is mandatory even when no demand happens to be due at that exact tick.

## State and decision rules

- A prior Unknown service outcome blocks a new mutation/reconfiguration until it is reconciled.
- The fallback must have current robust qualification for the exact client, profile, configuration, lifecycle state and tick.
- A fallback sharing an active failed domain is rejected.
- Handoff and requalification must be complete before activation.
- Provider availability, spare and propellant readiness are hard gates.
- Deterministic fault sets have no implied probabilities.
- Partial or unaccepted service does not count as accepted service.
- No spacecraft, valve, fluid or propulsion command is issued; `authority_granted` is always false.

## Frozen scenario corpus

The 26-case corpus covers nominal operation, qualified independent failover, shared power/depot/sub-tier failures, robust-negative and expired qualification, client/configuration mismatch, prior-outcome Unknown vs reconciled, incomplete handoff/requalification, missing spare/propellant, unavailable provider, unknown or missing fault domains, no-resupply service shortfall, increased demand, partial acceptance, duplicate allocations and service continuity false positives.

## Frozen replay and shortcut-baseline result

The frozen corpus contains 26 synthetic scenarios across 14 provider configurations. Independent expected outcomes are:
- 3 service-floor-met synthetic;
- 13 rejected plans;
- 6 indeterminate;
- 4 service shortfalls;
- 0 expected-result mismatches in a separate JavaScript replay of the deterministic contract.

A deliberately weak **nominal-only greedy fallback** was replayed on the same 26 scenarios. It uses the declared plan, nominal profile fit and the provider's declared availability/fault membership, but ignores robust UQ/effectivity, pair-scoped qualification, service-resource readiness, handoff/requalification completion, prior-outcome reconciliation, and whether a complete failure-domain graph exists.

| Evaluator | Service-floor claims | False continuity claims |
|---|---:|---:|
| Nominal-only greedy fallback | 16 / 26 | 13 |
| Frozen evidence-qualified oracle | 3 / 26 | 0 |

A false continuity claim is a nominal-only "floor met" decision where the frozen reference disposition is Rejected, Indeterminate or ServiceShortfall. These are adversarial synthetic fixture results, not an industry false-accept rate or comparison with a production spacecraft operations planner.

## Metrics

The receipt reports accepted and unmet service units, outage ticks, first restored-service tick, qualification status and accepted units per tick. It reports service-floor success, shortfall, indeterminate and rejected counts separately. No weighted aggregate can hide a service-floor failure.

## Baselines for future matched comparison

1. Static N-1 provider count / nominal capacity.
2. Nominal-only greedy fallback.
3. Supplier/fault-domain-aware fallback without robust UQ/effectivity.
4. Robust qualification-aware fallback (this benchmark).
5. Exact small-instance enumeration oracle.

Use identical provider sets, demand traces, fault sets, time windows and compute budgets. Report regret only where an exact oracle is practical. These are proposed comparison baselines, not claimed SOTA results.

## Integration boundaries

- #6329 / #6336 nominal interface conformance.
- #6351 / #6372 robust uncertainty/effectivity qualification.
- #6221 receipt composition.
- #1749 common-cause fault-set semantics.
- #1750 supplier concentration and qualified alternatives.
- #3254 no-resupply/continuity.
- #6256 / #6303 future ground test cell.

## Claim ceiling

A pass establishes deterministic synthetic reconfiguration and accounting semantics over this frozen corpus only. It does not establish real fault independence, statistical availability, physical mating, seal integrity, metrology truth, flight qualification, mission reliability, operational autonomy or commercial superiority.
