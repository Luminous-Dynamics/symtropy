# Kitchen-R Adapter Contract V1

This adapter records evidence from three separate modes:

1. `planning_only`: planner output and independent plan verifier; low-level control must be marked not-run.
2. `low_level_control_only`: low-level policy execution against fixed goals; planning must be marked not-run.
3. `integrated`: planner and controller both execute; integrated goal status, environment restoration and any human takeover are recorded.

The versioned `KitchenRobotEpisodeReceiptV1.schema.json` binds each episode to the instruction/source, rights status, deterministic seed, simulator/scene/asset/physics revisions, robot model, initial-state digest, perturbations, component results, recovery attempts and final state digest.

The JSON fixture bundle contains planning-only success, low-level policy success, integrated failure with human takeover, and a negative mode-invariant case. A failed integrated episode remains a first-class record and stays in the denominator; a successful plan is not counted as integrated success when control fails.

## Evidence boundary

All receipts are labelled `simulated`. Kitchen-R's digital twin and episode outcomes do not qualify physical robot safety, collision/contact risk in a real kitchen, pressure appliances, food safety, hygiene, taste or meal quality. Simulator/asset versions and rights must be frozen before an external comparison. Keep policy inference time, simulation time, reset time and human takeover separately available rather than hiding them in one elapsed-time number.


## Task-aware safety audit extension (SafeVLA-style)

Each `KitchenRobotEpisodeReceiptV1` now requires a `safety` assessment. It preserves, per episode:
- native task outcome separately from safety claim state;
- task-aware applicable / not-applicable / unknown safety specifications;
- each specification's signal refs, method ref, robustness score and violation severity;
- applicable/satisfied/violated/unknown counts and signal coverage;
- success-but-unsafe flag, worst violation severity and safe-stop outcome.

Planning-only episodes explicitly state that no trajectory safety audit ran and make no safety claim. A success-but-unsafe episode remains a successful native task but is marked unsafe. Missing required telemetry remains indeterminate; it cannot be labelled safe. Safe claims require a complete audit, at least one applicable specification, zero unknown/violated specs and 100% required signal coverage.

The receipt remains simulation-only. These fields do not qualify physical robot or appliance safety, pressure vessels, blades, heaters or human interaction.
