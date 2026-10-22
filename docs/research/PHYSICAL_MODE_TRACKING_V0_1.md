# PHYS-MODE-TRACK-001 — Modal correspondence across parameter and mesh changes

**Status:** Proposed verification contract, version 0.1  
**Parent:** [PHYS-MODE-TRACK-001 issue #2668](https://github.com/Luminous-Dynamics/symtropy/issues/2668)  
**Qualification context:** [model-class benchmark ladder PR #2663](https://github.com/Luminous-Dynamics/symtropy/pull/2663)  
**Scope:** Numerical mode correspondence only. Not a structural-solver qualification or physical validation.

## 1. Problem statement

A sequence of eigenfrequencies sorted by magnitude does not preserve physical modal identity when parameters change. Modes may cross, veer, cluster, or become repeated. Within an exactly repeated eigenspace, an eigensolver may return any orthonormal basis spanning the same subspace; individual eigenvectors therefore need not have unique identities.

A tracking method must answer two distinct questions:

1. **Correspondence:** which isolated modes or invariant subspaces in two declared model states are comparable?
2. **Confidence:** does the evidence support a unique one-to-one match, a cluster-level match, or only an ambiguous result?

Frequency proximity alone is insufficient. Mode-shape correlation alone is also insufficient near degeneracy or when degree-of-freedom (DOF) semantics differ. The tracker must be permitted to return *ambiguous*, *cluster match*, or *unmatched*; it must not create certainty by sorting or tie-breaking alone.

## 2. Mathematical object and conventions

Each model state provides a generalized symmetric eigenproblem

\[
K(\theta)\,\phi_i(\theta)=\lambda_i(\theta)\,M(\theta)\,\phi_i(\theta),\qquad
\lambda_i=\omega_i^2,\quad f_i=\omega_i/(2\pi).
\]

Here \(\theta\) denotes a frozen model state, such as a material parameter, boundary-condition variant, or mesh level. The contract must record the DOF ordering, units, constraints, mass formulation, eigensolver/version, mode-selection interval, and normalization convention for every state.

For an isolated mode pair represented in the **same comparison space** with a declared positive-definite metric \(W\), use a sign-invariant metric-weighted MAC:

\[
\operatorname{MAC}_W(u,v)=
\frac{|u^T Wv|^2}{(u^TWu)(v^TWv)}.
\]

Its result is in \([0,1]\), up to declared floating-point tolerance. It is invariant to sign and nonzero scalar rescaling. When the discrete models differ, the contract must define a physically meaningful common comparison space or cross-metric operator. It must not assume that nodal vectors of different meshes have the same meaning or silently interpolate them without specifying the mapping.

For a cluster, compare subspaces, not arbitrarily rotated basis vectors. After metric-consistent orthonormalization, calculate the singular values of the cross-basis overlap. These are cosines of principal angles. Report the principal-angle spectrum (or a declared derived norm) and cluster dimensions. Rotation of a basis within an exactly equal eigenspace must not change the subspace result.

## 3. Correspondence protocol

The v0.1 protocol is deliberately layered:

1. **Input admission.** Validate finite entries, symmetry of stiffness and mass within declared scaled tolerances, valid DOF mappings, supported constraints, and a positive-definite comparison metric where required. Reject incompatible model states rather than guessing.
2. **Spectrum checks.** Sort each spectrum numerically for deterministic serialization, while recording the original solver order. Verify eigen-residuals, normalization, mode count and expected null/mechanism classifications before attempting correspondence.
3. **Cluster discovery.** Form candidate clusters from a scale-aware relative eigengap rule. Freeze the exact rule and constants in the canonical request. Record the gaps and test threshold sensitivity; do not imply a cluster boundary is physically absolute.
4. **Isolated-mode matching.** Build a complete pairwise cost matrix with declared terms (at minimum frequency/eigenvalue drift and metric-weighted MAC where defined) and solve the global assignment problem. Do not use undocumented greedy matching. Persist cost components and assignment margins.
5. **Cluster matching.** Compare compatible clusters with principal angles in a common metric and an explicit dimensionality rule. When dimensions differ or no stable mapping exists, report partial/ambiguous/unmatched outcomes instead of forcing a bijection.
6. **Tracking decision.** Emit one of *matched*, *cluster match*, *ambiguous*, *unmatched*, or *invalid input*, including the exact rule and evidence that led to it. A deterministic tie-break may stabilize serialization, but must never be reported as evidence of unique physical identity.
7. **Receipt.** Bind the canonical request, source commit/tree, model-state inputs, DOF mappings, solver metadata, modal outputs, assignment costs, cluster definitions, principal angles, thresholds, and all terminal dispositions into a replayable receipt.

## 4. Synthetic oracle suite

Before any production mesh sweep, test small deterministic generalized eigenproblems with independently known correspondence.

| Case | Construction | Required behavior |
|---|---|---|
| Separated continuous modes | Smooth parameterized eigensystem with distinct eigenvalues | Stable one-to-one matching |
| True crossing | Two eigenvalue branches exchange numeric order while eigenvectors follow distinct branches | Frequency sorting alone must fail; tracker preserves correspondence or explains the continuation rule |
| Near-degenerate veering | Closely spaced eigenvalues with rapidly rotating vectors | Report correspondence quality and threshold sensitivity; avoid false confidence |
| Exactly repeated eigenvalue | Rotate the basis arbitrarily within a repeated invariant subspace | Cluster/subspace score remains invariant; individual labels may be ambiguous |
| Cluster entry/exit | Three-mode group crosses the frozen gap threshold | Deterministic cluster membership and explicit boundary sensitivity |
| Genuine subspace change | Perturb one physical direction while holding another fixed | Detect reduced subspace similarity |
| Sign/order/rescaling metamorphisms | Flip signs, permute eigenpairs, and rescale valid vectors | Correspondence is invariant to representation-only changes |
| Cross-mesh mapping | Analytical or independently constructed common-field modes projected from multiple mesh levels | Error is attributable to the declared projection and tracking terms, not hidden same-index comparison |
| Invalid model | NaN/Inf, duplicate IDs, incompatible DOF map or non-positive metric | Fail closed with explicit reason |

The exact matrices, parameter path, reference branch identities, expected clusters, and tolerances must be stored in canonical fixture files before implementation results are inspected. Avoid constructing the oracle from the output of the tracker under test.

## 5. Adversarial and mutation tests

At least one named mutation must target each behavior below. A killed mutant counts only when the expected assertion detects the intended defect; compile failures, timeouts, or unrelated failures do not count.

- Replace metric-weighted MAC with an unweighted dot product.
- Omit the denominator or use the wrong mass/overlap metric.
- Replace global assignment with greedy assignment.
- Remove or alter the cluster-gap rule or its scale normalization.
- Compare cluster basis vectors individually instead of using a rotation-invariant subspace metric.
- Drop the cross-mesh DOF mapping or apply an intentionally wrong projection.
- Convert angular frequency to Hz incorrectly, or compare eigenvalues to frequencies without conversion.
- Turn ambiguous matches into forced unique matches.
- Drop unmatched/invalid cases from the terminal census.

Each mutation run must occur in an isolated temporary copy, verify the authoritative source hash remains unchanged, and produce a machine-readable receipt naming the injected defect, expected detecting test, observed outcome, and source revision.

## 6. Evaluation metrics

Keep metric families separate rather than hiding failure behind a single blended score:

- **Pairing quality:** accuracy against synthetic known correspondence; false-match, unmatched and ambiguity rates.
- **Cluster quality:** principal-angle errors and cluster dimension agreement.
- **Continuity:** identity/subspace stability along a frozen parameter path; frequency-branch continuity is reported separately from eigenvector/subspace continuity.
- **Robustness:** sensitivity to gap thresholds, precision, solver ordering, basis rotation and mesh mapping.
- **Numerical validity:** eigen-residuals, metric orthogonality and positive-definiteness checks.
- **Efficiency:** runtime and peak memory at a declared mode count, DOF count and tolerance.
- **Evidence completeness:** percentage of attempted cases with a terminal receipt and source-bound deterministic replay.

Publish per-case results, uncertainty/ambiguity outcomes, and failures. A high average cannot conceal a false match in a degenerate case.

## 7. Fair external comparison

Only compare against an external tracking method after pinning its implementation/version, algorithm, thresholds and mode-selection semantics. Feed both methods identical eigensolutions first to isolate tracker quality from eigensolver quality. Then conduct a separate end-to-end comparison with solver, mesh, constraints, mass convention, precision and compute budget aligned.

Report quality-versus-cost curves, not just a single setting. Include frequency-sorted baseline, MAC-only baseline, global-assignment MAC baseline, and the cluster-aware method. Where input semantics are compatible, include a pinned reproduction of Chen & Wang (2026)'s clustering-free greedy association with adaptive filtering for time-varying modal trajectories. That paper addresses online trajectories under non-stationary excitation, which is not automatically the same problem as a static generalized-eigenproblem parameter sweep; therefore, compare algorithms first on identical already-computed modal inputs, then report any end-to-end application comparison separately. Use an analytical or independently generated oracle for synthetic cases and never treat two methods' agreement as independent proof when both share the same input or underlying solver defects.

## 8. Adaptive parameter-step control

Mode tracking should not be evaluated only at a uniformly spaced parameter grid. In a single-parameter Hermitian eigenproblem, mode ambiguity increases as eigenvalue gaps shrink and eigenvectors change rapidly; the 2026 Xiao, Sharif-Khodaei & Aliabadi preprint develops a gap-aware framework and adaptive sampling strategy for tracking through veering regions. Its assumptions and reported advantages must be checked before transferring the method to Symtropy's generalized structural models.

For the first adaptive experiment:

1. Propose a next parameter step from the previous accepted state.
2. Solve the candidate state and calculate the declared mode/subspace separation metrics, assignment margins, cluster changes, and eigen-residuals.
3. Accept a step only when the frozen confidence rule is satisfied; otherwise bisect or reduce the step.
4. Enforce declared minimum and maximum step sizes and a maximum solve budget.
5. At the minimum step or budget exhaustion, return an explicit ambiguous/unfinished disposition instead of forcing a match.
6. Save every accepted and rejected parameter point, including rejected-step reasons, so the path can be replayed and audit effort cannot disappear from the cost metric.

Compare adaptive sampling against uniform sampling and fixed-step continuation on identical parameter paths and error tolerances. Report eigensolve count, wall time, peak memory, maximum subspace-angle error, false-match rate, ambiguity rate, and convergence/failure rate. The primary efficiency claim should be fewer solves **at the same correspondence quality**, not merely fewer sampled points.

Adversarial cases must include a narrow veering region between coarse sample points, exact crossings with different symmetry behavior, repeated eigenvalues, a cluster that only appears over a small parameter interval, noisy candidate metrics, and a failure just below the minimum step. Also test whether threshold/step controls were tuned against the held-out path; adaptive efficiency must be evaluated prospectively.

The adaptive result is applicable only where the mathematical assumptions of the chosen tracker and parameterization are met. Do not generalize a theorem for Hermitian problems to arbitrary nonsymmetric, damped, nonlinear, or changing-constraint systems without a separate derivation and qualification.

## 9. Application order and claim ceiling

1. Qualify the synthetic oracle and named mutations.
2. Add cross-mesh comparison only after the physical meaning of the mapping is independently reviewed.
3. Apply the tracker to existing axial-bar and NBC05 fixtures without expanding their claim boundaries.
4. Apply it to continuous beam sweeps only after the BE25 mass-input discrepancy is resolved from authoritative input evidence.
5. Compare compatible modes/subspaces across refinements and parameter sweeps, recording explicit model-class and DOF assumptions.
6. Pursue experimental modal tracking in a separate validation program with sensor, fixture, environmental and repeatability uncertainty.

This document defines a proposed tracking qualification protocol. Passing it would support claims about mode correspondence on the declared fixtures and mappings only. It would not establish a complete NAFEMS/SOFiSTiK reproduction, physical validation, structural safety, code compliance, or state-of-the-art superiority.

## 10. References

- Lu et al. (2020), *A mode tracking method in modal metamodeling for structures with clustered eigenvalues*, Computer Methods in Applied Mechanics and Engineering. https://doi.org/10.1016/j.cma.2020.113174
- Ruiz, Bellido & Donoso (2017), *Eigenvector sensitivity when tracking modes with repeated eigenvalues*, Computer Methods in Applied Mechanics and Engineering. https://doi.org/10.1016/j.cma.2017.07.031
- Xiao, Sharif-Khodaei & Aliabadi (2026), *Rigorous foundations of adaptive mode tracking in single-parametric Hermitian eigenvalue problems*, arXiv:2603.13193. https://arxiv.org/abs/2603.13193
- Chen & Wang (2026), *Automated identification of time-varying modal parameter trajectories for structural systems*, Computers & Structures 330, article 108355. https://doi.org/10.1016/j.compstruc.2026.108355
- SLEPc, *EPS: Eigenvalue Problem Solver* documentation, including discussion of eigenvector accuracy and clustered spectra. https://slepc.upv.es/release/documentation/manual/eps.html
- SOFiSTiK, *Verification Benchmarks 2026*, BE25: Eigenvalue Analysis of Beam Structures Under Various Boundary Conditions. https://docs.sofistik.com/2026/en/verification/index.html
- NIST (2026-10-05), *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices*. https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing
