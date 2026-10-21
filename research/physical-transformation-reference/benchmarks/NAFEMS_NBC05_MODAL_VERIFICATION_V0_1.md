# Modal Truss Verification Contract v0.1

**Tracking:** [PHYS-MODAL-TRUSS-001 #2531](https://github.com/Luminous-Dynamics/symtropy/issues/2531)  
**Status:** benchmark intake and implementation contract; no modal solver or NBC05 reproduction is qualified by this document.  
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

Before reproducing NBC05, resolve and freeze:

- The section annotation's meaning and units (in particular whether \(r\) denotes a radius, and therefore whether a circular area \(A=\pi r^2\) is justified).
- Exact node coordinates and member connectivity from the source drawing.
- The truss model's support conditions and free degrees of freedom. The source paragraph's supports for the separate straight-bar *test structure* must not be silently reused as the truss boundary conditions.
- Which mass formulation is intended for each comparison; any consistent and lumped variants must be labelled as distinct model runs.
- The expected physical mode family and how any rigid/mechanism modes are identified or constrained.
- Whether the acceptance reference is analytical, a published numerical target, or an independently checked cross-solver result.

If one of these changes the mathematical problem and is unresolved, the adapter must return **not-qualified / missing-input**, not pass.

## Verification ladder

### M0 — Analytical axial bar (the first implementation target)

For a uniform one-dimensional axial bar fixed at one end and free at the other, the analytical longitudinal frequencies are

\[
f_n=\frac{2n-1}{4L}\sqrt{\frac{E}{\rho}}, \quad n=1,2,\ldots
\]

For the 1 m, 210 GPa, 7800 kg/m³ values displayed in NBC05, the fundamental is about 1.297 kHz. This analytical bar test is a **separate subproblem**, not an assertion that the whole truss has that frequency.

- Add element-level stiffness and mass matrices with explicit SI units.
- Support consistent and lumped mass as distinct named options; never silently change the default formulation across releases.
- Run 1, 2, 4, 8, and 16 equal elements and report the frequency error for each requested mode.
- Independently check the generalized-eigenpair residual and mass-normalization/orthogonality.
- Confirm the published convergence observations before pinning exact acceptance values: the solution reports that roughly 2 elements meet 5% error for the first mode while roughly 8 are needed for the second.

### M1 — Independent small-matrix eigenproblem

Create a tiny, analytically solvable \(K\phi=\lambda M\phi\) known-answer fixture that is independently derived (not assembled by the same production routine). Check eigenvalues, eigen-residuals, eigenvector normalization, and mode-order stability. Include a repeated/near-repeated eigenvalue case so mode comparison does not depend solely on sign or array ordering.

### M2 — NBC05 single-element-per-member reconstruction

Only after the unresolved inputs above are checked, freeze a machine-readable case manifest and independently derive or obtain eligible reference frequencies. Report the CS1/CS2 numbers as literature comparators, not as ground truth. A result counts as reproduced only when the benchmark identity, units, geometry, supports, mass model, degree-of-freedom map and target definition all match.

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
