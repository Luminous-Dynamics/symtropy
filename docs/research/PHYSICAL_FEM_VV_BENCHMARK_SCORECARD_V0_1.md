# External FEM Verification, Validation and Benchmark Scorecard v0.1

**Status:** research/benchmark protocol; no external solver is qualified by this document.  
**Current implementation baseline:** receipt/output byte verifier [#2307](https://github.com/Luminous-Dynamics/symtropy/pull/2307), normalized scalar reference comparator [#2312](https://github.com/Luminous-Dynamics/symtropy/pull/2312), and synthetic receipt-backed mesh-refinement assessor [#2327](https://github.com/Luminous-Dynamics/symtropy/pull/2327).  
**Open qualification gate:** [PHYS-EXTQUAL-001 #2321](https://github.com/Luminous-Dynamics/symtropy/issues/2321).

## 1. Claim discipline

Do not claim Symtropy is more accurate, faster, or more trustworthy than commercial or open-source FEM systems until a preregistered, matched benchmark produces evidence for that claim.

The near-term differentiator is not a new FEM algorithm. It is a solver-neutral, tamper-evident path from declared problem semantics through exact input/output bytes, independent references, equilibrium invariants, discretization studies, complete cost accounting and an explicit capability decision. This can compare actual results without mistaking solver banners, visually plausible plots, process exit codes or one close scalar for qualification.

Keep these statuses separate:

| Dimension | Question | A PASS does not establish |
|---|---|---|
| Contract/provenance | Are declared inputs, requests, receipts, executable/build identity and outputs bound to exact bytes? | Correct physics |
| Execution | Did the intended executable run within its declared resource envelope? | Numerical correctness |
| Reference agreement | Does the named quantity of interest meet a frozen, independent tolerance? | Accuracy of all fields |
| Equilibrium/invariants | Do independently declared loads and receipt-bound reactions satisfy tested balance identities? | Correct constitutive model |
| Discretization | Does the eligible mesh sequence behave within declared refinement assumptions and accuracy limits? | Physical realism |
| Physical validation | Does a specified model compare acceptably against independent measurements for its intended use? | Universal validity |
| Capability qualification | Is this exact solver build, adapter, formulation and use envelope authorized by the prior gates? | Permission outside that envelope |

Missing or incompatible evidence is UNRESOLVED, not zero and not PASS. Failed, divergent, malformed, timed-out and unsupported runs are first-class records and remain in the benchmark denominator.

## 2. Industry baseline and fair comparisons

| Reference point | Public evidence / strength | Lesson for Symtropy |
|---|---|---|
| NAFEMS benchmark programme | Independent finite-element tests spanning structural, thermal and vibration cases; analytical targets are used where feasible. [P18](https://www.nafems.org/publications/resource_center/p18/) and [linear-static benchmarks](https://www.nafems.org/publications/browse_buy/browse_by_topic/linear/p07/) | Use independent, well-defined cases instead of solver demonstrations. Review permissions before including published benchmark data. |
| Code_Aster | The current official site advertises 4,600 verification test cases; the archived v15 validation index lists 343 documents, showing that distinct catalogues/counting methods exist. [Current official site](https://code-aster.org/en), [archived v15 V6 validation index](https://code-aster.org/doc/v15/V6.html) | Mature systems maintain broad test catalogues; these figures are not directly comparable to NAFEMS benchmark counts or Symtropy's case count. Report case provenance, formulation coverage, independence, known-answer quality and mutation detection—not just a total. |
| NAFEMS code-verification guidance | NAFEMS says vendor QA commonly uses published benchmarks and encourages users to independently verify published results. [Code Verification](https://www.nafems.org/publications/code-verification/) | Reproduce published tests independently, freeze the input/source/expected values, and retain exact receipts and all negative controls instead of treating vendor-published verification manuals as the whole evidence story. |
| MOOSE | Publishes module-specific verification/validation reports and software-quality records, including solid mechanics. [V&V report](https://mooseframework.inl.gov/sqa/moose_vrr.html) | Qualify a named physics capability and its evidence, not an entire multiphysics framework with one badge. |
| ASME V&V 10 | ASME identifies V&V 10-2019 (R2025) as the current reaffirmed standard for verification, validation and uncertainty quantification in computational solid mechanics. [Official standard page](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics) | Use context-of-use-specific VVUQ, not only code verification or a solver-wide badge. Translate the framework into auditable project criteria without reproducing proprietary standard text. |
| NASA model/simulation practices | NASA-STD-7009B sets credibility practices and programme-defined acceptance criteria; NASA-HDBK-7009B (2026) is its implementation guide. [Standard](https://standards.nasa.gov/standard/nasa/nasa-std-7009), [handbook](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009) | Acceptance criteria and credibility evidence must be explicit for the intended use. |
| CalculiX candidate | Its documented mesh-refinement function is limited to specific element families and identifies load/constraint caveats. [Upstream documentation](https://github.com/calculix/cae/blob/master/doc/REFINE_MESH.html) | Pin adapter limits. A feature name like adaptive refinement is not evidence that every element/load/constraint combination is supported. |

These are capability/process reference points, not a numeric league table. Systems differ in element formulations, integration, constraints, solver algorithms, defaults and intended use; a fair comparison must freeze these differences or report separate profiles.

NIST's FEM accuracy work treats discretization/DOFs and element choices separately from geometry, material, load and model uncertainties. [NIST paper](https://www.nist.gov/publications/finite-element-method-solution-uncertainty-asymptotic-solution-and-new-approach). Mesh convergence alone cannot cover that broader uncertainty space.

## 3. Qualification ladder

Do not dispatch an external solver just because a capability manifest exists:

1. **Artifact contract:** canonical path confinement; exact request/receipt/deck/mesh/output digests; pinned source, executable, adapter and configuration identity; bounded process/resource policy.
2. **Analytical scalar check:** independently derived, rights-cleared case; exact observable identity and units; preregistered absolute/relative tolerances; malformed-result and fake-executable controls.
3. **Equilibrium check:** independently declared load resultants and coordinate frame; receipt-bound support reactions; force and moment residuals; explicit missing-data and zero-scale behavior. Track [PHYS-INVARIANT-001 #2341](https://github.com/Luminous-Dynamics/symtropy/issues/2341).
4. **Discretization check:** at least three eligible meshes; exact mesh-to-receipt input provenance; declared quantity of interest; report convergence and reference accuracy separately. Withhold Richardson/GCI estimates unless their assumptions hold.
5. **Independent replication:** second analytically distinct case and, where practical, a separately implemented solver formulation or independent oracle. Shared parsers/normalizers weaken independence and must be documented.
6. **Cost/error frontier:** compare uniform and adaptive refinement only after cases, observables, output precision and solver capability are qualified. Count meshing, estimator, remeshing, transfers, postprocessing, retries and failed runs.
7. **Measurement-based validation:** use independent experimental data with uncertainty and explicit intended use. This is separate from analytical verification and cannot be inferred from synthetic fixtures.

## 4. Starter benchmark corpus

Use independently derived or rights-cleared definitions at first. Freeze geometry, material assumptions, loads, boundary conditions, frames, quantity of interest, units, reference method, solver/adapter profile and tolerances before candidate runs.

| Case family | Primary purpose | Required observations | Key pitfall |
|---|---|---|---|
| Axial bar/truss under static load | Minimal equation, sign, unit and scalar-reference check | End displacement, support reaction, force balance | The theory and element formulation must match the declared kinematics. |
| Slender cantilever/beam | Displacement and moment sensitivity under refinement | Tip displacement, reaction force/moment, three-level series | Declare when Euler–Bernoulli assumptions apply; do not score a 3D solid against an incompatible beam idealization as exact. |
| Smooth-field patch or manufactured-solution case | Implementation/element consistency and known-answer convergence | Named field norm or quantity of interest, convergence trend | Manufactured forcing helps verify code but does not establish real-world validity. |
| Stress-concentration geometry with well-defined quantity | Local-resolution strategy | Preregistered integral/averaged or otherwise mathematically valid quantity; error per DOF and per second | Avoid peak stress at a mathematical singularity as a finite-tolerance target. |
| Deliberately unsupported/ill-posed case | Verify abstention and claim ceiling | UNRESOLVED, no dispatch or qualification | A successful process or plausible output must not override unsupported semantics. |

Keep development cases separate from confirmatory holdouts. Any case that informed policy parameters is no longer an untouched holdout. Do not inflate case counts by cloning cases, relaxing limits or tuning to the final test set.

## 5. Code-verification frontier and conditioning

Method of Manufactured Solutions (MMS) is a strong code-verification tool because the manufactured analytical solution can be mathematically complex without needing to represent a realistic experiment. Sandia's blind-test work reports that MMS can detect coding mistakes that alter observed order, while also demonstrating error classes that it does not expose: https://www.sandia.gov/research/publications/details/code-verification-by-the-method-of-manufactured-solutions-2000-06-01/ and https://www.osti.gov/biblio/759450.

A useful next research direction is to combine MMS with modified-equation analysis or local-truncation-error information where the formulation makes the analysis tractable. Sandia's local-truncation-error-informed verification work specifically motivates this because ordinary MMS may miss coding errors that are of the same order as the numerical method: https://www.sandia.gov/research/publications/details/local-truncation-error-informed-code-verification-2020-12-17/.

Proposed sequence:
1. establish an independently generated MMS solution, exact forcing and boundary data;
2. freeze expected convergence rates and use blind mutations;
3. inspect local residual/truncation-error structure, not only global observed order;
4. classify which injected mistakes are detected, which survive and which cases are out of scope;
5. keep this as code-verification evidence, separate from physical validation.

This is a research target, not a claimed feature of the current evaluator. It complements NAFEMS analytical benchmarks, receipt-bound equilibrium invariants, scalar observables and refinement studies rather than replacing them.

### Conditioning and input sensitivity are separate from residual size

A computed solution can have a small backward residual while having substantial forward error when the linear system is ill-conditioned. The numerical record should therefore include both a backward residual and an independently calculated forward error on known-answer cases; for a real solver, condition estimates must be interpreted within the solver/formulation and norm being assessed, not as universal error certificates. LAPACK's DGECON estimates reciprocal condition number through (RCOND=1/(\|A\|\,\|A^{-1}\|)): https://www.netlib.org/lapack/explore-html/d4/daf/group__gecon_ga4f9b830e19e12c7f082ddb497a57af18.html.

The new synthetic conditioning work [PHYS-TRUSS-CONDITION-001 #2497](https://github.com/Luminous-Dynamics/symtropy/issues/2497) / [draft PR #2498](https://github.com/Luminous-Dynamics/symtropy/pull/2498) exercises a two-degree-of-freedom reduced truss matrix near a mechanism. It is designed to report condition number, forward/backward error, fail-closed outcomes and response amplification for a controlled load perturbation. This fills a gap in fixed-output tests: a result can be arithmetically accepted yet be too sensitive to input changes for a particular engineering decision.

This remains a synthetic small-displacement, 2D pin-jointed-truss code-verification case. It is not an external FEM result, a probability distribution for loading uncertainty, a general sparse-matrix condition estimator, or physical validation. Transfer the principle to the FEM adapter only after its matrix/solver diagnostics and error semantics are independently understood.

### Interval enclosures: uncertainty is not a sensitivity multiplier

The reduced-truss extension [PHYS-UQ-TRUSS-001 #2500](https://github.com/Luminous-Dynamics/symtropy/issues/2500) / [draft PR #2501](https://github.com/Luminous-Dynamics/symtropy/pull/2501) propagates declared bounds for geometry, axial stiffness and load through a reduced 2x2 linear-elastic truss. It uses a geometry-specific determinant identity and refuses to issue displacement enclosures when strict positivity of the determinant cannot be established.

This is not a confidence interval: declared ranges are not probability distributions, and scenario bounds are not measured/calibrated uncertainty. Nor does a bounded corner grid prove interval soundness. Sound enclosures depend on arithmetic semantics, outward rounding and conformance tests; the independent interval-conformance gate remains a dependency. Record finite enclosure width and unresolved/abstention separately from reference error, mesh convergence and physical validation. A wide enclosure is evidence of low decision resolution, not a PASS.

## 7. Metrics and proposed targets

### Integrity and abstention
- Zero false PASS outcomes across the frozen adversarial corpus.
- 100% of scored quantities linked to exact input/output digests, case identity, units, coordinate frame, parser/normalizer revision and frozen acceptance rule.
- Identical inputs and pinned toolchain produce byte-identical machine-readable reports.
- Malformed, nonfinite, wrong-unit, wrong-frame, stale, missing-output, nonconverged and unsupported controls never become numerical PASS.
- Report unresolved/abstention rates as well as pass rates; never hide invalid runs from denominators.

### Numerical accuracy
- Report per-case absolute error and relative error only when its denominator is valid.
- Report force and moment residuals separately and predeclare the scale for normalized balance.
- Report every mesh level and its DOFs/elements. Withhold convergence order/extrapolation when mesh-ratio and sequence assumptions do not hold.
- Include threshold-sensitivity checks to prove a sign/unit/frame error cannot pass due to rounding.

### Compute efficiency
- Report wall time, CPU-seconds/CPU-hours, peak memory and GPU use separately from correctness.
- Plot error versus DOFs and total compute cost; report paired per-case cost to first meet the frozen quantity-of-interest tolerance.
- For later uniform-versus-adaptive work [PHYS-AMR-BENCH-001 #2331](https://github.com/Luminous-Dynamics/symtropy/issues/2331), the proposed stretch target is at least 25% lower median total CPU-seconds to the same eligible tolerance OR twice as many held-out cases meeting tolerance at equal compute, with no increase in false-pass/false-safe outcomes. This is a hypothesis to test, not an established result.

## 8. How to claim improvement over the state of the art

The primary objective is **more independently verified engineering accuracy per complete evidence and compute budget**, subject to a hard zero-false-PASS gate—not speed at the cost of validity.

A defensible first claim would be narrow: a specific pinned solver, adapter and case profile passed an independent, rights-cleared set of linear-static controls with measured error, force/moment balance, three-level refinement data, exact provenance, documented failures and compute cost. Do not say better than state-of-the-art FEM from that result.

The claim ladder:
- **Now:** contract tooling and synthetic test coverage; no external solver or physical result qualifies.
- **After solver execution:** a pinned backend passes named analytical verification cases.
- **After cross-checks:** a named capability has independent formulation/reference evidence across a declared envelope.
- **After experiments:** the named model/use has measured validation evidence and uncertainty treatment.
- **After matched benchmark:** a case-bounded superiority claim supported by paired data and uncertainty.

No stage implies the next one automatically.

## 9. References

- NAFEMS, [The Standard NAFEMS Benchmarks (P18)](https://www.nafems.org/publications/resource_center/p18/) and [Linear Static Benchmarks — Volume 1](https://www.nafems.org/publications/browse_buy/browse_by_topic/linear/p07/).
- NIST, [FEM Solution Uncertainty and Accuracy Assessment](https://www.nist.gov/publications/finite-element-method-solution-uncertainty-asymptotic-solution-and-new-approach).
- NASA, [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009) and [NASA-HDBK-7009B (2026)](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009).
- MOOSE, [Verification and Validation Report](https://mooseframework.inl.gov/sqa/moose_vrr.html).
- Code_Aster, [archived v16 validation collections](https://code-aster.org/doc/v16/).
- CalculiX, [mesh refinement documentation](https://github.com/calculix/cae/blob/master/doc/REFINE_MESH.html).
- Roache, [uniform reporting of grid refinement studies](https://doi.org/10.1115/1.2910291).

## Claim ceiling

This scorecard organizes verification and benchmark work; it is not a solver certification, physical-validation result, safety assessment, or permission to use a solver beyond its reviewed capability envelope.