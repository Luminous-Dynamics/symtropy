# Kitchen Planner Power Envelope V1

Status: implementation proposal on the draft kitchen planner PR. No household has been measured and no appliance profile is qualified by this file.

## API

The `power` module composes with the existing `KitchenPlanV1` and `KitchenScheduleReceiptV1` types. It does not fork the scheduling model.

- `PowerEnvelopeV1` declares continuous real-power limit, surge real-power limit, maximum contiguous surge duration, and background household real-power load.
- `TaskPowerProfileV1` binds one profile to each plan task and records a source reference plus evidence class.
- `PowerSegmentV1` represents piecewise-constant real power over a relative time interval. Segments must be ordered, contiguous and cover the complete task duration.
- `validate_power_envelope` independently rechecks the schedule and aggregates simultaneous real-power loads, including the declared background load.
- `build_power_constrained_schedule` first attempts the existing deterministic earliest-gap schedule. If that violates the envelope, it runs a deterministic event-boundary power-aware earliest-gap search that delays only tasks as needed. It then tries the serial topological baseline as a final conservative fallback. If all fail, it returns `NoScheduleFoundByHeuristics` and the violations.

Manual / non-electric work is represented with an explicit zero-watt segment. A missing profile is not treated as zero.

## Interpretation

- All limits are real watts (W), not VA/kVA. Apparent-power, power-factor, inrush waveform and power-quality limits require separate measured fields before a particular inverter/generator can be qualified.
- The envelope is an explicit site/user input. Do not infer available capacity from an appliance's marketing page or nameplate alone.
- A `Measured` evidence class is preserved but does not itself prove instrument calibration, correct placement or uncertainty. Those need separate evidence.
- Declared, modelled and simulated profiles remain visibly distinct; a valid schedule report is not automatically a measured household result.
- The event-boundary search is a heuristic, not a proof of globally earliest or optimal scheduling. It explores existing task/profile boundaries and surge-duration offsets rather than every millisecond. The serial fallback is conservative and may miss a better feasible interleaving. Failure means the implemented heuristics found no schedule, not that the plan is globally infeasible.
- This module is advisory only. It does not control relays, appliances, pressure cycles or household circuits.

## Tests added

The unit tests cover:
- parallel schedule overload followed by a minimal 5 ms stagger that remains within the surge-duration allowance;
- one task whose draw exceeds the surge limit;
- missing task profiles;
- background household draw;
- short startup surge versus sustained over-current;
- surges that exceed the duration allowance despite remaining below the peak-watt ceiling;
- gaps in a power profile;
- evidence-class preservation without promotion.

The planner PR now adds a dedicated `cargo test -p symtropy-kitchen-planner --all-targets` CI step. Exact-head cross-platform results are required before merge.
