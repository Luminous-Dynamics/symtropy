# Shipyard non-anticipative policy replay auditor (V0)

This dependency-free Rust crate audits **whether a recovery policy uses only information observable at each decision time** across a declared finite replay suite. It complements, but does not replace, the static geometry oracle, finite-scenario schedule auditor, or full JSON Schema / semantic benchmark contract.

## Paradigm shift: benchmark a policy, not just a collection of final schedules

A final schedule can look robust in hindsight while a dispatch policy that chose it could not have known which disruption would occur. V0 therefore treats each scenario as an event-observation trace plus a sequence of decisions. At a declared decision tick, only events from the **separate source-owned event log** with `observed_at_tick <= decision_tick` enter the information history. Candidate traces supply actions only; they cannot remove or retime source observations. The expected scenario inventory is also separate and must match both the source logs and candidate traces exactly. Scenarios with the same decision tick and identical observable histories must carry the same canonical current-action key.

This checks a necessary **non-anticipativity** property of adaptive policies. The source-defined expected scenario ID list is separate from the candidate traces; exact ID-set equality is required, so omitted or unexpected traces are incomplete. Load that list from the immutable benchmark manifest and bind it to the source revision. This crate checks coverage against the list but does not authenticate the manifest itself. It is not an optimizer and does not determine whether an action is physically or operationally feasible.

## Result semantics

- `Consistent`: every declared scenario and decision epoch is structurally present, and all compared scenarios choose the same action key when their observable histories are identical.
- `AnticipativityViolation`: the trace suite is structurally complete, but at least one shared information set selects different action keys.
- `Incomplete`: scenario/epoch coverage, ordering, identities, or action keys are malformed or missing. Incomplete is not a pass.

The audit fails closed for empty scenario or decision-epoch sets, duplicate scenario/event identities, non-increasing decision ticks, duplicate/missing decisions, unexpected decision ticks, and blank action keys.

## Input assumptions

- Event IDs identify immutable canonical event content. The same shared observation across scenarios must reuse the same canonical ID (or canonical observation key); otherwise equal histories may appear different. If an ID can refer to different contents, the caller must canonicalize the source events before using this checker.
- `action_key` must canonically identify the **action committed at this decision epoch** (ideally a digest of that action's canonical encoding), not an entire future schedule whose hidden branches differ. If the decision is itself a contingent policy node, compare the node's current committed action here and check later nodes at their own epochs. This crate checks equality of keys; it does **not** compute or authenticate their digest.
- Every scenario is replayed at the same declared decision epochs. If a system is event-triggered, represent no-op decisions explicitly rather than omitting them.
- The decision tick is part of the information-set key. The observable history includes both the event identity and its observation tick.

## Explicit non-claims

This crate does not check geometry, swept movement paths, resource feasibility, precedence, mandatory work, schedule costs, outcome fidelity, event truth, probability calibration, or physical safety. Use the independent polygon / robust-scenario verifier for the declared static constraints, and preserve its receipt separately. A policy can be non-anticipative and still be unsafe, infeasible, poor-quality, or based on fabricated inputs.

## Test strategy

Ten unit-test functions cover a valid adaptive two-branch policy, clairvoyant divergence before observation, event visibility at the exact decision tick, differing observation times, an exhaustive 4×4×4 (64-case) reveal/divergence matrix, missing actions, duplicate event IDs, source/candidate inventory omissions and extras, empty inputs, and deterministic report order. These fixtures are conformance tests, not industrial measurements or proof that the policy is optimal.

The exact-head workflow runs Rust 1.96 formatting, locked/offline tests, and Clippy with warnings denied. No validation status should be inferred from a queued workflow.
