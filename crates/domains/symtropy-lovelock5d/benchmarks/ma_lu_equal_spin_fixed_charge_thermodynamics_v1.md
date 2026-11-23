# Ma–Lü equal-spin fixed-charge thermodynamics: Gibbs identity through spin tenth order

Canonical source: Ma and Lü, *Quadratic curvature correction to 5D Myers–Perry metric*, JHEP 05 (2026) 081, DOI 10.1007/JHEP05(2026)081, publisher version of record (published 8 May 2026), Eqs. (3.20)–(3.26).

## What is checked

This benchmark specializes the publisher's fixed-conserved-charge parameter redefinition and thermodynamic corrections to equal spins a=b, then tests the Gibbs Legendre relation independently of the source's stated first-law verification.

The equal-spin parameter shifts from Eq. (3.22) are:

- delta_mu = -8/3 + 94 chi^2/15 - 561 chi^4/70 + 2047 chi^6/315 - 37921 chi^8/4620 + 90107 chi^10/15015
- delta_chi = (chi/mu) * (4 - 47 chi^2/5 + 1683 chi^4/140 - 2047 chi^6/210 + 37921 chi^8/3080)

The publisher Eq. (3.23) entropy correction uses the prefactor 9 pi^2 sqrt(mu) / sqrt(2). This differs from a common text-extraction misread.

## Independent fixed-charge Gibbs check

At fixed M, J_a, J_b, the first-order shifts of those conserved charges vanish by construction. The Gibbs correction must therefore satisfy:

DeltaG = -S0*DeltaT - T0*DeltaS - J_a0*DeltaOmega_a - J_b0*DeltaOmega_b.

For equal spins with r_h=1 and a=b=q:
- mu = (1+q^2)^2/2
- chi = sqrt(2)*q/(1+q^2)
- T0 = (1-q^2)/(2*pi*(1+q^2))
- S0 = pi^2*(1+q^2)^2/2
- J_a0 = J_b0 = pi*mu*q/2

Substituting the published equal-spin corrections from Eqs. (3.23), (3.24), and (3.26), the exact series residual is zero through q^10. The first possible unchecked order is q^12. Both negative controls fail as expected: omitting one angular plane, and using the erroneous entropy prefactor 9 pi/(2 sqrt(2 mu)).

## Reproduction

Public script: verify_ma_lu_equal_spin_fixed_charge_thermodynamics.py

- Git blob SHA-1: 9fab7f906d066df8b9c1468521d95e0d25f9ea44
- SHA-256: 012bac1a59e38d7b09782a231e75aef2809a2f9849835ebd1c06d7b86072f5e5
- Python 3.13.5 / SymPy 1.14.0
- Exact public bytes fetched, hash-checked, and executed locally; all checks pass.

Claim ceiling: exact specialization of the published thermodynamic formulae and fixed-charge Gibbs identity. This does not independently rederive the complete differential-form first law from the metric, nor establish finite-coupling physics.
