# Matched SOTA benchmark protocol: 5D equal-spin EGB black holes

**Purpose:** produce a fair, reproducible comparison between Symtropy's reduced DAE / horizon-series / spectral pipeline and published neural-field plus pseudo-spectral discovery methods. This is a frozen protocol, not a claim of numerical superiority.

## 1. Primary comparator and matched cases

The primary numerical-discovery comparator is Akribeia, which combines physics-informed neural-field discovery with pseudo-spectral refinement for higher-curvature rotating black holes. Its accepted Physical Review D paper reports five-dimensional EGB cases and a matched family with
\[
r_H=1,\qquad \Omega_H=0.33,\qquad \alpha\in[0,3],
\]
as well as additional parameter examples. Reproduce the paper's conventions, branch, boundary conditions, and definition of residual before comparing results.

Required cases (the paper-specific secondary point is omitted until its exact convention is frozen from the final source):

| Case | Parameters | Purpose |
|---|---|---|
| MP-0 | \(\alpha=0\) | Exact equal-spin Myers–Perry control |
| EGB-sweep | \(r_H=1,\ \Omega_H=0.33,\ \alpha\in\{0.1,0.7,2,3\}\) | Matched discovery/refinement sequence |
| Static-BD | \(\Omega_H=0\), Einstein branch | Exact Boulware–Deser control |
| Spin-2 | \(\lambda\in\{0.1,0.7,3,10\}\) | Slow-rotation perturbative BVP target |

Do not silently identify \(\alpha\) with \(\lambda\) when \(r_H\ne1\). Freeze the dimensional convention and map \(\lambda=\alpha/r_H^2\) explicitly.

The parameter values above define test cases; they do not assert that every point has a known regular solution.

## 2. Freeze the mathematical problem before comparing solvers

A matched comparison requires identical:

- action normalization and sign convention;
- spacetime dimension, equal-spin ansatz, and radial gauge;
- definition of \(r_H\), \(\Omega_H\), spin parameter, and Gauss–Bonnet coupling;
- horizon regularity conditions and asymptotic ensemble;
- radial compactification, collocation domain, and spectral basis;
- equation rows, constraint handling, and observable definitions;
- precision, hardware, thread count, memory cap, and timeout;
- training/tuning budget, initial guesses, continuation steps, and failed-run policy.

Run fixed-\((r_H,\Omega_H)\) and fixed-\((E,J)\) comparisons separately. Do not convert one ensemble's profile to another without recording the transformation and parameter-family zero mode.

Every solver gets the same information budget. A neural-field method must not receive an exact solution profile that is withheld from the spectral or continuation baseline. Conversely, the reduced DAE method may use the exact algebraic \(E_f\) pivot because it is derived from the equations, but all methods must be checked against the same original equations.

## 3. Compare methods, not brands

Run at least these pipelines:

1. **Akribeia-style baseline:** four-profile neural-field discovery followed by Chebyshev pseudo-spectral Gauss–Newton refinement.
2. **Classical spectral baseline:** direct Chebyshev collocation with horizon/asymptotic series boundary data and Newton/continuation.
3. **Reduced DAE method:** two differential profiles \((\widehat B_2,\widehat H_2)\), algebraic \(\widehat F_2\) reconstruction from the full scaled \(E_f\) row, and the same original-equation checks.
4. **Ablations:** remove horizon regularization; remove algebraic reconstruction; remove continuation; remove constraint residual; replace adaptive precision with binary64.
5. **Discovery extension:** pseudo-arclength continuation plus deflation for folds/disconnected components, compared against natural-parameter stepping.

The Akribeia-style baseline is a method reconstruction, not a claim to reproduce the authors' implementation unless their code and exact run configuration are available.

## 4. Metrics: do not let one residual norm hide a bad row

Report every equation separately and in aggregate:

- \(L^2\), weighted \(L^2\), and \(L^\infty\) residual norms for each original equation;
- maximum row residual and worst radial location;
- radial Noether-identity defect and \(E_g\) constraint residual;
- horizon compatibility mismatch and asymptotic boundary mismatch;
- spectral coefficient tail, resolution-to-resolution difference, and overlap-patch mismatch;
- mass, angular momentum, angular velocity, horizon area, surface gravity, and other declared invariant observables;
- thermodynamic first-law defect;
- Jacobian/inverse condition estimates and precision used;
- wall-clock time, CPU/GPU model, peak memory, energy if measurable;
- success, failure, timeout, and rejected candidate counts.

Reproduce the comparator's published residual definition for fidelity, but additionally report per-row and \(L^\infty\) metrics. A sum of several \(L^2\) residuals is not a substitute for local or per-equation error control.

For each metric, provide work-precision curves versus collocation order, number of training/update steps, precision, time, and memory. Publish all attempted cases, not only successful profiles.

## 5. Independent qualification leaderboard

Numerical refinement and mathematical qualification are separate leaderboards.

**Discovery/refinement:**
- time-to-first candidate;
- success rate across frozen initial guesses;
- time-to-target residual;
- work-precision and memory curves;
- branch/parameter coverage;
- missed/false event rate under continuation tests.

**Qualification:**
- exact controls and negative-control rejection;
- all original equations and Noether identity;
- gauge-invariant observable mismatch;
- interval-enclosed full-domain residual;
- spectral-tail and approximate-inverse bounds;
- validated boundary-map rank margin;
- radii-polynomial existence/local-uniqueness certificate;
- validated continuation-segment and event enclosure.

A candidate can lead the discovery leaderboard while remaining unqualified. Never collapse these categories into one score.

## 6. Preregistered acceptance tiers

- **B0 — exact controls:** exact MP and static BD recoveries; all deliberate mutations rejected.
- **B1 — matched reproduction:** reproduce published finite-coupling profiles and observables to the source precision under frozen conventions.
- **B2 — numerical refinement:** report per-equation \(L^\infty\), weighted \(L^2\), observable mismatch, spectral tails, and convergence across at least three resolutions. Initial targets: observable mismatch below \(10^{-7}\), then \(10^{-10}\), provided truncation and reference uncertainties are smaller.
- **B3 — robustness:** demonstrate stable results under independent gauges/charts, precision escalation, held-out radial nodes, and independent residual evaluators.
- **B4 — validated solution:** full-domain interval residual, tail, and inverse bounds.
- **B5 — existence:** first radii-polynomial or equivalent fixed-point certificate for a finite-coupling solution.
- **B6 — branch qualification:** validated continuation segments, fold/event classification, and branch-local uniqueness.

Do not set a universal raw residual threshold without scaling by equation units and conditioning. The target is controlled error in equations and observables, not cosmetic decimal depth.

## 7. Competitive strategy

Do not try to beat a neural/spectral method solely by producing more digits. The candidate differentiators are:

1. **Lower-dimensional discovery:** exploit the algebraic \(E_f\) row and radial Noether identity without removing any qualification check.
2. **Horizon-aware numerics:** derive Frobenius data and use overlapping horizon/interior/asymptotic patches with explicit match errors.
3. **Constraint-aware continuation:** use the radial identity and \(E_g\) residual as a constraint-drift detector.
4. **Validated answers:** enclose the full residual, spectral tail, and inverse operator; prove existence and local uniqueness near a computed profile.
5. **Branch topology:** pseudo-arclength and deflation, with physical zero-mode/transversality tests rather than visual profile interpretation.
6. **Physical qualification:** independently evaluate the full EGB principal symbol and strong hyperbolicity; see spin2_hyperbolicity_qualification_protocol_v1.md.

These are hypotheses to benchmark. No speed, accuracy, discovery, or proof advantage is pre-claimed.

## 8. Reproducibility contract

Every run receipt must contain:

- exact repository, commit, equation JSON digest, and source-generator digest;
- input parameter manifest and ensemble;
- solver method, basis, node count, initial guess, continuation/deflation settings;
- precision, hardware, software/library versions, and resource limits;
- per-row residuals, observable errors, constraint defects, tails, and condition estimates;
- full logs including warnings, failed solves, rejected candidates, and timeouts;
- independent verifier version and result;
- explicit claim tier B0–B6.

Freeze inputs and baseline configurations before inspecting comparative results. Do not tune one method on the final test cases and then report them as held out.

## 9. Sources and scope

- Akribeia, accepted Physical Review D paper (accepted 12 November 2026), DOI https://doi.org/10.1103/3kz2-6dvv. It is the primary neural-field/pseudo-spectral numerical baseline. Its reported residuals are scoped to its own equations, domain, norm, and precision.
- Lessard and Reinhardt, *Rigorous Numerics for Nonlinear Differential Equations Using Chebyshev Series*, SIAM Journal on Numerical Analysis (2014), DOI https://doi.org/10.1137/13090883X.
- AUTO-07p: https://auto-07p.github.io/ and BifurcationKit: https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/PALC/ for continuation/bifurcation method baselines.

The protocol does not report that Symtropy has reproduced Akribeia, beaten a benchmark, found a finite-coupling branch, or obtained a rigorous certificate.
