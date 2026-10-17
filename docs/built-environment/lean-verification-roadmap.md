# Lean-backed verification roadmap for Symtropy

- **Status:** proof-of-concept proposal; source proofs remain provisional until the exact-head Lean build and axiom-audit gates pass.
- **Scope:** public repository only; initial focus on elementary analytical housing reference equations.
- **Non-goal:** claiming Lean verifies the Rust implementation, FEM solvers, physical assumptions, consciousness-physics hypotheses, code compliance, or building safety.

## Paradigm shift: proof-carrying engineering evidence

A numerical benchmark supplies sampled evidence. A Lean theorem supplies a machine-checked implication from stated assumptions. A Rust test supplies evidence about the implementation. A solver study supplies convergence evidence. Physical measurements supply validation evidence. These are complementary and non-substitutable.

For every assurance claim, build an evidence chain with:

1. **Mathematical contract:** named definitions, explicit assumptions, units/dimensions, theorem identifiers, Lean version, Mathlib revision, source digest, and build result.
2. **Implementation link:** exact Rust function/crate revision, mapping from Rust inputs/outputs to the formal contract, generated known-answer vectors, and differential/property-test results.
3. **Numerical budget:** floating-point format, rounding assumptions, input conditioning, expected error bound, observed residual, and solver tolerance. Never reuse a real-arithmetic theorem as a floating-point guarantee.
4. **Physical/model boundary:** applicable geometry, material/constitutive model, loads, boundary conditions, validation data, uncertainty and exclusions.
5. **Decision record:** exact intended use, permitted scope, reviewers, unresolved hazards, and explicit block reasons.

A hash establishes identity, not correctness or execution. A proof establishes a consequence of assumptions, not the truth of those assumptions. A passing analytical comparison does not qualify an arbitrary FEM model.

## Initial Lean slice

The package under formalization/lean defines five closed-form equations:

- Rectangular second moment of area, I = b h^3 / 12.
- Uniform axial bar extension, delta = F L / (E A).
- Cantilever tip deflection, delta = F L^3 / (3 E I).
- Simply supported midpoint-load deflection, delta = F L^3 / (48 E I).
- Simply supported uniform-load deflection, delta = 5 q L^4 / (384 E I).

The catalogue at formalization/lean/contracts.json maps five contract IDs to 33 unique contract-linked theorem names, seven SI-unit vector theorem anchors, and one negative-control theorem. Alongside positivity, scaling and six exact-rational known answers, it records eight dimensional-homogeneity identities and five dimensionless groups. The seven-base exponent layer proves dimension equations, but the numerical functions still accept untyped real numbers; Rust linkage and benchmark ingestion remain explicitly unestablished.

## Program of work

### F0 — Reproducible proof environment
- Pin the Lean release and immutable Mathlib commit.
- Build in CI on every relevant pull request.
- Validate contract catalogue syntax and theorem-name mapping.
- Audit the Symtropy theorem namespace for unapproved axioms and proof placeholders.
- Preserve exact-head CI evidence; queued or skipped is not passed.

### F1 — Formal analytical benchmark contracts
- Promote the candidate links to the pinned axial-bar, cantilever, and simply-supported-beam fixtures only after housing-evidence PR #1653 is merged and the fixtures are present on the target branch.
- Extend coverage to reactions, bending moments, stress, geometry-derived section properties, sign conventions, and dimension analysis.
- Give every theorem an explicit assumption list and stable contract ID.
- Add counterexamples showing which preconditions matter; invalid physical inputs must produce an explicit rejection/block, not a plausible number.

### F1.5 — Dimensional homogeneity
- Check all supported closed-form equations against seven SI base dimensions represented as integer exponent vectors; require every declared unit label to resolve to a canonical vector and theorem anchor.
- Keep a published negative-control corpus; for example, removing the cantilever's cubic length term must be rejected as dimensionally inconsistent.
- Extend to thermal, fluid, energy and dynamics formulas only through explicit base-dimension definitions and reviewed unit mappings.

### F1.75 — Dimensionless invariants and scale similarity
- Use the five declared Pi-like groups (section shape factor, axial strain, and three normalized beam deflections) as cross-scale metamorphic contracts.
- The companion Rust reference tests on PR #1653 exercise five scale factors [0.5, 0.75, 1.25, 2, 4] for axial bars, cantilevers, and both simply-supported load families. These are authored tests; they are not counted as passed until CI reports the exact head successful.
- Under the explicitly stated geometric-similarity transform, require normalized deflection/strain and stress invariance; point force scales as s^2, distributed load intensity as s, area as s^2, second moment of area as s^4, bending moment as s^3, and reaction as s^2.
- Add sensitivity controls that deliberately remove required length powers from the axial, cantilever, and simply-supported formulas and require the normalized similarity checks to detect each mutant; the simply-supported control covers both midpoint point-load and uniform distributed-load cases.
- These Rust controls are authored but remain unverified until the exact-head Rust CI run passes. Report valid cases and killed mutants separately; do not combine them into one pass rate.
- Do not transfer these idealized linear-model similarities to nonlinear, buckling, thermal, fire, fatigue, material-scale, or other physics without additional assumptions and independent validation.

### F2 — Rust refinement/conformance
- Candidate formula mappings now exist in `formalization/lean/contracts.json` for `symtropy_housing_evidence::structural_reference` on housing-evidence PR #1653. The source is a shared analytical reference kernel, not the production FEM solver; mappings are marked candidate/not-merged and refinement remains NOT_ESTABLISHED.
- The kernel exposes validated private-payload SI quantity newtypes (`Meters`, `Newtons`, `Pascals`, `NewtonsPerMeter`, `SquareMeters`, `FourthMomentOfArea`, `NewtonMeters`) rather than raw f64 inputs. This makes quantity-category mistakes compile-time errors, while still requiring callers to normalize data to SI before construction. A `compile_fail` API example documents that intended type-safety property.
- The three housing fixture integration tests now invoke this shared kernel, convert parsed SI magnitudes to validated quantity types, compare outputs with declared oracle values, and retain geometry-consistency checks. Kernel functions have explicit inputs, outputs, domain checks and error branches.
- Next, freeze the candidate source/function map at a specific merged Rust revision, emit canonical input/output vectors, and prove/check the mapping from Lean's real-number specification to the Rust f64 implementation. Do not claim a verified refinement merely because tests call the same kernel.
- Run a pinned Rust-to-Lean extraction compatibility spike. [Aeneas/Charon](https://github.com/AeneasVerif/aeneas) currently documents Lean 4.35.0, matching this project's pin; check whether its supported subset faithfully covers the kernel's f64 operations before relying on it. [Hax](https://github.com/cryspen/hax) documents a Lean backend with float support but currently pins a different Lean version, so evaluate it in a separately pinned package rather than mixing toolchains.
- The [May 2026 Rust-to-Lean pipeline experience report](https://arxiv.org/abs/2605.30106) documents practical extraction gaps and recommends extracting a small semantically equivalent kernel when production dependencies/generics are unsupported. Use that route only with reviewed equivalence evidence; never silently replace production code with an easier but behaviorally different model.
- Generate known-answer test vectors from the catalogue and test the actual Rust functions. The candidate kernel now covers all five contract families; exact linkage to Lean and floating-point refinement remains open.
- Add differential, metamorphic, property-based and mutation tests for each contract.
- Require exact mapping from formal arguments to Rust fields and units; missing or unknown quantities block conformance.
- Keep the mathematical proof, implementation test, analytical fixture, and execution receipt as distinct artifacts in the evidence chain.

### F3 — Floating-point and algorithmic bounds
- Review [FloatLib](https://github.com/lean-dojo/FloatLib) first as a candidate numerical-semantics dependency: its current main branch pins Lean v4.35.0, matching this prototype, and documents IEEE binary/decimal formats, rounding, and error-bound machinery. Compare [FloatSpec](https://reservoir.lean-lang.org/%40Beneficial-AI-Foundation/FloatSpec) as an alternative. Before integrating either, audit theorem placeholders/axioms, dependency pins, license, performance, and compatibility with the intended Rust f64 behavior. Establish bounds for rounding, cancellation, overflow, underflow, and ill-conditioned inputs; define when values must be rejected or marked inconclusive.
- Move from identities over real numbers to a proved or bounded implementation relation only when the arithmetic model and compiler/runtime assumptions are explicit.
- For iterative solvers, prove algorithmic properties separately from empirical convergence tests and mesh/time-step refinement studies.

### F4 — Expanded domains, only after evidence gates
- Geometry and units; rigid-body and constraint invariants; energy bookkeeping; then selected nonlinear/thermal/structural cases with independent oracle data.
- Keep model assumptions and empirical evidence separately versioned. Do not infer empirical or regulatory validity from theorem count.
- Formalize consciousness/thermodynamic hypotheses as conditional mathematics only where definitions are precise; do not treat formalization as experimental confirmation.

## Proposed benchmark scorecard

Report exact numerators/denominators and revision identifiers; never use a single composite score to conceal a blocked gate.

| Gate | Initial acceptance target | Evidence |
|---|---|---|
| Proof build | 100% of committed Lean modules build on the pinned toolchain | exact commit + CI URL/conclusion |
| Proof hygiene | zero sorry/admit and no unapproved axioms in the Symtropy namespace | axiom-audit report |
| Contract coverage | 100% of declared theorem claims are mapped to one stable contract ID; zero duplicate theorem IDs | contract catalogue and CI mapping check |
| Rust conformance | 100% of approved known-answer cases pass and all seeded mutations are rejected or blocked | per-case results and exact Rust head |
| Property checks | all declared linear/cubic/quartic scaling metamorphisms pass over generated valid-domain inputs | reproducible seeds, counts and tolerances |
| Dimensionless similarity | 100% of declared valid scale-transform cases preserve the relevant Pi-like groups at s = 0.5, 0.75, 1.25, 2 and 4; 100% of the specific seeded missing-length-power mutants are killed | separate valid-case and mutation-case ledgers, exact inputs, output ratios, tolerances, and test revision |
| Rust extraction/refinement | 100% of pure-kernel functions are extracted without stubbed behavior; every proof has no `sorry` and is checked against the pinned Lean kernel | exact Rust head, extractor/toolchain hashes, generated Lean source digest, refinement theorem and axiom audit |
| Dimensional validity | zero unit-inconsistent contract bindings in the supported subset; every declared unit has a seven-base exponent vector | dimensional-homogeneity theorems, manifest mapping and CI consistency gate |
| Rust API units | 100% of supported formula arguments/results use explicit quantity-category types; no raw-f64 calculation entrypoint in the shared kernel | compile-time type distinctions, compile-fail example, exact Rust head and CI |
| Numeric honesty | every real-arithmetic contract used by floating-point code has an explicit error budget or stays labelled MATHEMATICAL_ONLY | error budget + conditioning analysis |
| Solver credibility | residual, equilibrium, refinement, solver-specific tolerances and uncertainty declared per benchmark | solver-specific report; no universal tolerance |
| Physical validation | validation data are independent of calibration where feasible and include measurement uncertainty | traceable validation dossier |
| Failure semantics | zero false passes in the published negative-control corpus; incomplete evidence is BLOCKED/UNSUPPORTED | mutation corpus and denominators |
| Reproducibility | exact Lean/Mathlib/Rust revisions, fixtures and result receipts are pinned | content digests and clean rerun |

## Research pointers

- [OpenAI math release](https://github.com/openai/math): a research corpus whose README distinguishes formalized from unformalized results. Import a specific result only after checking its Lean proof, dependencies, provenance and applicability; do not treat the release as a certified engineering library.
- [Mathlib](https://github.com/leanprover-community/mathlib4): reusable mathematical foundations; keep the dependency revision pinned.
- [LeanDimensionalAnalysis](https://github.com/ATOMSLab/LeanDimensionalAnalysis) and its [2025 paper](https://arxiv.org/abs/2509.13142): prior formal work on seven SI base dimensions and dimensional homogeneity; the upstream repository pins an older Lean release and is a research reference until compatibility is established.
- [Rust dimensioned](https://docs.rs/dimensioned/latest/dimensioned/): candidate compile-time unit system to evaluate for the Rust implementation.
- [Lean documentation](https://lean-lang.org/lean4/doc/): language and proof-development reference.
- [Aeneas/Charon](https://github.com/AeneasVerif/aeneas): safe Rust-to-Lean extraction path; current Lean backend pin matches Lean 4.35.0, but supported `f64` semantics must be checked before using it for the numerical kernel.
- [Hax](https://github.com/cryspen/hax): alternative Rust-to-Lean/F*/Rocq extraction tool. Its changelog records Lean float support, but the current Hax Lean library pins 4.31.0, so toolchain alignment is an explicit open dependency task.
- [Rust-to-Lean verification pipeline experience report (May 2026)](https://arxiv.org/abs/2605.30106): demonstrates kernel-checked refinement for selected production Rust and documents current extraction limitations.
- [FloatLib](https://github.com/lean-dojo/FloatLib): first candidate to evaluate for proved executable floating-point semantics and error bounds; version compatibility alone is not evidence of compatibility with Rust or of a verified Symtropy implementation.
- [FloatSpec](https://reservoir.lean-lang.org/%40Beneficial-AI-Foundation/FloatSpec): alternative formal floating-point project; inspect outstanding proof placeholders before reuse.

## Definition of done for this prototype

Do not call these theorems verified until the exact proposed head has a successful Lean build and axiom audit. Do not call the Rust implementation verified until a separately reviewed refinement/conformance bridge and implementation-side tests exist. Do not call the solver or building physically validated without solver-appropriate numerical evidence, empirical evidence, uncertainty treatment and qualified review.

## Shared Rust/Lean contract-vector bridge (candidate)

PR #1653 now authors a versioned vector corpus at `docs/built-environment/benchmarks/structural-reference-contract-vectors-v1.json` (candidate corpus commit `5f28fba1096e84b1ebd1fa83661ed70558b67ba3`), a Draft 2020-12 schema at `docs/built-environment/benchmarks/structural-reference-contract-vectors.schema.json` (candidate strict-schema commit `219f29a0a1ef3639077ad88e9d2b47a52dde3235`), and a Rust integration test at `crates/apps/symtropy-housing-evidence/tests/structural_reference_contract_vectors.rs`. The schema rejects unknown input/output fields and inconsistent load-case shapes (including both point-load and distributed-load parameters in one case); the Rust test validates it before evaluating vectors and includes seeded rejection controls. The five vectors map by stable ID to the five contract families in `contracts.json`: rectangular section, axial bar, cantilever tip load, simply supported midpoint force, and simply supported uniform load. Each of the 28 expected output fields is now mapped to an explicitly named Lean theorem in the manifest; 16 additional exact-rational theorem declarations cover section area, extreme-fiber distance, axial stress/reaction, beam moments, bending stresses, and support reactions. Four additional dimensional-homogeneity theorems now cover section area, axial F/A stress, bending M·c/I stress, and uniform-load maximum moment. Source-level reconciliation finds 61 unique theorem declarations and every manifest theorem name resolves. The Lean workflow now generates a temporary module and uses Lean `#check` against every manifest theorem identifier after building the project, rather than treating raw source-line counts as proof that commented-out names exist. None is counted as verified until the pinned Lean build and axiom audit succeed.

The manifest binding is explicitly `CANDIDATE_ON_OPEN_PR_1653_NOT_MERGED`; the test is authored but not executed here. Exact-current-head CI must compile and run the Rust test, and exact-current-head Lean CI must build/audit the theorem sources, before any run is called passing. A future conformance report should bind (a) the corpus Git revision and content digest, (b) the Rust source/binary identity, (c) the Lean source/toolchain revision, (d) per-vector actual/expected values, units, and tolerance verdicts, and (e) negative-control results.

This bridge improves traceability and makes regression drift easier to detect; it is not a proof that the Rust floating-point implementation refines the real-valued Lean specification. To establish a refinement argument, specify the input normalization, rounding/error bounds, overflow/underflow behavior, and output correspondence, then prove or independently check that argument. The vector corpus also shares a project-authored lineage with the formulas/fixtures, so an independent reviewer or separately implemented oracle remains important.
