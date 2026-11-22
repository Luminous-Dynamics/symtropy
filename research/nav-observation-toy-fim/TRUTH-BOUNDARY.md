# Controller/evaluator truth boundary — prototype v0.1

**Issue:** [NAV-ORACLE-BOUNDARY-001 #6666](https://github.com/Luminous-Dynamics/symtropy/issues/6666)  
**Parent programme:** [NAV-OBSERVATION-001 #6438](https://github.com/Luminous-Dynamics/symtropy/issues/6438)  
**Status:** deterministic in-process Rust boundary model; no full-process isolation claim.

## Architecture change

The controller proposal API accepts only `ControllerVisibleInput`:
- policy revision and deterministic seed;
- prior belief and declared decision costs;
- action candidates and their pre-acquisition expected values;
- hard energy, latency and bandwidth budgets;
- an input commitment.

It has no argument or field for evaluator truth, truth-manifest digest, adjudication, future event, or realized outcome. The immutable `ActionProposal` includes a versioned canonical byte encoding and an explicit acquire/abstain disposition.

Acquisition is a later event. It must carry the exact canonical proposal bytes and action identity. A mismatched proposal/event pair fails closed.

Only the separate `score_evaluation` function accepts `EvaluatorTruth`. Swapping that truth can change evaluator score, but cannot change the already-created proposal bytes.

## Regression results to expect

The deterministic tests cover:
- byte-identical controller proposal under evaluator-only truth swaps;
- evaluator score changes under the truth swap;
- canonical action trace invariant to candidate ordering;
- public input change can change the trace;
- hard resource constraints;
- deterministic tie-break;
- explicit abstention for no-positive-value and no-feasible-action;
- proposal/event mismatch rejection;
- no acquisition after abstention;
- duplicate candidate and invalid loss rejection.

## Security boundary

The current prototype is a pure in-process API/type-separation model. It demonstrates the absence of an evaluator-truth parameter on the proposal function and tests the resulting trace behavior. It is **not**:
- an OS process sandbox;
- a capability security system for arbitrary controller code;
- a formal information-flow or noninterference proof;
- proof of institutional/oracle independence;
- evidence that the full benchmark runner cannot leak truth through logs, environment variables, shared files, clocks, cache or mutable state.

The next enforcement step is a separate controller process receiving a manifest containing only public inputs, with evaluator truth held by an independent process/account. Use read-only inputs, no shared truth-bearing volume/environment, deterministic execution receipt, signed action proposal, post-hoc evaluator receipt and a truth-swap negative control.