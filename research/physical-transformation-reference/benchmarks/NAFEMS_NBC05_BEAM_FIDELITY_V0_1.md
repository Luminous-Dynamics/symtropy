# NAFEMS NBC05 Beam-Fidelity Contract v0.1

**Tracking:** [PHYS-NBC05-BEAM-FIDELITY-001 #2557](https://github.com/Luminous-Dynamics/symtropy/issues/2557)  
**Status:** specification and qualification plan; no beam-dynamics solver is implemented or qualified by this document.  
**Reference:** NAFEMS Benchmark Challenge 5, *Dynamic Characteristics of a Truss Structure*.

## The key finding

The [official NAFEMS solution](https://www.nafems.org/downloads/nbr5v3.pdf) distinguishes the easy-to-reproduce axial-bar frequency table from the physical structure's modal response. It states that intermediate nodes added to pin-jointed bar members can move off the member centreline, creating artificial transverse mechanisms unless additional kinematic constraints are applied. The alternative examined in the solution is a planar beam-element model with moments released at the three truss joints.

For its beam study, the solution reports:

- 200 beam elements per member were used to obtain frequencies within about 1% up to 1600 Hz in that study.
- 22 beam modes appeared below 1600 Hz, compared with only two axial-bar modes in the same frequency range.
- The beam model has a lower fundamental frequency than the bar-only model and introduces bending modes around the axial modes.
- A consistent beam mass model includes rotary-inertia terms; the considered lumped mass model does not make the same rotary-inertia contribution.

The figure marks beam modes at approximately 366, 496, 1021 and 1183 Hz, while the corresponding bar-model markers are about 451 and 1027 Hz. These are graphical literature comparators; use them as coarse sanity checks, **not** high-precision target values.

Primary sources:

- Challenge diagram and reported axial frequencies: https://www.nafems.org/downloads/nbc5_challenge.pdf
- Discussion of mechanisms, beam releases, mode families and rotary inertia: https://www.nafems.org/downloads/nbr5v3.pdf
- NAFEMS solution record: https://www.nafems.org/publications/resource_center/bm_jul_16_13/
- Verification/validation/UQ context: https://webstore.ansi.org/standards/asme/ASME102019R2025

## Frozen source interpretation

The source figure provides a 1 m horizontal span, 1 m vertical offset, two members meeting at the free upper-right joint, E=210 GPa, nu=0.3, density 7800 kg/m³ and section annotation r=0.01 m.

For a declared circular-section interpretation:

- A=pi*r²
- I=pi*r⁴/4
- radius of gyration r_g=sqrt(I/A)=r/2=0.005 m

That is an engineering assumption which must be preserved in the receipt; if the source's r label means something else, the beam benchmark input must be reinterpreted. Unlike the uniform axial-only frequency table, beam bending frequencies depend strongly on I and therefore on the section interpretation.

## Proposed model contract

### B0 — Element mechanics and sign conventions

The first element-matrix implementation is in draft [PR #2559](https://github.com/Luminous-Dynamics/symtropy/pull/2559), which adds a local 6-DOF Euler-Bernoulli frame-element reference in order [u1, v1, theta1, u2, v2, theta2]. It builds the axial EA/L block, bending EI/L³ block, axial/transverse consistent mass, and a separate rotary-inertia mass contribution. This is a local element primitive, not a global beam/modal solver.

The stacked global-axis transformation increment in PR #2560 adds congruence transforms for stiffness and each mass contribution, rejects geometry-length mismatch, verifies strain-energy invariance and global matrix symmetry, and checks rigid-body translation/rotation residuals. PR #2561 adds deterministic assembly with shared node translations and private per-member-end rotations, a frozen canonical request, and a Schur-complement oracle against independent axial-truss stiffness. This qualifies the assembly mapping if its exact-head workflow passes; it still does not solve the full NAFEMS NBC05 converged beam spectrum. B0 now separately pins the local, transformed and assembled contracts.

The draft CI mutation audit is intended to kill omitted/incorrect second moment, bending-coupling sign, omitted axial area, omitted rotary inertia, wrong rotary length exponent, transverse-mass factor error, rotary-mass off-diagonal sign error, and a local-to-global transverse sign defect. Only the mutations actually reported detected at an exact-head passing run count as evidence.

The new cross-mesh identity experiment in [PR #2641](https://github.com/Luminous-Dynamics/symtropy/pull/2641) addresses a different failure mode: mode ordinal is not stable across mesh refinement. It computes standard sign-invariant MAC over the exact intersection of rational physical stations, reports translation and rotation MAC separately, scales rotations by the frozen maximum member length for a combined score, and uses a Hungarian maximum-weight assignment to match the six modes globally. Undefined MAC remains null and is counted as unresolved rather than coerced to a confident match. The request freezes 1/2/4/8 elements per member and the DOF identity policy. Mutation gates target wrong rational stations, minimizing rather than maximizing MAC, and omission of rotation scaling.

The parallel [mesh-convergence ladder PR #2639](https://github.com/Luminous-Dynamics/symtropy/pull/2639) records the frequency trend and solver residuals over those same levels. The two reports are complementary: convergence tracks frequency changes; mode tracking assesses whether a mode at one mesh corresponds to a mode at another. Neither alone establishes reproduction of the source's full 200-elements-per-member result.


The stacked modal prototype in draft PR #2562 reuses the shared Cholesky-reduced generalized symmetric eigensolver, verifies all six free modes of the one-element-per-member fixture, reports mass normalization/orthogonality and eigenpair residuals, and rejects zero/near-zero modes instead of silently filtering them. A separate two-rotation simply-supported beam case compares the generalized eigenvalues and hertz conversion against an explicit 2×2 analytic determinant. It is a coarse algebraic check; it does not assert the published ~366/496/1021/1183 Hz graph markers as target values, and it is not a mesh-converged NBC05 result. Its exact-head test, mutation, deterministic-receipt and source-digest gates remain pending.

A static stiffness matrix or a correct axial-bar frequency does not qualify beam dynamics.

### B1 — Pin releases at all three truss joints

Do not simulate a pin simply by fixing the node's shared rotation to zero. At an ideal pin the connected member end rotations are not shared and no end moment is transmitted. Represent released member-end rotations explicitly, while the translational DOFs at the joint remain connected. At the two supported joints, restrain translations per the source model while preserving the intended moment release.

Add negative controls proving that:
- replacing a moment release with a rigid rotation makes the response change and is detected;
- fixing release rotations to zero is not silently accepted as an ideal pin;
- the reduced model has the expected rigid/mechanism mode count;
- no eigenvalues are silently dropped merely because they are near zero.

### B2 — Mass matrix and rotary inertia

Expose the mass formulation as part of the canonical request. For a consistent beam mass matrix, include translational mass and the selected beam theory's rotary-inertia terms; for a lumped mass variant, document exactly which rotary-inertia terms are retained or omitted. Do not transfer the axial-only sqrt(2/3) frequency ratio to a beam model: the matrices are different.

Verify mass-matrix symmetry and positivity on the constrained subspace; emit modal-mass normalization, generalized-eigenpair residual, orthogonality and a mode-shape comparison metric such as MAC. Confirm whether the solver models shear deformation and geometric rotary effects.

### B3 — Mesh and modal convergence

Start with small meshes and build an explicit convergence matrix (1, 2, 4, 8, 16, 32, 64, 100, 200 elements/member as feasible). Report mode count, frequency change, MAC-based mode correspondence and mechanism count at each level. Avoid computing a dense eigenspectrum at 400 elements if a sparse eigen-solver is needed; declare the backend and exact version.

The published 'within 1% up to 1600 Hz' figure is a result for the source's beam formulation and assumptions, not a universal promise for a different implementation. A result counts as a reproduction only when the DOFs, joint releases, cross section, beam theory, mass and rotary-inertia choices, solver, mode pairing and convergence trend agree.

### B4 — Cross-model interpretation

Publish three separate result families:
1. the 2-DOF axial-bar table reproduction (CS1 consistent / CS2 lumped);
2. axial-bar mesh-refinement with explicit centreline constraints and mechanism checks;
3. released-joint beam dynamics with bending and rotary inertia.

Compare across families to show which modes are lost by the axial-only abstraction. Never merge them into one modal error number.

## Required receipt

Every beam run must include:
- exact commit and canonical request SHA-256;
- source citations and assumption ledger, including section interpretation;
- coordinates, connectivity, element type, number of elements/member, restraints, release map and free-DOF map;
- material/section E, nu, rho, A, I, r_g and units;
- beam theory, shear deformation flag, consistent/lumped mass method and rotary-inertia treatment;
- solver/backend version, convergence status and requested/returned mode counts;
- frequency list, mode pairing, MAC, eigen-residuals, modal mass/orthogonality residuals;
- mechanism/rigid-mode count and classification;
- convergence data and the explicitly approximate/published target sources;
- claim ceiling: numerical reproduction of named beam model only, physical validation not performed.

## Acceptance gates (proposed, not yet achieved)

- Analytical low-dimensional beam element checks pass independently.
- No unexplained rigid/mechanism modes or silently discarded eigenvalues.
- Frequency and mode-shape convergence established against a predeclared stopping criterion.
- Approximate literature modes used for orientation only; quantitative pass/fail requires a precise, source-defined target model and eligible reference values.
- All identified mutations (bending stiffness omitted, wrong I exponent, removed rotary inertia, release semantics changed, axis transform sign) are killed by named tests.
- Only after the above can a published-beam-model reproduction be claimed. No physical validation, seismic response prediction, structural safety or code compliance is implied.
