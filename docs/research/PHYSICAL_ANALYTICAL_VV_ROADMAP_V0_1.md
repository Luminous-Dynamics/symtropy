# Physical Analytical Verification and Validation Roadmap v0.1

**Scope:** Symtropy physical transformation / analytical adapter  
**Related:** [PHYS-ANALYTICAL-001 #2284](https://github.com/Luminous-Dynamics/symtropy/issues/2284), [PHYS-UQ-001 #2292](https://github.com/Luminous-Dynamics/symtropy/issues/2292)  
**Status:** research and acceptance plan; no industry-superiority claim is made.

## The central paradigm shift

A mechanics engine should not emit one universal claim named PASS. It should identify precisely which claim passed:

1. **Contract validity:** input is well-formed and inside a declared request vocabulary.
2. **Equation verification:** a known analytical solution or independent oracle agrees within an explicit tolerance.
3. **Numerical verification:** a discretized solver converges and satisfies equilibrium/residual checks for an applicable case.
4. **Model validation:** predictions agree with independent observations for a named material, geometry, load, boundary and environmental envelope.
5. **Calibration:** parameters have been fit on an identified calibration set.
6. **Uncertainty qualification:** known uncertainty sources and their effect on outputs are quantified.
7. **Engineering acceptance:** a suitably qualified engineer determines whether evidence supports the intended use.

These are separate gates. Passing one does not satisfy the next. The current analytical adapter qualifies only a narrow subset of gate 2: three idealized closed-form cases. It is not a general finite-element solver and does not validate a real component or building.

## Current implementation and benchmark facts

The reference crate implements an axial bar, a cantilever with a tip point load, and a simply supported beam with a full-span uniform load. The adapter calls those existing functions rather than duplicating the formulas. Exact-head CI checks known-answer values, unsupported-fracture rejection, deterministic receipts, and metamorphic scaling laws for force, span, area, modulus, inertia and outer-fiber distance.

The next comparison should be matched by capability, not marketing language:

- **Commercial verification catalogs:** SOFiSTiK's 2026 verification manual publishes benchmark cases spanning linear beam stresses/deflections, trusses, nonlinear plasticity, geometric nonlinearities and buckling, dynamics/modal analysis, soil/continuum cases, and fire effects. Its documentation includes reference solutions and benchmark input files. Symtropy's three analytical cases are a necessary foundation, not a replacement for that breadth. Source: [SOFiSTiK VERiFiCATiON Benchmarks 2026](https://docs.sofistik.com/2026/en/verification/index.html).
- **Independent structural V&V practice:** NIST GCR 14-917-27 proposes a tiered verification, validation and calibration approach and discusses use of independent structural-analysis benchmarks to identify limits of numerical methods. Source: [NIST GCR 14-917-27](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=915449).
- **Solution uncertainty methods:** NIST describes sources of finite-element solution uncertainty that include computing platform, element choices, mesh density, convergence, geometry, material properties, loading and model choice. A tiny analytical-formula tolerance must not be represented as covering these sources. Source: [NIST FEM solution uncertainty and accuracy assessment](https://www.nist.gov/publications/finite-element-method-solution-uncertainty-asymptotic-solution-and-new-approach).
- **A modern robustness benchmark candidate:** the 2026 numerical SHM benchmark by Marafini et al. uses a fixed steel beam and combines environmental/operational variability, fast and slow damage, noise and sensor faults; the authors publish the generated data and code through Zenodo. It is synthetic and inherits its model assumptions, so it is a robustness test bed—not experimental ground truth for material or collapse prediction. Its scale is about 10 GB. Sources: [paper](https://arxiv.org/abs/2603.12069), [dataset DOI](https://doi.org/10.5281/zenodo.17900300).

## Proposed qualification ladder

### Gate A — closed-form equation contract

- All three cases match independent analytical known answers under declared tolerances.
- Every physical output includes a unit and the selected assumption profile.
- Non-finite or domain-invalid inputs produce an error or UNRESOLVED receipt, never a numeric PASS.
- Unsupported requested physics produces UNRESOLVED and no partial output.
- Identical request bytes and adapter revision yield identical serialized receipt bytes.
- Metamorphic tests check expected scaling relationships, not just one output example.
- Exact-head receipts bind the request digest, adapter revision and claim ceiling.

**Current evidence:** the adapter and reference-kernel workflows passed at exact head 0d1a49d7ab016803b21a5e2d95b303e815dfacb7. This qualifies the executable reference implementation against synthetic known-answer and metamorphic tests only.

### Gate B — numerical solver verification

Before adding an external solver, define an adapter capability manifest including supported physics, element/model classes, units, nonlinear/dynamic capabilities, convergence signals, solver version, configuration digest, output fields and known limitations.

Compare supported cases with independent reference results. For finite-element cases, require at least:
- element/mesh refinement studies and reported convergence metrics;
- equilibrium and force/reaction consistency;
- dimensional/unit checks and reaction balance;
- solver exit status plus explicit convergence status, not just a generated output file;
- an independent reference solution and tolerances fixed before running candidates;
- negative controls where the model is malformed, unsupported, poorly constrained, or intentionally outside validity.

Use selected openly documented SOFiSTiK benchmarks as a comparison *catalog*. Reproduce only cases with sufficient input/result/permission clarity; don't infer equivalence merely from similar titles.

### Gate C — independent model validation

Use measurements with traceable geometry, material tests, loading, boundary conditions and calibrated sensors. Keep calibration data separate from the validation holdout. Report errors by response type, specimen family and load regime; preserve failed, censored and outlier trials with adjudication records.

The 2026 synthetic SHM benchmark can test detection under simulated environmental variation, damage evolution and sensor faults. Because it is synthetic, it cannot establish real structural predictive accuracy by itself. At least one independent physical experiment or legally reusable measured dataset is required before claiming real-world validation.

### Gate D — bounded uncertainty and robustness

Track input bounds and their provenance, geometry/material/loading uncertainty, numerical/discretization error, model-form limitations, instrument uncertainty and environmental variability separately. The new UQ issue starts with deterministic interval propagation through the three closed-form cases. It must not infer probability distributions or confidence levels from bounds alone.

Report:
- absolute/relative error by output;
- interval width and whether the nominal output lies inside the interval;
- percent of requests returned as PASS versus UNRESOLVED by capability class;
- negative-control escape rate (target 0 in frozen fixtures);
- reproducibility across operating systems/toolchain/build identity;
- latency and memory only after measuring them at a pinned machine configuration.

## How to outperform the state of the art responsibly

The defensible differentiator is not to claim a more accurate formula than a trusted solver for the same equation. It is an end-to-end evidence contract that makes unsupported requests, assumptions, provenance, exact inputs, solver/configuration identity, convergence, uncertainty and claim limits machine-checkable—and then demonstrates this behavior on broader public benchmarks and independent measurements.

To claim superiority later, preregister a matched case list, inputs, reference outputs, numerical tolerances, model versions, and decision criteria. Compare against at least one open reference implementation and, where licensed and practical, one commercial benchmark output. Publish the entire case matrix including failures and unresolved cases. A benchmark run is evidence only for the named cases and environment; it is not a universal ranking.

## Non-goals

No result from this ladder alone authorizes construction, demolition, machinery control, code compliance or an occupied-building safety decision. Real-world actions require fit-for-purpose engineering review and evidence matching the actual site and system.


## Deterministic enclosure stress evidence

The bounded input interval API has known-answer and endpoint-combination checks. The next stress layer adds a fixed-seed interior sample sweep over all three models: 4,096 parameter points per case (12,288 total) with signed load ranges that cross zero, plus a load-scaling metamorphic check for interval endpoints. These checks are useful for detecting implementation mistakes, but they do not prove interval soundness for every floating-point input. The soundness argument still depends on correct interval propagation, outward rounding, domain checks and independently derived endpoint oracles. Record sample count, exact source SHA, compiler/toolchain and any counterexamples in the receipt/review notes.

### Benchmark comparison hierarchy

| Level | Reference / method | What it can establish | What it cannot establish |
|---|---|---|---|
| Formula oracle | Closed-form axial bar and Euler-Bernoulli beam references | Equation implementation and units for named idealizations | General structural response or real specimen validity |
| Independent benchmark catalog | [NAFEMS Standard Benchmarks](https://www.nafems.org/publications/resource_center/p18/) and [2026 SOFiSTiK verification cases](https://docs.sofistik.com/2026/en/verification/index.html) | Matched-case verification for selected solver features and reference outputs | A universal ranking from unrelated cases or different model fidelity |
| Credibility framework | [ASME V&V 10](https://www.asme.org/codes-standards/vvuq-standards) and [NIST GCR 14-917-27](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=915449) | A structured verification/validation/calibration/uncertainty process | Automatic certification or a numeric pass threshold for every application |
| Experimental validation | Rights-cleared measurements with traceable geometry, material tests, loading, boundary conditions and calibration | Error within a stated specimen/test envelope | Unseen materials, geometries, environments, damage modes or structural forms |

The practical competitive claim should be about evidence orchestration, honest abstention, reproducibility and uncertainty visibility—not that three closed-form equations outperform established FEA products. A superiority claim requires a pre-registered matched benchmark, equivalent inputs/model classes, fixed reference tolerances, recorded failures, and ideally independent reproduction.


## Gate B0 — capability-first solver dispatch

Draft implementation: PHYS-DISPATCH-001, issue #2319. The registry is a versioned, machine-validated allowlist with exact manifest-byte SHA-256 captured in the dispatch receipt. Resolution runs before formula evaluation. The current registry contains only the in-process analytical reference backend, whose claim ceiling is three named closed-form cases and whose real-world validation remains UNRESOLVED.

The dispatch truth table must establish:

- unique compatible case + assumption profile + units + all requested physics -> one selected backend;
- unsupported fracture, dynamics, contact or thermal physics -> UNRESOLVED, no backend, no numerical outputs, no model evaluation;
- unknown case ID, wrong unit contract or wrong assumption profile -> UNRESOLVED, no fallback;
- ambiguous compatible entries -> UNRESOLVED rather than choosing by registry order;
- selected backend whose numeric inputs cause formula-domain failure -> selection remains explicit but evaluation is UNRESOLVED, outputs null, and nested diagnostics are preserved;
- returned output keys must match the selected manifest case's output set exactly.

The first dispatcher never executes external or remote backends. Before allowing one, require a pinned executable digest/version, isolated invocation contract, units and serialization conformance, explicit convergence/termination mapping, resource/time limits, deterministic fixture receipts, and an independent benchmark qualification gate. A capability manifest alone must never be sufficient to run unreviewed code.

## How to rank against current solver practice

The comparison is deliberately tier-matched rather than a blanket claim that Symtropy is “better than FEA”:

1. **Contract and dispatch correctness:** accuracy of allowlist selection, zero fallback on unsupported physics, zero ambiguous selections, complete provenance binding, deterministic receipts.
2. **Equation verification:** error to named analytical references, unit/sign/metamorphic tests, domain/overflow abstention.
3. **Numerical solver verification:** NAFEMS / selected SOFiSTiK public cases with mesh refinement, equilibrium/reaction balance, residuals, convergence and fixed reference tolerances.
4. **Model validation:** independent calibrated experiments and held-out measured specimens. Synthetic datasets exercise robustness, not physical truth.
5. **Decision utility:** interval width, rejection/abstention rates, sensitivity to input bounds, runtime/memory on pinned hardware, and downstream false-confidence/false-pass rate.

Target: zero silent fallback, zero false PASS in frozen unsupported/invalid fixtures, 100% deterministic output for pinned request/manifest/revisions, and a published per-case error matrix. Set numerical superiority thresholds only after selecting matched references and preregistering the evaluation; do not compare unlike model classes or conceal unresolved cases.


The dispatcher accepts both the point-only request/receipt pair v0.1 and bounded-input pair v0.2. For v0.2 it checks that any declared intervals cover the exact inputs, preserve nominal values, use the correct SI unit, and obey positive-domain bounds before resolving capabilities. It then either returns a pre-evaluation abstention or nests the v0.2 analytical receipt after a single qualified backend is selected. The nested result's interval is still conditional on the declared equation and bounds, not a probability or model-validation claim.
