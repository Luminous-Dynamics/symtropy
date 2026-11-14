# PHYS-MMS-001 — manufactured plane-stress mesh verification

**Status:** executable fixture/audit implementation under review. No CalculiX mesh-convergence result is asserted until the exact-head workflow passes.

## Purpose

The one-dimensional axial bar is a useful process and analytical-smoke test, but linear T3D2 elements exactly represent its linear displacement field. A flat mesh error there cannot demonstrate convergence order. PHYS-MMS-001 introduces a smooth two-dimensional exact solution for CPS3 plane-stress triangles so that displacement, strain, and stress errors can be measured across genuinely approximate meshes.

This is a code-verification experiment, not experimental validation and not proof that a structure is safe.

## Exact manufactured problem

The domain is the unit square, with zero displacement on all four edges:

\[
u(x,y)=A\sin(\pi x)\sin(\pi y), \qquad
v(x,y)=B\sin(2\pi x)\sin(\pi y),
\]
where \(A=10^{-6}\,\mathrm{m}\), \(B=0.7\times10^{-6}\,\mathrm{m}\), \(E=210\,\mathrm{GPa}\), \(\nu=0.3\), and thickness \(t=1\,\mathrm{m}\).

For isotropic linear plane stress,
\[
D=\frac{E}{1-\nu^2}, \qquad G=\frac{E}{2(1+\nu)},
\]
\[
\sigma_{xx}=D(\epsilon_{xx}+\nu\epsilon_{yy}),\quad
\sigma_{yy}=D(\nu\epsilon_{xx}+\epsilon_{yy}),\quad
\tau_{xy}=G\gamma_{xy},
\]
using the exact derivatives of \(u,v\). The body force is defined analytically as \(\mathbf b=-\nabla\cdot\boldsymbol\sigma\), so the exact field satisfies \(\nabla\cdot\boldsymbol\sigma+\mathbf b=0\). The Rust module contains the explicit formula and a finite-difference derivative cross-check.

The mesh is a deterministic uniform \(n\times n\) subdivision of the square, each square split into two consistently oriented three-node CPS3 triangles. Consistent equivalent nodal body-force loads are generated with 12-point Gauss-Legendre quadrature in each axis after a Duffy map to each triangle; the deck binds all nodes, triangles, material, thickness, and zero edge displacement conditions.

## Levels, errors and expected order

The runner is to execute \(n\in\{4,8,16,32,64\}\), then independently bind each output to the exact input deck, model manifest, build manifest, solver binary, and process log. The audit computes errors at common numerical quadrature points:
- displacement \(L^2\) norm (units m²; expected asymptotic order approximately 2 for linear triangles);
- displacement \(H^1\) seminorm (units m; expected order approximately 1);
- stress tensor \(L^2\) norm (units Pa·m; expected order approximately 1);
- displacement \(L^1\) integral (units m³), quadrature-sampled \(L^\infty\) error (m), and maximum nodal error (m) as secondary diagnostics.

For each pair of levels \(n,2n\), observed order is
\[
p_{\mathrm{obs}}=\frac{\ln(e_n/e_{2n})}{\ln 2}.
\]
The final two refinement intervals (16→32 and 32→64) are the primary asymptotic checks. Initial qualification gates should require monotone reduction of the three primary errors and final observed orders in broad bands:
- displacement \(L^2\): \(1.4\le p\le 2.6\);
- displacement \(H^1\): \(0.4\le p\le 1.6\);
- stress tensor \(L^2\): \(0.4\le p\le 1.6\).

These are protocol gates for this particular smooth fixture, not universal FEM guarantees. If quadrature, locking, output precision or the element formulation prevents the gates from passing, preserve the failed reports and revise the protocol only as a documented preregistered change. Do not clip/project the solution or lower thresholds after inspecting results.

## Provenance and claim ceiling

The physical_mms generate command creates the deterministic input deck and a model manifest. The physical_mms audit command checks the exact generated deck, hashes the raw .dat bytes, run manifest, model manifest, binary, log, and build manifest, then emits the field-error report. The audit report labels a per-mesh result MEASURED_UNQUALIFIED; a single passing mesh is never described as convergence.

The workflow's convergence summary must include all five per-mesh report digests, raw solver process outcomes, node/element counts, all error values, observed orders and final status. Exact-head reproducibility and byte-bound provenance do not authenticate the correctness of an untrusted solver by themselves; the evidence supports only this fixture's code-verification claim.

No benchmark result, convergence order, general CalculiX capability, experimental validation, engineering-use authorization, or SOTA claim is asserted by this design document.

## References

- NAFEMS, *Code Verification Exemplars in Computational Solid Mechanics* (2026-03-03): observed-order studies against known exact solutions across multiple mesh resolutions.
- SUNDIALS / SciML work–precision benchmarking provides the complementary numerical-software lesson: accuracy, work, failures, and reproducible environment must be reported together rather than collapsing into one wall-clock number.
