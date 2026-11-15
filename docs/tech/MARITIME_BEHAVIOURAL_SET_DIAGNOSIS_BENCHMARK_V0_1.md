# Maritime bounded behavioural-set diagnosis benchmark v0.1

Programme: MARITIME-BEHAVIOURAL-SET-DIAGNOSIS-001 (#5040)
Delivery: executable synthetic P0 contract
Evidence class: synthetic known truth only

## Objective

Move fault diagnosability beyond independent point and interval signatures toward bounded sets of admissible physical output trajectories under parameter, disturbance and measurement uncertainty. For a fixed input trajectory, asset configuration and operating regime, each fault hypothesis induces a set of possible output trajectories.

The benchmark distinguishes four claims:

1. SEPARATION_PROVEN — certified outer enclosures are disjoint. Disjoint certified supersets imply the underlying admissible sets are disjoint.
2. INTERSECTION_WITNESS_FOUND — a common trajectory point lies in a verified inner region for each hypothesis. Because each inner region is declared a subset of its admissible set, this is a constructive common-behaviour witness.
3. NOT_PROVEN_SEPARABLE — conservative outer enclosures overlap, but no verified common inner witness is available. This does not prove physical non-identifiability.
4. NOT_EVALUATED — configuration, regime, model, solver, trajectory, uncertainty or certification evidence is unavailable or unqualified.

## Geometry contract

The P0 evaluator represents a flattened, labelled output-time trajectory as a vector in R^n. Each hypothesis supplies a finite union of closed axis-aligned boxes.

An outer-box pair is disjoint only if at least one coordinate satisfies:

    upper_A[k] < lower_B[k] OR upper_B[k] < lower_A[k]

The two union-valued outer sets are certified disjoint only when every cross-product outer-box pair is disjoint. Touching closed boxes are not disjoint.

For inner regions, the evaluator searches for a pair of verified inner boxes with non-empty intersection. It emits a deterministic point in that intersection. An unverified inner region cannot create an intersection witness.

An overlapping pair of outer boxes without a common verified inner point is NOT_PROVEN_SEPARABLE, not INTERSECTION_WITNESS_FOUND.

## P0 limits and certificate trust boundary

- The executable supports axis-aligned boxes and finite unions only. It does not preserve arbitrary cross-time or cross-channel correlation and can be conservative.
- Fixtures declare synthetic known-truth certificates. The executable validates certificate descriptors, containment rules and digests, but does not implement an external theorem prover, interval proof checker or independently verify a physical model certificate.
- A physical implementation will need sound outer-enclosure certificates and feasible inner-region witnesses, using validated interval arithmetic, constrained zonotopes, SMT or other justified numerical methods.
- Even exact behavioural-set overlap is scoped to the declared input trajectory, fault hypotheses, uncertainty bounds, configuration and regime. It is not a vessel-wide or safety claim.

## Frozen synthetic corpus

The 15 cases cover certified separation, a constructive common inner witness, overlapping outer approximations without a witness, unverified outer certificates, unsupported regime, configuration mismatch, touching closed bounds, union-of-box separation, a partially overlapping union, inner-region escape from its outer enclosure, dimension mismatch, unverified inner-witness certificates, malformed uncertainty digest, non-monotonic time grid and duplicate output dimensions.

## Metrics and competitive targets

Report certified pair-separation precision and false-separation rate; common-witness precision and recall; NOT_PROVEN_SEPARABLE rate and abstention coverage; outer-approximation conservatism gap where reference sets are available; witness-verification cost; box count; trajectory dimension and horizon scaling; worst-regime results; configuration sensitivity; runtime and memory.

Research targets, all aspirational and not achieved:

- zero false-separation claims on the frozen synthetic known-truth suite;
- zero intersection-witness claims without verified inner-region membership in both hypotheses;
- zero promotions of outer-box overlap to physical non-identifiability;
- 100% receipt binding to exact trajectory, uncertainty, model, solver and source digests;
- on matched physical or validated simulation cases, improve separation recall at fixed false-separation rate over interval-signature baselines while disclosing the conservatism cost.

The initial P0 tests contract correctness only; it produces no physical diagnostic accuracy score.

## Comparison with current methods and industry

The generic sensor-observability kernel (#4005) qualifies pairwise separation of measured interval signatures and independent measurement principles. #5040 adds a separate dynamic set-valued layer: bounded output trajectories under parameter, disturbance and measurement uncertainty, with an explicit distinction between a common witness and overlap of conservative enclosures.

Physics-informed digital twins can generate useful expected-response sets, but inverse parameters may be non-unique. A low residual at one optimizer-selected parameter vector is not a uniqueness proof. A digital twin should supply calibrated bounds and certificate provenance; Symtropy must report when sets are too conservative to distinguish hypotheses.

This is intended as a vendor-neutral assurance complement, not an immediate replacement for OEM monitoring or diagnostic products. Its differentiator is reproducible, configuration-bound evidence with explicit proof status and ambiguity, not an unsupported claim to outperform deployed systems.

## Authority boundary

Read-only. The evaluator performs no controller writes, actuation, active excitation, maintenance authorization, mode transition, restriction clearance or restoration of an operating capability.
