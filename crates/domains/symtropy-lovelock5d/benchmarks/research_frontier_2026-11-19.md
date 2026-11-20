# 5D rotating black-hole research frontier — 2026-11-19

This note updates the 5D equal-spin EGB programme against the late-2026 literature. It separates direct theory comparators from methodological and downstream-observable comparators. No cross-theory performance claim is valid without a matched equation set, dimension, symmetry, gauge, boundary data, precision and observable definition.

## 1. Competitive landscape

| Work | What it demonstrates | Relevance to this crate | What must not be claimed |
|---|---|---|---|
| Brihaye et al. (2010), JHEP 11 089, arXiv:1010.0860 | Equal-magnitude angular momenta in 5D EGB; cohomogeneity-one ODEs; Newton–Raphson BVP with adaptive mesh, generally 100–1,000 points and typical reported relative accuracy around \(10^{-6}\); selected cross-gauge checks | Direct classical reference for the same theory and symmetry | Their reported accuracy is not automatically the same norm as a field residual or a modern spectral error |
| Akribeia, Agurto-Sepúlveda et al., PRD accepted 12 Nov 2026, DOI 10.1103/3kz2-6dvv | Physics-informed neural fields followed by pseudo-spectral refinement; rotating higher-curvature solutions, including 5D equal-spin EGB and 7D cubic Lovelock, with multiple nonzero angular momenta | Closest modern discovery/solver comparator and the main matched target for a 5D EGB reproduction | Its extreme-precision residual statements and its published Myers–Perry GR-limit relative \(L^2\) solution-difference result are distinct metrics. Do not relabel a GR-limit solution comparison as a generic finite-coupling EGB residual |
| Li, Hu & Yunes, arXiv:2610.37515 (27 Oct 2026) | Unified construction of stationary axisymmetric Kerr deformations from tidal curvature and physical sources; closed forms for six higher-curvature interactions at leading coupling and generic subextremal spin without slow-spin expansion, numerical fitting or mode decomposition | A major conceptual comparator: source-to-curvature-to-metric reconstruction can complement radial ODE discovery and may inspire invariant reconstruction modules | It is 4D Kerr-based, leading-coupling, stationary-axisymmetric perturbation theory—not a full nonlinear 5D EGB branch |
| Lam, Chung & Yunes, PRD 113, 024030 (2026), DOI 10.1103/txxb-z8bx | Spectral/pseudospectral Kerr-deformation metrics for leading-coupling quadratic EFTs at high spin; reports below-machine-precision errors through \(a\le0.9\) and around \(10^{-8}\) through \(a\le0.999\) in its own setup | High-spin representation, adaptive spectral order and perturbation-to-direct-solver handoff methodology | Different dimension, theory and approximation order; these numbers are not a 5D EGB target |
| Husken et al., PRD 114, 064072 (22 Sep 2026), DOI 10.1103/jrnh-wsxw | Pseudospectral scalar-QNM corrections on rapidly rotating 4D scalar-Gauss–Bonnet and dynamical Chern–Simons backgrounds through \(a/M=0.99\); paper reports mode-dependent accuracy from \(\lesssim10^{-3}\) for \(l=m=0\) to \(\lesssim10^{-6}\) for higher multipoles | Downstream perturbation/QNM methods once a qualified background exists | These are QNM errors for 4D scalar-tensor EFTs, not background metric errors or 5D EGB results |
| van der Steen et al., PRD 114, 044087 (25 Aug 2026), DOI 10.1103/xq7z-gh6p | Pseudospectral scalar ringdown corrections in rotating EFT black holes, including cubic-curvature terms; paper reports relative errors below \(10^{-4}\) in its stated mode/spin regime and enhanced corrections near extremality | Further motivation for a background-to-observable pipeline and explicit near-extremal mode tracking | Not a matched five-dimensional EGB QNM benchmark |

Primary links:
- Brihaye et al.: https://arxiv.org/abs/1010.0860
- Akribeia: https://doi.org/10.1103/3kz2-6dvv
- Li, Hu & Yunes: https://arxiv.org/abs/2610.37515
- Lam, Chung & Yunes: https://doi.org/10.1103/txxb-z8bx
- Husken et al.: https://doi.org/10.1103/jrnh-wsxw
- van der Steen et al.: https://doi.org/10.1103/xq7z-gh6p

## 2. The paradigm shifts worth pursuing

### 2.1 From a single solver to a solution atlas

The object to build is not one monolithic solver but a connected atlas of overlapping solution charts over coupling, spin, radius and coordinate gauge:

- exact Myers–Perry at \(\alpha=0\);
- exact static Boulware–Deser at zero spin and finite \(\alpha\);
- exact first-order spin around BD;
- second-order spin response around BD;
- fully nonlinear pseudo-spectral equal-spin EGB solutions;
- horizon-regular and near-extremal charts;
- areal-base and isotropic radial gauges.

Each chart owns a declared domain, regularity conditions, residual evaluator and observable map. Neighboring charts overlap and must agree on invariant observables and transformed residuals. No one chart is assumed to be globally well-conditioned. This converts gauge singularities, perturbative breakdown and near-extremal stiffness into explicit atlas-boundary events rather than hidden solver failures.

### 2.2 Separate candidate discovery from truth qualification

Use Akribeia-like neural fields, classical adaptive BVP solvers, action-derived spectral continuation and analytic reconstruction as independent candidate generators. None is allowed to certify itself.

A separate verifier should:
1. rebuild the action-derived equations from a pinned source and convention;
2. evaluate every field equation and the retained radial constraint on held-out points;
3. verify the angular Noether identity and the sourced time-sector balance independently;
4. bound boundary and spectral-tail errors over the full exterior domain;
5. use Arb/interval arithmetic for denominators, residual enclosures and operator bounds;
6. only then attempt a radii-polynomial existence/uniqueness certificate;
7. separately check thermodynamic integrability, EFT validity and perturbative stability.

This is the main route to a stronger claim than a tiny collocation residual: an independently replayable evidence object for a specified approximate solution.

### 2.3 Structure-preserving reduction, with explicit abstention

Keep the unfixed action variations and \(E_g\) constraint. Use the exact angular Noether momentum \(P_w\) when its denominator is enclosed away from zero. Keep the finite-\(\alpha\) time-sector Eq. (2.14) as a sourced balance; never promote it to a second conserved flux.

If the interval for a reduction denominator includes zero, do not divide. Revert to the undivided equation or a less-reduced solver. This architecture is safer and more portable than optimizing around a guessed nonsingular branch.

### 2.4 Adaptive perturbation-to-spectral handoff

Do not choose a universal spin cutoff. For every \((\lambda,\chi)\), observable and requested tolerance, measure the discrepancy between perturbative orders and the direct nonlinear solution in their overlap. Increase perturbative order only when the next coefficient is well-conditioned; hand off to direct spectral continuation when the a-posteriori truncation budget is exceeded.

Near extremality, use a horizon-adapted chart and pseudo-arclength continuation. An unresolved regularity test should produce an explicit abstention, not extrapolation.

### 2.5 Move from metric solutions to physical predictions

Once a background passes independent qualification, add scalar/tensor perturbation operators and compute QNM shifts, absorption/response coefficients and invariant horizon observables. The late-2026 4D QNM papers show that high-spin corrections can grow sharply near extremality; our system must report mode-by-mode uncertainty and avoid interpreting a background fit as a stability result.

## 2.6 Strong-coupling coordinate atlas: isotropic horizon compression

The static BD areal/isotropic map reveals a nontrivial conditioning effect. For \(\lambda=\alpha/r_H^2\to\infty\), the areal horizon remains at \(\rho_H=1\), but its isotropic coordinate satisfies
\[
\xi_H\sim1.7923085278641586\sqrt\lambda\,e^{-\pi\sqrt\lambda/2}.
\]
This is coordinate compression, not a physical shrinking of the horizon or a curvature singularity. At \(\lambda=10^6\), the leading asymptotic already agrees with the high-precision integral to relative error \(3.92\times10^{-4}\).

This motivates a further atlas chart
\[
\eta=\log(\xi/\xi_H),\quad
d\eta/d\rho=1/(\rho\sqrt F),
\]
with optional compactification \(z=\eta/(1+\eta)\). A uniform isotropic-\(\xi\) grid should not be the only radial representation at strong coupling. Issue [#6206](https://github.com/Luminous-Dynamics/symtropy/issues/6206) owns endpoint transformations, interval Jacobians, and matched work-precision tests against areal/isotropic charts. The asymptotic script and fixture are research assets, not proof-grade remainder bounds.

## 3. Matched benchmark ladder

| Level | Required case | Acceptance evidence |
|---|---|---|
| L0 | Exact 5D MP (\(\alpha=0\)) and static BD (finite \(\alpha\)) | All five action equations vanish; independent profile/charge/first-integral checks; all negative controls rejected |
| L1 | Published finite-\(\alpha\) first-order slow rotation | \(P_w=-2a(2r_H^2+\alpha)=-4\mathcal W\); \(K=r^2s\); angular equation; exact cubic MP remainder |
| L2 | \(O(a^2)\) spin response \(B_2,F_2,H_2\) | Finite-\(\alpha\) BVP solved; exact MP coefficients at \(\alpha=0\); horizon and infinity series; constraint/Bianchi dependency; independent residual and spectral-tail convergence |
| L3 | Nonlinear 5D equal-spin EGB branch | Reproduce 2010/Akribeia parameter points at matched \((r_H,\alpha,\Omega_H)\); continuation from MP and BD seeds; same equations and observables |
| L3G | Two-gauge reproduction | Areal and isotropic formulations independently solve the same physical branch; transformed local residuals and invariant observables agree within a stated enclosure |
| L4 | Near-extremal and physical checks | Horizon-adapted continuation, conditioning and validity maps; thermodynamic first law; no extrapolation beyond resolved domain |
| L5 | Perturbation / QNM layer | Independently derived perturbation operator; mode convergence, boundary residuals, spectral tails and uncertainty budget on qualified backgrounds |

## 4. Work-precision scorecard

For each exact parameter point and solver, freeze: action normalization, dimension, metric/Hopf conventions, radial gauge, parameterization (fixed \(r_H\) or fixed \(E\)), boundary conditions, compactification, precision, collocation norm and observable definitions.

Report:
- maximum normalized \(E_b,E_f,E_g,E_h,E_w\) on both collocation and held-out points;
- full-domain residual enclosures, not just sampled residuals;
- angular-flux drift and time-sector source-balance defect;
- horizon Taylor mismatch, asymptotic \(U,V,W\), and coordinate-map Jacobian;
- \(E,J,T_H,A_H,S_{\rm Wald}\), ergosurface radius and first-law defect;
- cross-gauge invariant-observable mismatch;
- spectral coefficient tails, condition estimates, continuation step rejections and branch coverage;
- candidate-generation cost separately from independent-verification cost;
- wall-clock time, peak memory and energy per accepted/qualified solution.

A method that converges faster but fails held-out or domain-wide checks is not a successful solve.

## 5. Quantitative targets — prospective, not achieved

1. **Exactness gate:** 100% pass on frozen MP, BD and first-order slow-spin controls; zero accepted negative controls.
2. **Classical reproduction:** reproduce 2010 physical observables at their reported typical \(\sim10^{-6}\) relative-accuracy level before making an improvement claim.
3. **Matched improvement:** target \(\le10^{-7}\) relative error in common physical observables on matched reference points, with independent residual and cost reports. This is a target, not an achieved result.
4. **Neural-spectral comparison:** compare the same 5D equal-spin EGB cases and report common observables, held-out residuals, domain enclosures and total cost. Do not compare a 4D metric or MP-limit solution-difference number as if it were this metric.
5. **Gauge invariance:** target cross-gauge observable mismatch below the tighter of the declared discretization uncertainty and independent numerical enclosure, after remeshing and precision increase.
6. **Verified numerical result:** obtain at least one nontrivial finite-\(\alpha\), nonzero-spin solution with outward-rounded full-domain residual bounds; only then attempt a local-existence theorem.
7. **Physical layer:** only after L3/L3G, target a mode-converged scalar perturbation/QNM calculation with an explicit mode-by-mode uncertainty budget.

## 6. Immediate execution priorities

1. Finish the finite-\(\alpha\) \(O(a^2)\) BVP: derive horizon coefficients, asymptotic homogeneous modes, zero-mode/parameter fixing and the \(E_g\) constraint dependency.
2. Extend the gauge map from the executed static BD/linear-spin reference to independently varied isotropic-gauge equations.
3. Reproduce one published 5D nonlinear Akribeia/classical point and one matched gauge pair before attempting a broad parameter sweep.
4. Integrate an Arb/IEEE-1788 path so the fast f64 interval screen can be differentially checked and upgraded to rigorous enclosure.
5. Add perturbation/QNM work as a downstream stage after the background solution and its uncertainty are qualified.

## 7. Claim policy

A symbolic identity is not a numerical solution. A collocation solution is not a full-domain enclosure. An enclosure of a residual is not an existence theorem. An existence theorem is not an EFT-validity or stability statement. Keep each evidence level independently named in every fixture and release note.
