# Bounded extensive-form R1 policy enumerator v0.1

Tracked by [MCSC-R1-EXTENSIVE-FORM-001 #7864](https://github.com/Luminous-Dynamics/symtropy/issues/7864), following the one-epoch R1 oracle and multi-epoch non-anticipativity validator.

## Purpose

Move from validating a supplied multi-stage policy to finding the best bounded finite policy under same-information constraints. The solver chooses one action per information set, not one action per scenario, and evaluates complete horizon outcomes.

## v0.1 model boundary

The solver consumes a frozen finite set of exogenous scenario observation paths. Each scenario-stage contains:
- decision epoch, cutoff and SHA-256 commitment to the complete observable state;
- a common action-ID catalogue for every scenario in the same information set;
- scenario-specific evidence admissibility and hard-constraint status for each action;
- additive service, cost, emissions, latency and resource-use contribution for each action;
- optional time-to-minimum-viable and time-to-full-service attainment times.

Scenario observations are exogenous frozen paths in v0.1. Actions change the accumulated outcomes but do not generate new observations or alter the later path transitions. This is an extensive-form policy enumeration over a fixed scenario tree, not a closed-loop port digital twin.

## Algorithm

1. Validate common service context and identical strictly increasing epoch/cutoff schedules.
2. Group all scenario-stage decisions by (decision epoch, cutoff, observable-state digest).
3. Verify action-ID catalogues are identical inside every information set.
4. Intersect action feasibility across all scenario-stage members of each information set; an action must be explicitly admissible, hard-feasible and free of protected-floor breach everywhere in the set.
5. Enumerate exactly one common action per information set. Compute the Cartesian-product policy count with checked arithmetic before enumeration and enforce a caller-supplied maximum.
6. Replay each policy through every frozen scenario path, adding stage contributions with checked integer arithmetic.
7. Enforce horizon service conservation: receiver accepted + unmet + unresolved equals the fixed expected denominator; on-time + late + acceptance-time-unknown equals accepted.
8. Compute component-wise adverse service outcomes with a witness scenario for every dimension, plus a separate component-wise resource-use envelope and witnesses.
9. Return the complete Pareto frontier over service, recovery, stability, cost, emissions, latency and resource-use dimensions, without scalar weights.

## Outcome semantics

Stage contributions are additive across the horizon. `unmet_units` and `unresolved_units` are horizon residual classifications and should normally be supplied at the final stage, not repeated at every epoch. A policy whose aggregate accepted/unmet/unresolved quantity does not exactly equal the expected denominator is rejected.

On-time, late and acceptance-time-unknown classifications must partition receiver-accepted units at every stage and at the horizon. Unknown timing is not silently promoted to on-time.

Time-to-minimum-viable and time-to-full-service are absolute times from the common disruption epoch. The first attainment time across stages is retained. When a threshold is never reached, the output uses an explicit `service_deadline + 1` sentinel, which is documented by the caller's frozen context.

Resource use is tracked separately for crane minutes, shore-power Wh, yard slot-hours, labor-hours and freshwater litres. These are minimized in the Pareto comparison alongside the service vector. They are not converted into a scalar utility.

## Pareto and adverse aggregation

Each complete policy is evaluated for every scenario. For service dimensions, accepted and on-time service use the minimum across scenarios; losses, time-to-restoration, cost, emissions, churn and latency use the maximum, with scenario witnesses and deterministic lexicographic tie-breaking. Resource-use dimensions use their maximum across scenarios with witnesses.

Policy A dominates policy B only when A is no worse on every service and resource dimension and strictly better on at least one. The full nondominated frontier is returned. Equal vectors from different policies are retained rather than arbitrarily selecting one.

The component-wise envelope can combine adverse values from different scenarios; it is not necessarily a realizable joint scenario outcome and has no probability weights.

## Required controls

- identical information sets receive exactly one common action choice;
- later observable divergence permits later policy divergence;
- action feasibility is the intersection across all compatible scenarios;
- no common feasible action fails closed; include an explicit safe-abstention action where appropriate;
- exact service and accepted-timing partitions are enforced at horizon;
- protected-floor breach is excluded from feasible policy choices;
- checked arithmetic prevents cumulative counter overflow;
- search-budget and candidate-count overflow are explicit;
- duplicate scenario IDs, malformed digests, context mismatch and schedule mismatch are rejected;
- input ordering does not change the semantic result or Pareto frontier.

## Benchmark ladder

- Tiny exhaustive trees: 2–6 scenarios, 1–4 epochs, 1–4 actions per information set; exact comparison with an independent test-only enumerator.
- Medium synthetic trees: 8–16 scenarios and 2–6 epochs, only after tiny-tree reference agreement.
- Report scenario count, stage decisions, information sets, actions per set, candidate/evaluated policies, Pareto size, elapsed time, peak memory and budget exhaustion.
- Compare R0 clairvoyant, R1 extensive-form, and R2 online methods separately; R0 is not a fair online comparator.
- Target zero non-anticipativity, future-information, service-partition and protected-floor violations.
- For R2 comparisons, match evidence, action catalogues, constraints, hardware, CPU/memory and wall-clock budget.

## Deliberate limitations

v0.1 uses precomputed exogenous scenario paths and additive action contributions. It does not model action-dependent state transitions, physical resource inventory evolution, berth overlap between vessels, quay-crane interference, yard topology, queue spillback, common-cause propagation, scenario probabilities or causal effects. It does not authorize live actions.

The next research stage is a state-transition-aware extensive-form model where action choices change later available resources and observable evidence, while preserving non-anticipativity and exact replay.