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

The filter conditions on the public observation. Each observation field distinguishes
three cases: unobserved (`Any`), observed missing (`Missing`), and a concrete vocabulary
value (`Value(id)`). This prevents a missing execution acknowledgement from being
confused with telemetry that was simply not supplied to the inference process.
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

## Downstream decision-value reference

The `decision_value` module adds an exact one-step Bayes-risk evaluator over
finite terminal decisions. Its loss table is indexed by
`(decision, mechanism, next physical state)`; the current sensor and apparatus
states affect the posterior, while their terminal consequences must be encoded
in the declared physical-state context or extended by a later loss-table schema.
Abstention has its own explicit loss. Every possible public outcome is replayed
through the exact action-realization filter, and the command cost is charged once
even on a hazard branch. The caller supplies an explicit hazard loss and receives
the exact hazard probability separately.

The counterexample uses prior probabilities `(0.80, 0.19, 0.01)`. The two
ordinary mechanisms share the same optimal terminal decision; a rare critical
mechanism requires a different decision and has a 1,000-unit false-release loss.
The high-information detail probe separates the two ordinary mechanisms but
does not reduce downstream Bayes risk: expected terminal loss remains 0.99.
The lower-information critical-regime screen isolates the rare mechanism and
reduces terminal Bayes risk to zero. Under the same 0.01 experiment cost, the
exact total losses are 1.00 for the detail probe and 0.01 for the critical screen.

For reference, the exact observation-channel entropies correspond to mutual
information of approximately 0.702 bits for the detail probe and 0.081 bits for
the critical screen. These entropy values are descriptive diagnostics; selection
uses the exact rational downstream loss, not a floating-point entropy score.
This is a constructed finite counterexample, not a measured benchmark victory.
