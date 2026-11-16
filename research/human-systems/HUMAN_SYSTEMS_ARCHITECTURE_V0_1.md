# Human-systems simulation architecture v0.1

**Status:** design proposal; not an implemented or validated human model.  
**Parent:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625).

## 1. Core design principle

A human is not one state vector and cannot be made realistic by adding more scalar "health", "stress", "energy", "consciousness" or "resilience" values.

Represent the simulated person as a composition of independently specified subsystems. Each subsystem has its own state, equations, timebase, parameters, observation model, uncertainty and declared validity domain. The composed simulation must retain which subsystem produced each output.

## 2. Fidelity ladder

### H0 — situated synthetic actor

A synthetic actor moves through a Symtropy environment, receives task/resource events, and follows a transparent finite-state or utility policy. This is useful for software integration and scenario mechanics, not evidence of human behavior.

### H1 — bounded physiological subsystem

Add individual mechanisms with published equations or reusable external models. A sensible first slice is sleep pressure / circadian timing and activity load. Cardiovascular, respiratory, thermoregulatory, hydration, metabolic, endocrine and pharmacokinetic models are separate increments, not inferred from one generic "metabolism" scalar.

### H2 — observed/estimated internal state

Map sensor streams to explicit observations with units, event time, measurement uncertainty, missingness and sensor provenance. Keep measurements separate from latent states and from model predictions. No unmeasured variable becomes "ground truth" merely because an agent needs it.

### H3 — cognitive and behavioral agent

Use Symthaea as a candidate controller over declared goals, beliefs, uncertainty, memory and action costs. Treat the current FEP implementation as an agent architecture—not a validated human cognitive model. The event-time, modality, precision and missingness semantics in [Symthaea #12407](https://github.com/Luminous-Dynamics/symthaea/issues/12407) are prerequisite integration work.

### H4 — embodied human-in-environment system

Couple movement and contact to the environment. Symtropy rigid-body physics can represent gross spatial motion and collision; realistic joint/muscle forces, tendon dynamics and soft tissue require a qualified musculoskeletal backend such as OpenSim or another declared solver. Do not imply that a capsule-and-joint avatar is anatomical fidelity.

### H5 — heterogeneous population / social system

Compose multiple bounded agents with distinct parameters, schedules, resources, roles and social contacts. Calibrate population distributions and interaction rules separately. LLM-generated personas are not acceptable as a population ground truth.

### H6 — personalized digital twin

Requires individual-specific baseline calibration, repeated updates from valid observations, predictive performance for that person, uncertainty and external/prospective qualification in a precisely defined context. This is explicitly not the first-phase target.

## 3. Subsystem responsibility map

| Concern | Candidate owner | Current status / boundary |
|---|---|---|
| Spatial environment, collisions, rigid-body movement | Symtropy physics / Bevy | Existing generic physics; not musculoskeletal anatomy |
| Environmental fields and shared field primitives | `symtropy-lifesim-core` | Existing CPU field substrate; ecological channels are not human organ variables |
| Sleep/circadian model | Dedicated physiology adapter; CellML/peer-reviewed model candidate | Must be adult-appropriate and dynamically specified; existing Symthaea Nurture sleep schedule is developmental/age-schedule logic, not a validated adult Process S/C model |
| Whole-body system physiology | BioGears or qualified subsystem models from CellML/Physiome | External reference candidate; model-specific verification and validation still required |
| Musculoskeletal movement | OpenSim | External open-source solver candidate; verify selected model, boundary conditions and task fit |
| Beliefs and action selection | Symthaea FEP / cognitive loop | Candidate agent substrate; not empirically established human cognition |
| Interoceptive interpretation | Symthaea interoception / affect modules | Exploratory computational constructs; do not equate them to measured hormones or clinical state |
| Human behavioral data | Public datasets and later consented individual observations | Data source and sampling process must be explicit |
| Evidence and model lineage | Existing repository receipts / later Mycelix integration | A signature proves record integrity, not biological truth or clinical authority |

## 4. Typed subsystem contract

Each model adapter should declare the following before it is composed:

```text
SubmodelContractV1
  model_id + semantic_version + source_revision
  maturity_label
  state_variables[]: name, unit, valid_range, interpretation
  input_variables[]: name, unit, sampling, missingness semantics
  output_variables[]: name, unit, uncertainty representation
  parameter_set: values + provenance + calibration domain
  time_contract: event-time unit, integration method, supported dt range
  assumptions[]
  validity_domain[]
  unsupported_conditions[]
  deterministic_seed_policy
  validation_receipts[]
```

Unknown units, unsupported timebases, missing required inputs, out-of-domain parameters, or unqualified model revisions must be explicit errors or explicit missing-state outputs. They must not be silently converted to plausible-looking defaults.

## 5. Coupling and time

Use at least three clocks:

1. **Physical/event time:** elapsed time in the simulated person/environment.
2. **Observation time:** when the sensor or observer actually sampled the state.
3. **Compute/receive time:** when Symthaea received and processed the observation.

These clocks cannot be collapsed into one agent-cycle counter. Use a deterministic event scheduler with declared subsystem update order, multi-rate integration, explicit interpolation/hold policies, and bounded handling of late/duplicate/out-of-order observations.

The physical system advances once per event-time interval. A repeated or delayed sensor packet must not advance the human's physiology again. Missing observations provide no measurement likelihood; they are not zeros and are not prior-mean pseudo-observations.

## 6. State representation

Keep the following distinct:

- **physical state:** position, velocity, forces, body mechanics;
- **physiological state:** model-specific variables such as sleep pressure or heart dynamics;
- **latent cognitive state:** estimated beliefs/goals/precision in the declared agent model;
- **observed state:** actual sensor-derived values and timestamps;
- **inferred state:** posterior estimates and uncertainty;
- **behavior:** selected action and action outcome;
- **environmental context:** temperature, lighting, workload, resources, social events.

A single "human health score" must not replace these channels. If a downstream task needs a scalar, its definition, range, source, and limitations must be explicit.

## 7. Model selection

Prefer the smallest model that can answer the declared question:

- Use a simple two-process sleep model before a whole-body physiology engine when the question is sleep pressure.
- Use OpenSim only when muscle/joint mechanics change the answer.
- Use BioGears only when coupled system physiology is required and the chosen model is appropriate.
- Use a CellML submodel when one published mechanism suffices.
- Use Symtropy for spatial/environment dynamics and Symthaea for experimental agent policies—not as substitutes for physiological equations.

Never average incompatible submodels into a composite "human realism" score.

## 8. Prohibited shortcuts

- Do not infer neurotransmitter concentrations from a diagnostic label or a hand-authored severity score.
- Do not use Phi/IIT-like quantities as physiological measurements, health scores, or clinical endpoints.
- Do not describe game/keyboard/mouse telemetry heuristics as validated human stress biomarkers.
- Do not treat behavioral plausibility, verbal self-report generated by an LLM, or agent introspection as measured human behavior.
- Do not validate counterfactual interventions against observational association alone.
- Do not promote a simulation to a digital twin because it has an avatar, dashboard, or patient-shaped parameter file.

## 9. Validation references

- NASEM-aligned healthcare twin criteria emphasise individual personalization, dynamic updating and predictive capability; the 2025 scoping review found only 18/149 studies met all criteria and very few reported VVUQ: https://www.nature.com/articles/s41746-025-01910-w
- 2026 clinical-trial roadmap positions twins for exploratory design and carefully validated use, and stresses data fitness, causal assumptions, external validation and uncertainty: https://www.nature.com/articles/s41746-026-02871-4
- 2026 physiology review distinguishes prediction from explanation and says evidence is stronger for mechanistic insight/early feasibility than improved patient outcomes: https://www.annualreviews.org/content/journals/10.1146/annurev-physiol-040225-113230
