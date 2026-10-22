# PHYS-MMS-001 — sine manufactured solution and observed FEM convergence order

**Status:** implementation and CI qualification pending.  
**Solver profile:** stock CalculiX 2.22, built from the pinned source/build evidence in the parent PR.  
**Element family:** 1D linear truss (T3D2), uniform axial bar.  
**Purpose:** verify nontrivial field representation and observed convergence order, not just one analytical tip displacement.

## Paradigm shift: from one answer to a convergent solution family

The existing uniform axial-bar case is useful for proving that a real, pinned solver executes a deck whose tip displacement matches \(u=FL/(EA)\). However, its displacement field is linear and is represented exactly by a linear truss element. It cannot by itself prove the expected spatial discretization order.

This fixture instead freezes a smooth, nontrivial exact field, constructs exact consistent nodal loads, solves four increasingly fine meshes, reconstructs the piecewise-linear finite-element field, and measures normalized \(L^2\) and \(H^1\)-seminorm errors. The regression target is the **observed order of accuracy**, not a hand-picked nodal displacement. This follows the method of code verification described in the [NAFEMS 2026 Code Verification Exemplars](https://www.nafems.org/publications/resource_center/r0135/) and fits within the verification/validation/uncertainty-quantification framework of [ASME V&V 10-2019 (R2025)](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics).

## Mathematical fixture

Let \(x\in[0,L]\), with \(L=1\,\mathrm m\), Young's modulus \(E=210\,\mathrm{GPa}\), cross-sectional area \(A=0.01\,\mathrm{m}^2\), and displacement amplitude \(U=10^{-6}\,\mathrm m\). Use

\[
u(x)=U\sin(\pi x/L).
\]

The one-dimensional equilibrium equation is

\[
-\frac{d}{dx}\left(EA\frac{du}{dx}\right)=q(x),\qquad u(0)=u(L)=0,
\]

with manufactured distributed load

\[
q(x)=EA\frac{\pi^2}{L^2}U\sin(\pi x/L).
\]

For a linear finite element with shape function \(N_i(x)\), the nodal load is the consistent integral \(F_i=\int_0^L N_i(x)q(x)\,dx\). Uniform-grid closed forms are used to generate the CLOAD rows. For internal nodes \(x_i=ih\), \(h=L/n\),

\[
F_i=\frac{2EAU}{h}\left(1-\cos(\pi h/L)\right)\sin(\pi x_i/L).
\]

For either endpoint, the consistent force is

\[
F_0=F_n=EAU\left(\frac{\pi}{L}-\frac{\sin(\pi h/L)}{h}\right).
\]

Endpoint displacements remain fixed; endpoint load components are retained in the deck so the applied loading is complete. All transverse degrees of freedom are constrained to keep the model in the exact axial profile.

## Frozen mesh matrix and outputs

Run element counts \(n\in\{4,8,16,32\}\). For each independently executed CCX case, retain exact deck, .dat, .frd, process log, executable identity, build-manifest identity, numerical report, and process receipt digests.

The verifier checks:
- exact equality with the deterministic deck generated for the selected mesh;
- exactly one U-displacement section for set ALL, at time \(t=1\);
- exactly \(n+1\) unique, ordered nodes with finite values and negligible transverse displacement;
- maximum normalized nodal discrepancy from \(U\sin(\pi x_i/L)\) no larger than \(5\times10^{-6}\);
- normalized \(L^2\) and \(H^1\)-seminorm errors of the reconstructed piecewise-linear field, evaluated with eight-point Gauss–Legendre quadrature per element.

The reported metrics are

\[
e_{L^2}=\left[\frac{1}{L}\int_0^L\left(\frac{u_h(x)-u(x)}{U}\right)^2dx\right]^{1/2}
\]

and

\[
e_{H^1}=\left[\frac{1}{L}\int_0^L\left(\frac{L}{\pi U}\left(u_h'(x)-u'(x)\right)\right)^2dx\right]^{1/2}.
\]

For linear elements in the asymptotic regime, the expected rates are \(p_{L^2}\to2\) and \(p_{H^1}\to1\). For each adjacent pair, calculate \(p=\log(e_h/e_{h/2})/\log 2\). All three adjacent transitions must pass:
- \(p_{L^2}\in[1.8,2.2]\);
- \(p_{H^1}\in[0.9,1.1]\).

These are fixture-specific gates, not universal FEM tolerances. A deviation is a failed or unresolved verification result, not permission to relax the threshold after observing the data.

## Claim ceiling

Passing this test would be evidence that this pinned CalculiX build and this exact 1D linear-truss profile reproduce a smooth manufactured solution and show the expected convergence rates on the four checked meshes. It would **not** establish general 2D/3D FEM accuracy, contact/nonlinear/transient correctness, experimental physical validity, structural safety, or engineering-use authorization.

Subsequent targets should include a manufactured curved/displacement field with independently checked stress/energy norms; 2D plane stress/strain and 3D solid elements; nonuniform/adaptive mesh studies; then NAFEMS-style verification cases and documented uncertainty budgets. Realistic experimental validation remains a separate activity from code verification.
