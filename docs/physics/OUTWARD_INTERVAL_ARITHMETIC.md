# Outward Interval Arithmetic: Primitive Kernel and Integration Boundary

**Status:** research primitive; not yet a production broadphase enclosure guarantee  
**Parent qualification:** [#7023 — finite, outward-rounded broadphase bounds](https://github.com/Luminous-Dynamics/symtropy/issues/7023)  
**Related evidence owner:** [#460 — physics qualification corpus](https://github.com/Luminous-Dynamics/symtropy/issues/460)

## 1. Why this exists

The current support-bound path evaluates a shape support function, transforms the returned point, and stores the resulting binary64 coordinates as AABB endpoints. Those operations are ordinary floating-point evaluations. A mathematically correct support-map identity does not itself prove that the rounded endpoint lies outward of the exact represented geometry.

The new `symtropy_physics::interval` module contributes a small scalar kernel:

- finite, ordered closed intervals;
- outward-rounded addition and subtraction;
- endpoint-product interval multiplication;
- representable-neighbor expansion after each scalar operation;
- explicit errors for invalid inputs and arithmetic overflow.

It is intentionally **not** wired into `support_aabb`, `Aabb::from_body`, union propagation, Morton quantization, or BVH pruning.

## 2. Arithmetic contract

For finite binary64 input endpoints, each successful operation encloses the exact real result over the input intervals, subject to the declared arithmetic profile:

- IEEE-754 binary64 operations;
- round-to-nearest, ties-to-even;
- gradual underflow (no flush-to-zero / denormals-are-zero);
- no unsafe fast-math transformations that alter the basic operation result.

The primitive evaluates the basic operation, then expands the rounded lower endpoint with the predecessor and the upper endpoint with the successor. Multiplication widens each of the four endpoint products before taking the minimum and maximum.

The implementation uses explicit `checked_*_outward` names rather than operator overloads so callers cannot mistake these operations for ordinary approximate arithmetic.

**Fail-open rule:** a non-finite input, reversed interval, overflowing intermediate, or outward expansion that reaches infinity returns an error. Callers must treat the result as unknown/unprunable or propagate a typed world-step error. They must not replace the error with a zero-width, empty, or ordinary finite bound.

This stage does not offer division, square root, trigonometric, normalization, or vector/matrix APIs. Such operations require separately specified enclosures before they may participate in a collider bound.

## 3. What this does not prove

The primitive alone does not establish that:

1. `Shape::support` returns a support point with a certified error bound;
2. rotor reversal/rotation or `Transform::transform_point` is enclosed;
3. a transformed shape's true support extremum is represented by the current returned point;
4. compound-shape traversal and child transforms preserve enclosure;
5. AABB union, quantization, hierarchy nodes, or traversal preserve an enclosure;
6. a replayable separation witness proves anything about geometry outside the represented boxes.

Consequently, this module must not be cited as a fix for the whole #7023 theorem or as evidence that the existing broadphase has zero false negatives.

## 4. Integration sequence

### Stage A — arithmetic profile

Freeze and identify the supported floating-point profile per target/build. Add tests for signed zero, subnormals, cancellation, exact representable results, non-finite inputs, overflow, and compiler configurations. Any profile that cannot uphold the arithmetic preconditions must return unprunable/unsupported.

### Stage B0 — affine AABB enclosure over represented matrix inputs

The interval module now exposes `transform_aabb_affine_outward`. It accepts a finite, ordered local AABB, a finite binary64 matrix, and a finite binary64 translation. It evaluates each output coordinate as an interval dot product plus translation, widening every multiplication and addition.

The result encloses the exact affine image of every point in the represented local box under the **represented matrix and translation values**, subject to the arithmetic profile in Section 2. The test suite checks identity, negative coefficients, degenerate axes, rotation/shear-like matrices, all transformed corners, invalid inputs, and overflow.

This is a strictly narrower claim than enclosing a `Transform<D>` generated from a `Rotor<D>`:

- It does not prove the represented matrix is itself an enclosure of an ideal rotation or of the rotor's construction error.
- It does not prove `Rotor::reverse`, `Rotor::rotate_vector`, or any trigonometric/normalization path.
- It does not supply local bounds for arbitrary `Shape<D>` implementations.
- It is not connected to `support_aabb`, `Aabb::from_body`, or production pruning.

The next integration step is an optional shape-local bounds contract and an explicit bridge from a represented `Transform` matrix/translation to this affine enclosure, with unsupported shapes remaining unprunable.

### Stage B — conservative transformed primitives

Add typed bound providers that return either a certified finite enclosure or an explicit unprunable/unsupported outcome. Start with primitives whose geometry admits direct interval formulas (sphere, capsule, and hyperbox). Do not route an arbitrary `Shape<D>` through a nominally conservative default.

For each primitive, distinguish:
- local geometry values (the represented input);
- transform parameters (the represented input);
- interval evaluation of the transform;
- shape-specific extremum calculation;
- final outward enclosure.

Compound shapes are supported only when every child and child transform has a supported enclosure. If any child is unsupported or invalid, the parent remains unprunable.

### Stage C — hierarchy propagation

Carry the enclosure outcome through:
- body AABBs;
- static-cache entries;
- LBVH leaf AABBs;
- internal-node unions;
- broadphase query/prune tests;
- any quantized or compressed representation.

A parent containing an invalid/unprunable child must not prune that child's region. Quantization must round outward too; computing a rounded centroid or a rounded min/max once is not sufficient.

### Stage D — proof-carrying decisions

A future prune receipt should bind:
- collider and transform snapshot identity;
- arithmetic-profile identifier;
- bound-provider / algorithm version;
- certified endpoint bit patterns;
- tested axis and strict inequality;
- tree-node or leaf identity;
- the dependency revisions that make the snapshot current.

A checker should verify the strict AABB inequality independently. A separate enclosure checker should establish that the bound encloses the represented geometry. These remain two different theorems.

## 5. Falsification matrix

Add fixtures to the existing #460 corpus, not a parallel acceptance corpus:

- exact face, edge, and vertex tangency;
- adjacent representable values on either side of contact;
- very large translations with sub-unit and sub-ULP extents;
- very thin and highly rotated boxes;
- minimum subnormal and near-maximum finite values;
- intermediate overflow in transform and union calculations;
- NaN/infinite transforms and support outputs;
- degenerate Morton domains and quantized bounds;
- insertion-order, tree-build-order, and storage-order permutations;
- compound shapes with a single unsupported or invalid child.

Use an independent oracle. Prefer exact rational/analytic fixtures where possible and a higher-precision interval or exact-arithmetic reference for transformed cases. Do not reuse production `support_aabb`, `Aabb::from_body`, or the same interval implementation as its own oracle.

Required negative controls:
- invalid/unsupported child cannot produce a prune;
- deliberately inward-shrunk bound is rejected by the enclosure checker;
- exact tangency cannot be excluded by a strict-separation witness;
- changing a transform or collider snapshot invalidates its receipt.

## 6. Performance accounting

Correctness promotion is independent of performance promotion. On pinned hardware and exact source SHA, report:

- build and rebuild time;
- query time and total broadphase time;
- node visits and leaf tests;
- candidate count, narrowphase-tested count, active contact count;
- unprunable/unknown count;
- candidate inflation versus the independent oracle;
- interval-operation count and fallback count;
- memory footprint and allocation count;
- end-to-end quality-qualified simulated ticks per wall-clock second.

Do not compare this scalar primitive against an engine-level rigid-body result. A speed claim requires a matched workload, matching solver and sleeping/CCD semantics, hardware disclosure, repeat runs, and the same correctness corpus.

## 7. Promotion gates

The primitive may be reused by a geometry implementation only when its arithmetic preconditions are declared and tested. A supported primitive may be marked enclosure-qualified only when its independent oracle passes every frozen adversarial fixture. A broadphase path may be promoted only after enclosure is preserved through every hierarchy and prune operation.

Targets (not achievements):

- zero false-negative pairs in the frozen corpus;
- zero exclusion decisions from invalid or unprunable bounds;
- zero false-safe receipts under mutation and snapshot negative controls;
- deterministic outputs under each declared determinism profile;
- measured, repeatable throughput improvement in at least three workload families before algorithmic promotion.

## 8. Research directions after the kernel

The most promising field-level shift is an **adaptive-cost broadphase** rather than one universally selected tree:

1. Use cheap scalar/interval bounds for well-conditioned common cases.
2. Escalate to tighter or higher-precision enclosures only for numerically ambiguous axes.
3. Keep unsupported shapes in a conservative unprunable lane.
4. Choose among static BVH, flat cache, LBVH, sweep-and-prune, and brute force using declared workload features and measured crossover data.
5. Let mutation revisions determine whether old bounds and contacts can be reused.
6. Preserve an auditable receipt for every prune, fallback, and reuse decision.

The optimization objective is not merely fewer candidate pairs. It is lower **quality-qualified total work** without weakening geometric completeness, temporal semantics, or independent replay.
