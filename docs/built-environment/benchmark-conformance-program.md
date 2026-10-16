# Housing Engineering Benchmark and Conformance Program

- **Status:** proposed verification program, not solver qualification or building-safety certification.
- **Date:** 2026-10-16
- **Initial scope:** analytical cantilever and axial-bar references, followed by the first static-linear solver-adapter spike.
- **Rule:** predeclare every metric and acceptance threshold for the exact benchmark, output quantity, numerical method and intended use.

## Paradigm shift: from a green test to an evidence vector

A green test suite shows that a known set of examples behaved as expected. It does not establish that the examples cover the intended domain, that a solver is valid for another class of problems, or that a building is safe.

Evaluate separate questions: (1) contract integrity, (2) implementation verification, (3) numerical verification, (4) empirical validation, (5) uncertainty and decision fitness, and (6) qualified engineering acceptance. Evidence for one question must not silently satisfy another. A digest proves byte identity, not successful execution; agreement with an analytical formula establishes agreement with that formula, not real-world validity.

This program is informed by [NASA-STD-7009B](https://standards.nasa.gov/standard/NASA/NASA-STD-7009), currently listed as active, [NASA-HDBK-7009B](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009), and [ASME's VVUQ standards](https://www.asme.org/codes-standards/vvuq-standards). A [NIST report published on 5 October 2026](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing) argues for harmonizing uncertainty quantification and metrology and scaling credibility effort to model influence and decision consequence. These sources inform the proposal; this program is not claiming compliance with their standards.

## Benchmark levels

| Level | Question | Evidence required | Does not establish |
|---|---|---|---|
| B0 — Contract and negative controls | Does the verifier reject incomplete, contradictory, tampered or unsupported evidence? | Schema/semantic fixtures, mutation tests, unsafe-path controls, failed/blocked/unevaluated cases, missing mandatory claims | Correct physics |
| B1 — Analytical references | Does the implementation agree with known-answer cases? | Closed-form equations, assumptions, units, independent calculations, dimensional and property checks | FEM validity or physical adequacy |
| B2 — Numerical verification | Does the numerical method behave correctly over its declared domain? | Equilibrium/residual checks, mesh or timestep refinement, convergence studies, independent comparisons | Empirical validation |
| B3 — Empirical validation | Does the model agree adequately with independent observations? | Traceable measurements, measurement uncertainty, predeclared error metrics, validation data distinct from calibration where possible | Validity outside the assessed domain |
| B4 — Integrated housing workflow | Does the evidence chain remain correct across model changes and adapters? | IFC/unit/identity checks, dependency invalidation, cross-tool conformance, clean reruns, qualified review | Automatic statutory approval |
| B5 — Monitored deployment | Do predictions and installed outcomes remain consistent over time? | Commissioning, inspection/maintenance observations, drift and incident feedback, controlled change process | Universal safety for unobserved buildings |

The cantilever and axial-bar tests are B1 seeds, not a B2 FEM campaign, B3 validation, or B4 housing workflow. The axial-bar fixture and tests now exercise uniform axial stress, extension and support reaction, derive area from geometry, reject inconsistent declared area, and test mechanical scaling. The simply-supported beam remains an unimplemented next reference.

## Initial benchmark portfolio

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

These are properties of the stated idealized equations. They must not be applied blindly when self-weight, nonlinear response, shear deformation, support conditions or load definitions change. Tests should transform primitive inputs and recompute derived values, not leave contradictory derived fields behind.

## Gate-execution record and independent checking

Each invocation should record benchmark ID/version, case and artifact digests, solver/evaluator identity and exact executable/container digest, architecture/configuration, structured arguments, input digests, start/end times, exit/termination/convergence state, raw output digests, parser identity/version, output values and units, expected values, applied criteria, residuals, uncertainty and known limitations.

A future independent checker must reopen the result artifacts, validate schema and units, recompute the benchmark verdict against externally pinned criteria and report mismatches. It must not copy a runner's declared result string. An attestation can bind identity to a record; it does not establish physical truth or scientific correctness.

Required negative tests include a fake runner declaring PASS while output values are wrong; absent/stale/non-converged output; mismatched criteria; truncated result data; and a valid artifact hash paired with an incorrect numerical result. Until independent execution and result checking exists, the effective engineering result remains NOT_REPRODUCED.

## Sequenced implementation

1. Close B1 consistency gaps: enforce $c=h/2$, $I=bh^3/12$, finite outputs, explicit units and current scaling invariants.
2. Add axial-bar and simply-supported beam references with equations, assumptions, dimensional checks, positive controls and mutation controls.
3. Specify the gate-execution schema before implementing a runner: bind criteria/evaluator identities and record invocation, termination and raw outputs.
4. Implement an independent checker; test a fake runner that claims pass for wrong output.
5. Run a solver spike as a separate process; pin the solver build and license path, parse outputs independently, check equilibrium, compare references and perform mesh refinement.
6. Establish B3 validation using traceable physical observations not used for calibration, including measurement uncertainty and a bounded validation domain.
7. Build a versioned cross-tool conformance corpus with identical cases, expected outcomes and reports across supported tool/OS builds.
8. Require independent engineering review before extending scope: assumptions, validity domain, uncertainty, thresholds and evidence gaps.

## Stop conditions

Fail closed when benchmark/criteria are unpinned; required inputs are absent; units are missing or inconsistent; solver exits unsuccessfully, times out or does not converge; outputs are absent/non-finite; equilibrium/reference checks fail; artifacts do not match declared bytes; independent checking is unavailable; or the evaluation leaves its declared validity domain.

A green B0/B1 suite must never be described as solver validation, building-code compliance, safety certification or construction readiness.

## References

- [NASA-STD-7009B — Standard for Models and Simulations](https://standards.nasa.gov/standard/NASA/NASA-STD-7009)
- [NASA-HDBK-7009B — Implementation Guide](https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009)
- [ASME — Verification, Validation and Uncertainty Quantification](https://www.asme.org/codes-standards/vvuq-standards)
- [NIST — Harmonizing uncertainty quantification and metrology practices (5 October 2026)](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing)
- [NIST — Reference benchmarks for verification and validation of multiphysics simulations](https://www.nist.gov/publications/b-c-engineering-statistics-and-reference-benchmark-approach-verification-and-validation)
