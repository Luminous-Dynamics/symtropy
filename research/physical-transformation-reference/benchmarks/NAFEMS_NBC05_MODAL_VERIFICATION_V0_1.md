# Modal Truss Verification Contract v0.1

**Tracking:** [PHYS-MODAL-TRUSS-001 #2531](https://github.com/Luminous-Dynamics/symtropy/issues/2531)  
**Status:** benchmark intake contract plus a 1D axial-bar modal prototype with mode-shape diagnostics and deterministic receipt candidate; no modal truss solver or NBC05 reproduction is qualified.  
**Model class:** undamped, linear, small-motion free vibration. Separate from the static planar-truss reference.

## Why this is a model-class boundary

A passing static stiffness solve does not qualify a natural-frequency calculation. Modal analysis requires a mass operator as well as stiffness, a generalized eigenproblem, mode normalization/pairing, and explicit treatment of rigid and mechanism modes:

\[
K\phi_i = \omega_i^2 M\phi_i, \qquad f_i=\frac{\omega_i}{2\pi}.
\]

The NAFEMS Benchmark Challenge 5 (NBC05) solution reports that two commercial systems used the same stiffness matrix but different mass formulations: consistent versus lumped. Coarse-mesh frequencies differed significantly, with the consistent-mass results above and the lumped-mass results below the theoretical solution in the one-dimensional bar study; both converged with refinement. The solution also warns that subdividing pin-jointed truss members creates transverse mechanisms unless extra constraint equations keep intermediate nodes on the member centreline. These are formulation/constraint issues, not just solver tolerances.

Primary references:

- NAFEMS, [Challenge 5 statement PDF](https://www.nafems.org/downloads/nbc5_challenge.pdf).
- Ramsay, [Challenge 5 solution PDF](https://www.ramsay-maunder.co.uk/downloads/Benchmark%20Challenge%20005.pdf).
- NAFEMS, [Challenge 5 solution record](https://www.nafems.org/publications/resource_center/bm_jul_16_13/).
- NAFEMS, [Selected Benchmarks of Natural Frequency Analysis](https://www.nafems.org/publications/resource_center/r0015/).
- ASME, [VVUQ standards resource hub](https://www.asme.org/codes-standards/vvuq-standards).
- OpenSees, [basic linear truss example](https://opensees.github.io/OpenSeesDocumentation/user/examples/basicExamples/basicTruss.html) (static mechanics; not itself a modal qualification).

This document describes and cites the references; it does not redistribute their PDFs, figures, or full published tables.

## Source facts versus unresolved model inputs

The NBC05 figure identifies a planar, pin-jointed two-member truss with 1 m dimensions, Young's modulus \(E=210\,\mathrm{GPa}\), Poisson ratio \(\nu=0.3\), density \(\rho=7800\,\mathrm{kg/m^3}\), and a member-section annotation \(r=0.01\). It reports commercial-system outputs:

| Mode | CS1 | CS2 |
|---|---:|---:|
| 1 | 452.09 Hz | 369.13 Hz |
| 2 | 1114.6 Hz | 910.05 Hz |

Those are **reported solver outputs**, not by themselves exact target frequencies. They must not be converted into a hard-coded pass range without the full problem definition and independent reference.

The source-attributed **reported-table model** is now explicit:

- Interpret the figure's supports as the upper-left node A=(0,1 m) and lower-left node B=(0,0) fixed in both translations; the upper-right node C=(1 m,1 m) is free in x/y.
- Member AC is horizontal with length 1 m. Member BC is diagonal with length \(\sqrt{2}\,\mathrm{m}\). Both use the figure's common E and density.
- The official solution identifies CS1 as the consistent-mass system and CS2 as the lumped-mass system: https://www.nafems.org/downloads/nbr5v3.pdf.
- Preserve the annotation \(r=0.01\) as an assumption that it denotes the radius of a circular section. The area \(A=\pi r^2\) cancels from these axial-only frequencies because the same area multiplies stiffness and inertia; this does **not** make section interpretation irrelevant to beam bending / rotary inertia. Poisson ratio is recorded but unused by linear pin-jointed axial elements.
- This is a two-free-DOF problem. The independently reduced stiffness is \(K=EA\begin{bmatrix}1/L_h+c^2/L_d&cs/L_d\\cs/L_d&s^2/L_d\end{bmatrix}\), with \(c=1/\sqrt{2}\), \(s=1/\sqrt{2}\), \(L_h=1\) and \(L_d=\sqrt{2}\). The consistent free-node translational mass is \(\rho A(L_h+L_d)/3\,I\); row-sum lumped mass is \(\rho A(L_h+L_d)/2\,I\). The two eigenvalues of K can therefore be computed directly from trace and discriminant, without using the production eigen-solver.

The independent analytic values are 452.091943 / 1114.580038 Hz (consistent) and 369.131526 / 910.050790 Hz (lumped). These match the published rounded table within a predeclared relative tolerance of \(10^{-4}\) (0.01%). They reproduce the **reported single-element-per-member axial-bar calculation**, not the real structure's complete modal response.

The official solution also shows why this qualification boundary matters: refining pin-jointed bars creates transverse mechanisms without centreline constraints, and a beam-element model with released joints recovers bending modes and a substantially lower fundamental frequency. Full structural modal behaviour remains a separate benchmark model class, not implied by the table reproduction. If an implementation cannot reproduce the inputs/formulation above, it must return **not-qualified / missing-input**, not pass.

## Verification ladder

### M0 — Analytical axial bar (first implementation slice)

The current prototype assembles and solves only the one-dimensional fixed-free axial bar described below. It returns sorted frequencies, mass-normalized free-DOF shapes, normalized generalized-eigenpair residuals, and deviation from unit modal mass for explicitly selected consistent or lumped mass. It is not an NBC05 truss solve; a request-bound evidence receipt and mutation-qualified independent reference remain future gates.

For a uniform one-dimensional axial bar fixed at one end and free at the other, the analytical longitudinal frequencies are

\[
f_n=\frac{2n-1}{4L}\sqrt{\frac{E}{\rho}}, \quad n=1,2,\ldots
\]

For the 1 m, 210 GPa, 7800 kg/m³ values displayed in NBC05, the fundamental is about 1.297 kHz. This analytical bar test is a **separate subproblem**, not an assertion that the whole truss has that frequency.

- [Implemented prototype] Assemble reduced axial-bar stiffness and consistent or row-sum lumped mass matrices in SI units; solve the small generalized symmetric eigenproblem through Cholesky reduction and a Jacobi eigensolve.
- [Implemented tests] Compare the single-element consistent/lumped frequencies with closed-form frequency ratios, check first-mode convergence across 1, 2, 4, 8, and 16 elements, check that the second mode is within 5% at 8 elements for this bar test, and assert mass normalization and generalized-eigenpair residuals for computed modes.
- [Implemented in receipt candidate] Freeze canonical request bytes, SHA-256 the canonical input, bind outputs to the exact source commit and source-tree digest, run twice for byte-identical output, and gate the two 8-element consistent-mass frequencies against independently computed discrete-system values and analytical frequency error.
- [In draft mutation audit] Five isolated source mutants target consistent-mass diagonal/off-diagonal terms, Hz conversion, Jacobi eigenvector rotation, and the transposed-Cholesky back-solve. The script requires the named test to fail and verifies the production source hash remains unchanged. Exact-head CI has to pass before this is accepted as mutation-sensitivity evidence.
- [Implemented test] Compare the first three consistent- and lumped-mass mode shapes against independently sampled analytical sine shapes using sign-invariant MAC, with a predeclared threshold of (1-10^{-12}).
- [Remaining] Keep cross-mode mass orthogonality in the request-bound receipt, add a dedicated known-answer test for that metric, and extend mutation coverage to the shape oracle. The receipt workflow must pass at its exact head before evidence is qualified.

### M1 — Independent small-matrix eigenproblem

Create a tiny, analytically solvable \(K\phi=\lambda M\phi\) known-answer fixture that is independently derived (not assembled by the same production routine). Check eigenvalues, eigen-residuals, eigenvector normalization, and mode-order stability. Include a repeated/near-repeated eigenvalue case so mode comparison does not depend solely on sign or array ordering.

### M2 — NBC05 reported axial-bar table, then full modal fidelity

The implementation candidate now pins a machine-readable case manifest and independently derives the two eigenvalues of the reduced 2×2 model; it emits a request/source-bound receipt and checks the reported CS1/CS2 frequencies at the tolerance declared above. Count this only as a reproduction of the *published axial-bar table*. The exact-head CI and mutation-audit gates must pass before the new result is counted as a software-qualified reproduction.

A subsequent M2b/M3 must add subdivided member centreline constraints and zero/mechanism-mode classification; then use a released-joint beam model with second moment of area and appropriate rotary inertia to test physical modal completeness. The dedicated contract is [NAFEMS_NBC05_BEAM_FIDELITY_V0_1.md](NAFEMS_NBC05_BEAM_FIDELITY_V0_1.md). Do not call the current axial-only model the real structure's modal solution.

### M3 — Subdivision and mechanism tests

Refine each member while explicitly representing the kinematic constraints needed by the idealized truss model. Tests must separate:
- expected elastic modes,
- zero/near-zero rigid-body modes permitted by the boundary conditions,
- artificial transverse mechanisms,
- numerical eigen-solver failures.

Do not hide mechanisms by dropping all small eigenvalues. Count them, classify them, and fail closed if the count differs from the model contract. Centreline constraints for a bar-element idealization are not equivalent to modelling physical member bending.

### M4 — Cross-solver and convergence matrix

Compare against at least one independently configured open solver or published reference, with exact software/version, element formulation, mass formulation, constraints and degrees of freedom recorded. The baseline must use matching physics and quantities of interest. Beam-element models with appropriate bending/rotary-inertia assumptions can be valuable triangulation, but are not interchangeable with axial-truss results.

## Required evidence receipt

Each run should emit deterministic, schema-versioned JSON bound to the exact source commit and canonical request digest, containing at least:

- benchmark ID/revision, source URL, retrieval date, licence/usage note and unresolved-input list;
- solver/tool version, element type, units, stiffness and mass formulations;
- nodes, elements, properties, supports, constraints and free-DOF map;
- eigenvalues and frequencies with mode-pairing metadata;
- normalized eigen-residuals, \(\Phi^T M\Phi\) orthogonality error and \(K/M\) symmetry diagnostics;
- count and classification of zero/near-zero modes;
- relative frequency errors where an eligible target exists, mesh/convergence results, and method for pairing mode shapes (e.g. MAC);
- deterministic replay result, source/request hashes, and explicit claim ceiling.

The receipt must distinguish **no target available**, **input incomplete**, **numerical check failed**, **benchmark reproduced**, and **physical validation not performed**. Unknowns must not be encoded as zero-valued references.

## Qualification targets (not achieved)

- 100% of pinned analytical fixtures reproduce their independently derived expected values within published tolerances.
- Zero unexplained rigid/mechanism modes; no silently discarded eigenvalues.
- Eigen-residual and mass-orthogonality tolerances selected per conditioning and declared before evaluation.
- For an eligible external target, report mode-wise relative-frequency error and convergence; do not choose tolerance after seeing the result.
- Publish failed cases and negative controls alongside passes.

Performance claims come later. Compare runtime, memory and scaling only after frequency accuracy, mode count, constraints and mass formulation are matched. A faster model that has removed physical modes or changed the mass model is not an improvement.

## Claim ceiling

This contract is a research specification. It does not implement modal analysis, reproduce NBC05, validate a real structure, model member-bending vibration, provide a structural safety factor, or certify design/code compliance. The current static truss solver remains limited to small-displacement, linear-elastic 2D pin-jointed axial bars. Modal, nonlinear, contact, fracture, and physical-validation evidence require separate implementations and qualifications.
