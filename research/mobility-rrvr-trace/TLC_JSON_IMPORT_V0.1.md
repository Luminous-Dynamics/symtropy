# TLC JSON counterexample import v0.1

**Implementation:** `src/tlc_json.rs` and `src/bin/tlc_trace_import.rs`  
**Formal owner:** [PR #6819](https://github.com/Luminous-Dynamics/symtropy/pull/6819)  
**Trace owner:** [PR #6903](https://github.com/Luminous-Dynamics/symtropy/pull/6903)  
**Tracking issue:** [#6906](https://github.com/Luminous-Dynamics/symtropy/issues/6906)

## Source format

The importer targets the pinned TLC `-dumpTrace json` format:

- top-level `vars` array;
- `counterexample.state`: ordered `[state_number, state_value]` pairs;
- `counterexample.action`: ordered `[before_state, action_metadata, after_state]` triples;
- action metadata contains a declared `name` and may contain source `location`.

TLC JSON is not a type-preserving encoding of every TLA+ value category. A JSON array can stand for a set, tuple, sequence or function on an integer interval; a JSON object can stand for a record, map or variant. The importer therefore preserves every state/action frame as opaque `serde_json::Value` and hashes its canonical JSON representation. It does not guess TLA types.

References:
- Official TLC CLI: [`-dumpTrace json` / `-loadTrace json`](https://github.com/tlaplus/tlaplus/blob/master/USE.md#model-checking-with-tlc).
- A documented real trace shape and type-erasure examples: [TLC JSON traces](https://www.philipzucker.com/tla_traces/).
- TLA+ Debugger action-trace export for minimization workflows: [TLA+ debugger documentation](https://docs.tlapl.us/using:debugger).

## Import validation

The importer rejects:
- missing `vars`, `s` variable, counterexample, states or actions;
- fewer than two states or more than the RRVR trace event limit;
- state/action count mismatch;
- duplicate, non-sequential or malformed state frames;
- malformed transition triples;
- before/after frames that do not exactly equal adjacent state snapshots;
- missing action names and any action not present in the pinned 30-action map;
- empty source artifacts or missing context identifiers;
- output that fails the common RRVR `TraceBundle` validator.

For an accepted trace, the importer creates:
- one event per TLC action, with the exact mapped `TraceEventKind`;
- an explicit linear predecessor edge from each action to the prior action, because one TLC behavior is a sequential state-transition trace;
- event input commitments over the complete before/action/after triple;
- initial-state and final witness-state commitments;
- a target signature supplied by the workflow/owner (property ID and violation class are not guessed from console text);
- four raw artifact records: the exact JSON, model source, model config and pinned tool JAR.

The binary writes a JSON RRVR bundle. The workflow uploads the original TLC JSON, imported bundle, exact model/config/tool files and a SHA256SUMS manifest together.

## Critical minimization boundary

A TLC counterexample is a sequential state-machine execution. Every later transition consumes the state produced by its predecessor. Therefore, deleting an intermediate TLC transition while leaving all later state snapshots untouched is not a valid replay.

The generic RRVR minimizer correctly preserves explicit causal ancestors, so a fully linear TLC trace will generally not shrink through simple event deletion. **Do not claim that importer plus generic delta debugging has reduced a TLC counterexample.** A meaningful shorter TLC witness requires a model-aware replay/resynthesis oracle that re-executes candidate action schedules from the same initial state, reconstructs every intermediate state, and rechecks the exact invariant/witness signature. TLC's `-loadTrace json` supports trace validation/model-based testing; its exact role in such schedule resynthesis needs a separate qualified experiment.

This importer establishes a portable bridge and exact replay context, not a model-aware minimizer or semantic equivalence.

## Qualification and non-claims

Synthetic Rust tests validate the importer against known-answer TLC-shaped JSON fixtures. The formal-model workflow then exercises the real pinned TLC tool, real unsafe-retry negative control, importer CLI and artifact upload.

A pass establishes only that the JSON conforms to the supported TLC format and that the mapped RRVR bundle preserves source/state/action commitments. It does not prove the TLA model matches physical hardware, the state/action vocabulary is exhaustive for other models, or the counterexample demonstrates a real robot fault. Every imported artifact remains pinned to exact model/config/tool bytes.
