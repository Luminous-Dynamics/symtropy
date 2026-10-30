# N-dimensional simplicial-boundary mass-property integration

**Status:** experimental geometric mass-property kernel.
**Date:** 2026-10-30
**Branch:** `research/simplicial-mass-properties-2026-10-30`

## Why this direction

Analytic sphere/ellipsoid constructors and point-mass clouds are useful, but real rigid bodies usually arrive as CAD or triangle meshes. A physically valid contact engine needs the center of mass and rotational second moment from the same geometry that defines collision.

Brian Mirtich's classic polyhedral mass-properties method evaluates volume integrals in a boundary traversal, reducing them to lower-dimensional face integrals while reusing common terms to limit numerical work and error. The current Symtropy prototype takes a different, dimension-generic route: integrate signed D-simplices formed by joining each oriented (D-1)-simplex boundary facet to a nearby reference vertex. See [Mirtich, Fast and Accurate Computation of Polyhedral Mass Properties](https://people.eecs.berkeley.edu/~jfc/mirtich/massProps.html).

Modern mesh-processing documentation also distinguishes combinatorial closure/orientation from geometric validity: a closed orientable manifold can still contain degenerate or self-intersecting elements. The prototype checks the topology and degeneracy conditions it can establish, but self-intersection detection remains a separate gate. See [CGAL Polygon Mesh Processing](https://cgal.geometryfactory.com/CGAL/doc/main/Polygon_mesh_processing/index.html) and [CGAL Mesh Repair / Orientation](https://cgal.geometryfactory.com/CGAL/doc/main/PMP_Mesh_repair/index.html).

## Input contract

For spatial dimension D>=2:

- `vertices` is a list of finite (D)-vectors.
- Each boundary facet contains exactly D indices, representing an oriented (D-1)-simplex.
- Every codimension-two ridge must occur in exactly two facets.
- The two facets must induce opposite orientations on their shared ridge.
- The boundary must have one connected component.
- Facets must be geometrically non-degenerate.
- The surface must be self-intersection-free and bound one physical region.

Dimension examples:

- D=2: oriented line segments bound a polygon.
- D=3: oriented triangles bound a polyhedron.
- D=4: oriented tetrahedra bound a 4D body.

The first version rejects disconnected boundaries. This avoids silently guessing whether an extra connected component is a second solid, a nested cavity, or an incorrectly oriented shell. A later compound-boundary API should make component nesting and signed material semantics explicit.

## Integration formula

Let the first input vertex be (p). Work with local coordinates (r_i=x_i-p). For an oriented boundary facet with local vertices (r_1,ldots,r_D), its signed simplex volume with apex at p is:

\[
V_f=\frac{\det[r_1\;r_2\;\cdots\;r_D]}{D!}.
\]

For the simplex with vertices ((0,r_1,\ldots,r_D)), define (s=\sum_i r_i) and (Q=\sum_i r_i r_i^T). Its integrated first and second moments are:

\[
\int_f r\,dV=\frac{V_f}{D+1}s,
\]

\[
\int_f rr^T\,dV=
\frac{V_f}{(D+1)(D+2)}(ss^T+Q).
\]

Summing signed contributions over the closed oriented boundary gives volume (V), first moment (m_1), and raw second moment (M_2). The global boundary orientation may be inward or outward; all integrated moments are flipped together when the signed volume is negative.

For uniform density ρ:

\[
m=\rho |V|,\qquad
c=p+\frac{\operatorname{sign}(V)m_1}{|V|},
\]

\[
S=\rho\operatorname{sign}(V)M_2-mcc_{\mathrm{rel}}^T,
\]

where (c_{\mathrm{rel}}=\operatorname{sign}(V)m_1/|V|). S is the centered second-moment matrix consumed by `BivectorInertia`.

This is the same physical S used by the inertia, contact and Delassus paths; there is no separate mesh-only inertia convention.

## Implemented API

`BivectorInertia::from_oriented_closed_simplicial_boundary(vertices, facets, density, relative_tolerance)` returns `MassProperties<D>` containing:

- enclosed D-dimensional volume;
- total mass;
- center of mass in the input vertex frame;
- centered mass-aware `BivectorInertia<D>`.

It validates:

- finite vertices and positive finite density;
- valid, non-repeated facet indices;
- exactly two incidences for each codimension-two ridge;
- opposite induced orientations on every shared ridge;
- one connected boundary component;
- non-degenerate facets using their Gram determinant;
- finite integrals and non-zero enclosed volume.

It deliberately does **not** claim to repair or detect geometric self-intersections, non-simple boundaries, or nested shells. Those require a separate geometry-validation pipeline with robust predicates.

## Regression coverage

- 2D square: exact area 4, center (0,0), centered second moment diag(4/3,4/3).
- 3D cube: exact volume 8, center (0,0,0), centered second moment diag(8/3,8/3,8/3).
- 4D standard simplex: exact volume (1/24), centroid coordinates (1/5).
- Global reversal of the complete square boundary orientation gives identical positive mass properties.
- A square with a missing edge is rejected as open.
- A reversed individual edge is rejected for inconsistent boundary orientation.
- Two disconnected square boundaries are rejected rather than implicitly treating them as an assembly or cavity.
- A collinear tetrahedral face is rejected as degenerate.
- A cube translated to approximately (10^9) world-coordinate magnitudes preserves its local second moment and returns the translated center of mass.

These are source-level fixtures until exact-head CI completes formatting, Clippy, compilation and tests.

## Industry comparison and research boundary

Mirtich's 1996 method remains a classic reference for 3D polyhedral moments; VTK's `vtkMassProperties` is a practical triangle-mesh volume estimator based on the discrete divergence theorem and assumes closed surfaces. CGAL provides a much broader pipeline for orientation, repair, connected components and self-intersection checks. The Symtropy prototype is not a replacement for those mesh-repair capabilities. Its proposed differentiator is one mass-property integration kernel expressed over generic D-dimensional simplicial boundaries and directly returning the inertia operator needed by the N-dimensional physics core.

Do not claim this is more accurate than Mirtich, VTK or CGAL. The next comparisons should measure relative volume/COM/second-moment error against analytic primitives and independent high-precision polyhedral references across sliver facets, large translations, coordinate scales and triangulation permutations.

## Next engineering gates

1. Add geometric self-intersection detection or require a trusted external validation receipt.
2. Add component-aware boundaries with explicit positive-solid and negative-cavity semantics.
3. Add a transactional `RigidBody::try_set_mass_properties` that rebases collider children and moves the body origin to the computed COM without changing world geometry.
4. Compare 3D outputs against Mirtich's implementation and an independent high-precision reference.
5. Add area/volume/COM/moment convergence studies under mesh refinement.
6. Add provenance: mesh digest, density, unit system, COM frame, boundary orientation status, and integration tolerance.
