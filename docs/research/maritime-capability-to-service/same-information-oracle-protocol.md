# MCSC finite same-information oracle (R1) v0.1

Companion to [MCSC-SAME-INFO-ORACLE-001 #7834](https://github.com/Luminous-Dynamics/symtropy/issues/7834), under [MCSC #7686](https://github.com/Luminous-Dynamics/symtropy/issues/7686), composing KITE #7775 and its reusable replay engine.

## Scope of v0.1

This first executable oracle solves **one frozen decision epoch**. It is deliberately not a multistage optimizer, rolling-horizon scheduler, live recovery controller or clairvoyant scenario solver. Multi-epoch information trees are a later extension.

The question is narrower and exact:

> Given a finite set of scenarios, identical service obligations and a decision-time evidence snapshot, what action policies remain admissible when scenarios with identical observable information must receive the same decision?

## R0 / R1 / R2 distinction

- **R0 — clairvoyant bound:** may see future realized shocks. It is an information bound, not a deployable or fair online comparator.
- **R1 — same-information oracle:** groups scenarios by their KITE as-of snapshot at a common cutoff. It chooses one common action for each observable information set, then evaluates that policy over all scenarios in that group. It is compute-idealized finite exhaustive enumeration.
- **R2 — operational policies:** deterministic/FCFS, disruption management, rescheduling, proactive-reactive, rolling-horizon, robust and learned policies under the same evidence and a matched compute/wall-clock/action-publication budget.

R0, R1 and R2 must remain separate in scorecards. R1 v0.1 does not estimate what can be computed within a production deadline; its enumeration size and runtime must be reported.

## Information-set identity

The Rust implementation derives a deterministic grouping signature from:
- decision epoch and cutoff;
- logical evidence IDs and selected vintage IDs/revisions;
- visibility and admissibility dispositions;
- visible-but-not-yet-ingested/validated evidence dispositions.

Vintages not yet first-available are excluded from the observable signature. Event time does not make a backfilled record visible. A post-cutoff correction cannot alter the prior KITE snapshot.

The current signature is a deterministic grouping key, not a cryptographic commitment or source-authentication proof. Owner adapters still need exact source identity, immutable raw/vintage digests, authority and rights verification.

## Feasible action intersection

All scenarios sharing an information-set key must expose the same action-ID catalog. For an action to remain selectable at that information set, it must be:
- explicitly admissible in every scenario in the group;
- hard-constraint feasible in every scenario;
- free of protected service-floor breach.

An explicit safe-abstention action should be included in the action catalog where applicable. If no common action is feasible, v0.1 returns an error rather than inventing a fallback.

All scenarios in a solve must share exact instance ID/version, service ID, expected-unit denominator, deadline, decision epoch and cutoff. Outcome quantities are checked against the denominator; accepted, on-time, late, unmet and unresolved units cannot exceed the declared obligation. Minimum-viable recovery cannot occur after full restoration.

## Exhaustive enumeration and Pareto output

The solver enumerates one action for each distinct information-set group. It checks the Cartesian-product policy count before enumeration against a caller-supplied maximum. Overflow or budget exhaustion returns an error; no partial search is labelled optimal.

For each complete policy, the implementation constructs a **component-wise adverse envelope** across all frozen scenarios:
- receiver-accepted and on-time accepted quantities use the minimum;
- unmet, late, unresolved, service-deficit, recovery time, cost, emissions, requalification, replanning, plan churn and latency use the maximum.

This envelope may combine worst values from different scenarios and therefore need not equal any one realized scenario. It is a conservative vector summary, not a probability-weighted expectation or a physically realizable joint outcome.

The Pareto frontier preserves all non-dominated vectors. It does not scalarize cost against service or choose one universal winner. Any later scalar selection requires a documented decision-authority preference model after hard constraints, protected floors, rights and authority have passed.

## Implemented Rust API

The file research/maritime-capability-to-service/src/kite_oracle.rs provides:
- information_set_key
- solve_r1_finite
- dominates
- typed decision context, action outcome, service outcome vector, policy and result records.

The module composes crate::kite and does not create a second evidence store, common-cause graph, capability registry, receiver-service evaluator or authorization engine.

## v0.1 negative controls

- indistinguishable scenarios must share one selected action;
- different visible vintage histories may select different actions;
- indeterminate action evidence is not an admissible policy choice;
- a protected-floor failure cannot be promoted as feasible;
- differing action catalogs inside one information set are rejected;
- mismatched instance/service/denominator contexts are rejected;
- duplicate scenario IDs are rejected;
- exhaustive policy budget is checked before enumeration;
- Pareto service/cost trade-offs are preserved without scalarization;
- outcome quantities and recovery-time ordering are validated.

The unit and integration fixtures are synthetic, not a probability sample or operational port history.

## Benchmark reporting

For each frozen finite instance, report:
- scenario count, decision epoch, unique information-set count and actions per set;
- candidate policy count, budget, elapsed time and memory;
- common-action intersection and no-common-action cases;
- non-anticipativity violations (target zero);
- false future-information admissions (target zero);
- hard-floor and denominator violations (target zero);
- R1 component-wise adverse vectors and Pareto frontier size;
- separate R0/R1/R2 outcome and latency scorecards.

## Claim ceiling and next steps

A passing synthetic R1 oracle establishes only the declared one-epoch information grouping, common-action selection, finite enumeration and vector dominance semantics. It does not establish empirical event truth, operational performance, multistage optimality, probability-weighted risk, deployability, physical safety or authority to act.

Next:
1. independently compare the exhaustive output with an intentionally simple separate enumerator on all tiny one-epoch cases;
2. bind the current KITE manifest into a public API-level conformance test where its row schema can support it;
3. extend to 2–4 decision epochs using explicit scenario trees and non-anticipativity constraints at every node;
4. compare R2 policies under matched runtime budgets;
5. reproduce a published rolling/robust port benchmark before any SOTA claim.
