# Pöschl–Teller pole and causal-domain audit

This crate extracts the analytic known-answer layer from issue #4154 and the
published Pöschl–Teller decomposition in Arnaudo, Carballo & Withers,
arXiv:2510.18956v2 (29 October 2026), published as Phys. Rev. D 114, 024025
(2026), DOI 10.1103/1bcl-q79x.

## Frozen model and conventions

The dimensional barrier is

    V(x) = V0 sech²(x / b)
    λ = V0 b²
    ν² = 1/4 - λ

With time dependence exp(-i ω Δt), the homogeneous radial equation is

    [∂x² + ω² - V(x)] ψω(x) = 0

The QNM poles are

    ω_n^± = [-i(n + 1/2) ± sqrt(λ - 1/4)] / b, n = 0,1,...

The split Green-function terms have Matsubara poles

    ω_n(M) = i n / b, n ∈ Z, β = 2π b.

The Matsubara residues have opposite signs in the two split terms and cancel
in the full frequency-domain Green function. They are nevertheless required
for the correct regional time-domain representation. They are not extra QNM
poles.

The principal square root assigns the plus branch positive real frequency for
λ>1/4. For 0<λ<1/4, the plus branch is the less-damped purely imaginary
branch. At λ=1/4 the two displayed QNM branches coincide. This crate records
that frequency degeneracy only. It makes no exceptional-point, Jordan-chain
or resolvent claim.

## Causal regions

For a point source at x_s and observer at x_o, the direct and
potential-reflected null delays are

    t_direct = |x_o - x_s|
    t_reflected = |x_o + x_s|

Writing Δt=t-t', the regions are:

| Region | Condition | Representation |
|---|---|---|
| III | Δt < t_direct | zero retarded response |
| II | t_direct < Δt < t_reflected | Matsubara residue sum |
| I | Δt > t_direct and Δt > t_reflected | QNM residue sum |

The two equality surfaces are returned as explicit boundary statuses. Region II
can be empty for opposite-side source/observer pairs. The classifier accepts
an absolute boundary tolerance in c=1 units; it does not silently assign a
boundary sample to the interior of a convergence region.

## Scope

This crate validates formula transcription, dimensional scaling, barrier
symmetry and causal-region selection. It does not implement associated
Legendre functions, Wronskians, Green functions, residues, contour integrals,
the Matsubara/QNM coefficient sums or a time-domain PDE solver. Those remain
in #4109 and #4154.

Sources:
- Arnaudo, Carballo & Withers, Section II, Eqs. (8)-(37), Appendix B,
  Eqs. (B1)-(B17): https://arxiv.org/pdf/2510.18956
- Canonical physical integration task: https://github.com/Luminous-Dynamics/symtropy/issues/4154
