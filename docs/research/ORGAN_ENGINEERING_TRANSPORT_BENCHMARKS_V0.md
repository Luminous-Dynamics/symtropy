# Organ Engineering Transport Benchmarks V0

**Status:** research contract / proposal; not a validated biological model  
**Scope:** Symtropy research/domain layer, with Symthaea as the evidence-bounded design-search and experiment-planning partner  
**First wedge:** vascular perfusion and transport—not whole-organ generation

## Why this workstream

Engineered tissues and organoids still face incomplete maturation, batch variability, and difficulty establishing stable perfusable vasculature. The 2025 review *Strategies for the vascular patterning of engineered tissues for organ repair* identifies functional vasculature as a central constraint on engineered-tissue survival. A 2025 U.S. GAO assessment also identifies limited benchmarks, validation studies, and data sharing as obstacles to wider organ-on-chip adoption.

Sources:
- Janson et al., *Nature Biomedical Engineering* (2025), [Strategies for the vascular patterning of engineered tissues for organ repair](https://www.nature.com/articles/s41551-025-01420-w).
- U.S. GAO (2025), [Human Organ-On-A-Chip: Technologies Offer Benefits Over Animal Testing but Challenges Limit Wider Adoption](https://www.gao.gov/products/gao-25-107335).
- Huang et al., *Nature Reviews Bioengineering* (2025), [Organoid bioprinting: from cells to functional tissues](https://www.nature.com/articles/s44222-024-00268-0).
- Menzani et al., *Lab on a Chip* (published 2026), [Vascularizing organoids-on-chip for perfused and personalized models](https://pubs.rsc.org/en/content/articlehtml/2026/lc/d5lc00890e).

These sources motivate a research priority, not a claim that the platform already models organs or that perfusion alone is sufficient for organ function.

## Paradigm shifts

### 1. Design for function before designing for appearance

The primary object is a versioned functional system with explicit requirements, not an attractive organ mesh. Geometry is one representation of that system. A renderer must not be allowed to create or repair biological truth.

### 2. Treat perfusion and transport as shared infrastructure

Build reusable transport models that can be specialized later for a selected tissue: conduits, branching networks, pressure/flow boundary conditions, exchange surfaces, and transport fields. Make explicit which conservation laws, constitutive equations, and assumptions apply to each model.

### 3. Use multi-fidelity, causal digital twins

Separate:
- analytic/reference models used to verify equations and implementations;
- reduced-order network models used for fast design search;
- spatial numerical models used when gradients and geometry require them;
- experimental observations used to calibrate and test the models;
- biological conclusions, which remain bounded by the evidence actually collected.

A higher-fidelity visual or numerical model is not automatically a more biologically valid model.

### 4. Co-design function and manufacturing

Constrain candidate designs by printable/manufacturable geometry, supported feature sizes, material limits, tolerances, accessible ports, and measurable inspection surfaces. Optimize against a vector of competing outcomes rather than hiding trade-offs inside one unexplained score.

### 5. Make experiment selection active and falsifiable

Symthaea should propose discriminating experiments that separate competing model hypotheses, prioritize measurements expected to reduce decision-relevant uncertainty, and retain failed predictions. It must not convert internal confidence, analogy, or simulation success into biological evidence.

### 6. Make negative results and provenance first-class

Every result should identify its model and schema version, parameters and units, solver/version, boundary conditions, source data hashes, uncertainty treatment, and exact test/experiment fixture. Preserve failed and contradicted results. No undocumented fallback may count as a pass.

## System boundary

Proposed evidence flow:

```
requirements + source-admitted evidence
              |
              v
Symthaea: candidate generation, hypotheses, Pareto search, experiment selection
              |
              v
versioned design + explicit constraints + predicted observables
              |
              v
Symtropy research/domain layer: transport mechanics + fabrication constraints
              |
              v
analytic fixtures -> synthetic held-out cases -> physical bench observations
              |
              v
independent comparison + residuals + uncertainty + falsification receipts
              |
              +----> update model only under a versioned, auditable change
```

**Architecture rule:** this work must not expand Symtropy's rigid-body core by implication. The repository explicitly scopes soft-body simulation out of core. Any future fluid/transport solver should live behind a narrow research/domain boundary or independently licensed ecosystem crate after dependency and license review. The initial deliverable is a benchmark contract; it does not implement a solver.

**No authority laundering:** a passing analytic fixture establishes correctness for that fixture only. Synthetic cases establish properties of the synthetic model only. Agreement with a benchtop flow model establishes agreement for the tested conditions only. None independently establishes cell viability, mature organ function, transplant safety, immunological compatibility, or clinical efficacy.

## Proposed benchmark ladder

Thresholds below are engineering targets to be debated and preregistered before a benchmark run. They are not standards adopted by the biomedical field and do not constitute clinical acceptance criteria.

### Q0 — mathematical verification, no biological calibration

Use small, deterministic, versioned fixtures.

1. **Single-channel laminar-flow reference.** For the stated assumptions of steady laminar Newtonian flow in a rigid circular tube, compare implementation output against the Hagen–Poiseuille reference:
   `Q = ΔP / R`, with `R = 8 μ L / (π r⁴)`.
   - Proposed target: relative flow error ≤ 1e-6 for the analytic arithmetic/reference path, or a separately justified tolerance for a numerical solver.
   - Required scope metadata: units, viscosity, length, radius, pressure difference, regime assumptions, and reference implementation/version.
2. **Network continuity.** At each internal junction, sum of incoming and outgoing flow must balance within a declared numerical tolerance. Boundary fluxes must reconcile with total network flux.
3. **Metamorphic/property checks.** With other assumptions held fixed: doubling radius yields 16× flow; doubling viscosity halves flow; doubling tube length halves flow. Invalid units, non-finite inputs, impossible geometry, and unsupported flow-regime assumptions must reject or return an explicitly unsupported result—not be silently normalized.
4. **Pressure and conductance sanity.** For a passive network under the declared model, pressure/flow direction must agree with the boundary conditions; non-positive radius/viscosity or negative resistance must fail closed.
5. **Numerical refinement.** Spatial solver variants must demonstrate convergence on an analytic or manufactured solution, with mesh/time-step details and observed order of convergence reported. Do not demand a mesh-independent answer without a convergence study.
6. **Deterministic replay.** Identical versioned inputs produce identical canonical output and receipt, within only those floating-point tolerances explicitly declared by the profile.
7. **Negative controls.** Deliberately wrong units, omitted boundary pressure, a disconnected component, wrong radius exponent, and a duplicate edge must be caught by the relevant verifier.

Q0 is allowed to pass with zero biological claims.

### Q1 — synthetic design search

Define a versioned synthetic vascular-network corpus containing simple trees, loops, bottlenecks, disconnected regions, competing branches, manufacturing exclusions, and randomized held-out geometries. Pin generator code and seeds.

Compare the candidate designer against at least:
- a documented simple baseline (e.g. minimum-length feasible tree under the same constraints);
- a constraint-aware heuristic baseline;
- an ablated variant without Symthaea candidate selection.

Measure each objective separately:
- fraction of target regions supplied under the stated transport proxy;
- worst-case and distributional pressure drops;
- flow-distribution imbalance;
- material/volume use and estimated fabrication complexity;
- constraint violations and solver cost;
- sensitivity to parameter uncertainty.

**Proposed usefulness target:** on a preregistered set of at least 20 held-out synthetic cases, achieve ≥20% reduction in a named failure metric versus the strongest baseline at equal stated material/pressure constraints, with no increase in hard-constraint violations. This is a development target, not a field-standard threshold. If it is not met, retain the negative result and revise the hypothesis; do not change the metric after seeing outcomes.

Avoid a single composite “organ realism” score. Publish the Pareto frontier and all objective values.

### Q2 — instrumented physical transport validation

Only after Q0 is qualified, compare model predictions to an instrumented, non-biological or appropriate approved benchtop flow system. Use fixed calibration data and a held-out test set. Do not tune against the held-out cases.

Proposed initial targets, to be finalized with the laboratory after measurement uncertainty is characterized:
- mean absolute percentage error ≤10% for preselected, well-conditioned flow/pressure observables;
- prediction intervals with stated coverage and width, not point error alone;
- no unreported excluded runs or post-hoc boundary-condition changes;
- independent rerun from pinned design, configuration, and raw measurement hashes;
- measured uncertainty of sensors and boundary conditions propagated into comparisons.

A target that is below the instrument's actual resolution is invalid; widen or redesign the experiment before scoring the solver.

### Q3 — tissue-specific transport model

This gate requires a qualified biomedical collaboration, an explicitly selected tissue/cell system, and appropriate instrumentation. It is not authorized by Q0–Q2 alone.

Record and separately validate:
- cell/tissue identity and provenance;
- tissue geometry and cell-density assumptions;
- oxygen/nutrient transport and consumption model;
- perfusion and leakage;
- spatial viability and functional assays;
- independent batch and operator effects;
- calibration vs blind hold-out data;
- limits of applicability and competing explanations.

Do not apply one universal “maximum vessel-to-cell distance” or viability cutoff to every organ/cell type. Oxygen availability depends on cell demand, medium, geometry, culture conditions, and measurement method; tissue-specific limits must be justified by the selected evidence.

### Q4 — reproducible biohybrid prototype

Only after a tissue-specific model has passed Q3 should a biohybrid prototype claim be considered. Track biological function, durability, device reliability, batch yield, material variability, sterility/contamination controls, safety hazards, and manufacturing traceability as independent gates. A functioning chip or tissue patch is not a complete replacement organ.

### Q5 — translational / clinical claims

Outside the authority of this document. Requires the appropriate domain experts, quality system, ethics approvals, risk management, preclinical evidence, and applicable regulatory pathway. Simulation or a strong benchmark score cannot substitute for these.

## Benchmark manifest requirements

Every benchmark result must bind to:

- benchmark ID and schema/profile version;
- exact code commit and dependency/toolchain identity;
- input fixture hash, generator version/seed, and units;
- model assumptions and constitutive-law identifiers;
- boundary and initial conditions;
- solver choice, precision, tolerances, and convergence configuration;
- expected/reference output and its source;
- measured output, residuals, uncertainty, and excluded observations with reasons;
- baseline and ablation identifiers;
- status: `PASS`, `FAIL`, `BLOCKED`, or `NOT_RUN`;
- claim ceiling: what this result does and does not establish.

Queued, skipped, missing, stale, or non-executed checks are not PASS. A synthetic PASS may never be promoted to a physical-validation PASS.

## First implementation sequence

1. **Freeze Q0 definitions and a minimal data schema.** No dependencies, biological constants, or product claims.
2. **Add pure reference calculations and hostile fixtures** for tube resistance/flow and network conservation. Keep them independent of the proposed optimizer.
3. **Add a tiny deterministic network corpus** with metamorphic and negative-control cases.
4. **Implement a separately scoped reference transport module** only after reviewing existing crate boundaries and licensing. Compare analytic fixtures before adding optimization.
5. **Add Symthaea candidate selection as a separate adapter** that returns candidate designs plus objective vectors, assumptions, uncertainty, and provenance—not a “biologically optimal” verdict.
6. **Run ablations and held-out synthetic cases**, then publish an exact-head result even if it fails.
7. **Seek a laboratory partner** before any tissue-specific calibration or claim.

## Initial acceptance criteria for this contract

- Reusable perfusion/transport is identified as the first engineering wedge.
- Core rigid-body scope is unchanged.
- Analytic verification, synthetic design search, physical transport comparison, tissue-specific validation, and translational evidence are distinct gates.
- Targets are labeled as proposals, not external standards.
- Every benchmark has an evidence/claim ceiling and negative controls.
- No new biological calibration constants, medical advice, experimental protocol, clinical capability claim, or production dependency is introduced.
- Rust implementation begins only after the relevant mathematical fixtures and domain ownership are agreed; CI is not used as a substitute for local reasoning or exact-head evidence.

## Decision

Proceed with Q0 as the first executable work item. Keep the initial result deliberately small: a verified transport contract is more valuable than an unvalidated whole-organ simulator.
