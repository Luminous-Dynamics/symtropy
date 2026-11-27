# Symtropy 3D OBB Independent Reference Seed v0.1

**Tracking issue:** [#7748](https://github.com/Luminous-Dynamics/symtropy/issues/7748).

**Status:** synthetic code/solution-verification test; not physical validation or general rigid-body qualification.

## Independent reference algorithm

The test file is `crates/core/symtropy-physics/tests/obb3_reference.rs`.

For an oriented box with center c, half-extents h and orthonormal world basis u0, u1, u2, its support projection radius on a normalized axis n is:

`r(n) = h0 |u0·n| + h1 |u1·n| + h2 |u2·n|`.

The 3D OBB separating-axis reference uses the 15 candidate directions:
- uA0, uA1, uA2;
- uB0, uB1, uB2;
- uAi × uBj for i,j in {0,1,2}.

For each non-degenerate normalized axis, the boxes are separated when:

`|(cB-cA)·n| > rA(n) + rB(n) + epsilon`.

Near-zero cross-product axes are counted and skipped. The reference reads rotation matrices only to obtain the transformed box basis and does not call GJK, EPA, support maps or transformed-shape collision helpers.

## Executable corpus

- 8,192 deterministic OBB pairs with fixed LCG seed `0x3344534154563031`.
- Independently sampled half-extents, translations and three-plane rotations.
- False-positive and false-negative counts reported separately; failing case identity and geometry are preserved in the assertion message.
- A translation-only GJK implementation is the negative control and must disagree with the independent SAT classification on more than 10% of the corpus.
- Every sampled rotation must pass orthogonality and determinant checks.
- A second 1,024-case metamorphic corpus applies one common rigid rotation and translation to both shapes; both SAT and transformed GJK classifications must remain unchanged.

The SAT tolerance is scale-relative at 1e-10. Cross-axis degeneracy is handled explicitly at norm 1e-12. Cases are synthetic and uniformly sampled from the declared ranges; this is not a statistically representative physical distribution.

## Reproduction

```bash
cargo test --locked -p symtropy-physics --test obb3_reference -- --nocapture
```

The focused GitHub Actions lane checks formatting, runs this exact test on Rust 1.96.0, and retains source SHA, seed, test log and case counts as a 90-day artifact.

## Qualification boundary and next steps

This test addresses transformed narrowphase overlap classification only. It does not validate EPA penetration depths, contact manifolds, impulse resolution, stack stability, 4D collision semantics, continuous collision detection, material behavior, or experimental physical fidelity.

The next numerical geometry steps are:
1. separated-box distance and witness-point error against an independent closest-feature reference;
2. EPA penetration-depth and contact-normal error against SAT minimum-overlap depth for non-degenerate overlaps;
3. a carefully justified 4D independent reference rather than reusing the 3D 15-axis formula;
4. independently measured physical cases with instrument uncertainty;
5. a separate analytic TOI/CCD qualification track.

Industry timing comparisons belong on matched scenes and hardware. Report p50/p95/p99 only with correctness, stability, degeneracy and resource metrics. This microbenchmark is not directly comparable to whole-scene Godot Rapier/Jolt step times.
