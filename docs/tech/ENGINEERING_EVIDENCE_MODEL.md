# Engineering Evidence Model for Symtropy

**Status:** proposed architecture and research contract, 2026-11-09  
**Scope:** simulation evidence, verification and validation, model credibility, and engineering-system interoperability.  
**Claim ceiling:** this document defines a target discipline. It does not assert that every Symtropy solver or domain currently implements it.

## 1. Purpose

Symtropy should make every consequential simulation result reproducible and bounded by evidence. A numerical output is not self-authenticating: a solver can converge to the wrong answer, a model can omit the dominant physics, and a physically accurate result can still be inapplicable to a changed configuration.

The evidence model therefore separates:

- **Verification:** did we solve the specified mathematical/numerical problem correctly?
- **Validation:** is the selected model an adequate representation of the physical system for the intended use?
- **Calibration:** which parameters were estimated from observations, using what data and method?
- **Uncertainty quantification:** how do input, parameter, numerical and model-form uncertainties affect outputs?
- **Qualification:** does the accumulated evidence satisfy a specific requirement for a precisely identified configuration and operating domain?
- **Release authorization:** a lifecycle decision owned by the responsible engineering/product authority, not by a solver.

These terms must not be used interchangeably.

## 2. Evidence classes

Every input and result should carry an explicit evidence class. Suggested vocabulary:

| Class | Meaning | Example |
|---|---|---|
| `SYNTHETIC` | Generated to exercise code or a protocol | Fixture with invented material properties |
| `ASSUMED` | Engineering assumption not yet measured | Preliminary seal friction coefficient |
| `SOURCED` | Value transcribed from an identified source | Supplier datasheet revision |
| `ANALYTICAL` | Closed-form or independently derived reference | Free-fall trajectory |
| `NUMERICAL_REFERENCE` | Result from a trusted reference implementation or benchmark | Published canonical contact case |
| `SIMULATED` | Output from a named computational model | Symtropy transient run |
| `MEASURED` | Observation from identified physical instrumentation | Load-cell force trace |
| `CORRELATED` | Model compared with matched physical measurements | Joint torque/temperature correlation |
| `QUALIFIED` | Evidence accepted against a named requirement and exact applicability envelope | Environmental test pass for a serialized configuration |

Evidence class is not a universal ranking. A sourced value may be less relevant than a measurement; a measurement may be poorly calibrated; a correlated model may still be invalid outside its tested domain. Store source quality, uncertainty, applicability and review status separately.

Never silently promote synthetic or assumed data to measured, correlated or qualified evidence.

## 3. Immutable run identity

A run should be reproducible from an immutable manifest. At minimum, the manifest should bind:

- run ID and schema version;
- exact repository commit and build/toolchain identity;
- solver and model identifiers, versions and feature configuration;
- geometry, mesh, topology and material identifiers/revisions/digests;
- units and coordinate-frame conventions;
- initial conditions, boundary conditions, loads and environmental conditions;
- time integration, tolerances, solver options and random seeds;
- input-data references and evidence classes;
- parameter provenance and calibration dataset references;
- output artifact digests and machine-readable result summary;
- warnings, rejected inputs, unsupported features and termination reason.

A human-readable report is a view of this record, not its source of truth. Large CAD, mesh, sensor and solver artifacts should be stored by immutable reference and digest rather than copied into every report.

## 4. Verification and validation ladder

Use a staged ladder. A higher stage does not erase failures or limitations at a lower stage.

### V0 — Contract and input validity

- schema validation and version compatibility;
- unit and frame checks;
- finite-value/range checks;
- topology and referential-integrity checks;
- explicit rejection of unsupported or incomplete inputs.

### V1 — Code verification

- deterministic unit and property tests;
- dimensional-analysis checks;
- invariants and conservation tests where applicable;
- negative controls and adversarial fixtures;
- sanitizer/fuzz/property testing for parsers and geometry boundaries.

### V2 — Numerical verification

- analytical and manufactured solutions;
- convergence under time-step, mesh and tolerance refinement;
- residual and constraint-drift analysis;
- stability and conditioning studies;
- independent implementation or high-precision reference comparisons.

### V3 — Model validation

- comparison with identified experimental/reference datasets;
- calibration/validation data separation;
- predeclared error metrics and acceptance limits;
- uncertainty on both predictions and measurements;
- explicit operating-domain and failure-mode characterization.

### V4 — Configuration-specific qualification

- exact hardware/software/material/geometry configuration;
- named requirement and acceptance procedure;
- test equipment calibration and traceability;
- applicable environmental, load and duty-cycle cells;
- deviations, exclusions and independent review;
- explicit expiry, maintenance or change-trigger conditions.

### V5 — Lifecycle applicability

- configuration changes trigger impact analysis;
- evidence becomes stale when a changed dependency invalidates its applicability;
- service, wear, repair, calibration and environmental exposure update the as-maintained record;
- operational limits can be narrowed when evidence or condition degrades.

V4/V5 decisions belong to competent engineering and assurance authorities. Symtropy can compute and package evidence; it must not unilaterally declare a real product safe.

## 5. Uncertainty is a first-class output

Do not report a single precise number when the model supports only a range or probability distribution.

Separate at least:

1. **Input uncertainty:** measurement error and uncertain boundary conditions.
2. **Parameter uncertainty:** material, friction, damping, actuator and thermal parameters.
3. **Numerical uncertainty:** discretization, solver tolerance, truncation and convergence.
4. **Model-form discrepancy:** omitted or simplified physical phenomena.
5. **Scenario uncertainty:** unknown future loads, environment and usage.

Where a probability model is justified, report its assumptions, calibration and coverage. Where it is not justified, use bounded intervals, sensitivity ranges or explicit unknown states. Avoid false probabilistic precision.

Report sensitivity to influential parameters and identify when uncertainty changes the engineering decision. A design whose nominal prediction passes but whose credible uncertainty interval crosses a hard limit should not be reported as an unconditional pass.

## 5.1 Risk-scaled evidence depth

VVUQ effort should be proportionate to both **model influence** and **decision consequence**. This follows the risk-based direction in NIST's 2026 report harmonizing computational-model UQ with JCGM metrology practice. It is not permission to weaken a physics or semantic gate.

For every context of use, record:

- **Model influence:** how strongly does this model affect the decision, requirement margin, controller action or release criterion? Could an error reverse the decision?
- **Decision consequence:** what happens if the prediction is wrong? Consider human exposure, equipment damage, environmental impact, financial loss, recovery/reversibility and scale of deployment.
- **Uncertainty leverage:** which uncertain inputs or model assumptions dominate the decision? Can a feasible test or independent model resolve them?
- **Applicability exposure:** how far is the proposed use from tested geometry, materials, load, temperature, speed, wear, environment and configuration?

Use the following evidence-depth classes as a planning baseline:

| Class | Typical context | Minimum evidence package | Claim boundary |
|---|---|---|---|
| **E1 — Exploratory** | Low model influence and low decision consequence; idea generation, visualization or early trade study | Exact run/configuration identity; declared units/frames and assumptions; basic code tests; analytical sanity checks where available; explicit synthetic/assumed/simulated evidence classes | Supports exploration only. Cannot be promoted to product acceptance or safe-operation evidence. |
| **E2 — Engineering development** | Model influences design choices or requirement margins, but consequences are bounded/reversible and the operating envelope is not yet release-critical | E1 plus numerical verification/refinement; sensitivity and uncertainty analysis; an independent reference or solver for decision-critical paths; calibrated parameter provenance; at least one disconfirming/negative control; reviewer approval of applicability | Supports bounded design decisions inside a declared envelope. Extrapolation and unresolved model discrepancy remain visible. |
| **E3 — Consequential / qualification** | High model influence **or** high decision consequence: human proximity, safety-relevant actuation, structural/environmental limits, major financial exposure or operational release | E2 plus independent algorithm/model review; risk-appropriate VVUQ and metrology traceability; disjoint calibration and held-out validation conditions; physical correlation for consequential physical claims; explicit uncertainty budget and acceptance margins; failure/out-of-domain challenge; configuration-specific review and change-triggered reassessment | Supports only the named requirement, configuration and operating domain accepted by competent engineering authority. It is not universal certification. |

If either influence or consequence is high, use E3 until a documented assessment justifies otherwise. When the context of use is unknown, default to the more demanding class rather than assuming low risk. A class is a planning aid, not an automated safety score.

### Non-negotiable gates at every evidence class

- Invalid units, frames, identities, non-finite inputs or missing decision-critical semantics are rejected or marked indeterminate.
- Unknown, unsupported, non-equivalent or unbounded model paths never enter a matched score or an unconditional pass.
- A numerical residual, solver failure, capacity overflow, non-convergence or out-of-domain condition remains visible; recovery must not erase the original failure.
- Synthetic or assumed data are never relabelled as measured, correlated or qualified.
- A lower VVUQ class cannot authorize a decision whose consequence or model influence places it in E3.

### Choose uncertainty propagation honestly

Do not choose a probabilistic method merely because it produces a confidence interval. Use variance propagation when the assumptions and dependence structure justify a second-moment description and the decision is adequately described by it. Use distribution propagation, sampling or a validated surrogate when nonlinearity, tails, multimodality, threshold crossings or nonlinear dependencies materially affect the decision. When distributions cannot be justified, preserve bounded intervals, sensitivity envelopes and explicit unknowns.

Keep measurement uncertainty, parameter uncertainty, numerical uncertainty and model-form discrepancy separate. A narrow measurement uncertainty does not imply a credible model; a well-correlated nominal trajectory does not establish tail behaviour or out-of-domain prediction.

**Example — rotary-joint cartridge:** early architecture sketches can start at E1. Selecting gear ratio or thermal architecture against requirements normally raises the work to E2. Any model result used to accept torque/temperature margins, actuator derating, failure response or human-proximity operation is E3 until the hardware configuration, calibrated measurement method, independent model evidence and held-out validation are established.

Reference: NIST, *On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices* (published 2026-10-05), https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing.

## 6. Simulation-to-physical correlation

Correlation requires matched conditions and configuration identity.

A correlation record should identify:

- physical article serial/lot and as-tested configuration;
- exact simulation configuration and model revision;
- test procedure, instrumentation, calibration and sampling;
- synchronized input/load/environment histories;
- measured and predicted channels with units and frames;
- alignment, filtering and preprocessing methods;
- pointwise and aggregate error metrics;
- uncertainty in measurement and prediction;
- excluded data and reasons;
- residual structure and observed model discrepancy;
- applicable domain and untested extrapolations.

Do not hide phase error, peak error, bias or tail failures inside a single aggregate RMSE. Report metrics relevant to the physical decision.

For the first rotary-joint demonstrator, candidate correlation channels include torque, angular position/velocity, current, winding/case temperature, vibration, seal ingress indicators and energy consumption. Test and instrumentation design must be finalized by the responsible hardware team.

## 7. Federated engineering interfaces

Symtropy is a specialist physical-analysis service within a broader engineering digital thread.

| External authority | Symtropy consumes | Symtropy emits |
|---|---|---|
| CAD/PLM (Luminous) | configuration ID, geometry/mesh references, materials, requirements, change context | model/run references, physical-analysis evidence, affected assumptions |
| Sol-Atlas | frames, spatial geometry, scene/terrain records, provenance and uncertainty | physical feasibility envelopes, contact/clearance constraints, modelled state changes |
| Symthaea | bounded scenario proposals and planning questions | predictions, sensitivities, uncertainty and infeasibility explanations |
| Mycelix | identity, permission and attestation context | signed result digests and provenance references through an authorized integration |
| Test/measurement systems | calibration, sensor data, test procedure and article identity | correlation reports and model-discrepancy records |

No integration should silently reinterpret units, coordinate frames, timestamps, configuration IDs or evidence classes. Each interface needs a versioned schema, compatibility policy, negative controls and round-trip fixtures.

## 8. Benchmark programme

Build a public, versioned benchmark suite with frozen inputs, reference outputs, explicit acceptance tolerances and negative controls.

### Tier A — mathematical and numerical correctness

- free fall, ballistic motion and harmonic oscillator;
- rigid-body momentum and energy conservation in isolated systems;
- analytical sphere/sphere and primitive contact cases;
- constraint and joint kinematics;
- manufactured solutions for any future continuum/field solver;
- dimensional and coordinate-frame invariance tests.

### Tier B — algorithmic robustness

- nearly degenerate convex geometry;
- high aspect ratios and extreme mass ratios;
- grazing, persistent and rapidly changing contacts;
- high-speed impacts and continuous collision edge cases;
- stacked contacts, closed chains and joint-limit interactions;
- deterministic replay and malformed/adversarial inputs.

### Tier C — robotics reference tasks

- actuator torque-speed and thermal derating models;
- single and common-cause actuator faults;
- support polygon and contact feasibility;
- matched locomotion/manipulation tasks with fixed payload and terrain;
- degraded-mode energy, recovery and stability envelopes.

### Tier D — physical correlation

- instrumented rotary joint;
- calibrated impact/contact fixture;
- articulated test article;
- environmental exposure and post-exposure inspection;
- wear, drift and maintenance-cycle studies.

Publish accuracy, stability, throughput, memory, uncertainty coverage, failure rate and reproducibility separately. Keep synthetic, numerical-reference, simulation, measured and qualified datasets in separate strata.

## 9. Initial acceptance targets

These are proposed software/evidence-process targets, not claims about current performance or universal certification limits.

| Area | Initial target |
|---|---|
| Input integrity | 100% of invalid units, frames, missing required fields and unsupported model options rejected or explicitly marked indeterminate |
| Provenance | 100% of published benchmark runs bind exact source revision, configuration and input digests |
| Negative controls | Every hard acceptance gate has at least one seeded failing case |
| Oracle agreement | 100% of bounded hand-computable reference cases fall within predeclared tolerances |
| Convergence | Report refinement curves and observed order where the method supports it; no pass based only on one resolution |
| Replay | Byte-identical canonical reports for repeated runs on the declared deterministic platform and frozen toolchain |
| Uncertainty | Report prediction and measurement uncertainty for every physical correlation claim |
| Evidence freshness | 100% of dependency-changing edits invalidate or explicitly re-assess affected evidence |
| Correlation | No universal target before the physical test method, instrumentation, operating range and error budget are frozen |
| Performance | Publish p50/p95/p99 latency, throughput, memory and scaling under named hardware and workload |

Do not set arbitrary global physical tolerances. Each requirement must derive its acceptance limit from the intended use, risk, measurement capability and governing engineering standard.

## 10. First implementation sequence

1. Define versioned run-manifest and evidence-record schemas.
2. Add a minimal Rust manifest validator with strict units, frame IDs, evidence classes and digest syntax.
3. Add one analytical free-fall case and one negative control.
4. Add exact-head reproducibility receipt and deterministic canonical report.
5. Add a simulation result envelope with explicit applicability and uncertainty fields.
6. Connect one reference component to Luminous product/configuration identity.
7. Add Sol-Atlas geometry/frame references without duplicating spatial authority.
8. Add the rotary-joint model/test contract and later correlate against measured hardware.

The first vertical slice should be deliberately small. It should prove that a changed geometry or material revision changes the run identity and prevents stale evidence from being presented as current.

## 11. Explicit non-goals

This model does not imply that Symtropy currently has a universal FEA/CFD solver, verified material database, validated humanoid dynamics model, certified safety controller, production PLM connector, or physical test laboratory. Those capabilities require separately scoped implementation and evidence.

The objective is to make it difficult—not easy—for an unsupported engineering claim to pass through the system unnoticed.
