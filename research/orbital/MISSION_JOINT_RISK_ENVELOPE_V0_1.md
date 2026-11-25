# Mission-Level Joint Encounter Risk Envelope v0.1

**Status:** deterministic finite-support research kernel. Exact-head Rust validation pending.  
**Related work:** [#7502](https://github.com/Luminous-Dynamics/symtropy/issues/7502), pathwise support [#7443](https://github.com/Luminous-Dynamics/symtropy/pull/7443).  
**Claim ceiling:** exact aggregation over the supplied finite target measure and declared event inventory. No continuous-domain Pc or physical-completeness claim.

## Motivation

A per-encounter threshold does not bound total mission-sequence risk. Several encounters can each be below an operational mitigation threshold while their union exceeds a mission-level risk budget.

NASA's 2026 conjunction assessment handbook describes multi-encounter assessment and gives dependence-free bounds for the probability of one or more collisions:

\[
\max_i P(E_i) \le P\left(\bigcup_i E_i\right)
\le \min\left(1,\sum_i P(E_i)\right).
\]

It also discusses the independence-based survival product:

\[
P\left(\bigcup_i E_i\right)
\approx 1-\prod_i(1-P(E_i)),
\]

which is a model assumption, not a property of repeated conjunctions. Where the multi-encounter upper bound is decision-relevant, the handbook recommends more detailed Monte Carlo investigation. The primary reference is the [NASA Conjunction Assessment Handbook, 2026](https://www.nasa.gov/wp-content/uploads/2026/08/ca-handbook-volume1-nasa-sp-20205011318-rev2-vol1.pdf).

## Paradigm shift

Use one shared trajectory realization as the unit that carries all encounter outcomes across the mission horizon. Do not sample or classify each encounter independently and then infer dependence after the fact.

For each trajectory atom \(j\), the frozen event vector is

\[
\mathbf{x}_j=(1_{E_1},1_{E_2},\ldots,1_{E_K}).
\]

With exact finite target-measure mass \(w_j\), total mass \(W=\sum_j w_j\), and complete definite event outcomes:

\[
\widehat P(E_i)=\frac{\sum_j w_j 1_{E_i(j)}}{W},
\qquad
P_{\mathrm{union}}^{\mathrm{finite}}
=\frac{\sum_j w_j 1_{\{\exists i:E_i(j)\}}}{W}.
\]

The implementation also reports pairwise intersections, the complete event-signature histogram, Fréchet bounds, and an independence approximation. The exact union and mission-budget decision are computed from shared trajectory signatures—not from the independence approximation.

## Executable increment

The new Rust module is rare-event-kernel/src/mission_risk_aggregation.rs. It:
- pins a nonzero manifest, target-measure and event-inventory identity;
- requires canonical event IDs and trajectory atom IDs;
- requires caller declarations that event and trajectory inventories are complete;
- requires one definite contact/no-contact outcome per event per atom;
- rejects ambiguous, missing, duplicate and unknown outcomes;
- verifies the exact atom set and total integer mass;
- computes exact rational marginals, pairwise intersections, union mass, no-event mass and Fréchet bounds;
- emits joint event signatures and an independence approximation for comparison;
- compares exact finite-support union mass to a declared rational mission budget with integer cross multiplication;
- never grants operational authority.

The support bound is 32 events and 65,536 trajectory atoms in v0.1. The independence approximation is floating-point and diagnostic only. Exact rational outputs and the mission-budget comparison use integer arithmetic.

## Synthetic controls

The suite includes:
1. Independent events: the exact pathwise union matches the independence comparator.
2. Perfect positive dependence: two 50% marginals have a 50% union, while the independence approximation reports 75%.
3. Disjoint events: two 40% marginals have an 80% union, while independence reports 64%.
4. Repeated encounters: five events each have 1% marginal mass, but disjoint pathwise occurrence produces a 5% mission union, exceeding a 4% declared budget.
5. Ambiguous, missing, duplicate and unknown event rows.
6. Omitted trajectory support, incomplete event declarations and total-mass mismatch.
7. Input atom and per-atom outcome permutation invariance.
8. Invalid budget and noncanonical event inventory.

These are exact arithmetic controls over synthetic finite distributions, not propagated orbital cases.

## Benchmark programme

| Benchmark dimension | Control | Required result |
|---|---|---|
| Dependence | independent, perfect-positive, disjoint | exact pathwise union and correct comparator direction |
| Repeated events | each event below budget, union above | mission-level exceedance detected |
| Dependence-free bounds | known marginals | observed union inside sharp Fréchet interval |
| Event coverage | omit, duplicate, rename or ambiguously classify an event | fail closed |
| Trajectory coverage | omit, duplicate or alter an atom ID | fail closed |
| Mass integrity | zero, overflow, or wrong total mass | fail closed |
| Determinism | permute atoms and event rows | byte-equivalent mathematical report |
| Physical validation | same ephemeris, covariance, hard-body radius, event horizon and propagator | calibrated comparison with high-fidelity Monte Carlo |

For physical studies, report per-event marginal Pc, sum of marginals, Fréchet bounds, pathwise mission union, pairwise intersections, independence approximation error, joint-signature frequencies, confidence/coverage, trajectory evaluations, p50/p95/p99 runtime, memory and abstention rate. Compare against the applicable NASA CARA multi-encounter procedure and independently calibrated high-fidelity Monte Carlo.

Prospective targets: zero false independence claims; zero qualification of incomplete/ambiguous event inventories; exact rational control results; exact permutation invariance; and independently demonstrated interval calibration on physical cases. No speed or SOTA superiority is claimed yet.

## Limits

The kernel aggregates a declared finite target measure; it is not itself a Monte Carlo estimator or orbital propagator. It does not validate that integer weights are calibrated nominal probabilities, authenticate the source digests, prove that the trajectory support or event universe is physically complete, model continuous dynamics, or establish covariance realism. The completeness booleans and digests are supplied evidence pins, not mathematical proofs of the underlying physical claims. A budget comparison is only against the declared finite measure and budget. The kernel does not select or authorize maneuvers.
