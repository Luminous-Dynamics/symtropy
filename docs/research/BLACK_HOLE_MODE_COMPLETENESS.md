# Causal mode-completeness and region-appropriate Green-function expansions

Status: research specification plus analytic pole / thermal-period fixtures. No complete retarded Green function has yet been computed by Symtropy.

Canonical owner: [BH-MODE-COMPLETENESS-001 #4140](https://github.com/Luminous-Dynamics/symtropy/issues/4140).

## Scientific motivation

A July 2026 result changes the representation problem for black-hole response:

Arnaudo, Carballo & Withers, *Beyond quasinormal modes: A complete mode decomposition of black hole perturbations*, Phys. Rev. D 114, 024025 (2026-07-09), DOI [10.1103/1bcl-q79x](https://doi.org/10.1103/1bcl-q79x).

The paper shows that the retarded Green function can be represented using a convergent mode sum across spacetime regions: a Matsubara / Euclidean expansion at early times and a QNM expansion at late times, separated by a light cone scattered by the potential. It treats Pöschl–Teller, BTZ and Schwarzschild examples, and expresses the Schwarzschild branch-cut contribution as a convergent de Sitter-QNM sum in a controlled cosmological-constant limit.

This is not a new physical emission channel. It is a change of mathematical representation of the same retarded response. It matters because a prompt response need not be approximated by a finite QNM sum, and a residual relative to that finite basis is not automatically a separately emitted wave.

Related published comparators:
- Su et al., *Decomposition of the Schwarzschild Green's function*, PRD 113, 104013 (2026-05-08), DOI [10.1103/3n8r-8kbb](https://doi.org/10.1103/3n8r-8kbb): branch-cut direct response, QNM poles, late tail; compared with direct time-domain Regge–Wheeler evolution.
- Rosato, De Amicis & Pani, *Singular structures and causality of the Schwarzschild Green's function in the frequency domain*, PRD 114, 024024 (2026-07-09), DOI [10.1103/58mx-v2wz](https://doi.org/10.1103/58mx-v2wz): source-location dependence of branch-cut tails, delayed backscatter and horizon-surface-gravity redshift terms.

## First analytic fixtures

### Pöschl–Teller barrier

\[
V(x)=V_0\operatorname{sech}^2(x/b),\qquad
X''+[\omega^2-V(x)]X=S.
\]

With the convention \(e^{-i\omega t}\), left-ingoing and right-outgoing boundary conditions, the pole frequencies are

\[
\omega_n=\pm\sqrt{V_0-\frac{1}{4b^2}}-i\frac{n+1/2}{b}.
\]

For \(V_0=2, b=1\), \(\operatorname{Re}\omega=\pm\sqrt 7/2\), and \(\operatorname{Im}\omega_n=-(n+1/2)\). These are pole-location fixtures only. The QNM residues, Matsubara coefficients, causal transition and full time-domain response must be computed independently using a frozen source and Wronskian convention.

### Schwarzschild thermal-period fixture

In geometrized units, non-extremal Schwarzschild has

\[
\kappa=\frac{1}{4M},\qquad
\beta_H=\frac{2\pi}{\kappa}=8\pi M,\qquad
\Delta\omega_E=\frac{2\pi}{\beta_H}=\kappa.
\]

This pins the Euclidean thermal-circle period and Matsubara spacing. It does **not** supply the expansion coefficients, convergence region, transition prescription or remainder bound; those must be transcribed from the exact published construction.

## Objects to preserve

Keep separate:
- the retarded Green function and its source/observer arguments;
- QNM poles and their residues;
- branch-cut spectral density and tail;
- early-time Matsubara / Euclidean coefficients;
- late-time QNM coefficients;
- de Sitter-QNM limiting sequence for the Schwarzschild branch-cut representation;
- the scattered-light-cone transition and causal-region predicate;
- truncation remainder and numerical error;
- detector and extraction-operator output.

A finite sum that fits a finite interval is not a completeness certificate.

## Execution order

1. Freeze the exact paper equations, contours, source conventions, mode normalization and causal-region conditions.
2. Reproduce the Pöschl–Teller pole spectrum and independent retarded time-domain response.
3. Reproduce the early Matsubara and late QNM representations separately, including residue-asymptotic convergence and truncation-error curves.
4. Locate and test the scattered-light-cone transition; verify matching on the boundary without assuming pointwise uniform convergence.
5. Reproduce the BTZ example as a separate asymptotic/boundary-condition family.
6. Reproduce Schwarzschild scalar and Regge–Wheeler examples; compare the de Sitter-QNM branch-cut limiting series against the direct branch-cut integral and the Su et al. decomposition.
7. Vary source location relative to the potential peak to reproduce delayed backscatter tails and horizon-redshift terms.
8. Compare every representation against independent time-domain evolution from Symtropy #4052.
9. Apply rational-filter / QNM-removal interventions from Symthaea #11002 as a separate measurement axis.
10. Propagate representation truncation and discrepancy through Symthaea #10944 and #11021.

## Machine-readable starter manifest

The accompanying JSON freezes the model families, known analytic pole/thermal fixtures, representation kinds, comparison controls and claim ceilings. It is a test specification, not a computed Green-function artifact.

## Acceptance targets — prospective

- Pöschl–Teller pole locations: relative error <=1e-10 in well-conditioned fixtures.
- Retarded response reconstruction: complex waveform error <=1e-8 only in declared convergence domains after the mode truncation and precision study supports it.
- Early / late expansions: separate convergence curves, remainder estimates and causal-region validity.
- Causal transition: matching error decreases with refinement; unresolved nonuniform limits are reported as unresolved.
- Schwarzschild branch-cut limiting sequence: separate error from finite de Sitter mode truncation and error from the \(\Lambda\to0^+\) limit.
- Time-domain comparison: causal support, complex waveform, phase and amplitude errors under identical source / boundary conventions.
- No false independent-source attribution from a change of spectral representation.

## Ownership

- Symtropy #4140: region-appropriate mode representations, convergence and truncation qualification.
- Symtropy #4109: physical scattering / Green-function / boundary-response engine.
- Symtropy #4052: independent time-domain perturbation reference.
- Symtropy #4138: non-normal / structured spectral robustness and transients.
- Symthaea #11002: extraction operator, filter interventions and attribution.
- Symthaea #10944: waveform discrepancy and posterior calibration.
- Symthaea #11021: geometry-to-observation identifiability.
- Mycelix #5903: provenance and replication receipts.
- Sol Atlas #586: separate representation, physical waveform, extraction and posterior layers.

## Claim ceiling

No universal Matsubara expansion, complete QNM sum, de Sitter-QNM branch-cut representation or Kerr extension may be claimed outside the exact equations / backgrounds / causal regions for which convergence is independently demonstrated.
