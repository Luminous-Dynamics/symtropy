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
