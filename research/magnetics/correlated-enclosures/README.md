# Correlation-preserving magnetic functional enclosures (v0.1)

**Research issue:** [#7739 — MAG-CORRELATED-CERTIFICATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/7739)

## Research question

Can we preserve physically justified dependencies between magnetic field components, sensor channels and uncertainty sources to obtain materially tighter phase/amplitude certificates than component-wise intervals, without weakening conservative containment?

The component-wise outward-rounded interval pipeline remains the baseline and fallback. A sharper representation is useful only if it proves the same decision on the same input set and preserves every relevant physical and provenance constraint.

## Why dependency matters

The sensor transfer currently has the form:

`p = H B + o + e`

with interval physical field `B`, interval gain/cross-axis matrix `H`, offset `o` and residual `e`.

Component-wise intervals generally do not preserve the fact that multiple values may share one physical cause: for example, the same rotor eccentricity or probe registration offset affects multiple field components; a common gain-calibration term affects both channels; a temperature record may couple ring remanence, sensor gain and channel offset. Naively treating each component as independently variable may create impossible combinations and over-wide phase/amplitude ranges.

This motivates correlation-preserving enclosures, but a shared generator is admissible only when backed by a declared common-cause identity and bounded source evidence. Statistical correlation/covariance must not be silently reinterpreted as a deterministic joint bound.

## External formal reference

Fabian Immler's *Affine Arithmetic* entry in the Archive of Formal Proofs (26 October 2026) formalizes affine forms as zonotopes; provides affine operations and over-approximations for nonlinear multiplication/division; translates expressions to executable affine approximations; and verifies a zonotope/hyperplane intersection algorithm.

- Entry: https://isa-afp.org/entries/Affine_Arithmetic.html
- Formal document: https://isa-afp.org/browser_info/current/AFP/Affine_Arithmetic/document.pdf

This is a mathematical/formal reference, not a verification result for this repository. A Rust implementation would need its own soundness argument, exact representation mapping, floating-point outward-rounding audit, nonlinear remainder controls and adversarial tests.

## Candidate representation ladder

| Method | Correlation model | Role |
|---|---|---|
| Component intervals | None beyond each scalar range | Current correctness baseline |
| Centred mean-value form | Local derivative bound over the exact cell | Low-cost first improvement |
| Affine forms / zonotopes | Shared bounded generators | Preserve linear/common-cause dependence |
| Taylor models | Polynomial dependence plus interval remainder | Higher-order nonlinear phase/transfer enclosures |
| Hybrid escalation | Cheapest sound method first; subdivide/escalate when unresolved | Candidate production architecture |

Do not jump directly to a high-order Taylor engine. First measure where the current interval widths are generated and whether the dependencies have physical support.

## Frozen challenge suite

Each method receives the same frozen artifacts and must emit a certificate with the exact same scope:
- analytic point-dipole and finite-cuboid vector-field cases;
- independent FEM and Magpylib reference comparisons;
- physical field-discrepancy cells with residual Jacobian supremum bounds;
- bounded 2x3 sensor transfer, gain/cross-axis coupling, offsets and residual response;
- independent angular-reference phasors and their own calibration uncertainty;
- near-zero amplitude, phase branch-cut, threshold-touching, dropout and cross-axis adversarial cases;
- shared perturbations for eccentricity, standoff, tilt, temperature, calibration gain and coordinate registration where independently justified.

Required case metadata: immutable case ID, domain/cell/frame, units, geometry/material, solver/model identity, physical-measurement references, sensor and frontend, calibration source, reference instrument/calibration, operating condition, evidence roles, input digests, generator/source identities, and sealed holdout marker.

## Non-negotiable correctness gates

1. **Containment:** every analytic result is enclosed by the candidate certificate; independently sampled points are useful falsification probes but do not, by themselves, prove whole-set containment.
2. **No false-safe:** zero false-safe decisions on preregistered sealed analytic and physical challenge suites.
3. **Monotonic uncertainty:** enlarging an input set cannot make its certified lower functional margin stronger or convert unresolved to safe.
4. **Identity closure:** cell, coordinate frame, geometry/material, solver, sensor, calibration, reference instrument, operating condition and evidence role must match exactly.
5. **Bounded reduction:** symbol reduction, Taylor truncation, linearisation and model simplification require explicit remainder bounds.
6. **Fail closed:** unsupported operations, missing generator semantics and unbounded remainders are unresolved/errors; no silent point estimate.
7. **Determinism:** generator ordering, truncation, subdivision and serialization are deterministic and replayable.

## Metrics

Report separately:
- phase margin and amplitude-reserve enclosure width;
- component phasor bounds and joint-set diameter;
- safe, failing and unresolved cell count/volume;
- false-safe and false-failing counts on holdout cases;
- number of generators, truncation remainder, Taylor order, subdivision depth;
- runtime, peak memory and certificate size;
- number and cost of physical measurements required to reach the same decision.

Do not optimize a weighted score that hides false-safe results or amplitude dropout.

## Research targets (not achieved)

- At least 2x median tighter phase-margin enclosure than component intervals on a preregistered correlation-heavy suite.
- 100% analytic containment and zero false-safe classifications.
- No increase above 2x runtime/memory in the first useful tranche; report the full tightness/cost Pareto curve.
- Eventually at least 2x fewer physical observations than uniform mapping at the same zero-false-safe criterion on a sealed hardware holdout.

A candidate that fails the zero-false-safe/containment gate is rejected even if its enclosures are narrower or faster.

## Staged plan

- C0 — freeze common-cause / generator / uncertainty semantics and reference cases.
- C1 — establish interval baseline and property-based containment tests.
- C2 — add centred mean-value enclosures using the existing derivative-supremum contract.
- C3 — prototype affine forms/zonotopes with provenance-bound shared generators.
- C4 — investigate validated Taylor models for nonlinear sensor and phase expressions.
- C5 — hybrid subdivision and representation escalation coupled to active metrology.

## Claim ceiling

No correlated uncertainty model, narrower certificate, physical efficiency gain, Isabelle verification of our implementation or SOTA improvement is claimed by this specification.