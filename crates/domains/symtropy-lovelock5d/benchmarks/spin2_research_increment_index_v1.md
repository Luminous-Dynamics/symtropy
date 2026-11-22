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
