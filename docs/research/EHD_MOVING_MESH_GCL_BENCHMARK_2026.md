# EHD Moving-Mesh Geometric Conservation Benchmark (2026)

**Benchmark:** EHD-MOVING-GCL-001  
**Status:** analytic reference oracle and authored tests; Rust execution pending  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_moving_mesh_gcl_benchmark_v0_1.json](../../data/research/ehd_moving_mesh_gcl_benchmark_v0_1.json)

## 1. Purpose

A moving control volume changes measure even when the physical state is uniform. An ALE or moving-mesh discretisation must account for that geometric change without creating an artificial source or sink.

The geometric conservation law (GCL) is:

\[
V(t_1)-V(t_0)
=\int_{t_0}^{t_1}\int_{\partial V(t)}
\mathbf w\cdot\mathbf n\,dA\,dt,
\]

where \(\mathbf w\) is the grid velocity and \(\mathbf n\) is the outward normal.

A code that conserves mapped impulse on static meshes can still produce artificial momentum or mass changes when geometry moves. GCL is therefore a separate qualification receipt; it must not be folded into the impulse-remapping residual.

## 2. Implemented reference scope

The new pure-Rust reference module is:

`tools/ehd-reference-audit/src/gcl.rs`

It supports a single axis-aligned spatial box in one, two or three spatial dimensions. Lower and upper bounds move linearly between declared start and end states over a positive time interval. Bounds must remain finite and non-collapsed at both endpoints; linear interpolation then preserves positive width between them.

For spatial axis \(i\), let the lower and upper face displacements be \(\Delta l_i\) and \(\Delta u_i\), and let \(w_j(\tau)\) denote the linearly varying width on axis \(j\), with normalized time \(\tau\in[0,1]\). The time-integrated outward flux through the lower and upper faces is:

\[
\Phi_i^-=-\Delta l_i\int_0^1
\prod_{j\ne i}w_j(\tau)\,d\tau,
\qquad
\Phi_i^+=\Delta u_i\int_0^1
\prod_{j\ne i}w_j(\tau)\,d\tau.
\]

Because every width is linear in normalized time, the face-area product is a polynomial of degree at most \(N-1\). The reference implementation expands and integrates that polynomial exactly (up to floating-point arithmetic), rather than using time stepping or numerical quadrature.

Summing the face contributions gives:

\[
\sum_i(\Phi_i^-+\Phi_i^+)
=\int_0^1\sum_i
(\Delta u_i-\Delta l_i)
\prod_{j\ne i}w_j(\tau)\,d\tau
=V(1)-V(0),
\]

by the product rule. This provides a compact analytic manufactured-solution oracle.

## 3. ALE free-stream check

For a uniform scalar \(q_0\) with no physical flux or source, the isolated ALE geometric update is:

\[
q_1V_1=q_0(V_0+\Phi_g),
\qquad
\Phi_g=\int_{t_0}^{t_1}\int_{\partial V}
\mathbf w\cdot\mathbf n\,dA\,dt.
\]

A valid GCL receipt has \(\Phi_g=V_1-V_0\), so \(q_1=q_0\) up to roundoff. The module exposes this manufactured update as a check on the geometric identity. It is not a full finite-volume flow solver or a substitute for free-stream testing in an actual ALE discretisation.

## 4. Authored test matrix

The module currently defines eleven unit tests:

1. Static mesh: zero grid flux.
2. 1D translation and expansion.
3. 2D translation and anisotropic expansion.
4. 3D anisotropic motion.
5. Reversed motion: opposite integrated grid flux.
6. Independent temporal subdivision: additive swept measure.
7. Uniform-scalar ALE free-stream preservation.
8. Reject zero/infinite duration.
9. Reject collapsed/non-finite cell bounds.
10. Reject invalid tolerances.
11. CLI reference self-check.

These tests are authored but have not yet been run by Rust. The CLI check is also source code only until compilation/execution is available.

## 5. Evidence and tolerances

The API requires a case-specific absolute-plus-relative volume tolerance. There is no universal hard-coded acceptance threshold. Every report records initial/final measure, volume change, lower/upper face contributions, net integrated grid flux, residual and allowed residual.

GCL residuals must remain separate from:
- static spatial intersection coverage error;
- conservative impulse-transfer residual;
- momentum closure residual;
- charge continuity residual;
- energy closure residual;
- physical instrument uncertainty.

## 6. Limitations

This first reference oracle does not support:
- rotated or curved cells;
- general convex or non-convex polyhedra;
- face rotation, shear, sliding interfaces or nonconforming topology;
- remeshing, cell creation/deletion or connectivity changes;
- arbitrary nonlinear trajectories unless subdivided into a declared piecewise-linear path;
- fluid equations, numerical fluxes, plasma equations or a coupled solver;
- a production ALE solver's full discrete free-stream preservation.

The next geometry stages should add exact rigid translation/rotation and affine deformation fixtures for oriented polyhedra, then compare swept-face and space-time intersection measures against an independent geometry implementation. Only after those checks should this oracle be used to qualify a real moving-mesh solver adapter.

## 7. Proposed qualification gates (targets, not results)

- Zero accepted GCL residual above the case-specific, predeclared volume tolerance.
- Uniform-state preservation within a separately declared state tolerance.
- Integrated grid flux invariant under independent temporal subdivision of the same piecewise-linear trajectory.
- Forward/reverse trajectory flux antisymmetry.
- Per-face fluxes sum to the independent endpoint volume difference.
- Same frozen fixtures agree with an independent analytic or geometry-library oracle.
- Runtime comparisons are performed only after identical geometric semantics and accuracy are established.

## 8. Claim ceiling

This benchmark and the Rust module establish only an analytic reference path for linearly moving axis-aligned cells. They do not establish arbitrary moving-mesh GCL compliance, general geometry, solver free-stream preservation, a coupled EHD calculation, physical thrust validation, Rust test success or SOTA superiority.
