# Housing Engineering Benchmark and Conformance Program

- **Status:** proposed verification program, not solver qualification or building-safety certification.
- **Date:** 2026-10-16
- **Initial scope:** analytical cantilever, axial-bar and simply-supported-beam references, followed by the first static-linear solver-adapter spike.
- **Rule:** predeclare every metric and acceptance threshold for the exact benchmark, output quantity, numerical method and intended use.

## Paradigm shift: from a green test to an evidence vector

A green test suite shows that a known set of examples behaved as expected. It does not establish that the examples cover the intended domain, that a solver is valid for another class of problems, or that a building is safe.

Evaluate separate questions: (1) contract integrity, (2) implementation verification, (3) numerical verification, (4) empirical validation, (5) uncertainty and decision fitness, and (6) qualified engineering acceptance. Evidence for one question must not silently satisfy another. A digest proves byte identity, not successful execution; agreement with an analytical formula establishes agreement with that formula, not real-world validity.

This program is informed by [NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/NASA-STD-7009), currently listed as active, [NASA-HDBK-7009B](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009), and [ASME's VVUQ standards](https://www.asme.org/codes-standards/vvuq-standards). A [NIST report published on 5 October 2026](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing) argues for harmonizing uncertainty quantification and metrology and scaling credibility effort to model influence and decision consequence. These sources inform the proposal; this program is not claiming compliance with their standards.

Interoperability needs its own conformance gate. A [buildingSMART International IDS survey published in 2026](https://www.buildingsmart.org/ids-survey-results-2026/) found that the same Information Delivery Specification (IDS) can yield different checking results in different tools; 70% of respondents wanted some form of checker/authoring software certification. For Symtropy, the practical response is not to assume every checker is interchangeable: publish an explicit supported subset, a shared test corpus, expected diagnostics, and per-version conformance reports. An unsupported condition must report `UNSUPPORTED` or `BLOCKED`, never silently pass.

## Benchmark levels

| Level | Question | Evidence required | Does not establish |
|---|---|---|---|
| B0 — Contract and negative controls | Does the verifier reject incomplete, contradictory, tampered or unsupported evidence? | Schema/semantic fixtures, mutation tests, unsafe-path controls, failed/blocked/unevaluated cases, missing mandatory claims | Correct physics |
| B1 — Analytical references | Does the implementation agree with known-answer cases? | Closed-form equations, assumptions, units, independent calculations, dimensional and property checks | FEM validity or physical adequacy |
| B2 — Numerical verification | Does the numerical method behave correctly over its declared domain? | Equilibrium/residual checks, mesh or timestep refinement, convergence studies, independent comparisons | Empirical validation |
| B3 — Empirical validation | Does the model agree adequately with independent observations? | Traceable measurements, measurement uncertainty, predeclared error metrics, validation data distinct from calibration where possible | Validity outside the assessed domain |
| B4 — Integrated housing workflow | Does the evidence chain remain correct across model changes and adapters? | IFC/unit/identity checks, dependency invalidation, cross-tool conformance, clean reruns, qualified review | Automatic statutory approval |
| B5 — Monitored deployment | Do predictions and installed outcomes remain consistent over time? | Commissioning, inspection/maintenance observations, drift and incident feedback, controlled change process | Universal safety for unobserved buildings |

The cantilever, axial-bar and simply-supported-beam suites are B1 seeds, not a B2 FEM campaign, B3 validation, or B4 housing workflow. The axial-bar fixture/tests exercise uniform axial stress, extension and support reaction, derive area from geometry, reject inconsistent area, and test mechanical scaling. The simply-supported beam suite covers both midpoint point load and uniform distributed load, with closed-form deflection, bending moment, stress and reaction oracles plus invalid-input and scaling controls. An initial result-checker prototype now compares hashed machine-readable outputs against these fixtures; the next major step is authenticating execution and then numerical/FEM verification.

## Initial benchmark portfolio

The machine-readable [structural-reference-contract-vectors-v1.json](benchmarks/structural-reference-contract-vectors-v1.json) corpus currently pins five vectors across the five mathematical contract families, is constrained by [structural-reference-contract-vectors.schema.json](benchmarks/structural-reference-contract-vectors.schema.json), and is exercised by `tests/structural_reference_contract_vectors.rs`, which validates the corpus against the Draft 2020-12 schema before numerical comparisons. Each vector has a stable `contract_id`, primitive SI inputs, explicit idealization assumptions, and expected outputs. This is a cross-contract regression surface—not yet an independent second oracle or a proved Rust-to-Lean refinement. Its contract-manifest linkage remains a candidate while PRs #1653 and #1691 are open; CI must run against the exact latest heads before conformance can be called passing.

| Case family | Independent oracle/check | Purpose |
|---|---|---|
| Rectangular section | $I=bh^3/12$ and $c=h/2$, from declared dimensions | Geometry/property consistency |
| Axial bar | Closed-form axial extension and stress under a uniform axial load | Load, stiffness and units |
| Cantilever beam | Tip displacement, support reaction and bending stress under explicit Euler–Bernoulli assumptions | Existing reference; load/span/material sensitivity |
| Simply supported beam | Point-load and uniform-load analytical cases | Boundary conditions and sign conventions |
| Equilibrium | Applied forces and moments compared with reactions | Missing loads/restraints and incomplete outputs |
| Numerical refinement | Same problem on systematically refined meshes | Convergence rather than a single lucky result |
| Unit metamorphisms | Equivalent physical inputs in alternate supported units | Silent conversion errors |
| Result-integrity negatives | Truncated output, NaN/Inf, missing fields, stale result, timeout, nonzero exit, non-convergence | Prevent failures from becoming passes |
| Evidence-contract mutations | Wrong digests, reused artifact digests, missing gates, weaker profile pin, duplicate IDs, altered criteria | Scope and integrity enforcement |

Add torsion, plates/shells, nonlinear materials, buckling, dynamics, fire, thermal/moisture and geotechnical cases only after defining the mathematical assumptions, reference data and validation envelope. The number of supported domains is not itself a progress metric.

## Metrics and proposed targets

These targets are staged engineering targets, not universal code limits. Any candidate numerical threshold must be approved for the specific benchmark before execution.

| Metric | Initial target | Interpretation |
|---|---|---|
| Known-answer agreement | Every analytical case matches its pinned oracle within its preregistered per-output tolerance. The existing cantilever fixture uses relative tolerance $10^{-12}$ for its elementary calculation. | Do not transfer this tight tolerance to FEM or measured data. |
| Geometry consistency | 100% of seeded contradictions between primitive dimensions and derived properties are rejected or blocked. | Includes both $I=bh^3/12$ and $c=h/2$. |
| Positive controls | 100% of approved baseline cases pass at the exact pinned revision. | Missing or skipped cases never count as pass. |
| Negative controls | 100% of the enumerated seeded-invalid cases are rejected or blocked; zero false acceptance in that corpus. Always report numerator/denominator. | Does not prove zero false acceptance for all possible inputs. |
| Contract coverage | 100% of profile-required inputs/gates accounted for; zero unresolved mandatory artifact references. | Does not prove the profile itself is complete. |
| Units | 100% of declared unit-conversion and equivalent-unit tests pass. | Normalize units explicitly; never infer units from magnitude. |
| Static equilibrium | **Candidate:** normalized force and moment residuals no greater than $10^{-8}$ for initial simple linear-static cases, subject to benchmark and solver review. | Define normalization/sign conventions per case; not a universal code limit. |
| Numerical convergence | **Candidate:** final successive-refinement change no greater than 1% for declared global outputs, with trend supported by enough meshes and a stated error model. | Singular/local stress values need separate treatment; one comparison is insufficient. |
| Independent solver comparison | Predeclared per-output tolerances. A 1% global displacement/energy target may be evaluated after convergence and output mapping are justified. | Do not compare non-equivalent formulations, stress locations or boundaries. A second solver is corroboration, not infallible truth. |
| Clean rerun | 100% of controlled reruns reproduce the verdict and artifact identities; numerical outputs agree within case-specific tolerances. | Pin environment and toolchain. Do not demand raw output byte identity from nondeterministic tools without justification. |
| Cross-tool conformance | For the explicitly supported shared subset, 100% of checker implementations agree on verdicts across the pinned corpus; zero unresolved disagreements. | Report tool/version and diagnostic differences; unsupported feature paths must fail closed rather than pass. This is conformance, not certification. |
| Uncertainty | Every high-consequence output declares input/measurement uncertainty, sensitivity assumptions, model-form limits and impact on the decision. | Set a project-specific uncertainty budget; no universal percentage. |
| Empirical validation | Predeclare error metrics and acceptance bands relative to measurement uncertainty; use independent validation data where possible. | Show residuals, outliers, parameter ranges and domain gaps; do not reduce everything to mean error. |
| Independent review | 100% of high-risk profiles and validity-domain statements receive explicit qualified review before external engineering use. | Identity, authority and signature trust are separate controls. |

Reports must expose coverage and exceptions, not just a single score: cases run/passed/failed/blocked/skipped, negative-control results, largest normalized residual, maximum known-answer error, refinement deltas, artifact completeness, uncertainty, and unresolved reviews.

## Metamorphic testing: check laws, not only examples

For the current idealized cantilever with section and material unchanged, elementary theory implies:

- Doubling point load doubles tip deflection and bending stress.
- Doubling span multiplies tip deflection by eight and maximum bending stress by two.
- Doubling elastic modulus halves deflection while leaving the elementary bending-stress result unchanged.
- Doubling section width doubles $I$, so formula-level deflection and stress halve.
- Doubling section depth multiplies rectangular-section $I$ by eight. If $c=h/2$ is updated consistently, formula-level deflection falls by eight and extreme-fiber stress falls by four.

The Rust reference suites now sweep load, span/length, elastic modulus, width and depth/height across six deterministic scale factors (0.25, 0.5, 0.75, 1.25, 2 and 4) and check the corresponding law for each output. They also test small interaction grids: cantilever load×span×modulus, axial-bar load×length×modulus×area, and simple-span load×span×modulus. These catch combinations that one-variable-at-a-time testing can miss, while keeping each case within the declared linear-elastic idealization. These are properties of the stated idealized equations. They must not be applied blindly when self-weight, nonlinear response, shear deformation, support conditions or load definitions change. Tests should transform primitive inputs and recompute derived values, not leave contradictory derived fields behind.

## Gate-execution record and independent checking

A first, deliberately bounded prototype is implemented in `crates/apps/symtropy-housing-evidence/src/benchmark_result.rs`, exposed as `--validate-benchmark-result`. Its input contracts are `benchmark-result.schema.json` and `benchmark-result-output.schema.json`, and its output contract is `benchmark-result-report.schema.json`. Every generated report is validated against its own versioned schema before it is returned. It binds benchmark/case identity, the exact benchmark digest, producer-declared status, evaluator identity, declared exit/termination/convergence state, and the SHA-256 of a machine-readable numeric-output artifact.

The CLI requires out-of-band SHA-256 pins for both the benchmark fixture and the checker executable. Its report records the actual checker binary digest and the two embedded schema digests, identifies the benchmark/case, and emits a per-quantity ledger with observed and expected values/units, absolute error, allowed error and tolerance verdict. The checker reopens and hashes the machine-readable numeric-output file, validates its schema, rejects duplicate or unknown quantities and unit mismatches, checks exact output coverage, and compares finite outputs against equation-derived analytical values using the fixture's case-specific tolerance. It also checks the fixture's declared expected outputs against the derived equations and rejects contradictory derived input fields (for example, an area or second moment of area inconsistent with primitive dimensions). Positive-control test data are computed from primitive inputs instead of copying the fixture's expected values, so stale or mutated expected values are detected. The prototype accepts only `ANALYTICAL_REFERENCE` records with `NOT_APPLICABLE` convergence; all `NUMERICAL_SOLVER` records are blocked until solver-specific acceptance profiles and authenticated execution are implemented. Because the three supported fixtures use elementary closed-form equations, the prototype also enforces a maximum relative tolerance of `1e-9` for their fixture oracles; solver/FEM and empirical data require separately reviewed tolerance profiles. For a supported fixture, a false `CLAIMED_PASS` with numerically wrong outputs yields `REFERENCE_MISMATCH`; a matching file digest does not rescue bad values.

The prototype is intentionally not a complete execution attestation or an independently built verifier implementation. Its checking logic is in the same executable as the other verifier commands, and the checker hash pin identifies that exact binary; an independently implemented second verifier remains a separate goal. It checks parsed numeric outputs, not arbitrary raw solver stdout/stderr, and does not authenticate that the named evaluator ran, bind the output to a structured invocation/environment/input closure, or establish that a parser faithfully extracted the tool's original output. Evaluator identity and execution/convergence metadata are still producer-declared. An attestation can bind identity to a record but does not establish physical truth or scientific correctness.

The next execution record revision should add exact argument vector, controlled environment, all input/configuration digests, OS/architecture, start/end time, raw stdout/stderr digests, parser binary/version, result-file digests, residuals and uncertainty. The existing negative tests cover a false declared pass, altered oracle value, inconsistent and overflowed derived inputs, deliberately weakened tolerance, unsupported and non-converged numerical-solver records, unpinned benchmark/checker, output tampering, path traversal, duplicate/missing output quantities, and unit mismatch. Remaining work includes absent/stale output, mismatched invocation/criteria, truncated raw solver output, parser provenance, unexpected process termination, and numerical residual/equilibrium checks. Until authenticated execution, solver-specific parsing/verification and qualified review exist, an engineering conclusion remains `NOT_REPRODUCED`.

## Sequenced implementation

1. Complete and verify the B1 reference suite: maintain geometry consistency, finite outputs, explicit units, and metamorphic invariants for cantilever, axial-bar and simple-span load cases. Exercise all five contract families through the shared versioned vector corpus; pin the exact corpus and Rust source revisions before generating a Lean/Rust conformance report.
2. Extend the benchmark-result record with a pinned structured invocation, complete input/configuration digests, controlled environment, timestamps, and raw stdout/stderr + parser provenance.
3. Add a versioned solver adapter process that emits these execution records and a separately testable parser. Pin the solver build and resolve licensing before packaging; a declared zero exit code is insufficient.
4. Run the solver spike against all three analytical families, check equilibrium and residuals, test negative controls, and perform mesh refinement. Keep the reference checker separate from the solver/producer path.
5. Establish B3 validation using traceable physical observations not used for calibration, including measurement uncertainty and a bounded validation domain.
6. Build a versioned cross-tool conformance corpus with identical cases, expected outcomes and reports across supported tool/OS builds.
7. Require independent engineering review before extending scope: assumptions, validity domain, uncertainty, thresholds and evidence gaps.

## Stop conditions

Fail closed when benchmark/criteria are unpinned; required inputs are absent; units are missing or inconsistent; solver exits unsuccessfully, times out or does not converge; outputs are absent/non-finite; equilibrium/reference checks fail; artifacts do not match declared bytes; independent checking is unavailable; or the evaluation leaves its declared validity domain.

A green B0/B1 suite must never be described as solver validation, building-code compliance, safety certification or construction readiness.

## References

- [NASA-STD-7009B — Standard for Models and Simulations](https://standards.nasa.gov/standard/NASA/NASA-STD-7009)
- [NASA-HDBK-7009B — Implementation Guide](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009)
- [ASME — Verification, Validation and Uncertainty Quantification](https://www.asme.org/codes-standards/vvuq-standards)
- [NIST — Harmonizing uncertainty quantification and metrology practices (5 October 2026)](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing)
- [NIST — Reference benchmarks for verification and validation of multiphysics simulations](https://www.nist.gov/publications/b-c-engineering-statistics-and-reference-benchmark-approach-verification-and-validation)
- [buildingSMART International — IDS survey results 2026: software conformance and certification](https://www.buildingsmart.org/ids-survey-results-2026/)
