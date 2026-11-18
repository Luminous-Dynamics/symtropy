# Black-hole solver benchmark reproduction protocol v1

**Status:** frozen protocol and exact control cases; no benchmark runs are claimed.
**Owner:** [BH-SOTA-BENCHMARK-001 #6043](https://github.com/Luminous-Dynamics/symtropy/issues/6043).
**Case manifest:** benchmark_cases_v1.json.
**Result schema:** black_hole_solver_benchmark_v1.schema.json.

## 1. Benchmark philosophy

Keep four layers separate:

- discovery: branch found, initial-data basin and continuation behaviour;
- approximation: solution and equation residual accuracy;
- qualification: full-domain enclosures and theorem-backed local existence/uniqueness;
- physical validity: chart/observer/EFT/observational gates.

A tiny pointwise or collocation residual is not a proof of a solution. An interval-enclosed residual alone is not an existence theorem. A local theorem is not a physical EFT-validity statement.

## 2. Frozen exact controls

The initial matched tests are all static and four-dimensional. They intentionally do not claim a comparison with rotating Akribeia solutions.

### GR Schwarzschild control

Metric function B(r)=1-2/r with mass M=1 on r∈[3,6]. Exact reference values are B(3)=1/3 and B(6)=2/3. The Bach tensor vanishes identically. Require the independent full-component reference and the static reduced equations to return zero within their arithmetic enclosure.

### On-shell Mannheim–Kazanas control

Use
B(r) = -17/(10r) + 7/10 + r/10 - r²/100
on r∈[3,6]. Exact values: B(3)=103/300, B(6)=197/300 and Q=c0²-3c1c_-1-1=0. The derivative is
B'(r)=17/(10r²)+1/10-r/50,
B''(r)=-17/(5r³)-1/50<0.
Therefore B' decreases on the interval, and B'(6)=49/1800>0. The lapse is positive and monotone throughout the static annulus. Every independent Bach component, trace and covariant divergence vanishes.

### Off-shell negative control

Change only c0 from 7/10 to 11/10. Then Q=18/25, B(3)=223/300 and B(6)=317/300. The independent angular component and reduced projection are nonzero:
B_theta_theta = Q/(3r²),
B1_minus_B2 = 4Q/(3r⁴).
Meanwhile the first factorized reduced equation
B1_plus_B2 = -B(r)*(r*B''''(r)+4*B'''(r))/(3r)
remains exactly zero for both on-shell and off-shell four-term profiles. This is intentional: the first equation selects the four-term family; the second imposes Q=0. A verifier that checks only B1_plus_B2 has a false-positive control.
Consequently B1_minus_B2(3)=8/675 and B1_minus_B2(6)=1/1350, a factor-16 ratio. A verifier must exclude zero for this mutated vacuum residual; it must never silently treat this metric as an on-shell Bach vacuum.

## 3. Exact Chebyshev tail control

For the MK annulus, use r=9/2+(3/2)x, x∈[-1,1], κ=3 and q=3-2√2. With T_n(x) the usual Chebyshev polynomials,
1/r = [1/(d√(κ²-1))] * [1+2 Σ_{n≥1}(-q)^n T_n(x)].
The c0+c1r+c2r² terms are degree two in x. The only infinite tail is the inverse-radius term.

For one-sided physical coefficients a0+Σ_{n≥1}a_nT_n, the symmetric bilateral Laurent representation uses c0=a0 and c_n=c_-n=a_n/2. Since the two nonzero modes contribute equally, the bilateral weighted ℓ1_ν norm equals the one-sided weighted norm exactly.

At ν=2, qν=6-4√2<1. The tail after degree N is bounded by
(17/(15√2)) * (qν)^(N+1)/(1-qν).
Use the exact rational upper enclosures in mk_chebyshev_rational_tail_certificate_v1.json and the corresponding Lean arithmetic source. The integer-square witness is exact; no floating-point decimal is used in the rational upper bound.

This bound covers only the coefficient tail of the known 1/r term. It does not bound the Bach operator residual, fourth-derivative amplification, approximate inverse defect, nonlinear variation or the full BVP.

## 4. Execution protocol for the static controls

For each case:
1. Freeze the input case manifest, equation conventions, chart, units and branch identifier.
2. Generate a candidate using a separately declared solver (direct analytic fixture for plumbing; a spectral solver for an actual discovery test).
3. Evaluate the static full-component Bach formulas through an independent route.
4. Evaluate the published reduced projection equations as cross-checks, not as the sole oracle.
5. Evaluate at all collocation nodes and a pre-registered held-out set.
6. Run interval/ball enclosures across the complete annulus, using subdivision if interval dependency is too wide.
7. Check coefficient-tail and derivative-tail estimates independently.
8. Store source, candidate, interval-extension, environment and replay digests.
9. Run the frozen negative control after every positive case.
10. Return Verified, Rejected or Inconclusive; never reinterpret Inconclusive as evidence of nonexistence or correctness.

For the Rust static Bach API, component values depend on fourth derivatives. Its radial-divergence diagnostic differentiates B_rr, which depends only through B''' in this static specialization; therefore the full call requires Taylor order four. An order-three jet must be rejected. The general arbitrary-metric contracted-divergence evaluator still requires fifth-order metric data.

## 5. Reproduction of external comparators

### Akribeia rotating higher-curvature discovery

Paper: Agurto-Sepúlveda et al., PRD accepted 12 Nov 2026, DOI 10.1103/3kz2-6dvv; arXiv:2607.07418.

The paper's concrete comparison targets are split into two machine-readable cases in `benchmark_cases_v1.json`:

- **5D Einstein–Gauss–Bonnet, equal angular momenta:** the action uses `alpha_1=(16*pi*G)^-1`, `alpha=4*alpha_2/alpha_1`, and `alpha_0=0`. The paper fixes `r_H=1`, `Omega_H=0.33`, and sweeps `alpha∈[0,3]` in steps of 0.01. Its cohomogeneity-one ansatz has functions `b,f,g,h,w`, with radial gauge `g(r)=r^2`; horizon conditions include `f(r_H)=b(r_H)=0`, `w(r_H)=Omega_H`, and `h(r_H)=Theta_H`. The radial compactification is `r-r_H=tan(x)`. The reported neural field uses three hidden layers of width 20, Swish activations, and strong boundary reparameterization. Its global training uses 10^4–10^5 steps (Adam warm-up/stable/decay, then L-BFGS). The paper reports pre-refinement residuals around 10^-3, followed by Chebyshev pseudo-spectral Gauss–Newton with extended precision. At alpha=0 it recovers Myers–Perry; at alpha=3 it matches a prior 5D EGB numerical branch. The paper reports relative L2 error around 10^-160 for the Myers–Perry limit using 300-digit arithmetic.
- **7D cubic Lovelock, equal angular momenta:** the action conventions are `alpha=4*alpha_2/alpha_1` and `beta=16*alpha_3/alpha_1`, with `alpha_0=0`. It fixes `r_H=1`, `Omega_H=0.33`, `alpha=0.5`, and sweeps `beta∈[0,0.5]` in steps of 0.01, with a four-layer width-64 neural field. Crucially, `beta→0` at fixed `alpha=0.5` is a quadratic EGB limit, not the GR/Myers–Perry limit; GR requires `alpha=beta=0`.

The main paper pins the action coupling definitions, symmetry ansatz, radial gauge, horizon data and compactification. Exact asymptotic series coefficients and detailed per-point solution arrays still need extraction from the Supplemental Material before a fully matched numerical run. Neither case is matched to Symtropy's current 4D pure-Weyl static problem.

Before any matched run:
- pin the exact theory/action and normalization, dimension, symmetry ansatz and coupling definitions;
- reconstruct the horizon/asymptotic boundary conditions and radial compactification;
- reproduce at least one published point and the correct GR limit where available;
- compare PINN discovery cost/convergence separately from pseudo-spectral refinement cost/accuracy;
- retain the same branch/parameter points and residual norm;
- inspect the meaning of “certified” in the source: a continuous globally parameterized neural field and a quantified residual are not automatically an interval/radii-polynomial existence theorem.

Do not compare a 4D static Bach calculation directly against 5D/7D rotating Lovelock results. The first fair comparison is a separately implemented 5D EGB case with matched action, dimension, ansatz, gauge and boundary conditions.

### High-spin scalar QNM comparator

Paper: Husken et al., PRD 114, 064072 (22 Sep 2026), DOI 10.1103/jrnh-wsxw.

Reproduce only the same massless minimally coupled scalar perturbation on the same dCS/sGB background and with the same mode, normalization, frequency convention and boundary conditions. Published accuracy is approximately 1e-3 or better for l=m=0 and 1e-6 or better for higher multipoles, through a/M=0.99. These are reference targets only when the data and conventions are fully matched.

### Near-extremal EFT/regularity comparator

Paper: Lam, Horowitz and Yunes, PRD 114, 104044 (13 Nov 2026), DOI 10.1103/ddyq-94zb.

Follow fixed-surface-gravity and fixed-horizon-angular-velocity families separately. Record scalar invariants and infalling tidal forces as distinct diagnostics with an explicit observer/tetrad. Reproduce the theory-specific dCS/sGB boundary-layer and EFT trends without transferring those conclusions to pure Weyl gravity.

## 6. Required metrics

### Discovery and approximation
- convergence/failure rate over frozen initial seeds;
- branch consistency and continuation coverage;
- collocation residual;
- held-out residual;
- full-domain residual enclosure;
- boundary/axis/horizon residual enclosures;
- coefficient and derivative tails;
- gauge-fixed solution difference against a reference;
- precision/resolution convergence.

### Qualification
- Y0, Z0, Z1 and Z2 enclosures;
- injectivity and approximate inverse evidence;
- radii-polynomial interval and exact sign;
- Lean theorem and axiom report;
- interval backend and extension digests;
- local ball radius and explicitly stated function space;
- all failed/inconclusive attempts retained.

### Physical validity
- coordinate-chart regularity;
- curvature invariants and derivative invariants;
- freely falling tidal tensor for a named observer family;
- extra-field gradients and horizon regularity;
- boundary-layer resolution;
- theory-specific EFT operator ordering;
- observational likelihood only when a declared forward model and data likelihood exist.

### Cost
- wall time, peak memory, hardware and precision;
- discovery cost and verification cost separated;
- energy where measurable;
- interval subdivision count and arithmetic operation count;
- proof-check time and kernel dependency footprint.

## 7. Negative-control suite

The following controls must be run, not just described:
1. Off-shell c0 mutation Q=18/25; residual enclosure must exclude zero.
2. Sign-flip the Bach tensor convention; exact Schwarzschild/MK fixtures must fail.
3. Understate the analytic Chebyshev tail; enclosure must reject the bound.
4. Remove a boundary/gauge row; approximate inverse must expose a null or unresolved mode.
5. Mutate an approximate inverse but reuse its old Z0/Z1 values; proof must fail.
6. Mutate one spectral coefficient while preserving small collocation residual; held-out/full-domain checks must catch it.
7. Give an order-four metric jet to the full static Bach API; it must return insufficient derivative order.
8. Change chart, length units or coupling normalization without changing the manifest; preflight must reject.
9. Keep scalar invariants finite but inject a divergent infalling-tidal fixture; physical tidal gate must fail.
10. Reuse dCS/sGB QNM reference data for another action; theory-identity check must reject.
11. Swap fixed-spin and fixed-surface-gravity continuation while reusing the branch ID; manifest validation must reject.
12. Underflow/precision downgrade that invalidates an interval extension; qualification must become inconclusive or fail.

Target: zero accepted false positives in the frozen mutation suite. “Inconclusive” is permitted and is not counted as a correct scientific conclusion about the physical system.

## 8. SOTA comparison and target policy

A matched comparison table must include:
- method and discovery strategy;
- dimension/action/problem;
- solution domain and parameter range;
- published/observed residual;
- independent full-domain enclosure status;
- existence/uniqueness theorem status;
- physical-validity gates;
- runtime, hardware and precision.

The first goal is parity on a matched published case. The second goal is stronger qualification: full-domain enclosures, exact analytic tails, operator inverse bounds and formal local existence/uniqueness. The third goal is expanding the validated parameter domain without losing those gates.

Do not set universal 1e-12 or 1e-15 targets before a matched baseline and condition study. The QNM paper’s reported 1e-3/1e-6 mode accuracies apply only to its stated scalar modes and theories. No “beats Akribeia” claim is allowed until an equivalent case is reproduced.

## 9. Claim ceiling

Current files provide exact analytic controls, rational tail enclosures and theorem-arithmetic plumbing. They do not yet provide a compiled Rust test pass, a Lean build, a full-domain Bach interval enclosure, a radii-polynomial BVP proof, a rotating solution or a physical EFT-validity result.


## 10. Analytic static-vacuum classification

The static spherical Bach vacuum family has an independent symbolic reduction in `static_bach_vacuum_classification_v1.md`. On a connected interval with r>0 and B(r)>0, the first reduced projection integrates to the four-term Mannheim–Kazanas form, and the second projection imposes Q=c0²−3c1c_-1−1=0. This exact family is a known-answer control, not the first substantive numerical discovery target.

Issue #6048 owns the Lean formal classification. The radii-polynomial path in #6019 is now reserved for genuinely non-integrable sourced/modified-action/rotating BVPs after their equation and constraints have been frozen.


## 10. Factorized static operator and parameter-kernel control

The generic static sum equation factors as
`B1_plus_B2=-B(r)*(r*B''''+4*B''')/(3r)`. On a static annulus with r>0 and B>0 it integrates to the complete four-term profile. The difference equation imposes the remaining Q=0 constraint. This factorization and its three-dimensional MK tangent kernel are documented in `bach_static_reduction_factorization_v1.md`.

For every four-term case, record both equations separately. The off-shell c0 mutation must still have B1_plus_B2=0 and B1_minus_B2!=0. In a radii-polynomial setup, the three exact MK parameter directions must be projected or augmented; an unhandled approximate inverse is singular.


## 11. Akribeia matched 5D equal-spin EGB reproduction

Owner: [BH-AKRIBEIA-REPRO-001 #6063](https://github.com/Luminous-Dynamics/symtropy/issues/6063).

The first matched higher-curvature discovery case is the paper's 5D equal-angular-momentum Einstein–Gauss–Bonnet family. This is selected because it has both an exact Myers–Perry GR-limit control and an independent published EGB numerical branch at alpha=3.

Freeze the paper convention:
- alpha_1=(16*pi*G)^-1, alpha_0=0, alpha=4*alpha_2/alpha_1;
- cohomogeneity-one U(2)-symmetric Hopf-frame metric with functions b,f,g,h,w;
- areal/hopf-base gauge g(r)=r^2;
- r_H=1, Omega_H=0.33;
- alpha from 0 to 3 in steps 0.01;
- horizon regularity f(r_H)=b(r_H)=0, w(r_H)=Omega_H, h(r_H)=Theta_H plus the full near-horizon series from the supplement;
- asymptotically flat conditions using r-r_H=tan(x), with the exact compactified domain and endpoint map taken from the pinned supplement.

The paper uses a 3x20 Swish neural architecture for this case, 10^4–10^5 training steps (Adam stages followed by L-BFGS), then Chebyshev pseudo-spectral Gauss–Newton refinement in extended precision. The published ~1e-160 relative L2 difference at 300 digits applies to the exact Myers–Perry GR-limit recovery, not to all EGB residuals.

### Architecture gate

The existing Symtropy public relativity crate currently has a 4x4 metric representation. Do not pass its 4D tensors into a 5D solver. The first implementation should be a dedicated 5D Hopf-frame/cohomogeneity-one Lovelock system with a frozen basis and a separate field-equation derivation. General dense 5D tensors are not required if the invariant reduction is independently derived and checked against the action variation.

### Execution order

1. Extract and hash the paper's Supplemental Material, near-horizon expansions, asymptotic series, one-form normalizations and exact reference data.
2. Reconstruct exact 5D Myers–Perry functions in the paper's gauge; check horizon quantities, asymptotic charges and the alpha=0 field equations.
3. Reproduce the 2010 alpha=3 EGB solution and compare all five metric functions/observables.
4. Reproduce the full alpha sweep with matched branch and compactification.
5. Only then compare discovery algorithms and add independent interval/operator/tail qualification.

If the paper provides no raw solution arrays, digitized curves must carry explicit digitization error and cannot be promoted to exact reference data. Akribeia's residual-threshold “certification”, a full-domain residual enclosure, and a radii-polynomial existence theorem remain three different evidence levels.


## 12. Second radial first integral: time-sector equation (2010 Eq. 2.14)

The 2010 source also states that a particular EGB field-equation combination is a total radial derivative:
\[
R_t{}^t+\frac{\alpha}{4}\left(H_t{}^t+\frac12L_{GB}\right)
=\frac{1}{2r^2}\sqrt{\frac{f}{bh}}\,
\frac{d}{dr}\left[
\sqrt{\frac{fh}{b}}r^2(hww'-b')\right].
\]
For a full EGB solution, this gives the constant radial quantity
\[
P_t=\sqrt{\frac{fh}{b}}r^2(hww'-b')=2\mathcal U,
\]
where \(\mathcal U\) is the \(1/r^2\) coefficient of the asymptotic lapse. The regular horizon limit is
\[
P_t=r_H^2\sqrt{\frac{f_1h_H}{b_1}}
(h_H\Omega_Hw_1-b_1).
\]

This is not a second cyclic Noether momentum. It is a first integral of a particular field-equation combination. Use it as a separate constraint and independent residual diagnostic, not as a replacement for the complete EGB equations.

Where \(r^2\sqrt{fh/b}\) is resolved and nonzero,
\[
b'=hww'-\frac{2\mathcal U}{r^2\sqrt{fh/b}}.
\]
Together with the angular flux equation this offers a three-profile formulation: candidate functions \(f,h\) plus an integrated \(w\) profile and the lapse \(b\) recovered from the time-sector relation. More precisely, the primary unknown profiles can be reduced to \(f,h\) with two radial quadratures for \(w\) and \(b\), subject to the remaining EGB equations and global charges \((\mathcal U,\mathcal W)\). This is a candidate architecture to benchmark, not a proved reduction of the full nonlinear BVP.

For exact 5D equal-spin Myers–Perry at alpha=0, independently verify both
\[
P_w=-4\mathcal W,\qquad P_t=2\mathcal U
\]
at held-out exterior radii and using analytic horizon limits. The Rust crate now has separate time-sector flux and b-prime reconstruction helpers. Their floating-point denominator guards prevent obvious singular division but are not rigorous enclosures.

The complete 5D first-integral exact fixture is located at crates/domains/symtropy-lovelock5d/benchmarks/akribiea_5d_mp_exact_fixture_v1.json.
