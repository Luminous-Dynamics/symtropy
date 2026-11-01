# SIM-REL-039 — exact finite physical challenge compiler

Status: first exact one-node thermal challenge implementation on a research branch. It is not a validated digital twin of a real furnace, does not prove physical safety, and does not yet cover mechanical contact or general hybrid dynamics.

## 1. Why a physical challenge track is necessary

SIM-REL-036 establishes exact finite statistical-decision tasks. Those fixtures do not test whether a scientific agent can distinguish a commanded intervention from the action actually realized by an apparatus, or interpret readings from a miscalibrated sensor.

SIM-REL-039 starts the bridge with a deliberately small physical model: a one-node lumped thermal process, a commanded heat pulse, a bounded actuator-delivery fraction, an additive sensor bias, and a finite horizon. The objective is to qualify the causal data path and exact replay before adding more complicated mechanisms.

## 2. Exact discrete thermal model

The model uses normalized rational units and explicit Euler integration for

`C,dot T = P_{mathrm{realized}} - k(T-T_{mathrm{ambient}})`

with the step rule

`T_{n+1} = (1-alpha)T_n + alpha T_a + (\Delta t/C)P_{mathrm{realized}},qquad
\alpha=\Delta t,k/C.`

To keep the reference dynamics monotone and stable, the recipe compiler requires:

- `C > 0` and `dt > 0`;
- `0 <= dt*k/C <= 1`;
- initial temperature not below ambient;
- finite horizon no greater than 32 steps;
- actuator delivery fraction in `[0,1]`;
- a safety ceiling no lower than any initial temperature in the recipe grid.

The commanded power and realized power are separate:

`P_{mathrm{realized}}=\eta P_{mathrm{commanded}},qquad 0\leq\eta\leq1.`

Sensor readback is likewise separate from true process state:

`T_{mathrm{readback}}=T_{mathrm{true}}+b,qquad b\geq0.`

These are intentionally simple fault models. Delayed actuation, backlash, saturation, sign-changing calibration errors, cross-talk, multi-node conduction and phase changes are not implemented in this increment.

## 3. Independent exact step verification

The simulator evaluates the recurrence using retained, ambient and delivered-energy terms. The verifier does not call the simulator. It checks the rearranged exact balance identity:

`C T_{n+1}+\Delta t,kT_n
=CT_n+\Delta t,kT_a+\Delta t,P_{mathrm{realized}}.`

The checker also binds each row to the previous temperature, commanded/realized power, sensor readback, step index and hazard status. It rejects a trajectory that continues after a terminal hazard or truncates before the horizon without a hazard.

This is an independent implementation of the *discrete algebraic balance check*, not an independent physical experiment. Both simulator and verifier still share the declared model parameters and intended equation. The verifier can detect corrupted records and arithmetic inconsistencies; it cannot show the equation describes the real apparatus.

## 4. Challenge artifact separation

The compiler produces three typed artifacts:

1. **Public task:** initial temperature readback, ambient value, commanded power, horizon and safety ceiling. It does not expose hidden heat capacity, conductance, actuator delivery, sensor bias or seed.
2. **Sealed oracle:** hidden parameter values, true initial temperature and exact step receipt.
3. **Generation receipt:** task/recipe/version, family group, split, generator version and seed.

The challenge asks whether the commanded intervention remains inside the declared safety envelope, and explicitly permits abstention when the available readback does not justify a safe claim.

The generator validates family-group assignment through the pre-registered SIM-REL-036 split registry. The stronger replay function regenerates all three artifacts from the exact recipe, registry and evaluator-held seed, then compares the results byte-for-byte at the typed-value level.

The current fixture is one deterministic heat pulse with one sampled hidden mechanism/actuator/sensor configuration. It is not yet an interactive adaptive experiment or a learned model-family inference task.

## 5. Fault attribution and evidence chain

The receipt keeps separate:
- commanded power;
- realized power after actuator delivery;
- true process temperature before and after each step;
- sensor readback after each step;
- hazard flag and final safe-envelope outcome.

This prevents a dropped/partial command from being silently counted as a successful intervention and prevents sensor bias from being mistaken for process state.

The intended future integration is:

`\text{Command} \rightarrow \text{Actuator realization} \rightarrow
\text{Thermal transition} \rightarrow \text{Sensor readback} \rightarrow
\text{Inference} \rightarrow \text{Decision}.`

Only the first four stages are represented by this finite thermal fixture. The inference/decision component remains a separate benchmark harness responsibility.

## 6. Qualification

Current source tests are designed to verify:
- deterministic generation from a fixed seed;
- exact replay from recipe + split registry + seed;
- algebraic balance consistency;
- rejection of altered temperature, realized power, sensor reading and hazard flag;
- stability-guard rejection;
- rejection of unregistered or incorrectly assigned split groups;
- registered Calibration and SealedHoldout task generation.

These tests are not yet exact-head qualified. The branch workflow must run formatting, focused tests, full locked crate tests, strict Clippy, exact checkout identity and a clean-tree check.

## 7. Expansion path

- **SIM-REL-039a:** two-node thermal conduction, signed sensor bias, sensor drift and delayed/partial actuation with exact finite references.
- **SIM-REL-039b:** lumped mechanical mass-spring-damper fixtures, actuator backlash and bounded friction; maintain an explicit energy/dissipation balance.
- **SIM-REL-039c:** finite contact and thermal-mechanical coupling only after separate mechanical and thermal oracles agree on shared fixtures.
- **SIM-REL-026 bridge:** validated interval/reachability enclosures for continuous dynamics. The abstraction residual must be derived from the physical solver/enclosure, not guessed from empirical fit.

For each expansion, preserve the exact small-domain oracle overlap, publish unsupported-case boundaries, and add negative controls for model and sensor faults. Simulation-only success is never a physical-safety certification.
