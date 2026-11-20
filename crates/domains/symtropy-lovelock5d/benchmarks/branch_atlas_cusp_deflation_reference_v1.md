# Executed branch-atlas controls: cusp geometry and deflated root discovery

**Owner:** [#6333 BH-5D-EGB-BRANCH-ATLAS-001](https://github.com/Luminous-Dynamics/symtropy/issues/6333)

**Status:** executed finite-dimensional method controls. These results do not classify the target 5D EGB black-hole solution set.

## Why this matters

AUTO-07p and BifurcationKit are mature references for pseudo-arclength continuation and bifurcation analysis. The next benchmark should test more than whether a corrector traverses a simple fold: it should distinguish folds from cusps, find multiple roots, reject duplicates, and re-qualify every candidate against the original equations after deflation.

## Deflated multiple-root control

The scalar control is (f(x)=x^3-x=0), with exact roots (-1,0,1). From deterministic seeds (2,-2,0.3), Newton iteration solves the deflated residual
[
\widetilde f(x)=f(x)\prod_{r\in R_{known}}(1+|x-r|^{-2}).
]
Each candidate is accepted only after checking the **original** residual (|f(x)|) and rejecting duplicates. The run recovered all three roots with maximum absolute error (2.29\times10^{-12}).

The finite seed budget is a method control, not a proof of general root or branch completeness.

## Cusp control

For (F(u;\mu,\nu)=u^3+\mu u+\nu), the simultaneous fold equations (F=F_u=0) give
[
\mu=-3u^2,\qquad \nu=2u^3,
]
and the exact discriminant (4\mu^3+27\nu^2=0). Five values of (u), including the cusp at the origin, pass the residual checks. A sign-mutated fold parameterization has original-equation residual (-0.5) and is rejected.

## Reproduction

Run the standard-library script in this directory:
`python branch_atlas_cusp_deflation_reference.py`
It emits the full machine-readable fixture. The committed summary fixture records the exact roots, tested cusp points and negative-control residuals.

## Production EGB gates

A candidate must additionally pass the original discretized action equations, radial constraint, horizon and asymptotic conditions, Noether/Bianchi defect, and independent physical admissibility checks. Deflation is a search mechanism, never a substitute for the original residual or an existence proof.

## Claim ceiling

Executed: scalar deflation, exact cubic cusp controls, and mutation rejection. Not established: completeness for arbitrary nonlinear systems, any EGB fold/cusp/disconnected component, or performance superiority over AUTO-07p or BifurcationKit on a matched EGB problem.
