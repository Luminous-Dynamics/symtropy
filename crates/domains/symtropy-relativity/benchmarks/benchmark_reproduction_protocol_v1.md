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

For the Rust static Bach API, component values depend on fourth derivatives. The current API also computes a numerical radial covariant-divergence diagnostic by differentiating a component; the full call therefore requires fifth-order metric data. An order-four jet must be rejected by that API.

## 5. Reproduction of external comparators

### Akribeia rotating higher-curvature discovery

Paper: Agurto-Sepúlveda et al., PRD accepted 12 Nov 2026, DOI 10.1103/3kz2-6dvv; arXiv:2607.07418.

Before any matched run:
- inspect the full article and supplementary methods;
- obtain the exact theory/action, dimension, symmetry ansatz and coupling conventions;
- identify a published benchmark branch with enough data to reconstruct the solution and boundary conditions;
- determine what the paper means by “certified” and what its reported residual verification actually encloses;
- reproduce a published reference branch before trying new parameter values.

Do not compare a 4D static Bach calculation directly against a 5D rotating quadratic/cubic-curvature result. Compare discovery methodology only until a genuinely matched case exists.

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
