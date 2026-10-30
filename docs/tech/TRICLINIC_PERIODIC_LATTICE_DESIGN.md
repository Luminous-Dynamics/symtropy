# Triclinic Periodic Lattice: Design and Qualification Contract

**Status:** draft implementation and tests; no Rust execution or performance results claimed.  
**Tracking:** periodic geometry extension beyond the orthogonal Phase A domain.

## Why this exists

The current PeriodicBox abstraction models an orthogonal cell. This is an appropriate first step, but it does not represent a general skewed/triclinic cell. Mature simulation and geometry systems already represent triclinic periodic cells and explicit lattice-image offsets. Symtropy should not claim equivalent coverage until the cell basis, image arithmetic and nearest-image operation are independently qualified.

The new PeriodicLattice type is separate from PeriodicBox. It uses the columns of a full-rank square basis matrix as lattice vectors and supports D=1,2,3 with a periodic-axis mask in fractional coordinates.

## Coordinate contract

Let B be the basis matrix, x0 the origin, and f = inverse(B) * (x - x0).

For each periodic lattice axis i, wrapping selects n_i = floor(f_i) and returns canonical fractional coordinate f_i - n_i in [0,1). Non-periodic fractional coordinates are unchanged. The returned Cartesian representative is x_c = x0 + B f_c.

The explicit image offset n must reconstruct the original point:
x = x_c + B n
within a scale-aware floating-point residual. The API returns a typed precision failure when the coordinate index is outside the supported exact-integer envelope or reconstruction is unreliable.

Periodic axes are lattice directions, not necessarily Cartesian x/y/z directions. A periodic y-lattice direction in a tilted cell can therefore also translate the Cartesian x coordinate.

## Closest-lattice-vector contract

For a displacement d, the minimum-image problem is:

minimize ||d - B n|| over integer n_i on periodic axes, with n_i = 0 on inactive axes.

Component-wise rounding of fractional coordinates is not generally a nearest-image algorithm for a skewed basis. The prototype therefore:
1. maps d into fractional coordinates and rounds each periodic coordinate to a deterministic center;
2. enumerates successive integer shells around that center;
3. evaluates each candidate in Cartesian coordinates;
4. derives a conservative lower bound for all unsearched images using sigma_lower = 1 / ||inverse(B)||_F, which is no larger than the smallest singular value;
5. returns a result only when the lower bound on every unsearched image is larger than the best candidate distance plus a floating-point margin.

The search is bounded by max_candidates. Candidate-budget exhaustion or an inability to certify the result returns a typed error. It must never silently return the best candidate seen so far as though it were proven nearest.

Tie handling is deterministic and designed to preserve reversal symmetry at exact half-cell ties. This needs dedicated differential tests across skewed bases, not just orthogonal equivalence tests.

## Scope and explicit limits

The draft supports square full-rank basis matrices for D=1,2,3. It does not yet support:
- arbitrary dimensions greater than three;
- basis reduction or a production lattice-reduction strategy;
- triclinic AABB image enumeration, collision candidates, GJK, CCD, contacts or rigid-body dynamics;
- changing/deforming lattices;
- general non-Euclidean metrics or curved manifolds.

The Frobenius lower bound is conservative. A highly skewed or ill-conditioned basis may require many candidates and correctly fail under a small budget. A future optimization may introduce basis reduction (for example, LLL-like preprocessing for low dimensions), but it must preserve the same certification contract and exact offset semantics.

## Independent correctness corpus

The current Rust tests include:
- diagonal-basis equivalence with PeriodicBox wrapping and minimum-image displacement;
- skewed-cell fractional wrapping and reconstruction;
- deterministic seeded comparisons against a wider brute-force integer-image oracle;
- one-active-axis behaviour where inactive fractional coordinates do not wrap;
- explicit refusal when the candidate budget cannot certify the nearest image;
- singular basis, non-finite coordinate, and invalid inactive-axis offsets;
- exact half-cell tie reversal symmetry.

Before any CI/test pass is claimed, add or retain:
- a separately implemented brute-force CVP oracle over a wider offset range;
- random and adversarial skew matrices, including nearly dependent basis vectors;
- random rotations of a fixed lattice and unimodular basis changes, with equivalent physical results;
- wrap/reconstruct invariance under integer lattice translations;
- offset composition and inverse-offset identities;
- 3D fixtures with independent reference results;
- non-periodic-axis and mixed-mask metamorphic tests;
- numeric-range and candidate-budget negative controls.

No failure seed may be discarded. Minimize and preserve each counterexample as a fixed regression fixture.

## Benchmark programme

Compare, at equal correctness:
1. component rounding (diagnostic baseline only; not accepted as a correct general solver);
2. certified shell enumeration;
3. an independent brute-force integer-image oracle on bounded fixtures;
4. a future reduced-basis/branch-and-bound implementation.

Sweep:
- dimensions 2 and 3;
- orthogonal, moderately skewed, highly skewed, and near-singular cells;
- 1, 2 and 3 active periodic axes;
- candidate budgets from small to exhaustive;
- displacements near Voronoi-cell facets and vertices, where multiple nearest images compete;
- small and large fractional-coordinate magnitudes inside the precision envelope.

Report p50/p95/p99 latency, candidates evaluated, shell depth, certification success rate, budget exhaustion rate, error against brute force, and reconstruction residual. Include exact basis matrices, masks, seeds, source SHA, Rust version and raw output. Never report speedup without the candidate count and correctness receipt.

## Fair industry comparison

- GROMACS documents triclinic periodic cells and the minimum-image convention, including cutoff restrictions needed to ensure that one image is sufficient for short-range interactions: https://manual.gromacs.org/current/reference-manual/algorithms/periodic-boundary-conditions.html
- LAMMPS documents periodic boundaries and triclinic tilt-factor offsets, including the fact that crossing one periodic face in a tilted cell can translate coordinates in another Cartesian direction: https://docs.lammps.org/boundary.html
- CGAL's periodic 3D triangulation explicitly represents points together with lattice image offsets on a flat torus: https://doc.cgal.org/latest/Periodic_3_triangulation_3/

These are design comparators, not like-for-like performance competitors for this minimum-image function. The fair Symtropy claim is a certified, bounded, fail-closed closest-image query with explicit image offsets. It is not that triclinic periodicity is novel.

## Required integration gates

- [ ] Exact-head formatting and compile checks pass.
- [ ] Independent 2D/3D brute-force differential tests pass.
- [ ] Basis-change and lattice-translation metamorphic tests pass.
- [ ] Candidate-budget refusal and all numeric negative controls pass.
- [ ] Benchmark reports include certification rate and candidate counts.
- [ ] Only then begin triclinic AABB candidate enumeration.
- [ ] Only after triclinic narrow-phase and CCD tests pass, consider PhysicsWorld integration.

## Claim ceiling

This is a draft D=1/2/3 triclinic coordinate and closest-image model. It does not establish triclinic collision dynamics, general periodic rigid-body physics, arbitrary topology simulation, or physical pocket dimensions.
