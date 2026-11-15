# NeuroTwin Simulation Contract v0.1

**Status:** research interface contract. Symtropy is not presently a validated physiological, neurological or clinical simulator.
**Purpose:** define what must be true before Symtropy is used for any biomedical counterfactual experiment.

## 1. Current capabilities and boundary

Symtropy is a reusable N-dimensional physics and simulation substrate with deterministic record/replay, geometry, collision and constraint infrastructure. Its generic callback architecture can couple arbitrary metrics to physical entities.

These are useful computational foundations, but they do not establish that Symtropy currently models neurons, neural tissue, pharmacology, brain networks, disease progression or therapeutic response. A scalar labelled “health”, “trust” or Φ must never be treated as a physiological state variable merely because the engine accepts a generic metric.

Do not describe the generic physics engine or consciousness-coupled mechanics as a digital brain twin, medical simulator, or clinical decision-support system.

## 2. Biomedical model admission contract

A biomedical model may be admitted only with a versioned model card defining:

- **Target system:** explicit biological system, population and intended research question.
- **State vector:** named state variables with units, domains, observability and measurement methods.
- **Equations:** governing equations, causal assumptions, approximations and known omitted processes.
- **Parameters:** source, units, estimation method, uncertainty, identifiability and population applicability.
- **Inputs:** physically or clinically actionable intervention variables and units.
- **Boundary conditions:** initial conditions, environmental conditions and validity range.
- **Numerical method:** solver, discretization, convergence criteria, stability and error estimates.
- **Calibration:** independent empirical data, calibration protocol and frozen version.
- **Validation:** held-out data, external replication, sensitivity, uncertainty coverage and failure cases.
- **Provenance:** source dataset manifests, code commit, compiler/toolchain, seeds and output digests.

If any quantity is not identifiable from available evidence, label it assumed or unidentifiable. Do not fit parameters merely to make the model reproduce the desired treatment outcome.

## 3. Intervention semantics

A counterfactual must correspond to an intervention that can be defined and measured outside the simulator.

Examples of research inputs that might eventually qualify after specialist modelling and validation:

- Medication: compound, dose, route, timing and a validated pharmacokinetic / pharmacodynamic model.
- TMS: coil geometry, placement, orientation, waveform, pulse parameters and a validated electric-field / neural-response mapping.
- DBS or other stimulation: electrode configuration, pulse waveform, amplitude, frequency, pulse width and clinically validated constraints.
- Rehabilitation / psychotherapy: protocol identity, dose, frequency, adherence and a justified representation of exposure.

An arbitrary edge-weight change in a connectome is not a physical treatment. It may be a mathematical perturbation study, but must be labelled as such and must not be interpreted as a clinical intervention effect.

## 4. Numerical verification is not biological validation

Keep these evidence classes distinct:

1. **Software verification:** code does what its specified equations and discretization say.
2. **Numerical verification:** solver convergence, stability and discretization error are bounded.
3. **Parameter validation:** estimates are identifiable and calibrated against appropriate independent measurements.
4. **Biological validation:** predictions generalize to held-out biological data and relevant populations.
5. **Intervention validation:** predicted responses match independently observed intervention effects.
6. **Clinical utility:** prospective evidence shows the model improves patient-relevant outcomes or decisions.

Passing an earlier class does not imply a later one. Deterministic replay proves reproducibility of a simulation, not truth of its biology.

## 5. Counterfactual and uncertainty requirements

Every simulation output must bind:

- Immutable model and input hashes.
- Units and all parameter values / distributions.
- Initial state and observation window.
- Intervention identity, dose and validity range.
- Solver configuration and numerical error estimate.
- Calibration domain and external-validation status.
- Sensitivity to parameters and competing model structures.
- Prediction interval or set-valued range.
- Out-of-domain detection and abstention reason.
- A statement of which outputs are observed, inferred, simulated or unidentifiable.

For a non-identifiable or out-of-domain scenario, return an explicit unsupported/abstain status; do not emit a precise numeric effect.

## 6. Research integration with Symthaea

Symthaea may propose state representations, candidate mechanisms and experiment hypotheses. Symtropy should accept only a typed, versioned model that has passed this admission contract.

HDC embeddings can be used for indexing or representation only if the simulator does not mistake similarity for physical equivalence. CfC/LTC models may approximate temporal dynamics only after their outputs are trained and validated against measured trajectories. Model fitting and validation sets must be separate.

The integration interface should be a pure input/output boundary with:
- model identity and schema version;
- typed state and intervention vectors with units;
- data provenance and consent receipt reference;
- declared uncertainty semantics;
- output provenance and validation status.

Mycelix is the source of authorization and provenance receipts, not a validator of biological truth.

## 7. Safety and intended use

All v0.x biomedical scenarios are offline research simulations. They cannot authorize medication, stimulation, surgery, diagnosis, triage, crisis response or rehabilitation plans. No patient-specific treatment recommendation may be generated from a model that has not passed prospective clinical validation and the regulatory assessment appropriate to its intended use.

**v0.1 outcome:** Symtropy remains an honest general simulation substrate and becomes a candidate host for biomedical models only after explicit scientific admission and independent qualification.
