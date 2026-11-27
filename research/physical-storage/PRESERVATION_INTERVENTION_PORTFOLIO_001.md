# Preservation Intervention Portfolio — v0.1

**Programme:** PRESERVATION-INTERVENTION-PORTFOLIO-001  
**Tracking:** [Symtropy #7673](https://github.com/Luminous-Dynamics/symtropy/issues/7673)  
**Watch source:** [Symtropy #7654](https://github.com/Luminous-Dynamics/symtropy/issues/7654)  
**Service qualification:** [Symtropy #7582](https://github.com/Luminous-Dynamics/symtropy/issues/7582)  
**Offline trust closure:** [Symtropy #7636](https://github.com/Luminous-Dynamics/symtropy/issues/7636)

## 1. Purpose

Select bounded portfolios of preservation interventions from a set of candidate actions, subject to hard cost, energy, elapsed-time, downtime, action-count, evidence, dependency, approval-authority and source-retention constraints.

This is a planning layer, not an executor. It does not mutate archives or release authority.

## 2. Paradigm shift: from alert lists to intervention portfolios

Preservation Watch reports which evidence-backed conditions are active, stale or unknown. The portfolio planner consumes the exact unresolved alert IDs and proposes bundles of actions that can jointly address them.

A portfolio is not considered successful merely because its action candidates list an alert under `addresses_alert_ids`. That field records intended coverage only. Every output carries postcondition evidence and approval requirements. Preservation Watch clears an alert only after an external verifier authenticates the completed remediation receipt and evaluates a new observation that explicitly supersedes the alert.

This separates:
- sensing and watch-state evaluation;
- planning and budget trade-offs;
- authorization and approvals;
- physical/digital execution;
- independent postcondition verification.

No single component self-certifies the whole lifecycle.

## 3. Inputs and trust boundary

The caller supplies:
- target and unresolved alert IDs;
- available prerequisite-evidence IDs;
- currently available approval-authority IDs;
- one currency code for all cost bounds;
- maximum cost, energy, elapsed duration and downtime;
- maximum action count;
- candidate actions and their dependencies, prerequisites, intended alert coverage, upper bounds and required approvals.

Each candidate includes a bounds evidence receipt ID and SHA-256 digest. The planner checks identifier/digest shape and the supplied `BoundsEvidenceStatus`. It does not calculate the digest, retrieve the receipt, authenticate the estimator, prove the estimate is conservative or validate its measurement boundary. The external evidence adapter owns those claims.

Only externally verified upper bounds are eligible. Missing/unverified bound evidence, missing prerequisites or unavailable approval authorities exclude a candidate. Invalid bound evidence is classified separately from missing evidence.

## 4. Hard constraints

A returned portfolio must:
- address every alert ID in the policy at least once;
- include every selected action's declared dependencies;
- have all required prerequisite evidence available;
- have every required approval authority available in policy;
- use only one frozen currency;
- remain within cost, energy, duration and downtime upper bounds;
- remain within the maximum action count;
- retain original source copies until independent destination acceptance;
- contain no destructive candidate.

A missing dependency, dependency cycle, unknown alert, mixed currency, malformed receipt reference, empty approval/postcondition set or arithmetic conversion invariant rejects the contract. Budget overrun is infeasible, not "almost passing".

Duration and downtime are summed as conservative serial upper bounds. The reference does not model parallel schedules or resource contention. The result is conservative for a serial execution plan, but is not necessarily the optimal schedule.

### Approval availability

The policy includes the currently available approval-authority IDs. A candidate whose required approvals are not all available is excluded with `MissingApprovalAuthority`. The planner does not infer that an action can proceed merely because an approval role is named. Output still lists every required approver; availability is a planning constraint, not authorization or approval.

## 5. Exact Pareto selection

v0.1 caps the candidate set at 16, so exhaustive enumeration visits at most (2^{16}-1=65,535) non-empty subsets.

For each valid subset, the scorer checks coverage, dependencies, preconditions and all hard caps. It computes:
- cost upper bound;
- energy upper bound;
- duration upper bound;
- downtime upper bound;
- action count.

Portfolio A dominates B when A is no worse on every objective and strictly better on at least one. The output is the exact non-dominated set, sorted by a stable lexicographic order. No arbitrary weighting hides trade-offs.

Action count is included as a fifth minimization dimension so an unnecessary zero-resource helper does not survive alongside an otherwise identical smaller plan.

Aggregate bounds are accumulated in u128 and compared with u64 policy caps. This allows the planner to skip a subset whose total exceeds a policy cap without aborting consideration of other subsets.

## 6. Dispositions

- **NoActionRequired:** no unresolved alerts and no candidate actions.
- **Feasible:** one or more portfolios meet all hard constraints; exact frontier returned.
- **Infeasible:** no portfolio meets coverage and budget/structural constraints.
- **EvidenceIncomplete:** no feasible portfolio remains because available candidates lack verified bounds or required precondition/approval evidence.
- **InvalidEvidence:** no feasible portfolio remains and candidate bounds include invalid evidence.

An eligible alternative portfolio may remain Feasible even if other candidates are excluded; all exclusions remain in the receipt.

## 7. Metrics and benchmark gates

Report:
- exact disposition and exact Pareto-frontier match;
- all enumerated subsets and wall-clock planning time;
- candidate and frontier counts;
- budget headroom by resource;
- evidence/approval exclusion reasons;
- dependency and coverage failures;
- deterministic output hash across reordered inputs;
- action-count distribution and redundant-action elimination;
- upper-bound estimation error versus later independently measured service receipts.

Prospective finite-corpus targets:
- 100% exact disposition and Pareto frontier;
- zero budget overruns;
- zero incomplete dependency closures;
- zero plans with missing prerequisites or unavailable approval authority;
- zero destructive/source-discarding selected actions;
- zero auto-executed interventions;
- zero alerts cleared by the planner;
- deterministic results for candidate-order permutations.

These are protocol targets, not measured operational performance.

## 8. Scaling boundary

The exhaustive solver is a reference oracle for at most 16 actions, not a production-scale optimizer. At 16 candidates, 65,535 subsets are finite and bounded for the reference implementation; this does not demonstrate real-time large-scale planning.

A future dynamic-programming, branch-and-bound or integer-programming implementation must reproduce the exact v0.1 frontier on the frozen corpus and property-generated small cases before it can be considered as a replacement. Optimizer speed is secondary to exactness and evidence integrity.

## 9. Evidence ceiling

Current bounds and fixtures are synthetic. No cost, energy, repair duration, downtime, service success, failure probability or preservation improvement is measured. A Verified status in a fixture is a declared test input, not an actual signed estimator receipt.

No intervention executor, media writer, hardware actuator, key manager, payment integration or authority-grant interface exists in v0.1. Any real operation requires separately approved execution, independent postcondition verification and a new Preservation Watch receipt.

## 10. Related standards and systems

The portfolio planner composes the following evidence lanes rather than replacing them:
- OAIS ISO 14721:2025 Preservation Objectives and Preservation Watch;
- BagIt RFC 8493 package fixity/transfer (not active-attack authenticity);
- OCFL 1.1 object/version layout;
- PREMIS 3.0 preservation events, agents and environment;
- Symtropy Physical Recovery Capsule and Offline Trust Closure;
- Symtropy Preservation Service Tournament;
- Mycelix common-cause cut-set assessment.

References:
- ISO OAIS: https://www.iso.org/standard/87471.html
- CCSDS OAIS Edition 3: https://public.ccsds.org/Pubs/650x0m3.pdf
- BagIt RFC 8493: https://www.rfc-editor.org/rfc/rfc8493
- OCFL 1.1: https://ocfl.io/1.1/spec/
- PREMIS: https://www.loc.gov/standards/premis/
