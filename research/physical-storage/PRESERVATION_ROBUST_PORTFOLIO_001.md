# Preservation Robust Portfolio — v0.1

**Programme:** PRESERVATION-ROBUST-PORTFOLIO-001  
**Tracking:** [Symtropy #7703](https://github.com/Luminous-Dynamics/symtropy/issues/7703)  
**Parent planner:** [Symtropy #7673](https://github.com/Luminous-Dynamics/symtropy/issues/7673)  
**Solver tournament:** [Symtropy #7676](https://github.com/Luminous-Dynamics/symtropy/issues/7676)

## 1. Research question

Does a preservation intervention portfolio remain feasible across a frozen set of coherent operating conditions, rather than only under one nominal vector of cost, energy, duration and downtime estimates?

The current planner is a useful exact small-instance reference for verified point upper bounds. It does not yet test sensitivity to correlated disruptions or changing service conditions. A portfolio that is cheapest in normal operation can become infeasible when a reader, site, power source, supplier, identity service or staff capability is unavailable.

This track extends the planning model. It does not replace Preservation Watch, the service scorer, the trust capsule or the existing nominal planner.

## 2. Paradigm shift: from point bounds to scenario-qualified envelopes

A scenario catalog is a frozen, versioned set of mandatory operating conditions. Example scenario families include:
- nominal operation;
- primary-reader outage or replacement;
- site or power disruption;
- supplier unavailability;
- identity/key service unavailable;
- offline recovery with no network access;
- staffing or specialist unavailability;
- compound events, where the conditions are jointly plausible.

Each scenario is a *coherent joint state*. Do not create impossible scenarios by independently combining mutually exclusive events. Common-cause dependencies should be represented inside the scenario or in a separately assessed dependency graph.

Every candidate action must carry one externally verified upper-bound vector for every mandatory scenario:
`(cost, energy, elapsed duration, downtime)`.

A missing or unverified scenario bound excludes that action from robust planning. Invalid evidence remains distinguishable from absent evidence. The planner must never fill a missing scenario with the nominal value, zero, an average or a guessed multiplier.

## 3. Static robust v0.1 semantics

The action set is chosen before the operating scenario is known. It is therefore a *static robust portfolio*, not an adaptive policy.

For portfolio `P`, scenario set `S`, resource `r`, and externally supplied action bound `u(a,s,r)`:

`W(P,r) = max over s in S of sum over a in P of u(a,s,r)`.

A portfolio is robust-feasible only when:
- all unresolved alerts are covered;
- all selected dependencies are closed;
- prerequisite evidence and approval authorities are available;
- source retention and non-destructive constraints hold;
- every resource cap is satisfied in every mandatory scenario.

The Pareto vector is:
`(W_cost, W_energy, W_duration, W_downtime, action_count)`.

No weighted sum is emitted. The exact non-dominated robust frontier is returned. For every resource objective, the receipt lists all scenario IDs tied for its worst-case value. Different resources may have different limiting scenarios.

This objective-wise worst-case vector is conservative and auditable. It is not an expected value or a probability of failure.

## 4. Scenario identity and evidence closure

A run binds:
- scenario-set ID;
- scenario-set SHA-256 digest;
- exact ordered scenario IDs;
- target and policy digest;
- action-bound receipt IDs and SHA-256 digests.

The ordered scenario catalog is canonicalized before hashing. Changing the scenario set creates a new run identity; a frontier produced under one catalog must not be silently reused under another.

Every action must contain exactly one bound for each catalog scenario. Duplicate or unknown scenario IDs are contract errors. Missing scenario bounds make that candidate evidence-incomplete. A scenario bound is eligible only when its external evidence status is Verified and its receipt reference is well-formed.

The planner checks structure and arithmetic; it does not authenticate the receipts, certify the estimator, validate the physical measurement boundary, or prove that an upper bound is conservative. Those remain independent evidence-adapter responsibilities.

## 5. What this model deliberately does not assume

- No scenario probabilities are inferred from the number or naming of scenarios.
- No independence is assumed between hazards.
- No expected-cost, chance-constraint or CVaR calculation is made.
- No scenario-dependent action switching or adaptive recourse is permitted.
- No action is approved, authorized, executed or treated as remediation.
- No scenario catalog proves completeness over all possible real-world events.

Expected-value or distributionally robust extensions require calibrated probability evidence, a declared sampling frame, treatment of dependence, independent calibration review and held-out validation. Adaptive recourse requires non-anticipativity constraints and a separate proof that decisions do not use future information.

## 6. Reference implementation boundary

The first exact robust reference retains the v0.1 limit of 12 candidate actions (4,095 non-empty subsets). It enumerates candidate subsets, computes each resource total per scenario, rejects any portfolio that exceeds a cap in any mandatory scenario, computes the objective-wise worst-case envelope, then filters the exact robust Pareto frontier.

The worst-case envelope adds scenario evaluation to each subset. It does not remove the exponential subset enumeration or prove a scalable solution. PAUGMECON, SAUGMECON, MOBAB-CP and CP-SAT remain separate solver-tournament comparators under #7676.

## 7. Frozen corpus and prospective benchmarks

The v0.1 corpus covers:
- one-scenario nominal equivalence;
- rejection of a nominally cheap portfolio that exceeds a cap during a reader outage;
- incomparable cost/energy robust trade-offs;
- missing, unverified and invalid scenario evidence;
- exact hard-cap boundaries and scenario-only overruns;
- dependency closure and multi-alert coverage;
- destructive/source-discarding exclusions;
- ties across multiple limiting scenarios;
- deterministic output under action and scenario permutations.

Prospective targets:
- 100% known-answer disposition and exact frontier agreement on the frozen small corpus;
- zero missing/unknown/duplicate scenario false acceptance;
- zero per-scenario budget violations;
- zero dependency, approval, prerequisite, source-retention or destructive-action violations;
- exact equality with the nominal planner when the catalog contains one scenario and resource units/inputs are equivalent;
- deterministic canonical output independent of input ordering;
- independent implementation agreement before increasing the action cap.

Performance reporting must include scenario count, candidate count, subset count, feasible-subset count, Pareto-front size, model-construction time, search time, peak memory, median/p95 over repeated runs and all incomplete/time-out outcomes.

## 8. How to compare with industry and research

The standards and product comparison is complementary, not competitive:
- OAIS ISO 14721:2025 defines the archival reference model and preservation responsibilities.
- ISO 16363:2025 defines audit/certification criteria for trustworthy digital repositories.
- NDSA Levels 2.1 (March 2026) adds environmental sustainability guidance.
- PREMIS 3.0 provides preservation event/agent/object metadata.
- BagIt RFC 8493 and OCFL 1.1.1 provide interoperable packaging and object-layout foundations.
- SPECstorage Solution 2020 measures application-style storage performance; it is not an end-to-end accepted-recovery benchmark.
- SNIA Emerald measures storage-system power efficiency under defined procedures; its subsystem boundary must not be relabelled whole-service energy.

This track adds an explicit stress-test envelope around the intervention portfolio. It does not claim that these standards or industry products already provide, or fail to provide, any particular feature without an implementation-level comparison.

## 9. Claim ceiling

Synthetic scenarios establish only that the model follows the declared contract for those inputs. They do not establish physical reliability, archive lifetime, energy, carbon, restoration throughput, actual outage likelihood, supplier resilience, recovery-time objectives or a SOTA improvement. Operational deployment requires independently qualified scenario definitions, measured bounds, authenticated evidence, independent validation and separately authorized execution.
