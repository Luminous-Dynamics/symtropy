# Robust furnishing geometry v0.1: bounded uncertainty, fail-closed decisions

**Status:** implementation prototype and test contract. The new interval classifier is a geometry primitive, not an accessibility checker, room-measurement system, furniture solver, building-code verifier or safety certification.

## Why this is the next step

The fixed-geometry oracle can be exactly correct about the rectangles it receives while still receiving a misleading point estimate: a room width measured with a phone, a product dimension copied from a seller, a nominal catalogue size, and a precise survey may look like the same number but carry very different uncertainty. A brittle system silently treats those numbers as equally exact.

The paradigm shift is to represent geometric inputs as bounded sets and make a three-way decision:

- **Definitely clear:** every geometry admitted by the stated bounds is clear under the current rectangle predicate.
- **Definitely overlapping:** every geometry admitted by the bounds overlaps under that predicate.
- **Indeterminate:** the bounds admit both clear and overlapping cases. A hard constraint must not be promoted to pass.

The implementation in `tools/furnishing-layout-audit/src/robust_geometry.rs` uses closed intervals and worst-case endpoints; it does not sample a few likely dimensions and mistake the absence of a sampled collision for proof. The ranges form a Cartesian uncertainty set. If future evidence establishes correlations between dimensions, those correlations may narrow the set, but they must be explicit and independently tested.

## Research and standards boundary

[SceneEval (WACV 2026)](https://openaccess.thecvf.com/content/WACV2026/html/Tam_SceneEval_Evaluating_Semantic_Coherence_in_Text-Conditioned_3D_Indoor_Scene_Synthesis_WACV_2026_paper.html) evaluates text-conditioned indoor scenes across explicit requirements and implicit properties such as collisions and navigability. Our claim should not be that adding a collision test is novel; the proposed difference to test is a conservative uncertainty contract whose unknowns remain visible to the decision layer.

[ISO 21542:2021](https://www.iso.org/standard/71860.html) is a relevant source for accessibility and usability of the built environment. This primitive does not encode or certify its clauses: required widths, turning, approach, reach, transfer, egress, installation tolerances and individual task models must remain separately sourced, versioned, jurisdiction-scoped and reviewed.

[buildingSMART IDS 1.0](https://www.buildingsmart.org/standards/bsi-standards/information-delivery-specification-ids/) can specify computer-interpretable IFC information requirements, but buildingSMART explicitly distinguishes IDS property/information requirements from geometry. Use the applicable IFC/IDS vocabulary for product/building properties and keep spatial geometry and measurement evidence in a distinct contract.

## Numerical contract

- Units are metres.
- Bounds are inclusive, finite, and ordered. Width/depth lower bounds must be greater than zero.
- Invalid or non-finite bounds are rejected at construction; the classifier does not attempt recovery by clamping.
- The existing `1e-9 m` overlap epsilon is only a numerical predicate tolerance. It is **not** a human clearance, wheelchair-space requirement, installation allowance, or construction tolerance.
- A `DefinitelyClear` result certifies only non-overlap of the represented axis-aligned rectangles under that predicate. It does not certify usable circulation, a functional envelope, safety, code compliance, or accessibility.
- `Indeterminate` is not pass. The calling policy should block promotion of any mandatory constraint until evidence narrows the interval or a qualified rule owner resolves it.
- Product/room dimensions, evidence provenance, freshness and confidence are not encoded in this geometry-only type yet. The next data-contract step must carry source ID, measurement method, bounds/tolerance, capture time, coordinate transform and units alongside the ranges.

## Frozen correctness cases added in the Rust module

The first unit tests cover:

1. Exact edge contact under the declared overlap epsilon.
2. Fixed rectangles that genuinely overlap.
3. Position uncertainty that admits collision and no collision.
4. Product-width uncertainty that admits both outcomes.
5. Separation on one axis proving clear even where the other axis remains uncertain.
6. Widening an interval degrading a definite-overlap result to indeterminate, never to clear.
7. Rejection of non-finite, inverted and non-positive-extent inputs.

These are seed cases, not exhaustive qualification. In particular, property-based and brute-force reference tests remain required before the classifier becomes a gate in the full furnishing oracle.

## Benchmark protocol and acceptance targets

| Measure | Proposed gate | Why |
|---|---:|---|
| False-safe interval results on exact known-truth cases | 0 | A possible collision must not be classified definitely clear |
| Agreement against independent endpoint/reference enumeration on frozen cases | 100% | Validate the three-way semantics rather than only a few examples |
| Symmetry, `classify(a,b) == classify(b,a)` | 100% | Input order must not change the decision |
| Monotonicity under wider uncertainty bounds | 100% of generated cases | Widening uncertainty may reduce certainty, never justify a stronger pass |
| Repeated result determinism | Exact | No randomized sampling or order-dependent result |
| Runtime | Measure first; target after baseline | Do not invent an SLA before same-host microbenchmarks and allocation/throughput profiling |

For generated validation, enumerate all endpoint combinations on small discrete rectangles and compare the interval result with an independent exact/epsilon-aware reference. For larger randomized cases, freeze seed, generator version and corpus digest; report any mismatch as a hard failure. Include nearly touching gaps on both sides of the numerical epsilon, very large finite bounds, negative coordinates, invalid bounds, narrow product tolerance, uncertain room dimensions and rotations handled by the separate transform layer.

After classifier correctness, integrate bounded geometry with the existing BVH so uncertainty checking does not devolve into all-pairs comparison for sparse layouts. Re-run sparse, tight-functional-clearance and dense-overlap tiers on a named exact head. Report candidate pairs, true pairs, p50/p95/p99, peak memory and all findings. Dense outputs remain output-sensitive; do not promise a sparse-workload SLA when the number of true collisions is quadratic.

## Next work sequence

1. Extend the type with overflow-safe derived-endpoint validation and a stable serialized evidence envelope.
2. Add a test-only exhaustive reference enumerator plus metamorphic/property-based cases for symmetry, monotonicity, translation invariance and epsilon boundaries.
3. Connect measured/certified/seller-estimated/unknown provenance to the intervals. Missing provenance for a mandatory hard constraint remains indeterminate.
4. Integrate uncertainty-aware broad-phase queries with the current BVH without weakening the existing fixed-rectangle predicate.
5. Create a frozen corpus from rights-checked furnished-room distributions and independently labelled room/task cases. A scene dataset alone is not accessibility ground truth.
6. Evaluate product-dimension disagreements and user corrections in a consent-based field study before claiming fewer installation surprises or improved resident outcomes.

## Honest claim boundary

This slice adds a conservative, deterministic, interval-based rectangle overlap classifier and unit tests. It does not yet load ranges from room/product evidence, audit a complete layout with uncertainty, support general rotated polygons, model human movement, provide field validation, or establish superiority to SOTA. The research hypothesis is that carrying bounded uncertainty into the decision contract can reduce false-safe layout claims while leaving the existing exact-geometry and evidence gates intact.
