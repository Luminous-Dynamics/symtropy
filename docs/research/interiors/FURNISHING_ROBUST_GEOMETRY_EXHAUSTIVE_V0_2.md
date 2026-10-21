# Robust geometry v0.2: independent endpoint and metamorphic qualification

**Status:** test and research proposal stacked on robust interval geometry v0.1. This adds stronger checks for the interval classifier; it does not certify real-world layouts or accessibility.

## What changed

The v0.1 code has a simple but safety-relevant contract: classifying two bounded rectangles must not silently discard any geometries represented by the input ranges. Hand-written examples are necessary, but not enough to show that all range relationships are handled correctly.

This slice adds three deterministic test families in `robust_geometry.rs`:

1. **Exhaustive endpoint reference:** a rectangle has four independently bounded quantities (x, y, width, depth). For one interval rectangle, those quantities have 2^4 endpoint combinations; for a pair, 2^8 = 256 combinations. Each combination is converted into the existing fixed-geometry `Rect` and evaluated using its exact overlap predicate. Across 128 deterministically generated interval pairs, the implementation is compared against 32,768 exact endpoint combinations.
2. **Symmetry and common translation:** `classify(a,b)` must equal `classify(b,a)`. Applying the same finite translation to both rectangles must not change the overlap classification.
3. **Uncertainty-widening monotonicity:** enlarging a represented uncertainty set must not strengthen a result to an incompatible certainty. A definitely-clear set cannot become definitely-overlapping when widened; a definitely-overlapping set cannot become definitely-clear; an indeterminate set stays indeterminate because the original clear and overlapping cases remain represented.

No random crate, hidden seed, model-based expected output, or timing shortcut is used. The generated cases are frozen by their deterministic arithmetic generator and reported case indices in assertion failures.

## Why this is a useful distinction

[SceneEval (WACV 2026)](https://openaccess.thecvf.com/content/WACV2026/html/Tam_SceneEval_Evaluating_Semantic_Coherence_in_Text-Conditioned_3D_Indoor_Scene_Synthesis_WACV_2026_paper.html) shows why layout evaluation should retain separate scores for explicit requirements and implicit plausibility (including collisions and navigability). Our geometry tests address a narrower question: whether the reported classification is justified for every geometry represented by bounded input intervals.

[buildingSMART IDS](https://www.buildingsmart.org/Standards/Bsi-Standards/Information-Delivery-Specification-Ids/) standardizes computer-interpretable IFC information requirements but explicitly excludes geometry from IDS v1.0. Product identity, specification, source revision and dimension-provenance checks should use standard information vocabularies where appropriate; spatial uncertainty remains a separate contract.

[ISO 21542:2021](https://www.iso.org/standard/71860.html) covers built-environment access, circulation, egress and evacuation, with requirements/recommendations scoped by the actual design context. This rectangle oracle remains a mathematical primitive: it does not implement that standard, certify any jurisdiction, or represent the full diversity of human movement and tasks.

## Important limitation of endpoint enumeration

The reference enumeration applies to this **axis-aligned rectangle model with independent closed intervals and the current monotone rectangle overlap predicate**. It is not an assertion that testing corners alone proves correctness for arbitrary polygons, correlated probability distributions, non-linear transformations, articulated doors/drawers or swept human movement.

The geometry semantics must be revisited if the model changes. A future extension with correlation should define a valid joint uncertainty set and independently verify its containment semantics, rather than pretending the Cartesian interval is automatically tight.

## Proposed acceptance and performance protocol

### Correctness hard gates

- Zero mismatches against endpoint reference for all generated interval pairs.
- Zero symmetry or common-translation violations for the fixed deterministic corpus.
- Zero widening-monotonicity violations.
- Zero false-safe outcomes on the declared interval corpus.
- Existing fixed-geometry tests and independent brute-force BVH comparison remain unchanged and pass.
- Each failing generated case must print the deterministic case index and intervals.

### Next validation expansion

1. Sweep representable offsets around the existing 1e-9 m numerical epsilon, including just below, at and just above the boundary. The epsilon is **not** a human clearance or installation tolerance.
2. Add boundary fixtures for extremely large but finite inputs, derived-endpoint overflow and values near floating-point spacing limits.
3. Add shrinkable property-based generation only if the dependency and deterministic seed policy are pinned; retain a dependency-free deterministic smoke corpus in ordinary CI.
4. Extend the reference oracle to check candidate-pair generation separately from interval classification.
5. Benchmark the classifier itself across exact, narrow and wide ranges. Record p50/p95/p99, operations per second, allocations/peak memory and classification counts on a pinned runner. The existing fixed-oracle benchmark is a regression guard, not an interval classifier performance claim.
6. Integrate robust broad-phase candidate discovery with the BVH only after the independent classifier contract is stable.

### Proposed targets, not results

| Measure | Target |
|---|---:|
| Exhaustive reference mismatch | 0 |
| Symmetry/translation violations | 0 |
| Monotonicity violations | 0 |
| False-safe in fixed known-truth interval suite | 0 |
| Timing | Establish independent matched baseline before setting SLA |
| Real-room/task success | Requires annotated measured cases and human/professional review |

## What would count as better than the industry state of practice?

Do not claim superiority from a passing unit test or a faster kernel alone. A defensible comparison needs the same fixed corpus and machine; the same coordinate/unit and numerical policy; an independent reference implementation; results broken out by interval width, scene density and task type; source provenance and failure receipts; then a separate user/field evaluation for task success, corrections and installation surprises. An interval classifier can qualify uncertainty handling, but not by itself prove better furniture plans or better resident outcomes.
