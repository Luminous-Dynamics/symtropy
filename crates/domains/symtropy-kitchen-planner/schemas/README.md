# Power-Constrained Kitchen Plan Fixture Contract

- `PowerConstrainedPlanV1.schema.json` describes the serializable plan/envelope/task-profile shape.
- `fixtures/power-envelope-v1.json` contains synthetic-only positive and negative fixtures for the power-aware scheduler.

The fixture named `missing-task-profile-is-rejected-by-runtime` is intentionally structurally valid as a `PowerConstrainedPlanV1` shape. JSON Schema cannot express exact one-to-one task/profile coverage. The Rust runtime verifier must reject the omitted task profile. The same is true for duplicate IDs, task-duration coverage, time ordering and aggregate overload.

No fixture values describe a real South African household, appliance, inverter or branch circuit. Replace them only with site- and appliance-specific profiles carrying source, method, calibration and uncertainty evidence.

Runtime implementation:
- `src/power.rs`
- Draft PR: https://github.com/Luminous-Dynamics/symtropy/pull/2230


### Background load rule

The declared household background load is continuous, not a task startup transient. Runtime validation therefore requires `background_real_power_w <= continuous_real_power_limit_w` even when the surge ceiling is higher. The negative fixture `background-load-above-continuous-limit-rejected` is structurally valid but must fail the Rust input verifier. JSON Schema cannot express this cross-field numeric comparison by itself.
