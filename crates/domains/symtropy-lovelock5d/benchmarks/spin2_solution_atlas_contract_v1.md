# Symtropy 5D EGB solution atlas: evidence and qualification contract

**Status:** proposed research architecture. This is a data and proof contract, not a claim that a finite-coupling branch, interval certificate, complete branch catalogue, or hyperbolicity region has already been computed.

## 1. Paradigm shift: from profiles to a qualified moduli atlas

A numerical solver normally returns one or more metric profiles. That is not yet a useful description of the solution space: a profile may be a false root, a point on an unknown branch, a duplicate representation of another point, a singular endpoint, or a stationary solution whose time-dependent evolution equations are not hyperbolic.

Symtropy should therefore return a **solution atlas** rather than a directory of profiles. The atlas is a typed graph:

- A **node** represents a local solution germ at a declared physical parameter point and ensemble.
- An **edge** represents a continuation segment connecting compatible solution germs, together with a tangent, parameter interval, patch overlaps, and event information.
- A **certificate** is a separate object attached to a node or edge. It records exactly what was verified, over which domain, with what arithmetic, and under which assumptions.
- A **physical-admissibility record** is orthogonal to stationary-solution existence. It records the result of the full gravitational principal-symbol analysis for a stated gauge and background domain.

The key change is that a profile is no longer the deliverable. It is one data object inside an evidence graph whose claims are machine-readable and independently replayable.

## 2. Three independent qualification axes

Do not collapse these into one Boolean `valid` field.

### A. Local endpoint regularity

Record:
- horizon row valuations and row scalings;
- algebraic pivots and their positivity domain;
- reduced principal rank and determinant valuation;
- horizon residue and indicial polynomial;
- analytic free-data count and compatibility constraints;
- singular-branch logarithmic obstructions;
- infinity indicial modes and admissible/forbidden subspaces;
- overlap consistency between horizon, interior, and asymptotic charts.

This axis answers: *does the local differential system admit the declared regular endpoint germ, and how many independent local data does it carry?*

### B. Global stationary boundary-value problem

Record:
- complete original-equation residuals and radial Noether defect;
- endpoint mode counts at both boundaries;
- the boundary-data map, rank, nullity, and conditioning margin;
- the Fredholm index under an explicitly stated function space and operator domain;
- solution existence and local uniqueness certificate, when available;
- branch continuation and event enclosures.

This axis answers: *does the candidate solve the full declared stationary BVP, and is the solution locally isolated or part of a branch?*

### C. Physical spacetime admissibility

Record:
- background digest and approximation order (for example, spin-squared with an explicit higher-order remainder);
- full EGB principal symbol and physical polarization decomposition;
- gauge-fixed first-order evolution formulation;
- characteristic speeds, eigenbasis/symmetrizer margins, and uniformity over the declared domain;
- interval-covered or merely sampled spacetime/covector domains;
- constraint and gauge-mode separation.

This axis answers: *does the time-dependent gravitational system have the required well-posed evolution properties on this background?*

A point can pass A and B while C remains unknown or fails. A spin-squared background also cannot certify a finite-spin solution if omitted higher-order terms are as large as the hyperbolicity margin.

## 3. Minimum node contract

Each node should have a stable content digest and at least these groups of fields:

| Group | Required fields |
|---|---|
| Identity | `node_id`, schema version, parent atlas, source commit, creation receipt |
| Problem | action digest, equation-generator digest, equation-fixture digest, spacetime dimension, theory and sign conventions |
| Ensemble | fixed horizon/rotation or fixed charges, with every fixed quantity explicit |
| Coordinates | radial gauge, coordinate charts, horizon normalization, compactification, chart transformations |
| Parameters | dimensionful couplings, dimensionless couplings, horizon radius, spin/rotation parameter, approximation order |
| Horizon germ | row valuations, reduced residue, indicial polynomial, compatibility equations, free-data basis |
| Infinity germ | asymptotic exponents, source/response split, admissible subspace, boundary normalization |
| Candidate | profiles or coefficient representation, interpolation basis, spectral order, precision |
| Residuals | each original equation separately, constraint, Noether identity, horizon mismatch, infinity mismatch, held-out nodes |
| Observables | mass, angular momentum, angular velocity, area, temperature, entropy, first-law defect, declared uncertainties |
| Numerics | solver, initial guess, continuation settings, runtime, memory, hardware, all warnings and failed attempts |
| Qualification | independent status for endpoint, BVP, interval validation, branch, and PDE admissibility |
| Provenance | hashes of code, data, inputs, outputs, environment, verifier, and receipts |

Store exact rational expressions and symbolic fixtures where feasible. For numerical values, store the precision and rounding model; never serialize a decimal string without indicating whether it is exact, floating point, or an interval enclosure.

## 4. Edge contract: branch segments, not just point clouds

An edge should include:

- source and target node IDs;
- physical parameterization and ensemble;
- pseudo-arclength coordinate, tangent, and orientation;
- parameter interval and maximum step;
- predictor/corrector method, tolerances, and precision;
- patch-overlap and matching residuals;
- Jacobian/inverse bounds and transversality information;
- interval enclosures for the solution arc, when available;
- event labels and event brackets.

Supported event labels should include at least `turning_point`, `fold`, `branch_intersection`, `symmetry_or_mode_change`, `extremality_boundary`, `BVP_singularity`, `hyperbolicity_margin_zero`, and `unclassified`. An event is not classified merely because a plot looks folded: require a rank or transversality test and preserve `unclassified` when the evidence is insufficient.

Pseudo-arclength continuation is the default for passing folds. Deflation is a discovery aid for finding additional roots or disconnected branches. Neither proves the atlas complete. A completeness claim requires a separate global argument, such as degree/homotopy bounds, validated covering of the admissible solution set, or a theorem bounding all possible branches in the declared domain.

## 5. Evidence states: monotone, typed, and fail-closed

Use separate states rather than an overloaded status string:

- `symbolic_exact`: an identity was reduced exactly under explicit assumptions;
- `source_replayed`: the committed source bytes and exact inputs were replayed;
- `numeric_candidate`: a floating-point or arbitrary-precision candidate exists;
- `numerically_refined`: convergence was demonstrated across declared resolutions/precisions;
- `interval_validated`: residual, tail, and inverse bounds are enclosed on a stated domain;
- `existence_local_uniqueness`: a fixed-point/radii-polynomial or equivalent theorem certifies a nearby solution and local uniqueness;
- `continuation_validated`: a connected segment and its event boxes are enclosed;
- `sampled_only`: a survey passed at finitely many points, with no uniform-domain theorem;
- `inconclusive`: the proof or numerical test did not decide the question;
- `rejected`: a negative control or independent test falsified the candidate/claim.

A later failure must not erase the earlier receipt. Instead append a superseding status with the failed predicate and preserve the prior artefact. `queued`, `skipped`, `timeout`, missing status, and not-run are not passes. A timeout is not evidence of mathematical singularity.

Every certificate must state its claim ceiling and domain. For example, an exact horizon principal jet is not a full-exterior determinant theorem; a finite collocation residual is not a full-domain interval residual; a validated stationary BVP is not a strong-hyperbolicity theorem.

## 6. Proof-producing numerical pipeline

The recommended staged solver is:

1. **Exact problem builder:** produce action, equation, gauge, parameter, and normalization digests. Generate all original equations before gauge fixing; retain the radial Noether identity.
2. **Structure extractor:** eliminate only algebraic fields such as \(\widehat F_2\), derive principal and lower-order matrices, endpoint valuations, indicial data, and Noether dependencies.
3. **Endpoint-germ constructor:** generate horizon and infinity series with compatibility and independent-data counts; automatically reject inconsistent endpoint data.
4. **Patch solver:** solve on overlapping horizon/interior/infinity charts, matching both fields and derivatives. Use independent held-out nodes and the original equations to detect cancellation or scaling artefacts.
5. **Discovery engine:** natural-parameter continuation initially, then pseudo-arclength through folds; add deflation for additional branches. Keep all initial guesses, rejected roots, and failed starts in the receipt.
6. **Independent evaluator:** evaluate every unscaled original equation, the radial constraint, the Noether identity, and physical observables using a separate code path.
7. **Validated numerics:** enclose truncation tails and residuals, bound the inverse linearized operator, and apply a radii-polynomial or equivalent fixed-point theorem.
8. **PDE qualification:** build and qualify the full gauge-fixed EGB principal symbol independently of the stationary ODE pipeline.

The equation-aware reduction can lower the solve dimension. It must never remove the original field equations from the independent qualification stage.

## 7. Benchmark design: publish a Pareto frontier, not one score

### Discovery and refinement leaderboard

Report:
- time to first candidate and time to source-matched residual;
- success rate over a frozen set of initial guesses;
- per-equation \(L^\infty\), weighted \(L^2\), and ordinary \(L^2\) residuals;
- observable error against an independent reference;
- work-precision curves across at least three spectral resolutions and multiple arithmetic precisions;
- branch/parameter coverage, missed events, duplicate roots, false candidates, and rejected attempts;
- peak memory, CPU/GPU model, wall time, energy where measurable.

### Qualification and proof leaderboard

Report:
- exact-control recovery and negative-control rejection;
- endpoint mode-count and boundary-map correctness;
- full-domain interval residual and spectral-tail bound;
- approximate-inverse and operator conditioning bounds;
- existence/local-uniqueness certificates;
- validated continuation length, branch event boxes, and parameter coverage;
- physical characteristic and strong-hyperbolicity margins over an explicit domain.

Do not combine these into a single scalar score. A method may be faster but less reliable, discover more candidates but certify fewer, or certify one local branch without proving completeness. Publish the Pareto frontier and the complete run ledger.

## 8. Baselines and fair comparisons

Use four distinct baselines:

1. **Published neural-spectral discovery:** reproduce Akribeia using the authors' tagged code and fixed conventions where possible.
2. **Classical spectral/Newton:** direct Chebyshev collocation with horizon/asymptotic series and Newton/Gauss–Newton refinement.
3. **Continuation/deflation:** AUTO-07p or BifurcationKit-style branch discovery, with identical equation and boundary data.
4. **Validated numerics:** Chebyshev/radii-polynomial or interval-Newton qualification on the same reduced BVP.

The current Symtropy O(a²) slow-rotation system and Akribeia's full nonlinear finite-spin profiles are not a direct same-problem comparison. First compare perturbative coefficients extracted from multiple decreasing-spin full solutions, then build a full nonlinear Symtropy solver for direct finite-spin comparison. Freeze the fixed-(\(r_H,\Omega_H\)) and fixed-(\(E,J\)) ensembles separately.

## 9. Initial acceptance ladder

| Gate | Required evidence | Claim ceiling |
|---|---|---|
| A0 | exact Myers–Perry and static Boulware–Deser controls; sign and gauge negative controls | exact limits only |
| A1 | clean source replay, independent equation evaluation, endpoint mode counts | source-replayed perturbative problem |
| A2 | at least one finite-coupling slow-rotation BVP, three resolutions, precision escalation, all original rows | numerically refined candidate |
| A3 | multi-spin coefficient extraction against full nonlinear reference, expected truncation scaling | matched perturbative coefficient |
| A4 | global boundary-map rank/Fredholm analysis and branch continuation/deflation | discovered branch topology, not completeness |
| A5 | full-domain interval residual, tail, and inverse bounds | validated local BVP solution |
| A6 | radii-polynomial or equivalent existence/local-uniqueness theorem | certified nearby exact solution |
| A7 | validated continuation arcs and event boxes | certified branch segments |

Keep physical hyperbolicity on a second ladder: H0 principal-symbol derivation; H1 pointwise physical roots/eigenvectors; H2 adaptive exterior/covector sampling; H3 local interval boxes; H4 full exterior coverage; H5 a formulation-level strong-hyperbolicity theorem. Do not infer H-level passes from A-level passes.

## 10. What would constitute a genuinely field-shifting result?

A strong paper would not merely show one profile with more digits. It would deliver:

1. an exact reduced system with a proved endpoint mode count;
2. a matched finite-coupling numerical branch with independent residual and observable evaluation;
3. a validated existence/local-uniqueness certificate for at least one nontrivial rotating higher-curvature solution;
4. validated continuation through a nontrivial branch event, or a rigorous exclusion region;
5. an independent physical hyperbolicity result on a declared domain;
6. a public machine-replayable atlas in which every claim points to inputs, algorithms, and proof receipts.

That would change the output of higher-curvature black-hole numerics from “a plausible family of profiles” into “a branch with local proof, global coverage statement, and physical-admissibility status.” The strongest novelty claim should attach to the mathematical and physical results actually established, not to the software architecture alone.

## References

- Agurto-Sepúlveda et al., *Neural-Spectral Discovery of Rotating Black Holes Beyond General Relativity*, accepted PRD 12 November 2026, [DOI](https://doi.org/10.1103/3kz2-6dvv), [arXiv:2607.07418](https://arxiv.org/abs/2607.07418).
- Lessard and Reinhardt, *Rigorous Numerics for Nonlinear Differential Equations Using Chebyshev Series*, SIAM J. Numer. Anal. 52 (2014), [DOI](https://doi.org/10.1137/13090883X).
- [BifurcationKit pseudo-arclength continuation](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/PALC/) and [deflated continuation](https://bifurcationkit.github.io/BifurcationKitDocs.jl/dev/DeflatedContinuation/).
- Reall, Tanahashi and Way, *Causality and Hyperbolicity of Lovelock Theories*, CQG 31 (2014), [arXiv:1406.3379](https://arxiv.org/abs/1406.3379).
- Papallo and Reall, *On the local well-posedness of Lovelock and Horndeski theories*, PRD 96 (2017), [DOI](https://doi.org/10.1103/PhysRevD.96.044019); Kovács and Reall, *Well-posed formulation of Lovelock and Horndeski theories*, PRD 101 (2020), [DOI](https://doi.org/10.1103/PhysRevD.101.124003).