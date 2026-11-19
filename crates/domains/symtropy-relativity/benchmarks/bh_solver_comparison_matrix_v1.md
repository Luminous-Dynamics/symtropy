# Five-dimensional rotating EGB solver comparison matrix v1

**Purpose:** compare classical adaptive BVP integration, neural-to-spectral discovery, and the Symtropy action/Noether/perturbative pipeline on identical five-dimensional equal-spin EGB problems.

**Theory/source:** Brihaye, Kleihaus, Kunz and Radu, JHEP 11 (2010) 089, arXiv:1010.0860; Agurto-Sepúlveda et al., Akribeia, arXiv:2607.07418, accepted PRD 12 November 2026.

## 1. Three generations of solver

| Pipeline | Published/implemented method | Published reference performance | What it is good at |
|---|---|---|---|
| 2010 classical EGB solver | Newton–Raphson BVP method with adaptive mesh selection | Typically 100–1,000 mesh points; typical relative accuracy about 10^-6 | Direct nonlinear solution, horizon-to-infinity boundary-value control, branch continuation from MP and static BD |
| Akribeia (2026) | Physics-informed neural fields, then Chebyshev pseudo-spectral Gauss–Newton refinement in extended precision | Paper-specific high-precision residual/solution-difference results; its ~10^-160 figure is scoped to the Myers–Perry GR-limit relative L2 solution difference at 300-digit arithmetic | Candidate discovery from random initializations, parameterized global neural fields, spectral refinement |
| Symtropy target | Unfixed radial-action variation, exact angular Noether flux, slow-rotation/MP/BD seeds, spectral continuation, independent equation/interval verifier | No fully nonlinear result yet; exact-reference and action-level infrastructure is authored, Rust unexecuted | Preserve constraints and conserved structure, reduce candidate dimension where safe, carry independent evidence and fail closed |

The 2010 solver accuracy is a paper-reported typical relative accuracy, not a residual norm directly comparable to Akribeia's reported metrics. A fair benchmark must recompute common observables and residual norms from matched raw solution profiles.

## 2. Symtropy's intended paradigm shift

Do not try to beat the other approaches by training a larger network or simply increasing collocation order. Build a layered solver where each mechanism has a clear role:

1. **Action-level structure:** derive the five cohomogeneity-one Euler–Lagrange equations from the unfixed radial action and retain the radial constraint after applying g=r^2.
2. **Exact seeds:** static Boulware–Deser for finite alpha, exact Myers–Perry for alpha=0, and the published first-order slow-rotation profile for finite alpha.
3. **Noether-aware candidate generation:** impose the exactly conserved angular current P_w=-4W; on the BD/linear-spin reference, prove K=r^2*s>=r^2 for alpha>=0. For arbitrary nonlinear candidates, require outward-rounded interval enclosures of K before division.
4. **Perturbative homotopy:** move from slow rotation through the O(a^2) backreaction problem, then continue to larger dimensionless spin chi=a/r_H^2.
5. **Independent qualification:** action-derived residuals, full-domain flux checks, boundary/horizon expansions, independent observable reconstruction, and eventually interval/existence proof.

The time-sector Eq. (2.14) current must remain a source-balance diagnostic at finite alpha:
\[
P_t'(r)=\frac{\alpha}{6}r^2\sqrt{\frac{bh}{f}}L_{GB}.
\]
It is not a second conserved charge. The earlier dual-conserved-flux reduction was withdrawn.

## 3. Frozen benchmark ladder

### L0 — exact algebraic and static controls
- alpha=0 equal-spin Myers–Perry: all five action equations vanish.
- finite-alpha static Boulware–Deser Einstein branch: all five action equations vanish.
- exact MP first/second derivatives, horizon data and asymptotic U,V,W.
- static 5D Gauss–Bonnet scalar and the finite-alpha time-sector source balance.

### L1 — finite-alpha first-order spin
- compare the exact published slow-rotation w(r) and its first two derivatives;
- verify P_w=-2a(2r_H^2+alpha)=-4W at held-out radii;
- verify K=r^2*s>=r^2 on the static background;
- confirm E_w vanishes at linear spin order and the remaining even-sector equations begin at O(a^2);
- at alpha=0, compare with exact MP and recover the known O(a^3) cubic mismatch.

### L2 — second-order spin
- solve for B2,F2,H2 in b=F+a^2B2, f=F+a^2F2, h=r^2+a^2H2;
- at alpha=0 recover exact MP coefficients:
  B2=-u(1-u^2)/r_H^2,
  F2=-u(1-u)/r_H^2,
  H2=u;
- retain E_g as a radial constraint and document Bianchi dependencies.

### L3 — fully nonlinear rotating EGB
- matched r_H=1, alpha sweep [0,3], and same horizon-angular-velocity points as the published 2010/Akribeia cases;
- compare alpha-continuation from MP and Omega-continuation from BD/slow-rotation seeds;
- map the existence boundary and extremal limit without assuming regularity at extremality;
- compare ergosurface radius, ADM energy, equal angular momenta, temperature, area, and Wald entropy.

## 4. Matched solver experiment

Run the same equation normalization, gauge, horizon and infinity conditions, compactification, radial domain, parameter points, precision, and residual norm for:

A. Classical adaptive BVP: Newton–Raphson with adaptive mesh, configured to reproduce the 2010 setup.
B. Neural-spectral: Akribeia-style neural candidate plus Chebyshev Gauss–Newton refinement.
C. Spectral continuation: exact MP/BD/slow-rotation seeds, pseudo-arclength continuation, and Noether-flux constraints.
D. Source-balanced action solver: same action-derived equations with Eq. (2.14) retained as a source-balance residual; it is not a second conserved flux.

Candidate-generation and qualification costs must be measured separately. A method that produces a small collocation residual but fails held-out or domain-enclosed checks is not a successful solve.

## 5. Common scorecard

Record for every parameter point:
- convergence success/failure and number of distinct branches found;
- horizon-to-infinity coverage and boundary mismatch;
- max normalized E_b,E_f,E_g,E_h,E_w residual on collocation and held-out points;
- full-domain outward-rounded residual enclosure, when available;
- angular flux drift P_w(r)+4W and K(r) enclosure;
- time-sector source-balance defect, not time-sector flux constancy;
- asymptotic U,V,W and horizon Taylor coefficient mismatch;
- ergosurface, temperature, horizon area, ADM energy, angular momentum and Wald entropy mismatch;
- spectral coefficient tails, condition estimates and continuation step rejection;
- runtime, peak memory, energy and verification/proof cost.

Akribeia's ~10^-160 relative L2 MP-limit figure is retained in its original scope. It is not a nonzero-alpha EGB field residual target.

## 6. Prospective targets (not achieved results)

- 100% pass rate on the exact MP, BD and slow-rotation known-answer gates.
- Zero accepted negative controls.
- Recover the 2010 reference observables within their reported accuracy before claiming improvement.
- Demonstrate matched observable errors at least one order of magnitude below the classical 2010 typical 10^-6 level while also reporting residual norms and computational cost.
- Show robust branch discovery from both MP and BD seeds, including an explicit failure map near extremality.
- Obtain full-domain residual enclosures for at least one nontrivial finite-alpha rotating solution.
- Produce one local-existence certificate only after the approximate inverse and nonlinear operator bounds are independently verified.

The one-order-of-magnitude objective is a prospective target, not a statement that Symtropy currently outperforms either published solver.

## 7. Negative controls

- Change alpha normalization by a factor of four.
- Flip Hopf-fiber orientation without changing w.
- Remove the alpha-dependent angular-flux term.
- Use the gauge-fixed K(r) with g not equal to r^2.
- Divide through K when its interval enclosure contains zero.
- Use the Einstein-only time-sector current at finite alpha or set its source to zero.
- Omit the (g-h) angular cross-term or vary after gauge fixing and discard E_g.
- Use b*g'*h'/g instead of the published b*g'*h'/h action term.
- Compare fixed-horizon-radius and fixed-ADM-mass branches as if they were identical.
- Treat the first-order slow-rotation solution as a finite-spin exact solution.
- Label the MP ~10^-160 relative L2 result as an EGB equation residual.

Each mutation must fail or yield inconclusive.

## 8. Evidence status

- The 5D reference crate and action-derived ODE evaluator are authored.
- Independent symbolic development checks have covered the static and first-order controls.
- 22 Rust tests are authored; none has been compiled or executed because this environment has no Rust toolchain.
- The committed SymPy scripts have not yet been replayed from a clean checkout.
- The fully nonlinear finite-alpha rotating EGB solver and interval/existence certificate are not implemented.


## 9. Additional 2026 spectral comparator: arbitrary-spin EFT metrics

Lam, Chung and Yunes, *Phys. Rev. D* 113, 024030 (15 January 2026), present a spectral/pseudospectral method for stationary, axisymmetric black-hole metrics in leading-coupling EFT extensions of GR. Their approach expands radial and polar deformations spectrally, solves algebraic systems at fixed spin, and fits the coefficients as functions of spin. The paper reports errors below machine precision for \(a\le0.9\) and \(\lesssim10^{-8}\) for \(a\le0.999\), and shows that slow-rotation approximations can break down around \(a\sim0.2\)–\(0.6\), depending on perturbative order and requested accuracy. This is a high-value methodological comparator, but it is **not a matched 5D pure EGB theory benchmark**: it treats 4D Kerr-based quadratic-gravity EFTs at leading order in couplings.

Implications for our programme:
- Do not let the \(O(a^2)\) or \(O(a^3)\) expansion become the production solution at moderate/high spin.
- Measure the perturbative truncation error against a direct spectral/nonlinear solution as a function of \(\chi=a/r_H^2\), coupling \(\lambda=\alpha/r_H^2\), and requested observable tolerance.
- Build an adaptive-order handoff: use the lowest perturbative order that meets a declared error budget; switch to direct spectral continuation when the next-order remainder fails the budget.
- Use the overlap region to cross-check the direct solver against the perturbative series. Do not extrapolate the slow-spin series beyond a validated overlap.
- For any-spin results, report separate field residuals and observable errors; an accurate fitted metric is not automatically a rigorous solution of the exact field equations.

Akribeia remains the closer discovery-method comparator for nonlinear higher-curvature rotating branches, while the Lam–Chung–Yunes paper is the strongest comparator here for high-spin spectral representation and explicit slow-rotation breakdown analysis.

## 10. Gauge and representation cross-checks

The original 2010 EGB study reports that selected physical parameters were recalculated in two different gauges and agreed excellently. Reproduce this as a mandatory cross-gauge test, not just a code-implementation detail:
- choose two independently derived radial gauge formulations;
- transform the same physical solution between them;
- compare invariant quantities \(E,J,T_H,A_H,S_{\rm Wald},r_e\);
- compare field residuals after transforming each solution into the same invariant frame;
- require agreement under radial remeshing and increased spectral order.

This is especially important because the areal-base gauge \(g=r^2\) is only valid in its declared exterior region and should not be silently extrapolated through coordinate degeneracies.

## 11. Adaptive spin-order / spectral handoff benchmark

For \(\chi\) on a frozen grid from the slow-spin regime toward the branch endpoint:
1. evaluate first- and second-order perturbative models;
2. estimate the next omitted coefficient using an independently computed higher-order term or nonlinear overlap data;
3. compare observable error and normalized equation residual against direct spectral refinement;
4. determine the largest \(\chi\) at which each perturbative order satisfies each declared tolerance;
5. switch to the direct solver before the perturbative truncation loses accuracy;
6. near extremality, use a horizon-adapted coordinate and pseudo-arclength continuation; return unresolved at a singular gauge or failed regularity condition.

The benchmark target is a calibrated *accuracy-versus-spin envelope* rather than a single claimed “valid up to spin X” threshold. The threshold depends on coupling, observable and tolerance.
