# Symtropy spin-2 research increment — 22 November 2026

This increment extends the open research branch `research/spin2-regularized-pivot-2026-11-21`. It is deliberately additive: it does not alter the original equations, weaken an acceptance gate, or promote an unverified result.

## New public artifacts

- `spin2_mp_frobenius_resonance_v1.md`: exact Einstein-limit spectral projectors, analytic-branch recurrence, singular-branch logarithmic obstruction, and a diagonal horizon-adapted state.
- `verify_spin2_mp_frobenius_resonance.py`: standard-library-only rational verifier. It checks the residue factorization, projectors, diagonalizing transformation, and source alignment.
- `spin2_mp_frobenius_resonance_v1.json`: machine-readable output.
- `spin2_mp_frobenius_resonance_replay_v1.json`: replay and negative-control receipt, with the source-provenance boundary recorded.
- `spin2_akribeia_matched_comparison_addendum_v1.md`: corrects the problem-class mismatch between Symtropy's current `O(a^2)` slow-rotation system and Akribeia's full nonlinear finite-spin profiles.
- `spin2_solution_atlas_contract_v1.md`: proposed node/edge/certificate model for a proof-carrying solution atlas.
- `spin2_mp_frobenius_A1_v1.md` and `spin2_mp_frobenius_A1_v1.json`: exact action-specialized Einstein-limit A1,g1, analytic first recurrence, and singular-branch logarithmic obstruction.
- `spin2_mp_frobenius_A2_v1.md` and `spin2_mp_frobenius_A2_v1.json`: exact A2,g2 and analytic recurrence through U2.
- `verify_spin2_mp_frobenius_A2.py`: exact-rational verifier for the committed A0,A1,A2 recurrence fixture and Myers–Perry controls.
- `spin2_mp_frobenius_A2_replay_v1.json`: exact Git-blob replay receipt for the committed fixture and verifier.
- `spin2_akribeia_source_audit_v1.md`: source-tag and run-configuration audit of the public Akribeia / DiffLearning / deterministic Gauss–Newton repositories.

## Replay

From the repository root, run:

```bash
python crates/domains/symtropy-lovelock5d/benchmarks/verify_spin2_mp_frobenius_resonance.py \
  --indicial crates/domains/symtropy-lovelock5d/benchmarks/spin2_mp_horizon_indicial_v1.json \
  --output /tmp/spin2_mp_frobenius_resonance_v1.json
```

The verifier requires only Python's standard library. Its result is exact rational linear algebra over the supplied `residue_A0` and `inhomogeneous_g0` fields. It does **not** rerun the action-level equation generator or certify the byte identity of the entire input JSON. The committed replay receipt records this boundary. CI was not run for this increment.

## Main exact consequence

For the Einstein-limit residue, `A0 = u w^T` with eigenvalues `0,0,0,-1`. The basis matrix

```text
T = [[1,0,0,0],
     [0,1,0,1],
     [0,0,1,0],
     [0,0,6,3]]
```

has determinant 3 and gives `T^-1 A0 T = diag(0,0,0,-1)`, `T^-1 g0 = (0,0,0,-8/3)`. The transformed fourth coordinate `Hhat2'/3 - 2*Hhat2` is fixed to `-8/3` at an analytic horizon. With `Bhat2(1)=0`, the two local data are `Bhat2'(1)` and `Hhat2(1)`.

The analytic Taylor recurrence has invertible `n I - A0` for every integer `n >= 1`; the exponent gap does not obstruct the analytic branch. The exact A1,g1 extraction gives the singular-branch obstruction `P0*A1*u = (1,2,3,18)`, so the excluded exponent -1 branch necessarily contains a logarithm. The exact A2,g2 extraction now supplies the analytic U2 coefficient. With Bhat2(1)=0, the regular branch is fixed by two local data b1=Bhat2'(1) and h0=Hhat2(1) through U0,U1,U2.

## What this does not claim

No finite-coupling BVP, general-coupling lower-order residue, full-exterior determinant sign theorem, interval existence/uniqueness certificate, validated continuation segment, or strong-hyperbolicity result is claimed.

## A1/A2 replay boundaries

- The A1 action-specialized derivation ran locally and reproduced its local JSON output byte-for-byte. Its committed public derivation source was subsequently repaired for a newline-literal issue, but exact-byte execution of that committed derivation source remains pending; its replay receipt distinguishes local source from public source.
- The A2 action-specialized derivation ran locally using exact SymPy arithmetic at lambda=0. The committed A2 fixture and committed A2 recurrence verifier were fetched and replayed using their exact Git blob bytes. The verifier passed analytic recurrences n=0,1,2, the nonzero logarithmic obstruction, and the Myers–Perry recurrences n=0,1,2. The action-specialized derivation source remains a recorded local source, not a committed source.
- Neither replay is a GitHub Actions/CI pass. The latest status query returned no status entries.

## Akribeia source audit

The released Akribeia paper references DiffLearning v2.1.1 and the deterministic Gauss–Newton refiner v1.1.1, while the Akribeia v1.0.0 README installation commands clone DiffLearning v2.1.0 and the refiner v1.0.0. This is a reproducibility ambiguity to resolve, not evidence of a defect. The README and paper configurations also distinguish a neural family over coupling from the 5D refiner orchestration's selected fixed-coupling run. See spin2_akribeia_source_audit_v1.md.

## Current research frontier

The next mathematical bottleneck remains the finite-coupling lower-order horizon residue. A direct lambda=1/2 action-specialized experiment exposed removable horizon 0/0 terms and did not yield a qualified residue; no finite-coupling residue is claimed. Next work should reuse the exact scaled-row and algebraic-pivot reduction, specialize rational coupling before coefficient expansion, and calculate only the horizon jets needed for C0,s0 before considering A1(lambda).


## A3 Einstein-limit horizon-series extension (22 November 2026)

New files:
- `derive_spin2_mp_horizon_frobenius_A3.py`: direct alpha=0 action-specialized derivation; five radial fields are varied before imposing the radial gauge.
- `spin2_mp_frobenius_A3_v1.json` and `spin2_mp_frobenius_A3_v1.md`: exact A0..A3, g0..g3 and analytic U0..U3.
- `verify_spin2_mp_frobenius_A3.py`: exact-rational recurrence and Myers–Perry verifier.
- `spin2_mp_frobenius_A3_replay_v1.json`: source, fixture, verifier, and replay boundaries.

The new coefficients are
`A3=[[0,0,0,0],[0,-3,-97/4,-43/8],[0,0,0,0],[0,0,-51/4,23/8]]`,
`g3=(0,2849/3,0,209)`.
For the analytic branch, with b1=Bhat2'(1), h0=Hhat2(1):
`U3=(2*b1-44, 444-10*b1, 8*h0-12, 8*h0+12)`.
The same two local horizon data remain free. The Myers–Perry U3 vector is (-52,484,-4,20). The exact public A3 fixture and exact public verifier blobs were replayed locally; all recurrences n=0..3 passed. The action-specialized derivation source was executed locally, while exact-byte execution of its committed public copy is still pending.

## SOTA and reproduction additions

- `spin2_sota_benchmark_scorecard_v1.md`: separates discovery/refinement, mathematical qualification, and PDE hyperbolicity; defines proposed metrics such as validated branch yield, qualification conversion, proof latency, proof debt, and claim coverage.
- `spin2_akribeia_reproduction_manifest_v1.json`: records the paper/README dependency-pin discrepancy, inspected code-file blob identities, the observed 5D configuration, and the conditions needed for a source-faithful reproduction.

The source audit is not a completed Akribeia run. The local environment has no detected Wolfram Engine, and no Akribeia training or spectral refinement is claimed.

## Remaining frontier

The finite-coupling lower-order residue is still open. The lambda=1/2 specialization attempt encountered removable horizon 0/0 expressions and did not yield a qualified result; this is a symbolic-computation bottleneck, not evidence of mathematical singularity. Priorities remain rational-coupling jet extraction, infinity modes, global boundary-map rank/Fredholm index, finite-coupling BVP, and interval/continuation qualification.


## Einstein-limit infinity indicial anchor (22 November 2026)

The A3 action-specialized reduction was extended to logarithmic radius `tau=log(r)) and state `V=(Bhat2,r*Bhat2_prime,Hhat2,r*Hhat2_prime)`. The frozen-coefficient matrix is
`[[0,1,0,0],[0,-2,0,0],[0,0,0,1],[0,0,8,2]]`,
with zero leading source and polynomial `sigma*(sigma-4)*(sigma+2)^2`. Its modes are two independent (r^{-2}) decays, one constant, and one (r^4) growing mode. The repeated (-2) eigenvalue is semisimple at leading order; subleading asymptotic logarithms remain open.

Artifacts:
- `spin2_mp_infinity_indicial_v1.md/.json`
- `verify_spin2_mp_infinity_indicial.py`
- `spin2_mp_infinity_indicial_replay_v1.json`

The committed fixture and verifier were replayed from exact Git blob bytes. This is only a (lambda=0) frozen-coefficient endpoint anchor, not the full asymptotic recurrence, boundary-data map or Fredholm index.

## Updated research index: SOTA scorecard and comparator manifest

The public scorecard and machine-readable reproduction manifest are:
- `spin2_sota_benchmark_scorecard_v1.md`: separates discovery, refinement, mathematical qualification and PDE hyperbolicity; proposes validated branch yield, qualification conversion, proof latency and claim coverage.
- `spin2_akribeia_reproduction_manifest_v1.json`: preserves the Akribeia/DiffLearning/refiner pin discrepancy, observed 5D settings and uncompleted reproduction gates.


## Einstein-limit infinity resonance classification (22 November 2026)

A further endpoint calculation analyzes the first nontrivial asymptotic recurrence at (x=1/r). The (x^2) resonance operator (-2I-A_{infty,0}) has rank two and nullity two. Its compatibility condition is exact:
`A_{infty,2} e_B + g_{infty,2}=0`.
The constant bounded mode therefore does not force a logarithm at this order. The nullspace supplies the two independent (r^{-2}) decay coefficients. After removing the constant normalization mode and the growing (r^4) mode, the bounded asymptotic germ has two free decay coefficients.

An exact rational verifier and receipt are committed:
- `spin2_mp_infinity_resonance_v1.md/.json`
- `verify_spin2_mp_infinity_resonance.py`
- `spin2_mp_infinity_resonance_replay_v1.json`

The first verifier attempt caught and corrected a sign mistake in the n=1 determinant; the exact value is (+5). The corrected public fixture and verifier blobs were both replayed locally and passed. This balanced (2)-horizon-data versus (2)-decay-data count motivates a (2\times2) global boundary map, but does not establish its rank, uniqueness or Fredholm index.


## Exact Einstein-limit global boundary map (22 November 2026)

The `spin2_einstein_global_boundary_map` increment closes the horizon-to-infinity matching problem at `alpha=0`, `r_H=1), and (O(a^2)).

The independent five-field Euler–Lagrange variation is re-derived, with (g=r^2) imposed only after variation. The exact reduced principal matrix has
`det M=(r^2-1)/4>0` for every (r>1), and the algebraic (E_f) pivot is `3*r^5/(r^2-1)^2>0). The complete analytic horizon family is given in `spin2_einstein_global_boundary_map_v1.md); substitution verifies all four original even-sector rows exactly.

The global forbidden-mode map is
[
(C_B,C_H)^T=operatorname{diag}(1/2,2)(b_1+4,h_0-1)^T,
]
where (C_B=lim B_2) and (C_H=lim H_2/r^4). Its determinant is (1), singular values are (1/2,2), and Euclidean condition number is (4). The additional asymptotic growth satisfies (C_F=lim F_2/r^2=-C_H), so it is not an independent third boundary condition. Asymptotic flatness uniquely selects ((b_1,h_0)=(-4,1)), recovering Myers–Perry.

Artifacts:
- `derive_spin2_einstein_global_boundary_map.py`: action-first exact derivation and all-four-row verification.
- `spin2_einstein_global_boundary_map_v1.json`: machine-readable exact reduced system, solution family, map and claim ceiling.
- `verify_spin2_einstein_global_boundary_map.py`: independent exact-rational reduced-system verifier with a wrong-sign (H_2) negative control.
- `spin2_einstein_global_boundary_map_v1.md`: global completeness and uniqueness argument.
- `spin2_einstein_global_boundary_map_replay_v1.json`: local replay receipt and exact public Git blob identifiers.

The local derivation and independent verifier passed. The public source/fixture/verifier blob identities are recorded; execution of the exact committed derivation blob and CI remain separate gates. This is not a finite-coupling or full nonlinear uniqueness result.

## Next frontier: first coupling derivative of the global map

The next mathematical increment is the derivative of the boundary map with respect to (lambda=alpha/r_H^2) at (lambda=0). This should reveal whether the diagonal Einstein-limit map deforms smoothly and quantify coupling-induced mixing between the horizon data and forbidden asymptotic amplitudes. The finite-(lambda) horizon residue and interval BVP remain subsequent gates.


## First coupling derivative of the asymptotically flat branch (22 November 2026)

At λ=α/r_H^2=0, the first exact coupling derivative of the five-dimensional O(a^2) EGB branch has been derived from the reduced action. With the MP solution at zeroth order, the corrections are `B21=(r^2-1)*(11*r^6+11*r^4+22*r^2+9)/(9*r^10)`, `F21=(r^2-1)*(22*r^4-11*r^2-35)/(18*r^8)`, and `H21=-(11*r^4+14*r^2+3)/(6*r^6)`. All four original order-λ even-sector equations `Eb,Ef,Eg,Eh` vanish exactly. The selected horizon data shift as `b1(lambda)=-4+(106/9)*lambda+O(lambda^2)`, `h0(lambda)=1-(14/3)*lambda+O(lambda^2)`. The leading asymptotic corrections remain decaying: `r^2 B21 -> 11/9`, `r^2 F21 -> 11/9`, `r^2 H21 -> -11/6`.

Artifacts: `derive_spin2_global_branch_lambda1.py`, `spin2_global_branch_lambda1_v1.json`, `verify_spin2_global_branch_lambda1.py`, `spin2_global_branch_lambda1_v1.md`, and replay receipt to follow. This is the derivative of the asymptotically flat branch, not the derivative of the full boundary-map Jacobian and not a proof of finite-λ analyticity or interval existence.


## First coupling derivative: replay receipt and sensitivity continuation contract

The first-coupling branch increment is now accompanied by `spin2_global_branch_lambda1_replay_v1.json`. The local exact action extraction gives zero in all four original order-λ rows, and the exact-rational equation-row replay rejects a wrong-sign (H_{21}) mutation. The derivation source, fixture and verifier Git blob identities are recorded separately; exact execution of the committed source blob and CI remain pending.

The new `spin2_coupling_sensitivity_continuation_contract_v1.md` specifies how the solution tangent, endpoint derivatives, observables, boundary-map conditioning and interval tubes should be propagated along a finite-coupling branch. The present result supplies only the tangent of the asymptotically flat root; it does not yet supply the derivative of both global-map columns.

## Updated state-of-the-art landscape (22 November 2026)

New comparison artifacts:
- `spin2_sota_landscape_2026-11-22.md`: source-grounded comparison of Akribeia, arbitrary-spin spectral metrics by Lam–Chung–Yunes, and the generic-spin analytic metric-reconstruction programme by Li–Hu–Yunes.
- `spin2_sota_reference_metrics_v1.json`: structured reference metrics and explicit problem-class caveats.

The updated strategy separates (i) 5D/7D full nonlinear Lovelock, (ii) 4D Kerr at leading EFT coupling with generic spin, and (iii) the current 5D (O(a^2)) perturbative global map/sensitivity. It adds a sensitivity-carrying, proof-producing continuation target rather than one residual-only leaderboard. No cross-track superiority claim is made.


## Boundary-map coordinate invariance and metric-aware conditioning (22 November 2026)

A new exact-rational metamorphic qualification found that raw Euclidean conditioning is basis-dependent even when the physical boundary map is unchanged. Under nonsingular horizon and asymptotic coordinate changes, \(\widetilde J=RJS^{-1}\): rank/nullity are invariant, while determinant magnitude and Euclidean singular values are not. For the current \(J_0=\operatorname{diag}(1/2,2)\), output rescaling alone produces condition number 1 or 16 instead of 4 without changing the solution problem.

New artifacts:
- \`spin2_boundary_map_normalization_v1.md\`: transformation law, weighted/generalized singular values, coupling-dependent basis derivative, and reporting contract.
- \`verify_spin2_boundary_map_normalization.py\`: standard-library-only exact-rational metamorphic verifier.
- \`spin2_boundary_map_normalization_replay_v1.json\`: exact public Git-blob replay receipt.

The verifier was fetched by public Git blob SHA and executed locally byte-for-byte. It passed determinant covariance, the full first-order basis-transport derivative, and generalized singular-value invariance when the domain/range metrics are transformed with the coordinate bases. It also demonstrated that the raw Euclidean condition number can change from 4 to 1 or 16 by output scaling alone.

This does not invalidate the exact Einstein-limit full-rank result. It narrows interpretation: \(\kappa_2=4\) is the correct value in the published raw coordinates, not an invariant physical conditioning value. Future finite-coupling continuation must record the raw map, freeze declared endpoint metrics before a parameter sweep, and report metric-aware conditioning separately from rank/invertibility.

Claim ceiling: coordinate-covariance identities only; no calculation of the missing \(J_1\), finite-coupling BVP, interval existence theorem, or hyperbolicity result is established by this increment.


## Boundary-map normalization and observable-overlap branch controls (22 November 2026)

Two exact/metamorphic controls extend the branch qualification architecture.

### Coordinate-aware boundary-map conditioning

New artifacts:
- \`spin2_boundary_map_normalization_v1.md\`
- \`verify_spin2_boundary_map_normalization.py\`
- \`spin2_boundary_map_normalization_replay_v1.json\`

The exact-rational verifier was fetched and executed from the exact public Git blob. It passed coordinate covariance for the determinant, the first-order transformation law for coupling-dependent endpoint bases, and invariance of generalized singular-value data when domain/range metrics are transformed consistently. It demonstrates raw Euclidean condition numbers 4, 1 and 16 for different endpoint-amplitude normalizations of the same physical \(J_0\).

The consequence is methodological: the published \(\kappa_2=4\) remains correct in its basis, but physical loss of invertibility must be tracked through rank/nullity and conditioning in frozen, declared endpoint metrics. This does not supply the missing finite-coupling \(J_1\).

### Observable self-overlap versus a true fold

New artifacts:
- \`branch_atlas_projection_overlap_reference.py\`
- \`branch_atlas_projection_overlap_fixture_v1.json\`
- \`branch_atlas_projection_overlap_replay_v1.json\`

The exact public verifier blob (Git SHA-1 \`14b3981b1092b93a7bf1c0d54349d21cea471c14\`) was replayed locally. The synthetic branch \(U(\lambda)=(\lambda,\lambda^2)\) has the same scalar observable at \(\lambda=\pm1/2\), while its two full states are different, the original residuals vanish exactly, and the full state Jacobian remains the identity. The distinct \(u^2-\lambda=0\) control exhibits a true simple parameter fold.

This encodes a critical Akribeia-inspired distinction: repeated values of a metric profile component do not establish a fold, bifurcation or disconnected black-hole solution. Those require full-solution Jacobian, tangent and transversality evidence. The target 5D EGB branch has not yet been reproduced or classified.

Related new tasks:
- #6831 — coordinate-aware boundary-map conditioning from the reduced action.
- #6833 — distinguish observable crossings from true solution-branch folds.


## Complete first coupling derivative of both boundary-map columns (22 November 2026)

The first coupling derivative of the entire \(2\times2\) horizon-to-infinity forbidden-mode map is now derived, rather than only the tangent of the asymptotically flat physical root.

In the fixed Einstein-limit horizon and asymptotic coordinates,
\[
\boxed{
J_1=
\begin{pmatrix}
1/2&19-2\pi^2\\
0&67-6\pi^2
\end{pmatrix}.
}
\]
Numerically the second column is \((-0.739208802178717238,\;7.78237359346384829)^T\). The exact derivative of the affine map offset remains \((-53/9,28/3)^T\), independently fixed by the physical branch tangent. The determinant slope is \(69/2-3\pi^2\).

The derivation varies all five radial fields before imposing \(g=r^2\), extracts the four original even-sector \(O(\lambda a^2)\) rows, reconstructs \(F_{21}\) through the positive algebraic \(E_f\) pivot, and reduces the other two evolution rows. The \(b_1\) homogeneous sensitivity is elementary. The \(h_0\) sensitivity reduces to a scalar self-adjoint ODE; a normalized Green--Lagrange projection evaluates its growing amplitude through a log/dilogarithm primitive, with no asymptotic numerical fit.

New public artifacts:
- \`extract_spin2_boundary_map_lambda1_rows.py\`: exact action-row extractor.
- \`spin2_boundary_map_lambda1_action_rows_v1.json\`: all four original coefficient rows and the \(E_f\) reconstruction.
- \`verify_spin2_boundary_map_lambda1.py\`: independent endpoint projection/verifier.
- \`spin2_boundary_map_lambda1_v1.json\`: exact and numerical \(J_1\), determinant and conditioning slopes.
- \`spin2_boundary_map_lambda1_derivation_v1.md\`: derivation and claim ceiling.
- \`spin2_boundary_map_lambda1_replay_v1.json\`: source blob hashes, replay environment, fixtures and evidence boundaries.

The two source Git blob SHA-1 values match the exact locally executed source bytes. The scripts ran locally using Python 3.13.5 / SymPy 1.14.0. The coefficient-row fixture is committed in normalized JSON formatting; the published row expressions are the same expressions emitted by the extractor. This is a local exact-source replay, not a CI pass.

This closes the first derivative of both columns at \(\lambda=0\). The next gates are finite-difference convergence against independently corrected small-positive-\(\lambda\) BVPs, finite-coupling invertibility under frozen endpoint metrics, a finite-coupling BVP, and interval-qualified continuation. No finite-coupling existence or hyperbolicity claim is made.


## Endpoint singular-frame sensitivity (22 November 2026)

The full \(J_1\) also yields exact first-order rotations of the left/right singular directions in the frozen raw Einstein-limit endpoint basis. The least-constrained right direction rotates with component slope \(0.0985611736238288\), and the least-constrained left direction with \(0.3942446944953152\). Complementary maximum directions rotate oppositely. These raw-coordinate slopes are not basis invariant; the continuation benchmark should compare them only under the same endpoint normalization or after consistently transporting declared metrics.

The derivation and machine-readable result now include these values as additional finite-difference regression targets.


## Independent numerical oracle for the Green projection (22 November 2026)

A separate arbitrary-precision numerical quadrature now checks the non-elementary endpoint projection without importing SymPy, the action-row fixture, or the symbolic Green--Lagrange verifier.

Using mpmath 1.3.0 / Python 3.13.5, a tanh-sinh quadrature after \(s=1-t^2\) endpoint regularization independently reproduces:
\[
C_H=67-6\pi^2,\qquad C_B=19-2\pi^2.
\]
At 40, 60 and 80 requested decimal digits, the largest reported discrepancy is \(1.44\times10^{-46}\) for \(C_H\) and \(4.80\times10^{-47}\) for \(C_B\). These are numerical discrepancies against the closed form, not interval enclosures.

Artifacts:
- \`verify_spin2_green_projection_quadrature.py\`: separate numerical oracle; no SymPy or equation fixture imports.
- \`spin2_green_projection_quadrature_v1.json\`: precision sweep and numerical outputs.
- \`spin2_green_projection_quadrature_v1.md\`: integral transformation and reproduction contract.
- \`spin2_green_projection_quadrature_replay_v1.json\`: exact public Git source blob SHA and local replay receipt.

The public source blob SHA-1 is \`1c87297c407ed288ef521b685985ba9352370b07\`; its SHA-256 was independently recomputed locally as \`3d1d30e290c3bffa410075fbce84659081be2e3073225c24490f584fc297d88c\`, matching the source digest embedded in the numerical fixture. This is a local replay, not a GitHub Actions pass.

This reduces risk in the dilogarithmic endpoint calculation. It does not verify the action-row source or establish finite-\(\lambda\) BVP existence.


## Second solver backend: 4D Kerr source-to-metric reconstruction (22 November 2026)

Opened [#6891 BH-4D-KERR-SOURCE-RECONSTRUCTION-001](https://github.com/Luminous-Dynamics/symtropy/issues/6891) to reproduce a distinct perturbative 4D route from Li, Hu & Yunes, arXiv:2610.37515 (27 October 2026). The target is the composition:
\[
\text{conserved EFT source}
\rightarrow \text{stationary Teukolsky curvature}
\rightarrow \text{universal Newman--Penrose reconstruction}
\rightarrow \text{independent linearized Einstein-equation verifier}.
\]

This is separate from nonlinear 5D equal-spin EGB and only applies at the stated leading order around Kerr. The new task freezes conservation, tetrad, reflection, horizon/asymptotic charge completion and no-mode-decomposition controls. It is a portfolio expansion, not evidence that the 5D task is complete.


## Current SOTA audit: Akribeia, Ma--Lü and source reconstruction (22 November 2026)

The SOTA landscape and scorecard now record the exact metric classes separately:
- Akribeia 5D EGB family: \(r_H=1,\Omega_H=0.33,\alpha\in[0,3]\) at 0.01 steps, with a separate \((\alpha,\Omega_H)=(2,0.68)\) comparison. Its random-initialized PINN plus pseudo-spectral Gauss--Newton method is a direct nonlinear EGB comparator. The \(10^{-85}\) MP relative \(L^2\) profile error and \(10^{-80}\) 7D cubic-Lovelock refined residual are distinct controls, not the same 5D EGB metric.
- Ma--Lü 5D MP quadratic-curvature correction: linear coupling, two-spin series through tenth order, thermodynamic cross-check. Issue #6915 audits the candidate Riemann-squared / Gauss--Bonnet mapping, action coefficient, equal-spin limit and gauge before comparing coefficients.
- Li--Hu--Yunes 4D Kerr source reconstruction: six interactions at leading coupling, no mode decomposition, sampled \(\Psi_0\) relative discrepancy below \(1.4\times10^{-22}\) at \(a/M=0.9999\) (five angles, 200 radii, reference \(\ell_{\max}=100\)); separate exact thermodynamic checks. This is not a full-domain error or runtime bound.

The programme now scores discovery, branch coverage, matched equations and boundary conditions, equation rows, held-out error, observables, physical qualification and validated existence separately. No cross-category residual ranking is allowed.


## Independent second quadrature: endpoint-log-subtracted composite Gauss--Legendre (22 November 2026)

A second implementation of the Green endpoint integral was written and executed locally, using composite Gauss--Legendre quadrature rather than tanh--sinh. It analytically subtracts the first thirteen odd-power \(\log t\) endpoint terms after \(s=1-t^2\), integrates the regularized remainder with four panels of 24 Gauss--Legendre nodes each, then restores the subtracted integrals exactly.

The independent result again reproduces
\[
C_H=67-6\pi^2,\qquad C_B=19-2\pi^2.
\]
At 40 digits the discrepancies are \(1.86\times10^{-35}\) and \(6.21\times10^{-36}\); at 60--80 digits they are approximately \(1.05\times10^{-49}\) and \(3.49\times10^{-50}\).

The exact local script was run with Python 3.13.5 / mpmath 1.3.0 before publication. Its local Git blob SHA-1 7e454db8297d14c1f86d196b3e4a1ef2648e6ae7 matches the fetched public branch file exactly. Artifacts:
- verify_spin2_green_projection_gauss_legendre.py
- spin2_green_projection_gauss_legendre_v1.json
- spin2_green_projection_gauss_legendre_v1.md
- spin2_green_projection_gauss_legendre_replay_v1.json

This is a second numerical quadrature family. It is not an interval enclosure, action-row replay or finite-\(\lambda\) BVP result.


## Ma–Lü source pin and exact first-coupling overlap (23 November 2026)

### Version of record and convention

Canonical source: Ma and Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081, publisher version of record published 8 May 2026. The version-of-record Eqs. (2.8) and (2.9) explicitly include the angular-axis factors \(x^2d\psi^2\), \((1-x^2)d\phi^2\), and define
\[
\chi_a=a/\sqrt{\mu},\qquad \chi_b=b/\sqrt{\mu}.
\]
This supersedes the incorrect \(a/\mu\) convention extracted from the earlier arXiv-v1 text. For equal spins and \(q=a/r_h\),
\[
\mu/r_h^2=(1+q^2)^2/2,\quad \nu_h=\Omega_Hr_h=q/(1+q^2),\quad \chi=\sqrt2\,\nu_h.
\]
The q(\(\nu_h\)) series is checked through \(\nu_h^{11}\), and the equal-spin Eq. (3.9) mass-parameter expansion through \(\chi^6\).

### First-coupling source identity and parameter conversion

The Ricci-flat MP Euler-tensor identity gives \(\alpha_R=\alpha_{GB}/4\) at first coupling only. This does not imply a generic finite-coupling equivalence. The fixed-Symtropy map separately distinguishes \(\nu_h=\Omega_Hr_h\) and \(\nu_S=\Omega_HR_H\). Its verified first-coupling expansions include
\[
R_H/r_h=1+5\epsilon_S/3+\nu_S^2/2+277\epsilon_S\nu_S^2/90+\cdots,
\]
\[
q=\nu_S+\nu_S^3/2+\epsilon_S(7\nu_S-3619\nu_S^3/90)+\cdots,
\]
\[
a/R_H=\nu_S+16\epsilon_S\nu_S/3-2068\epsilon_S\nu_S^3/45+\cdots,
\]
\[
\mu/R_H^2=1/2-5\epsilon_S/3+\nu_S^2/2+454\epsilon_S\nu_S^2/45+\cdots,
\]
\[
\chi=\sqrt2(\nu_S-\nu_S^3/2+7\epsilon_S\nu_S-5509\epsilon_S\nu_S^3/90)+\cdots.
\]
Here \(\epsilon_S=\alpha_R/R_H^2=\lambda_{GB}/4\); the converted \(\mu/R_H^2\) is a background MP parameter, not the corrected ADM mass.

### Full angular block

With \(A=r^2+a^2\), \(D=2\mu a^2/A\), \(G=A(1+\alpha_RH_7)\), and \(\eta=(1-x^2)d\phi+x^2d\psi\), the version-of-record H5/H6 functions reduce to
\[
H_5=H_7-\frac{4a^2x^2}{45}S(r),\qquad
H_6=H_7-\frac{4a^2(1-x^2)}{45}S(r),
\]
\[
S(r)=165/r^4+330\mu/r^6+150\mu^2/r^8.
\]
Direct substitution into all three angular components of Eq. (2.8), including the axis factors, yields
\[
ds^2_{\rm ang}=G[d\theta^2+(1-x^2)d\phi^2+x^2d\psi^2]
+D(1+\alpha_RH_4)\eta^2+O(\alpha_R^2,a^4).
\]
Independent exact SymPy checks pass for dpsi², dphi², dphi dpsi, axis regularity, and negative controls.

### Original Symtropy field rows and ADM mass

On the MP-connected branch, \(c_B=c_H=0\) are the zeroth-order horizon-mode offsets. The fixed-boundary \(B_{21},F_{21},H_{21}\) profiles give exact rational zero in all four original action rows \(E_b,E_f,E_g,E_h\); the positive algebraic \(E_f\) pivot independently reconstructs \(F_{21}\). The sensitivity outputs \(B'_{21}(1)=106/9\), \(H_{21}(1)=-14/3\) are not cB/cH inputs.

Ma–Lü Eq. (3.4) is independently matched at equal spin through \(\chi^2\): \(H_{1,-2}=16/3-188\chi^2/15\), so the 5D Komar prefactor \(3\pi/8\) gives \(\Delta M=2\pi-47\pi\chi^2/10+O(\chi^4)\), equal to the paper.

### Reproduction and claim boundary

Exact public-source local replays:
- equal-spin kinematic/parameter script: Git blob SHA-1 30616c178a051f0a2fd120aba8a9d03dfc8c865e;
- fixed-boundary map: Git blob SHA-1 f6b1d5eb9b084403fc1935d84d419c33c3e1a0b9;
- asymptotic mass check: Git blob SHA-1 32d91014925518358ad92395ca6aec020d383d6d.

The full angular/fixed-boundary and ADM-mass checks now pass in the consolidated verifier replayed byte-for-byte from its public Git blob (SHA-1 b1f9c4a2802cca0369c9eb036dfd6fd3c55e3bfd; SHA-256 e39cd8f5257acdf291bf8b7ea056767b35c9ab67a4f7963e977d5a867634650f). The four original rows have exact rational zero and the fixture-driven row verifier has been replayed byte-for-byte with its exact public fixture; the angular block and fixed-boundary checks are included in the consolidated VOR verifier. The new spin-10 charge script independently replays Ma-Lü Eqs. (3.4) and (3.9) through chi^10.

Still open: finite-positive-coupling corrected BVPs, finite-difference convergence of the full boundary map, interval existence/local uniqueness on a branch segment, and separate physical-principal-symbol/strong-hyperbolicity qualification. These first-coupling results are not finite-coupling existence, global uniqueness, or hyperbolicity results.


## Ma–Lü equal-spin thermodynamic charges through tenth order

Added exact-source verifier: verify_ma_lu_equal_spin_thermodynamic_charge_series.py (Git blob SHA-1 e75dcd96941dadbc1f5f0940521d959060dbc776; SHA-256 038f21d147e07da45e1e3bb68e73b9bba8b3f5e102b2e05e250a068b09e31242). Exact execution under Python 3.13.5 / SymPy 1.14.0 passes:
- Eq. (3.9) \(\mu/r_h^2\) equal-spin coefficients through \(\chi^{10}\);
- Eq. (3.4) \(\Delta M\) through \(\chi^{10}\);
- re-expansion in the GR fixed-boundary spin \(\nu_S\) through \(\nu_S^{10}\);
- independent \(H_1\)/Komar metric-mass agreement through \(\chi^2\);
- total fixed-boundary \(M/R_H^2\) through \(O(\epsilon_S\nu_S^2)\);
- each fixed-boundary \(J/R_H^3\) through \(O(\epsilon_S\nu_S^3)\).

Do not claim total fixed-boundary mass to \(\nu_S^{10}\): the high-order fixed-boundary \(H_2/H_4/H_7\) reparameterization remains open. The \(\nu_S^{10}\) statement applies to the standalone \(\alpha_R\Delta M\) observable after the zero-coupling spin conversion.

The benchmark landscape and scorecard now distinguish Ma–Lü's tenth-order perturbative spin series from Symtropy's current equal-spin metric/row match through \(O(\chi^2)\). Current evidence demonstrates an independent reproduction/qualification step, not a metric-order SOTA win.
