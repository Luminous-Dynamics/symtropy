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
