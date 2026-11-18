# Near-extremal EFT validity and regularity benchmark protocol v1

**Status:** research/benchmark specification, not an executed numerical result.  
**Owner:** [BH-EFT-VALIDITY-001 #6014](https://github.com/Luminous-Dynamics/symtropy/issues/6014).  
**Related:** [BH-SPECTRAL-VALIDATION-001 #5992](https://github.com/Luminous-Dynamics/symtropy/issues/5992), [Bach evaluator #5937](https://github.com/Luminous-Dynamics/symtropy/issues/5937), [Taylor jets #5965](https://github.com/Luminous-Dynamics/symtropy/issues/5965), [chart contract #4073](https://github.com/Luminous-Dynamics/symtropy/issues/4073).

## 1. Motivation and current research comparators

Two 2026 results change the benchmark requirements.

### Near-extremal EFT validity

Lam, Horowitz and Yunes, *Living on the edge of effective field theory: Near-extremal black holes in quadratic gravity*, Phys. Rev. D 114, 104044 (published 13 November 2026), DOI [10.1103/ddyq-94zb](https://doi.org/10.1103/ddyq-94zb).

The paper constructs rotating exterior solutions in dynamical Chern–Simons (dCS) and scalar Gauss–Bonnet (sGB) gravity along families controlled by fixed surface gravity and fixed horizon angular velocity. Its reported near-extremal behaviours differ by theory: the dCS family approaches an asymptotically flat extremal exterior with a deformed NHEK limit, while the sGB family develops a logarithmic scalar horizon singularity and a near-horizon boundary layer. It also reports that scalar curvature invariants can remain finite even when infalling-observer tidal forces diverge as inverse powers of surface gravity.

The methodological lesson is broader than those models: scalar-invariant finiteness is not a sufficient regularity or EFT-validity certificate. The dCS/sGB conclusions themselves must not be transferred to pure Weyl gravity without a derivation.

### High-spin QNM comparator

Husken et al., *Quadratic gravity corrections to scalar quasinormal modes of rapidly rotating black holes*, Phys. Rev. D 114, 064072 (published 22 September 2026), DOI [10.1103/jrnh-wsxw](https://doi.org/10.1103/jrnh-wsxw).

The paper reports pseudo-spectral scalar test-field QNM corrections on rapidly rotating dCS/sGB backgrounds up to dimensionless spin 0.99. Reported accuracy is better than approximately 1e-3 for l=m=0 and 1e-6 for higher multipoles. These are theory- and mode-specific comparator figures, not universal targets and not direct benchmarks for pure Weyl perturbations.

### Neural-spectral discovery comparator

Agurto-Sepúlveda et al., *Neural-spectral discovery of rotating black holes beyond general relativity*, PRD accepted 12 November 2026, DOI [10.1103/3kz2-6dvv](https://doi.org/10.1103/3kz2-6dvv).

Akribeia combines physics-informed neural-field discovery and pseudo-spectral refinement for rotating higher-curvature black holes, demonstrated in higher-dimensional quadratic/cubic-curvature settings. It is a discovery-method comparator; its theory, dimensionality, symmetry ansatz and numerical domain must be matched before making performance comparisons. The present protocol does not presume that Akribeia lacks residual checks; it specifies additional independently replayable validity diagnostics for Symtropy.

## 2. Paradigm shift: separate regularity questions

A single finite/non-finite flag loses essential physics. For every background, parameter point and stated observer family, report independent statuses:

1. **Chart regularity:** coordinate components behave correctly in the declared chart; chart singularities are not treated as curvature singularities.
2. **Scalar-invariant regularity:** declared polynomial curvature and matter invariants remain bounded.
3. **Observer-frame regularity:** freely falling tidal tensor components remain bounded for a fully specified tetrad/congruence.
4. **Extra-field regularity:** scalar/vector/tensor fields and their covariant derivatives satisfy the declared horizon boundary conditions.
5. **EFT ordering:** the local dimensionless expansion parameters for omitted higher-order operators remain controlled.
6. **Numerical resolution:** spectral tail, boundary layer, solver error and arithmetic enclosure are separately bounded.
7. **Observational admissibility:** only available after an explicit forward model, likelihood and uncertainty model.

A solution can pass some gates and fail others. A finite Kretschmann scalar must never override a divergent infalling tidal component.

## 3. Parameterization and branch identity

Do not use spin alone as the continuation coordinate near extremality.

For a stationary axisymmetric black hole, freeze and record:
- ADM mass M and angular momentum J;
- dimensionless spin chi = J/M^2 (in geometrized units, with sign/axis convention);
- horizon radius and area;
- surface gravity kappa_H and dimensionless kappa_H M;
- horizon angular velocity Omega_H and dimensionless Omega_H M;
- all theory couplings in pinned dimensions/normalization;
- coordinate chart, gauge, horizon normalization and asymptotic normalization;
- branch ID and continuation parameterization.

Keep separate continuation families:
- fixed chi;
- fixed kappa_H M;
- fixed Omega_H M;
- jointly constrained kappa_H M and Omega_H M.

Record the map between 1-chi and kappa_H M for each theory/branch. Near-extremal scaling is not assumed universal across theories.

Exact extremality is a distinct endpoint problem. Finite-spin extrapolation must not be labeled an exact-extremal solution.

## 4. Diagnostic matrix

| Layer | Required quantities | Failure / abstention condition |
|---|---|---|
| Geometry | Kretschmann, Weyl/Ricci scalars, action-specific invariants | Invariant unbounded or precision/domain unresolved |
| Derivative geometry | Covariant derivatives of curvature, Cotton/Bach where applicable | Derivative invariant or tensor identity unresolved |
| Observer frame | Electric/magnetic Weyl tidal tensors, Riemann-frame components, geodesic-deviation eigenvalues | Divergence, observer dependence not specified, or tetrad not regular |
| Extra fields | Scalar values, first/second covariant derivatives, horizon regularity | Log/power singularity or boundary condition failure |
| Near-horizon layer | Boundary-layer width, matched asymptotic error, horizon residual | Layer under-resolved or no overlap region |
| EFT truncation | Action-specific dimensionless operator ratios, next-order estimate | Higher-order terms not parametrically suppressed |
| Numerics | Spectral tail, off-grid residual, conditioning, ball enclosure | No full-domain bound or inconclusive inverse estimate |
| Perturbations | Complex QNM frequencies, residues, eigenfunction residuals | Background/operator error exceeds claimed shift |

### Observer definition

Every tidal result must include:
- geodesic initial data and conserved energy/angular momentum;
- observer four-velocity and orthonormal tetrad;
- tetrad transport rule (parallel/Fermi–Walker or explicitly stated alternative);
- tetrad orientation and sign convention;
- radius/affine parameter at which each component is evaluated;
- chart transformation and normalization provenance.

“Freely falling tidal force” without an observer specification is not a reproducible observable.

## 5. Theory-specific EFT validity

For each action separately:

1. Pin the operator basis, coupling dimensions and exact normalization.
2. Derive dimensionless local expansion parameters from the relevant curvature, derivative and extra-field scales.
3. Track the magnitude of the first omitted operators, not merely the retained correction.
4. Separate numerical discretization error, spectral truncation, background perturbation order and EFT truncation.
5. Evaluate the full exterior and the near-horizon boundary layer; do not sample only the horizon and infinity.
6. Report the largest domain in (coupling, spin, kappa_H M) where the ordered expansion is demonstrated.

Do not import a dCS validity bound into sGB or a pure-Weyl calculation. A solution of a truncated EFT equation is not automatically a controlled approximation to a UV-complete theory.

## 6. Reproduction ladder

### P0 — Kerr and chart controls

- Recover exact Kerr horizon, surface gravity and angular velocity.
- Compare scalar invariants between Boyer–Lindquist and ingoing Kerr charts.
- Check the extremal limit with exact formulae where available.
- Demonstrate that chart-coordinate divergences do not become scalar-invariant false positives.

### P1 — dCS and sGB external controls

- Reproduce selected finite-spin backgrounds from the 13 November PRD study.
- Continue fixed-kappa and fixed-Omega_H families independently.
- Reproduce the dCS deformed NHEK limit and its SO(2,1) x U(1) symmetry diagnostics.
- Reproduce the sGB logarithmic scalar behaviour and near-horizon boundary-layer trend.
- Compare scalar invariants and infalling tidal components independently.
- Match gauge, couplings, normalization, chart, observer frame and boundary conditions.

### P2 — high-spin scalar QNM controls

On the same dCS/sGB backgrounds, reproduce selected scalar test-field modes from the 22 September PRD study:
- chi = 0, 0.5, 0.9, 0.95 and 0.99 where reference data permit;
- l=m=0 and selected higher multipoles;
- complex frequencies, not only real parts;
- independent angular/radial residuals and spectral convergence;
- separate background uncertainty, perturbation-solver error and EFT truncation.

A pure-Weyl perturbation operator is a different theory. It cannot be declared matched to dCS/sGB QNM numbers.

### P3 — pure Weyl and general higher-curvature branches

Only after the general Bach/equation and chart contracts are qualified:
- compute the same invariant and observer-frame atlas for a declared pure-Weyl action;
- include horizon-regular and asymptotic domains;
- test whether the branch is physically EFT-controlled (if an EFT interpretation is asserted);
- report branch existence/uniqueness evidence separately from stability and observability.

## 7. Negative controls

1. Schwarzschild-coordinate horizon singularity must not be called a curvature blow-up.
2. A finite scalar-invariant / divergent tidal-frame fixture must fail the observer-frame gate.
3. Change infalling energy/angular momentum and preserve the observer label in the result.
4. Swap fixed-spin and fixed-surface-gravity continuation; the branch manifest must change.
5. Omit an extra-field derivative operator from the EFT estimate; the validity certificate must fail closed.
6. Mutate coupling units or normalization; preflight rejects before solving.
7. Extrapolate a finite-spin sequence to exact extremality; status remains extrapolated, not exact.
8. Under-resolve a known near-horizon boundary layer; full-domain residual/regularity gate fails.
9. Present dCS/sGB QNM data as a pure-Weyl matched benchmark; theory identity check rejects it.
10. Mutate spectral coefficients while keeping only collocation residuals small; held-out/full-domain residual gate catches it.

## 8. Benchmarks and reporting

Report a multidimensional Pareto surface:
- maximum invariant and observer-frame tidal magnitudes vs kappa_H M;
- extra-field gradient norms and boundary-layer width;
- local EFT operator ratios and estimated next-order remainder;
- QNM complex-frequency discrepancy and residuals;
- spectral resolution and convergence rate;
- interval/ball enclosure widths where available;
- wall time, peak memory, precision, subdivisions and energy where measurable.

Do not collapse regularity, EFT validity, numerical accuracy and observations into one score.

Prospective targets:
- reproduce published dCS/sGB control curves within their reported uncertainties;
- preserve separation between scalar and observer-frame diagnostics;
- show tetrad/chart convergence on at least three refinement levels;
- report a declared validity region and explicit abstention outside it;
- match selected scalar QNM modes at or below the published mode-specific error where the same background/mode is used;
- zero accepted cross-theory mismatches or mutated boundary/observer conventions.

These are proposed gates, not current Symtropy results.

## 9. Evidence artifact

Every result should emit a versioned record containing:
- action and equation digest;
- coupling definitions and dimensions;
- source/background solution digest;
- M, J, chi, kappa_H M, Omega_H M and horizon data;
- chart, gauge, signature and coordinate ordering;
- observer congruence and tetrad manifest;
- invariant/tidal/extra-field values and enclosures;
- EFT truncation estimates and omitted-operator policy;
- spectral residual, truncation, conditioning and boundary diagnostics;
- QNM operator, mode, frequency convention and residuals;
- exact software/dependency version, command and exit status.

## 10. Claim ceiling

A passing EFT-validity map is restricted to the named action, branch, coupling/spin domain, observer family, chart and truncation order. It does not prove global stability, UV completion, astrophysical formation or empirical viability. dCS/sGB controls do not establish any property of pure Weyl gravity.
