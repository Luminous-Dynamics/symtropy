# Human-systems simulation in Symtropy

**Programme:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625)  
**Status:** research design; no validated human simulator, physiological digital twin, or clinical capability is claimed.  
**Scope:** synthetic-first, evidence-bounded human-system and human-in-environment simulation.

## Why this programme exists

Symtropy can simulate physical environments and rigid-body motion. Symtropy's `symtropy-lifesim-core` provides deterministic field primitives and generic living-system abstractions. Symthaea provides cognitive/FEP, HDC/LTC, affective, interoceptive and behavioral-telemetry components.

These are useful building blocks, but they do not add up to a physiologically faithful person. We should not confuse:

- a rigid-body avatar with a human musculoskeletal model;
- a scalar energy/stress variable with whole-body physiology;
- an active-inference agent with a validated model of human cognition;
- a hand-authored psychiatric scenario with a patient-specific clinical twin;
- a convincing synthetic narrative with measured human behavior.

The first objective is therefore not a virtual human that looks convincing. It is a testable system that predicts a narrow set of human state variables and task outcomes under declared conditions, exposes uncertainty, and fails closed outside its evidence envelope.

## First context of use

Start with a **human day / human operator** in an explicitly modelled environment:

- time of day and sleep opportunity;
- accumulated sleep pressure and recovery;
- physical activity and task demand;
- workload, interruptions and environmental conditions;
- measured/estimated state with uncertainty;
- bounded behavioral choices such as continue, pause, rest, or change task.

This gives us a non-clinical research target useful for human-centred robotics, workplaces, education, care environments and assistive systems. It also allows public observational datasets to inform selected state and sensor models without requiring controlled patient data.

## Architecture and evidence boundaries

- [Human-systems architecture v0.1](HUMAN_SYSTEMS_ARCHITECTURE_V0_1.md) — subsystem responsibilities, coupling contracts and fidelity levels.
- [Benchmark protocol v0.1](HUMAN_SYSTEMS_BENCHMARK_PROTOCOL_V0_1.md) — known-answer simulations, public-data evaluation, baselines, metrics and counterfactual gates.
- [Dataset and model-resource register v0.1](DATASET_AND_MODEL_RESOURCE_REGISTER_V0_1.md) — dataset scope, access terms and external model engines.
- [Human observation-process protocol v0.1](HUMAN_OBSERVATION_PROCESS_PROTOCOL_V0_1.md) — synthetic sensor worlds, missingness, delay, clock semantics, identifiability, transfer and SOTA comparison. Tracked by [#5664](https://github.com/Luminous-Dynamics/symtropy/issues/5664).
- [Machine-readable 12-scenario catalogue](HUMAN_OBSERVATION_SCENARIOS_V0_1.json) — synthetic arbitrary-unit scenario definitions; not human data.

### Fidelity labels

| Label | Meaning | Permitted conclusion |
|---|---|---|
| Mechanistic reference | Equations and numerical behavior verified against declared analytic/reference cases | The code implements the declared equations |
| Synthetic-qualified | Known-answer synthetic tests pass across declared seeds/conditions | The simulator behaves as designed on those synthetic worlds |
| Public-data calibrated | Parameters fit public observational data under a documented split | Calibration to that dataset/domain only |
| Externally evaluated | Locked model passes a subject- or study-held-out evaluation on another dataset | Generalization only to that evaluation domain |
| Intervention-qualified | Counterfactual predictions match suitable intervention evidence under declared assumptions | Intervention claim limited to that evidence and context |
| Personalized twin | Person-specific state is repeatedly updated and prospectively predicts the individual within a qualified context | Only the specific state variables and decisions actually validated |

A label is not promoted merely because code exists, tests pass, a 3D body renders, or an LLM can describe the agent's experiences.

## What is already available

- **Symtropy physics:** dimension-generic rigid-body motion, collision detection, joints and deterministic record/replay. It does not presently provide validated muscle, tendon, soft-tissue, organ or whole-body physiology.
- **Symtropy LifeSim core:** CPU-first spatial fields and generic metabolic/stress primitives shared by ecological simulations. Those fields are not human physiology.
- **Symthaea FEP / active inference:** a candidate cognitive-agent substrate. Its public FEP observation contract has known gaps around per-dimension missingness, calibrated precision, modality semantics and physical event time; see [Symthaea #12407](https://github.com/Luminous-Dynamics/symthaea/issues/12407). Do not silently bypass those gaps.
- **Symthaea therapeutic psychiatry simulator:** explicitly hand-authored and unvalidated; it prohibits real client data and clinical use. It is not the starting human physiology engine.
- **External references:** OpenSim (musculoskeletal simulation), BioGears (whole-body physiology reference engine) and CellML/Physiome models (published subsystem equations). Their own assumptions and validation limits must remain visible.

## Non-goals

No complete human replication, consciousness claim, diagnosis, treatment selection, prescribing, therapeutic advice, autonomous intervention, or patient-specific clinical digital twin in this phase. Public observational data can qualify measurement and prediction components; it cannot substitute for randomized/interventional evidence when validating treatment effects.

## Reproducibility

No participant-level records belong in this repository. Dataset adapters must be local-only, record exact source/version/license/split metadata, and emit only rights-compatible aggregate receipts. Every simulation must record model/configuration revisions, seed, timebase, initial conditions and parameter provenance.
