# EHD model discrepancy, validation envelopes and predictive abstention (2026)

**Status:** research specification v0.1  
**Tracking:** [EHD-DISC-001 #6681](https://github.com/Luminous-Dynamics/symtropy/issues/6681)  
**Related:** [EHD-OED-001 #6660](https://github.com/Luminous-Dynamics/symtropy/issues/6660), [thrust observability #6641](https://github.com/Luminous-Dynamics/symtropy/issues/6641), [control-volume reducer #6650](https://github.com/Luminous-Dynamics/symtropy/pull/6650)  
**Claim boundary:** benchmark specification only; no physical calibration or model-discrepancy correction is claimed.

## 1. Purpose

Build a model-credibility layer that prevents calibration from hiding incorrect physics and prevents empirical corrections from being mistaken for transferable physical laws.

A model can fit its calibration data while:
- physical parameters trade off against an unconstrained discrepancy function;
- numerical error is absorbed into a physical coefficient;
- measurement artifacts are learned as physics;
- an omitted mechanism is represented by an empirical coefficient;
- a correction improves training fit but fails on held-out geometries or operating regimes;
- predictions are extrapolated beyond the validation envelope with unjustifiably narrow uncertainty.

This benchmark complements EHD-OED-001:
- **Identifiability** asks whether parameters and competing explanations can be distinguished.
- **Discrepancy qualification** asks whether the selected physical model remains adequate and where its predictions may be trusted.

## 2. Research grounding and comparators

These methods are established research, not a new Luminous statistical theory:

- Yang, Dong & Wu (2026), [Bayesian experimental design for model discrepancy calibration: An auto-differentiable ensemble Kalman inversion approach](https://doi.org/10.1016/j.jcp.2025.114469), *Journal of Computational Physics* 545, 114469. Published 15 January 2026; addresses experiment design in the presence of model discrepancy.
- Sung & Tuo (2024), [A review on computer model calibration](https://doi.org/10.1002/wics.1645). Reviews calibration, model inadequacy and parameter/discrepancy unidentifiability.
- Yeo (2020), [NISTIR 8298](https://doi.org/10.6028/NIST.IR.8298), industrial CFD V&V and UQ procedures.
- ITTC Procedure 7.5-03-01-01 (2024), [Uncertainty Analysis in CFD Verification and Validation](https://www.ittc.info/media/11950/75-03-01-01.pdf).

A directly relevant plasma comparator is [ANEMONE (2025)](https://doi.org/10.1016/j.cpc.2025.109749), a 3D three-component plasma-fluid framework for solid-state electroaerodynamic propulsion. Its declared outputs include corona inception voltage and energy-conversion efficiency; this is not equivalent to an independently validated complete thrust/neutral-flow model.

An adjacent but non-equivalent comparator is the 2026 OpenFOAM electro-flow-focused microjet study. It treats a liquid-gas multiphase jet with electrostatic body forces and charge conservation. It is useful for multiphysics and interface-method comparison, but must not be merged into an atmospheric ion-wind EHD propulsion leaderboard.

The contribution sought is an evidence-qualified workflow that keeps error sources distinct, enforces conservation independently, evaluates model discrepancy on held-out regimes, and abstains outside a declared support envelope.

## 3. Error taxonomy

The benchmark must distinguish:

1. **Code/implementation error:** code does not solve its declared mathematical equations.
2. **Numerical solution error:** mesh, timestep, quadrature, iteration, coupling, precision or roundoff.
3. **Input and parameter uncertainty:** uncertain coefficients, geometry, gas properties, boundary conditions and electrical waveform.
4. **Measurement uncertainty/artifacts:** calibration, sensor response, timing, force stand, optical reconstruction and parasitic loads.
5. **Model-form discrepancy:** missing physics or invalid closure assumptions inside the declared equation set.
6. **Extrapolation uncertainty:** prediction outside the validated geometry, operating-condition, discharge or numerical envelope.

These contributions are not assumed statistically independent. Root-sum-square combination is permitted only when a defensible covariance model and its evidence are supplied. Otherwise use a declared conservative bound or joint uncertainty model.

## 4. Mathematical model and identifiability warning

A generic observation model may be written:

\[
y(d)=G_M(\theta,d)+\delta_M(d)+\epsilon(d)
\]

where:
- \(G_M\) is the prediction of the version-pinned physical/numerical model;
- \(\theta\) are physical parameters;
- \(\delta_M\) is model-form discrepancy at a declared observable boundary;
- \(\epsilon\) is measurement error.

This decomposition is not automatically identifiable. A flexible \(\delta_M\) may absorb a wrong parameter value or omitted physics; parameter fitting may conversely absorb discrepancy into a coefficient. Profile likelihood, independent observables, physically meaningful restrictions and held-out condition families are required.

The discrepancy model is not an alternate solver and must not be represented as a physical mechanism unless a separately specified closure and evidence justify that interpretation.

## 5. Conservation boundary: discrepancy cannot launder a physical failure

Conservation and model-fit residuals are different quantities.

- Charge, mass, momentum and energy qualification remains independent and event/species-local where relevant.
- A statistical correction to a thrust or flow quantity of interest must identify the precise observation boundary and remain outside conservative source/flux terms.
- If a proposed discrepancy modifies an internal source or flux, it must carry the matching species reactions, boundary terms and complete momentum/energy ledger entries.
- A model discrepancy term cannot turn a failed momentum or energy ledger into a pass.
- Missing physical channels remain unknown/out-of-scope, never silently represented by a zero or an unconstrained correction.

Adversarial tests must intentionally make a discrepancy term improve the force prediction while breaking momentum or energy closure. The qualification layer must reject that result.

## 6. Work packages

### D0 — Error-source separation corpus

Create manufactured reference observations where known amounts of parameter error, measurement noise, numerical bias and model discrepancy are injected independently and in combinations.

Required negative controls:
- numerical bias is fitted as a collision coefficient;
- measurement offset is fitted as a physical thrust gain;
- omitted mechanism is fitted as an input-property correction;
- two sources of error are structurally confounded.

The benchmark reports the correct injected category where identifiable and abstains where it is not.

### D1 — Parameter/discrepancy confounding

Compare:
- no-discrepancy calibration;
- unconstrained discrepancy;
- physically structured discrepancy;
- an orthogonality/constraint-based comparator;
- profile-likelihood practical-identifiability diagnostics.

An excellent training fit with non-identifiable physical parameters is not an accepted calibration. Report parameter intervals and discrepancy uncertainty separately.

### D2 — Conservation-compatible discrepancy

Initially apply discrepancy at a declared observable boundary, such as a thrust prediction residual. Do not insert an arbitrary correction directly into the collision source.

If internal-source correction is later considered, require:
- source identity and units;
- charged/neutral or species-paired reaction terms;
- boundary flux contribution;
- energy partition and thermodynamic ledger entries;
- independent numerical and physical qualification.

### D3 — Multidimensional validation envelope

Represent support along independent dimensions:
- electrode geometry family, scale and topology;
- voltage/current waveform family and frequency/duty regime;
- gas species, composition and humidity;
- pressure and temperature;
- discharge mode and ionization regime;
- neutral-flow inlet and boundary conditions;
- solver formulation, mesh, timestep and coupling configuration.

Do not collapse this into one undocumented scalar distance. Report per-axis support, joint cohort coverage, nearest validated cohorts and categorical regime mismatches. A numerically close point in one dimension may be physically dissimilar in another.

Hold out complete geometry, waveform, gas/pressure and experimental-session families. Random time-sample splitting within one experimental session does not establish transfer validity.

### D4 — Predictive uncertainty and abstention

Evaluate:
- predictive interval coverage and width;
- proper scoring rules such as CRPS and log score where distributions are available;
- residual structure and bias;
- calibration separately by regime/cohort;
- out-of-envelope detection;
- prediction stability under admitted uncertainty in inputs and parameters.

The system returns a prediction only when the required solver, validation envelope, evidence identity and uncertainty gates pass. Otherwise it returns an explicit NotEvaluable or OutOfEnvelope disposition with the unsupported dimensions identified.

### D5 — Discrepancy-aware experiment design

Integrate with #6660. Select experiments to distinguish:
- physical parameter shifts;
- model-form discrepancy;
- measurement artifacts;
- numerical/coupling error.

Keep parameter-information gain and model/discrepancy discrimination as separate outputs. Prefer measurements that challenge the largest unresolved discrepancy over measurements that merely improve calibration fit.

## 7. Adversarial benchmark cases

- Discrepancy exactly cancels a deliberately wrong physical parameter.
- Discrepancy term repairs thrust error while the momentum/energy ledger remains wrong.
- Mesh/timestep error is fitted as a collision coefficient.
- Thermal or cable-force artifact is fitted as neutral viscosity or thrust gain.
- Training/holdout leakage across repeated source figures or session samples.
- Model discrepancy extrapolated far beyond support with a narrow interval.
- Aggregate interval coverage hides failure in a specific regime.
- Correlated error components incorrectly combined by RSS.
- Timestamp or coordinate-frame mismatch is learned as a physical response.
- Predictive score is high while parameters remain non-identifiable.
- A relevant physical/artifact alternative is omitted from the model set.

## 8. Prospective qualification targets

Targets, not achieved results:

| Gate | Prospective target |
|---|---|
| Physical-ledger integrity | No discrepancy term can cause an independently failed charge/mass/momentum/energy gate to pass |
| Predictive coverage | 90–98% empirical coverage for nominal 95% intervals on preregistered held-out cohorts |
| Coverage reporting | Per cohort/regime, including Monte Carlo uncertainty |
| Out-of-envelope detection | At most 1% false in-envelope classification on a preregistered OOD negative-control set, with detection power reported |
| Predictive score | Improve against frozen uncorrected baseline without violating coverage or bias gates |
| Parameter recovery | Remains within independently qualified intervals; otherwise mark confounded |
| Extrapolation | Explicit abstention or separately qualified extrapolation disposition |
| Provenance | Every prediction binds model, parameters, inputs, evidence, discrepancy version and validation envelope |

Thresholds must be versioned and compared at matched observation count, channels, solver fidelity and resource budget.

## 9. Evidence receipt

A future model-credibility receipt should include:
- solver/model equations and version hashes;
- numerical verification cohort and discretization uncertainty;
- parameter schema, scales, estimates and profile intervals;
- measurement/calibration identities and covariance policy;
- discrepancy model class, parameters, training support and observable boundary;
- conservation ledger results before/after discrepancy treatment;
- validation envelope and held-out family identities;
- predictive coverage, proper scores, residual diagnostics and abstentions;
- exact evidence/data digests and claim ceiling.

The receipt proves which evidence and qualification rules were applied. It does not prove that a model is physically complete.

## 10. Ownership and integration

- **#6660 EHD-OED-001:** practical parameter identifiability and model/artifact discrimination.
- **#6641/#6642:** independent force-balance, velocity-field and solver observability.
- **#6650:** control-volume momentum reconciliation.
- **#6640:** species-resolved collisional energy.
- **#6627:** transport polynomial exactness and moving-mesh numerical qualification.
- **Symthaea:** generate competing explanations, challenge calibration and propose discriminating experiments.
- **Symtropy:** freeze validation cohorts, enforce independent qualification gates, version discrepancy models and replay decisions.
- **Specialist solvers and measurement pipelines:** authoritative physical equations and raw observation reduction.

## 11. Exit criteria

1. D0 detects injected error classes on identifiable manufactured cases and abstains on confounded cases.
2. D1 demonstrates the parameter/discrepancy confounding negative control.
3. D2 demonstrates that statistical correction cannot override failed physical ledgers.
4. D3 emits a multidimensional, versioned validation envelope.
5. D4 demonstrates calibrated held-out uncertainty and explicit out-of-envelope abstention.
6. D5 selects a held-out experiment that reduces a declared model/discrepancy ambiguity at matched cost.
7. A source-pinned EHD dataset and an independent published measurement cohort are used before any physical discrepancy or extrapolation claim.

**Current state:** specification only. No model-discrepancy implementation, EHD calibration, coverage result, extrapolation detector or physical validation is claimed.
