# PHYS-CCX-MMS-P1-001 — manufactured 2D elasticity convergence protocol

**Status:** protocol frozen for implementation; no CalculiX execution or numerical qualification has been performed by this change.  
**Class:** code verification, not physical validation.  
**Intended solver:** pinned CalculiX build, using linear three-node plane-strain triangles (CPE3).  
**Relationship to PHYS-CCX-EXEC-001:** a separate next-stage case. The existing axial-bar fixture remains useful for execution/provenance checks, but its linear displacement field is exactly represented by linear elements and cannot establish general convergence.

## 1. Question and claim ceiling

Can a pinned solver, request/deck generator, output parser, and numerical postprocessor reproduce the expected asymptotic convergence rates for a smooth, nontrivial elasticity solution across four mesh resolutions?

A passing result may support only code-verification claims for this specified linear-elastic manufactured problem and exact source/build/request/deck/output identities. It does not demonstrate physical validity, general structural safety, nonlinear or dynamic FEM correctness, or superiority to commercial or open-source solvers.

This is protocol work only. Do not label the model validated, the solver generally verified, or the result engineering-use qualified.

## 2. Manufactured problem

Use the nondimensional unit square \(\Omega=[0,1]^2\), plane strain, small displacement, homogeneous isotropic linear elasticity, \(E=1\), and \(\nu=0.3\). The exact field is smooth, non-affine, and zero on the entire boundary.

Let \(k=\pi\), with amplitudes \(A=1\) and \(B=0.7\):

\[
u_x(x,y)=A\sin(kx)\sin(ky),\qquad
u_y(x,y)=B\sin(2kx)\sin(ky).
\]

Prescribe the exact displacement at all boundary nodes. Both components are zero on the square boundary, but their interior field is nontrivial. The body force is chosen so the exact field satisfies equilibrium.

For plane strain, define

\[
\mu=\frac{E}{2(1+\nu)},\qquad
\lambda=\frac{E\nu}{(1+\nu)(1-2\nu)}.
\]

Using the Navier operator, the manufactured force density is

\[
\mathbf f=-\mu\nabla^2\mathbf u-(\lambda+\mu)\nabla(\nabla\cdot\mathbf u).
\]

The explicit expressions to implement and unit-test are

\[
\begin{aligned}
f_x={}&(\lambda+3\mu)A k^2\sin(kx)\sin(ky)\\
&-4(\lambda+\mu)B k^2\cos(ky)\cos(2kx),\\
f_y={}&(2\lambda+7\mu)B k^2\sin(2kx)\sin(ky)\\
&-(\lambda+\mu)A k^2\cos(kx)\cos(ky).
\end{aligned}
\]

The global body-force resultant is an independent oracle check:

\[
\int_\Omega f_x\,d\Omega=4A(\lambda+3\mu),\qquad
\int_\Omega f_y\,d\Omega=0.
\]

This is a manufactured mathematical benchmark, not a model of a real structure.

## 3. Discretization and mesh family

- Element formulation: three-node, linear plane-strain triangle (CPE3).
- Material: \(E=1,\nu=0.3\); no plasticity, contact, damping, geometric nonlinearity, or stabilization.
- Domain and thickness convention: unit square, plane strain; document the solver's section/thickness convention in the deck receipt.
- Boundary: impose the analytical displacement for each boundary node; do not infer node sets from stale node numbering.
- Loading: create consistent nodal loads from the body force,
  \[
  \mathbf F_i=\int_\Omega N_i(x,y)\mathbf f(x,y)\,d\Omega,
  \]
  and record the exact force vector that is emitted into the deck. Do not approximate the spatially varying force by one constant element load.
- Mesh levels: uniform \(n\times n\) Cartesian cells with each cell split along its lower-left to upper-right diagonal, \(n\in\{8,16,32,64\}\). Every node/element identifier, connectivity list, and boundary inventory is generated deterministically from the frozen recipe. Maximum element diameter halves between levels.
- The source term and load-vector generator must be tested separately from the CalculiX result parser.

Use a 32-by-32 Gauss–Legendre product rule under a Duffy transform for each reference triangle for load integration and error norms. Independently repeat these integrations with a 48-by-48 rule. Require the relative Euclidean difference between the resulting global load vectors to be at most \(10^{-12}\), and the relative difference between each reported global error norm to be at most \(10^{-10}\). If these integration checks fail, the case is unqualified; do not silently change the quadrature or tolerance after inspecting solver convergence.

## 4. Error measures and observed order

Parse nodal displacement from the actual CalculiX output, bind it to the precise mesh/request/deck/output digests, and evaluate the linear finite-element interpolant within each triangle. Do not compute convergence from endpoint or maximum nodal displacement alone.

For each mesh, report both the absolute and normalized L2 displacement error:

\[
E_{L^2}=\left(\frac{\int_\Omega\|\mathbf u_h-\mathbf u\|^2\,d\Omega}
{\int_\Omega\|\mathbf u\|^2\,d\Omega}\right)^{1/2}.
\]

Also report the relative plane-strain energy error:

\[
E_E=\left(\frac{\int_\Omega
(\boldsymbol\epsilon_h-\boldsymbol\epsilon):\mathsf C:
(\boldsymbol\epsilon_h-\boldsymbol\epsilon)\,d\Omega}
{\int_\Omega\boldsymbol\epsilon:\mathsf C:\boldsymbol\epsilon\,d\Omega}\right)^{1/2},
\]

where \(\mathsf C\) is the plane-strain constitutive tensor and exact strains are evaluated analytically. Record a relative stress L2 error as a diagnostic, but do not substitute it for either primary gate.

For successive uniform refinements, calculate

\[
p_i=\frac{\log(E_i/E_{i+1})}{\log(h_i/h_{i+1})}.
\]

For smooth solutions with quasi-uniform, shape-regular P1 triangles, the preregistered asymptotic expectations are order approximately one in energy/H1 norm and order approximately two in displacement L2 norm. These are theoretical expectations, not results observed from this solver.

## 5. Frozen qualification gates

All gates are conjunctive. A failed or missing gate means qualification is false.

1. **Input and provenance:** source archive, compiler/toolchain, build features, executable, request, generated mesh, deck, log, output, and verifier source are hashed. Each level's result points to exactly one request, deck, executable and output. Reject missing, stale, cross-mesh, or mismatched evidence.
2. **Solver process:** clean successful exit; expected nonempty result files; no timeout; all reported fields finite; exact requested step and output are present.
3. **Boundary condition:** parsed boundary displacements agree with the analytical prescription within a scale-aware \(10^{-10}\) absolute/relative mixed bound.
4. **Force and reactions:** the load generator's global resultant agrees with the analytical integral above to relative \(10^{-10}\); solver reactions plus applied loads satisfy global equilibrium to a scale-aware relative \(10^{-8}\) bound. Record force components separately so cancellation cannot hide a component error.
5. **Numerical errors:** \(E_{L^2}\) and \(E_E\) are finite and positive at all levels and decrease strictly at every refinement.
6. **Observed order:** for each of the last two refinement intervals, \(0.75\le p_E\le1.25\) and \(1.5\le p_{L^2}\le2.5\). Publish every per-level error and order, including coarse levels that do not meet the asymptotic window.
7. **Repeatability:** run the \(n=16\) case twice from identical hashed inputs. Numerical outputs and deterministic numerical receipts must agree within declared floating-point bounds. Keep nondeterministic wall-clock timings in a separate raw timing file so they do not contaminate deterministic receipt equality.
8. **Quadrature independence:** the 32-by-32 versus 48-by-48 checks in Section 3 pass for loads and both primary error norms.
9. **Negative controls:** qualification must reject (a) a one-percent perturbation to a deterministic interior nodal load, (b) a deck bound to a different mesh request, (c) a one-byte output tamper, (d) a missing output field, (e) a wrong material parameter in the request/deck binding, and (f) a receipt whose executable digest does not match the executed binary.
10. **No result laundering:** failed solver runs and failed numerical gates remain in the report with their error state and work performed; never omit them from a summary or plot only successful runs.

The numerical thresholds are fixed before qualification. Any proposed change must be reviewed as a protocol revision with a new version and a clear reason; do not loosen a gate to get a pass.

## 6. Required evidence receipt

Emit a deterministic machine-readable JSON receipt containing:

- protocol/version and qualification state;
- repository commit SHA, verifier source digest, request/deck digests, mesh recipe and per-level mesh digests;
- source archive checksum, build manifest, compiler identity, build flags, executable digest, and solver version;
- process exit state and relevant log/output digests for each mesh;
- analytical load resultant, load-vector digest, boundary error and force/reaction residuals;
- per-level DOF/node/element counts, \(h\), \(E_{L^2}\), \(E_E\), stress diagnostic and all observed orders;
- integration-rule cross-check results;
- all negative-control outcomes and explicit failure reasons.

A separate timing record must include warm-up policy, repetitions, raw durations, host/CPU identity, thread settings, filesystem/storage note, compiler flags, and executable/source digest. Performance is descriptive until repeated measurements are made on pinned hardware and compared at matched achieved error against other solvers.

## 7. Implementation sequence

1. Add analytic unit tests for displacement, strain, stress, and both body-force components at fixed points; check force-resultant identities independently.
2. Implement deterministic mesh/request generation and consistent nodal-force integration. Cross-check quadrature order before invoking CalculiX.
3. Add a strict output parser bound to the exact per-mesh request, deck, solver binary, and output bytes.
4. Execute all four meshes plus the \(n=16\) repeat, preserving process receipts on both success and failure.
5. Run the convergence and negative-control gates; archive raw machine-readable receipts and logs.
6. Only after this family is qualified, extend toward 2D patch/geometry variation, 3D solids, transient dynamics, and nonlinear materials.

## 8. References and rationale

- NAFEMS, *Code Verification Exemplars in Computational Solid Mechanics*, R0135, 3 March 2026: observed order of accuracy from multiple mesh resolutions and an exact solution. https://www.nafems.org/publications/resource_center/r0135/
- Aycock, Rebelo and Craven, *Method of manufactured solutions code verification of elastostatic solid mechanics problems in a commercial finite element solver*, Computers & Structures (2020). https://doi.org/10.1016/j.compstruc.2019.106175
- NAFEMS code-verification discussion of the method of manufactured solutions and observed convergence. https://www.nafems.org/community/the-analysis-agenda/code-verification/

The protocol deliberately does not claim that this exact case has been executed, passed, or benchmarked against industry solvers.
