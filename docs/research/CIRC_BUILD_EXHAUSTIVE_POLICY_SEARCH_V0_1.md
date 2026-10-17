# CIRC-BUILD-004 — exhaustive finite lifecycle policy search v0.1

**Owner:** Symtropy  
**Composes:** CIRC-BUILD-002 multi-cycle horizon model and its independent reference simulator  
**Claim ceiling:** exhaustive only over the declared finite synthetic action universe

## Paradigm shift

A Pareto frontier over a hand-authored shortlist proves only that no candidate in the shortlist dominates another. It does not establish that the shortlist contains the best feasible policy or that a superior sequence was not omitted.

For the finite reference model, enumerate every sequence in the declared action universe. This provides an exact completeness benchmark against which heuristics, pruning, and learned policy proposals can be measured later.

Also keep the end-of-horizon asset state visible. The exhaustive search adds terminal condition reserve as a maximized objective alongside lineage-weighted service hours; it does not assume that two plans delivering equal historical service leave equivalent assets.

## Frozen search space

- Six canonical action profile IDs: continue, repair, remanufacture, replacement with selective recycling, replacement with energy recovery, and replacement with mixed residual disposal.
- Three service cycles.
- Exactly 6^3 = 216 ordered action sequences.
- Diagnostic negative-control profiles remain in the fixture but are excluded from the candidate action universe by an explicit, frozen allowlist.
- Candidate identity is the ordered action sequence joined by slash; action IDs containing slash are rejected.
- Enumeration order is canonical and does not depend on the order in which the action allowlist is supplied.

## Per-policy evaluation

Every sequence passes through the same lifecycle hard gates and independently structured reference transition. A policy is eligible only if it delivers the full service horizon and survives source evidence, safety/function/inspection, condition, repair-budget, and mass-fate constraints.

The receipt reports candidate and unique-ID counts, eligible/rejected counts, all rejection dispositions, every candidate's condition trace and resource/fate totals, and production and reference Pareto frontiers. The finite frontier minimizes lifecycle cost, virgin material, energy, GHG impact, water, and residual waste; it maximizes lineage-weighted service hours and terminal condition reserve in basis points. No scalar score is used.

## Source-side reference result

An independent enumeration of the frozen source fixture produces:

| Result | Expected count |
|---|---:|
| Unique ordered policies | 216 |
| Eligible, complete-horizon policies | 198 |
| Rejected for condition-floor violation | 6 |
| Rejected for repair-budget violation | 12 |
| Non-dominated policies | 2 |

Expected terminal-reserve-aware frontier:
- continue/continue/remanufacture
- continue/continue/repair

The state transition is checked against the independent reference implementation for every candidate; the Pareto frontier is checked against full pairwise integer-vector enumeration.

## Proposed acceptance targets

- 100% production/reference transition agreement across all policies.
- 100% agreement with the independent exhaustive frontier oracle.
- Exactly 216 unique policy identities and a fully reconciled candidate census.
- Zero incomplete-horizon plans in the eligible set.
- Rejection counts by disposition sum to rejected count.
- Action-list input-order permutations produce identical per-policy outcomes and frontier.
- Every report binds exact manifest-byte SHA-256, source revision, tool version, action universe, horizon, and claim ceiling.
- Candidate enumeration is bounded to at most 1,000,000 policies; larger spaces are rejected rather than allowed to exhaust unbounded resources.

## Claim ceiling

This is exhaustive only over the declared six-action, three-cycle synthetic universe. It does not establish real maintenance optimality, empirical remaining useful life, material genealogy, LCA, environmental benefit, procurement authorization, or physical-control authority.
