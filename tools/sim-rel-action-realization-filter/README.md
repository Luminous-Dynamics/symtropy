# SIM-REL-025 action-realization filter reference

This crate is the first bounded implementation slice for
[SIM-REL-025 / issue #3116](https://github.com/Luminous-Dynamics/symtropy/issues/3116).
It is deliberately a **filtering reference**, not yet a policy optimizer or an
actuator controller.

## Research contract

A command and the intervention that physically occurred are different variables.
The hidden state is the joint tuple:

`(mechanism, process state, sensor state, apparatus state)`.

For a command, a finite joint kernel supplies exact rational outcomes over:

- next mechanism/process/sensor/apparatus state;
- realized intervention class;
- optional execution readback;
- scientific measurement;
- terminal hazard.

The filter conditions on the public observation. A missing readback is represented
by omitting that field from the observation, not by assuming successful execution.
The posterior retains the correlation between the next joint state and the
realized intervention. Its next-step belief marginalizes the last action label only
because all persistent consequences must already be encoded in the next apparatus
and process state.

A model whose future depends on an earlier realized action must include sufficient
history in its apparatus/process state. This is the Markov-sufficiency obligation.

## Included falsification fixture

The binary fixture has `measurement = mechanism XOR execution_error`.
Without execution readback, the measurement leaves the mechanism posterior at
`(1/2, 1/2)`. With a reliable readback, the same measurement identifies the
mechanism exactly. This prevents the filter from treating a commanded intervention
as proof that the intervention occurred.

Additional tests cover dropped interventions, persistent apparatus-state effects
across repeated commands, impossible observations, exact normalization and
malformed kernels.

## Exactness and limits

All probabilities use checked rational arithmetic. Kernel rows and the initial
belief must normalize exactly. Zero-likelihood observations return a typed error;
the filter never normalizes an empty posterior. Arithmetic overflow and malformed
state/action references fail closed.

This reference is exact only for its finite declared kernel, prior and observation
semantics. It does not infer continuous action errors, validate a physical
apparatus, optimize experiment policies, authorize actuation, or certify safety.
The next stages are independent multi-step replay, exhaustive policy-tree
comparison, a bounded risk/cost/abstention optimizer, and integration with the
SIM-REL-013/016 contracts through an explicit versioned adapter.

## Qualification

The dedicated workflow pins Rust 1.96.0 and checks formatting, locked tests,
strict Clippy, exact checkout identity and a clean tree. A passing hosted run is
required before this source is described as qualified.
