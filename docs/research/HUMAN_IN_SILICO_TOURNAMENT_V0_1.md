# Human In-Silico Model Tournament v0.1

**Status:** synthetic-only research protocol; not physiological validation.  
**Parent:** [Symtropy #5847](https://github.com/Luminous-Dynamics/symtropy/issues/5847).  
**Programme:** [HUMAN-MULTISCALE-COUPLING-001 / #5772](https://github.com/Luminous-Dynamics/symtropy/issues/5772).

## Purpose

Qualify the mathematics and software involved in composing independently sourced physiological model components. The benchmark is not a patient simulator, personalized digital twin, medical device, occupational safety instrument, or clinical decision system.

The federation must be able to answer:
1. Do each component's equations and numerical implementation behave as declared?
2. Do units, timebases, fluxes and state transitions remain coherent across model boundaries?
3. Does coupling add measurable value beyond independently running each component?
4. Does the federation remain stable under multi-rate integration, delay, missingness, model discrepancy and regime change?
5. Can the candidate recognize when the available observations do not identify the requested quantity?

## Hard boundary: no human subjects or participant data

This track uses no recruitment, human experiments, interviews, surveys, wearable collection, personal monitoring, patient records, clinical records, identifiable or de-identified participant-level data, human-subject raw sensor records, human-record-derived training examples, or personal user data.

All benchmark trajectories and virtual parameter ensembles are generated in silico from frozen equations, declared synthetic parameter ranges and deterministic seeds. Parameter vectors are mathematical model configurations, not simulated identities, demographic groups or clinical populations.

Published mathematical models and source code may be used as reference implementations after source, version and license audits. A model's historical human validation does not turn our synthetic software tests into human validation. We do not download or reuse source-study participant records in this track.

Real non-human measurements may be tested separately for generic methods (for example, Drosophila sensorimotor, plant hydraulic or fungal network data). Such results qualify software or methods only; they do not validate human physiological outputs or authorize cross-species parameter transfer.

When a claim would require human-subject evidence, label it **BLOCKED — HUMAN EVIDENCE NOT PERMITTED IN THIS TRACK**.

## Architecture and evidence boundaries

### Roles

- **Symtropy — simulated world / plant:** scenario inputs, environment, admitted model execution, state and flux accounting, solver configuration, deterministic replay and scorer-side latent truth.
- **Symthaea — candidate inference / policy:** estimator-facing observation events, state estimates, uncertainty, system identification, information requests, candidate policies and abstention.
- **Neutral coupling contract:** versioned typed ports, units, sign conventions, event time, clock domain, cadence, interpolation, uncertainty, provenance, validity domain and balance-group semantics.

The candidate must not receive simulator truth, alternate-world IDs, seeds, scorer maps, future exposures or hidden structural discrepancy settings.

### Evidence maturity labels

| Level | Meaning | Does not establish |
|---|---|---|
| M0 — illustrative | Concept or diagram | Executable behavior |
| M1 — mathematical | Equations, units, assumptions and domain are documented | Correct implementation |
| M2 — software verified | Analytic fixtures, invariants, numerical tests and replay pass | Biological accuracy |
| M3 — synthetic benchmarked | Held-out synthetic worlds and negative controls are scored | External physiological validity |
| M4 — non-human empirical method test | Generic method tested against permitted non-human observations | Human physiology |
| M5 — human empirical validity | Requires separate human evidence and governance | Out of scope for this programme |

Evidence maturity is specific to each output and context of use. Do not promote a subsystem merely because code exists or a synthetic test passes.

## First bounded federation

Start with a small system that tests meaningful coupling:

1. Sleep homeostasis: Process S with explicit wake/sleep intervals.
2. Circadian drive: fixed-period Process C baseline and a separately versioned light-responsive Process L/P oscillator.
3. Workload and activity: explicit synthetic boundary conditions.
4. Thermal response: source-pinned, multi-node thermoregulation reference with lower-fidelity comparators.
5. Water balance: admitted only when a source model defines its source, sink and transfer terms.
6. Cardiovascular response: a separately sourced model; do not infer cardiac output, blood volume or renal state directly from a sweat-rate output.
7. Observation world: separate sensor sampling, noise, drift, missingness, transport delay and availability timestamps.

Do not automatically add glucose, endocrine, immune, renal, neural or musculoskeletal subsystems. Each requires source equations, typed ports, balance semantics, validity limits and verification fixtures.

## Model and standards landscape

This is a component tournament, not a contest to crown one universal whole-human simulator.

| Domain | Candidate references | Comparison purpose and caveat |
|---|---|---|
| Sleep | Borbély Process S/C; isolated Symtropy Rust reference | Equation reproduction; not sleep staging or alertness validation |
| Circadian | Fixed Process C; Kronauer/Jewett Process L/P | Test whether light input changes oscillator phase; a fixed Fourier wave is only a periodic baseline |
| Thermoregulation | Gagge two-node; Stolwijk multi-node; JOS-3; aJOS-3; Fiala-family implementations where rights permit | Fidelity/runtime comparison and model disagreement under matched boundary conditions |
| Heat strain | ISO 7933 Predicted Heat Strain | Standards comparator, subject to access/licence; not a mechanistic whole-body twin |
| Cardiovascular | PhysioNet CVSim / RCVSIM | Hemodynamic dynamics where declared outputs and context match |
| Whole-body reference | BioGears | Useful reference candidate; preserve beta status and self-reported validation limitations |
| Musculoskeletal (later) | OpenSim / Moco | Muscle-driven mechanics and optimal control; rigid-body physics is not a muscle model |
| Model interchange | CellML, SBML Level 3, SED-ML, MIASE, COMBINE/OMEX | Reproducibility and exchange, not proof of biological validity |

Primary sources:
- [JOS-3 source](https://github.com/TanabeLab/JOS-3)
- [JOS-3 paper](https://doi.org/10.1016/j.enbuild.2020.110575)
- [Kronauer/Forger/Jewett light-response model](https://doi.org/10.1177/074873049901400609)
- [Jewett/Forger/Kronauer revised limit-cycle model](https://doi.org/10.1177/074873049901400608)
- [PhysioNet physiological models](https://physionet.org/about/content/)
- [BioGears documentation and validation statement](https://biogearsengine.com/documentation/index.html)
- [OpenSim Moco documentation](https://opensim-org.github.io/opensim-moco-site/docs/1.3.0/)
- [Physiome Model Repository](https://models.physiomeproject.org/)
- [SBML specifications](https://sbml.org/documents/specifications/)
- [CellML tools](https://www.cellml.org/tools/modeling-environments)

### Source-audit cautions

- Published JOS-3 descriptions do not consistently report the same node count (83 versus 85). Resolve against the exact pinned implementation and state inventory rather than hard-coding a secondary-source count.
- Published JOS-3 and 2025 multi-segmentation results are model-specific and condition-specific. Do not promote paper-reported metrics to universal accuracy guarantees or claim independent reproduction without reproducing the source-compatible protocol.
- BioGears describes its current deployment as beta and states that rigorous sensitivity analysis has not been completed and that its team is unaware of independent third-party validation. It is a comparator, not ground truth.
- ECGSYN is a useful synthetic ECG signal generator; waveform realism is not proof of underlying physiological accuracy.
- CellML, SBML and SED-ML aid model and experiment exchange. Schema validity alone does not validate equations or a model's domain.

### Real environmental forcing without human records

A safe route to add genuinely observed data is to use **environmental boundary conditions**, not human response data. The NASA POWER hourly API provides globally located hourly meteorological and solar time series (hourly archive from 2001 to near-real-time), with UTC or local-solar-time options. ERA5 provides hourly atmospheric and land-surface reanalysis from 1940 onward on a roughly 0.25-degree grid and is distributed under CC BY 4.0. Candidate variables include air temperature, dew-point/derived humidity, wind components and solar radiation.

Sources: [NASA POWER hourly API](https://power.larc.nasa.gov/docs/services/api/temporal/hourly/), [ERA5 hourly single-level dataset](https://cds.climate.copernicus.eu/stac-browser/collections/reanalysis-era5-single-levels).

Use these data only as real external forcing for synthetic model runs: real weather in, simulated physiology out. This validates the ingestion, timestamp, unit-conversion and boundary-condition pipeline against observed environmental variability. It does **not** validate the model's human response, and it must not be described as a real-human scenario.

A second, strictly separate method lane may use existing public non-human data in the repository (for example Drosophila sensorimotor experiments, plant hydraulic/stomatal observations and fungal-network measurements) to test generic observation, event, conservation and causal-evaluation methods. Keep taxa, model equations and parameter provenance separate; no cross-species physiological claims or human parameter transfer.

Every adapter must pin exact release/commit, license, equations, parameter defaults, dependencies, solver, initial-state procedure, output units and unsupported inputs before admission.

## Synthetic challenge catalogue

The companion [scenarios.json](../../benchmarks/human-systems/v0.1/scenarios.json) defines 16 synthetic scenario families:

| ID | Scenario | Primary falsification target |
|---|---|---|
| SYN-001 | Clean reference trajectory | Equation/replay mismatch |
| SYN-002 | Sleep/wake transition boundary | Integrating across an unrepresented state transition |
| SYN-003 | Light pulse / circadian phase response | Treating fixed Process C as light entrainment |
| SYN-004 | Thermal step and recovery | Incorrect thermal response or unit mapping |
| SYN-005 | Workload pulse and rest interval | Incorrect boundary-condition timing |
| SYN-006 | Structural model disagreement | Hiding model-form uncertainty in an ensemble mean |
| SYN-007 | Heat/water balance closure | Unpaired source/sink or duplicated flux |
| SYN-008 | Multi-rate coupling | Solver instability or update-order dependence |
| SYN-009 | Missingness regimes | Treating missing as zero or assuming MCAR |
| SYN-010 | Clock, delay and packet faults | Double advancement or use of unavailable future data |
| SYN-011 | Common-cause sensor outage | False independent-sensor assumptions |
| SYN-012 | Observationally equivalent worlds | False point certainty when state is not identified |
| SYN-013 | Assignment/delivery/exposure mismatch | Attributing effects to undelivered actions |
| SYN-014 | Coupling semantic corruption | Wrong units, sign, direction or double-counted flux |
| SYN-015 | Out-of-domain input | Silent extrapolation beyond declared validity |
| SYN-016 | Observed environmental forcing with synthetic response | Treating real weather inputs as human response validation |

Use separate truth generators and candidate models to reduce inverse-crime risk. Use analytic known-answer cases and structurally distinct model families for cross-model claims. Keep seeds, model discrepancy and scorer truth outside candidate-visible inputs.

## Benchmark scorecard

Do not compress these axes into a single realism or intelligence score.

### A. Equation and numerical correctness
- Native-unit analytic/reference error.
- Dimensional correctness and explicit conversion audit.
- Positivity and boundedness where mathematically required.
- Solver convergence order and tolerance sensitivity.
- Deterministic replay and event-boundary correctness.

### B. Conservation and coupling
- Mass, water, heat and energy residual for each declared balance group.
- Interface closure residual per edge.
- Flux duplication/loss rate.
- Coupling-interval and update-order sensitivity.
- Error added by federation relative to independently run components.

### C. Prediction and identifiability
- MAE/RMSE and bias in original units, per output.
- CRPS/log score only for explicitly probabilistic outputs.
- Interval coverage and width; calibration by scenario/regime.
- Parameter recovery and structural/practical identifiability.
- False-certainty rate and selective risk.
- Worst-regime and lower-tail metrics, not just averages.

### D. Causal/event semantics
- Exact recovery of the declared synthetic estimand where identifiable.
- False causal claim rate under null and observational equivalence.
- Correct separation of assignment, delivery, exposure, response and observation.
- Abstention or identified-set correctness when truth is not recoverable.
- No future observation, hidden state or scorer-truth access.

### E. Resource efficiency
- p50/p95/p99 simulation and inference latency.
- Peak memory and allocation count.
- Energy per simulated hour where measurable on pinned hardware.
- Accuracy/calibration/coverage versus runtime Pareto frontier.
- Model startup and parameter-update costs.

### F. Evidence integrity
- Exact source, model, executable, input and config digests.
- Candidate/scorer process and information boundary.
- License and data-rights manifest.
- Explicit validity domain and unsupported conditions.
- Failed, skipped and pending outcomes retained as distinct states.

## Pre-registered software targets

These are engineering targets, not physiological targets:

- 100% of malformed-unit, unsupported-port, stale-event and duplicate-event negative controls reject before state mutation.
- 100% of candidate interfaces are unable to read latent truth, alternate-world metadata or future observations.
- Zero false point-certainty claims on exact observational-equivalence fixtures.
- Fixed-seed replay is exact on the same pinned execution environment; cross-platform tolerance is reported separately.
- Conservative analytic fixtures close to their declared numerical tolerance. Composed fixtures report residuals relative to the tighter participating solver tolerance; any looser bound requires a written, testable justification.
- Measured convergence order matches the declared method on smooth, well-conditioned fixtures.
- Out-of-domain inputs reject or return a typed unsupported status; they are not silently extrapolated.
- A better-than-SOTA claim requires a source-compatible, independently reproduced, held-out comparison with a preregistered primary metric and no material regression in calibration, lower-tail performance or compute.

## Reproducibility receipt

Every run emits a machine-readable receipt containing:
- benchmark ID and schema version;
- human_subject_data_used: false;
- human_participant_records_used: false;
- exact scenario IDs and seeds;
- truth-generator and candidate executable digests;
- model revisions, equation sources and license references;
- parameters, units, initial/boundary conditions and solver settings;
- observation-process configuration and candidate information cutoff;
- scorer version and metric definitions;
- per-scenario results, failures, abstentions and unsupported conditions;
- hardware/runtime/resource measurements;
- evidence maturity label and explicit claim ceiling.

The two false fields attest to run inputs; they do not prove model validity. They must be generated from the run manifest and checked in review.

## Follow-on: qualify the co-simulation master algorithm

The typed port graph and individually verified component models do not, by themselves, qualify the federation. FMI 3.0.2 supplies Model Exchange, Co-Simulation and Scheduled Execution interfaces, including clocks, event handling, intermediate updates and early return. The FMI specification deliberately leaves the co-simulation algorithm to the importer: global time advancement, data exchange, clock triggering and event handling belong to the master/importer.

The follow-on benchmark is [HUMAN-MULTIRATE-CO-SIM-001 / #5871](https://github.com/Luminous-Dynamics/symtropy/issues/5871). It compares synchronous Jacobi, staggered Gauss-Seidel, paired shared-flux exchange, iterative/implicit coupling, waveform relaxation, adaptive communication steps and clocked execution on abstract synthetic systems with analytic or independent high-accuracy truth.

The catalogue includes **SYN-017**, a two-compartment exact-oracle challenge. It measures state error, conservation residual, communication-step convergence, update-order sensitivity, event ordering, rollback cost and capability handling independently. This is a numerical software benchmark; the states and coefficients are dimensionless/arbitrary test fixtures, not physiological values.

FMI references:
- [FMI 3.0.2 specification](https://fmi-standard.org/docs/3.0.2/)
- [FMI 3.0 Co-Simulation overview](https://fmi-standard.org/docs/3.0/)

An FMI package or a successful FMU step is not evidence that the imported model is credible, that the master algorithm is stable, or that cross-model fluxes are conserved. Keep the FMI runtime optional; the core synthetic benchmark must run without an FMI dependency.

## First implementation sequence

1. Freeze this protocol and the scenario catalogue.
2. Audit the isolated Process S/C Rust reference in [PR #5634](https://github.com/Luminous-Dynamics/symtropy/pull/5634); retain its software-only claim ceiling.
3. Reuse the HOBS synthetic sensor world (#5664) and evaluator boundary (#5730); do not create a second observation stack.
4. Reproduce and pin one reference thermoregulation implementation before writing an adapter.
5. Define exact JOS-3 ports through #5779, including units, supported cadence, sign, boundary conditions and validity domain.
6. Compose only the first demonstrator; add cardiovascular engines only after source ports can be mapped without guessed conversions.
7. Run model ablations, model-discrepancy tests and multi-rate sensitivity.
8. Publish a result card with passed, failed, blocked, pending and not-applicable states.

## Claim ceiling

A successful v0.1 run may establish that a versioned synthetic model federation is reproducible, dimensionally coherent, numerically stable within stated tolerances, robust to specified observation faults, and able to abstain on non-identifiable scenarios.

It does not establish accuracy for any real human, a personalized or universal human digital twin, clinical/occupational/safety validity, cognition or private mental-state inference, treatment/intervention effectiveness, or generalization to a real population. Human empirical validity remains untested and outside this benchmark.
