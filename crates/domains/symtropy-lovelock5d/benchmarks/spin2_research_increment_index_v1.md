# Symtropy spin-2 research increment — 22 November 2026

This increment extends the open research branch `research/spin2-regularized-pivot-2026-11-21`. It is deliberately additive: it does not alter the original equations, weaken an acceptance gate, or promote an unverified result.

## New public artifacts

- `spin2_mp_frobenius_resonance_v1.md`: exact Einstein-limit spectral projectors, analytic-branch recurrence, singular-branch logarithmic obstruction, and a diagonal horizon-adapted state.
- `verify_spin2_mp_frobenius_resonance.py`: standard-library-only rational verifier. It checks the residue factorization, projectors, diagonalizing transformation, and source alignment.
- `spin2_mp_frobenius_resonance_v1.json`: machine-readable output.
- `spin2_mp_frobenius_resonance_replay_v1.json`: replay and negative-control receipt, with the source-provenance boundary recorded.
- `spin2_akribeia_matched_comparison_addendum_v1.md`: corrects the problem-class mismatch between Symtropy's current `O(a^2)` slow-rotation system and Akribeia's full nonlinear finite-spin profiles.
- `spin2_solution_atlas_contract_v1.md`: proposed node/edge/certificate model for a proof-carrying solution atlas.

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

The analytic Taylor recurrence has invertible `n I - A0` for every integer `n >= 1`; the exponent gap does not obstruct the analytic branch. The possible logarithm is in the excluded exponent `-1` homogeneous branch and is controlled by `P0 A1 u`. `A1` remains uncomputed; extracting `M2` and `C1` is the next exact gate.

## What this does not claim

No finite-coupling BVP, general-coupling lower-order residue, full-exterior determinant sign theorem, interval existence/uniqueness certificate, validated continuation segment, or strong-hyperbolicity result is claimed.