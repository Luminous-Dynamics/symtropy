# AV Active Sensing Multi-Round Executor v0

**Issue:** #6555  
**Single-round decision-value benchmark:** Symtropy #6546 / PR #6549  
**EVSI evaluator owner:** Symtropy #6222 / #6225  
**Observation replay and eligibility:** Symtropy #6201 / #6253  
**Foreground/background isolation:** Symtropy #6249  
**Modality necessity:** Symtropy #6305 / PR #6307  
**Long-horizon evidence memory:** Symtropy #6357 / PR #6359  
**AV consumer:** Symthaea #12837

## Objective

Provide a small event-sourced protocol executor for genuine multi-round active audiovisual acquisition:

1. candidate sees the current admissible evidence and public action catalogue;
2. candidate submits exactly one request, answer, abstention, stop or cancellation;
3. executor validates the turn and reserves resource budget;
4. action result remains evaluator-private until the configured arrival tick;
5. executor appends the typed outcome and exposes it to the candidate;
6. candidate receives a new turn bound to the new transcript digest and may re-plan.

This is benchmark infrastructure. It is not a production scheduler, transport queue, clock service, sensor driver, AV fusion engine, physical robot controller, EVSI calculator or memory system.

## Rust crate

The implementation is an isolated crate under this directory:

- Cargo.toml / Cargo.lock: pinned dependencies.
- src/lib.rs: typed episode, action, candidate turn, event transcript and executor.

Run from the repository root:

    cargo fmt --manifest-path research/av-active-sensing-multiround-v0/Cargo.toml --all -- --check
    cargo test --manifest-path research/av-active-sensing-multiround-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/av-active-sensing-multiround-v0/Cargo.toml --locked --all-targets -- -D warnings

## Protocol invariants implemented

- **No batch hindsight:** hidden action fixtures are stored in the executor but never returned by CandidateView.
- **Request is not observation:** an accepted request appends ActionAccepted; no new observation exists until the fixture arrival is advanced and an ActionOutcome event is appended.
- **Replanning:** the executor allows only one pending action in v0; the candidate must receive its outcome before requesting a second action.
- **Transcript binding:** each candidate turn carries the exact latest SHA-256 transcript head and monotonically increasing turn ID.
- **Append-only replay:** every candidate submission, acceptance, outcome and terminal decision is recorded in a sequence-ordered hash chain.
- **Deadline eligibility:** delivered observations arriving after the deadline remain visible with Late status and cannot be cited as on-time decision evidence.
- **Idempotency:** request IDs cannot be reused within an episode; action-use caps are enforced.
- **Budget reservation / settlement:** the catalogue cost is reserved at request acceptance; actual fixture consumption is settled at outcome/cancellation.
- **Failure semantics:** partial, unavailable, failed and cancelled outcomes cannot mint an observation reference.
- **Evidence references:** final answers and abstentions can cite only observations already visible and available by the decision deadline.
- **Common-cause metadata:** observations preserve source dependency groups for later dependence-aware scoring.

## Current v0 limits

- One pending action at a time.
- One deterministic fixture outcome per action ID and episode; v0 enforces `max_uses = 1` for every action.
- Logical ticks, not measured wall-clock time.
- Action costs are exact integer scenario units (audio bytes, frames, millijoules) until hardware telemetry backs them.
- Transcript hashes establish deterministic integrity/identity within the harness; they are not signatures, producer authentication or authorization.
- The candidate-visible event transcript is append-only, but there is not yet a persistent remote event broker or process sandbox.
- The executor returns one outcome when explicitly advanced; it does not own a real-time worker or physical sensor queue.
- Multi-action concurrency, dynamic cancellation races, retries, clock transformations, partial-stream continuation and long-horizon memory adapters remain later work.

## Tests

Unit tests cover:
- first observation hidden until delivery;
- second action selected in a later candidate turn after the first result;
- stale transcript digest rejected without mutation;
- late observation visible but not eligible for a timely answer;
- budget reserved before result and settled after delivery;
- answer rejected while an action remains pending;
- deterministic transcript digests for identical replay;
- duplicate request identity rejected;
- cancellation settles reserved budget without producing evidence.

A passing unit suite validates this finite executor contract only. It does not validate real model replanning, physical sensing, external EVSI calibration, production concurrency or SOTA performance.

## Relationship to PR #6549

PR #6549 is a single decision-round action-selection benchmark. This crate is the next event-sequenced primitive that enables a candidate to observe each actual result and choose a subsequent action in a fresh turn. It deliberately composes with #6201/#6253 and #6222/#6225 instead of re-implementing them.

## Future qualification

Next increments should add a scenario runner that supplies a sequence of CandidateView states to an actual policy adapter, plus adversarial fixtures for cancellation/arrival races, delayed duplicate events, partial results followed by follow-up acquisition, stale clock generations, transcript replay tampering and independent cost metering. Then compare fixed sampling, random budget-matched, greedy EIG, myopic EVSI, deadline-aware EVSI, AVP-style plan–observe–reflect, VAP-style keyframe selection and Symthaea native/hybrid policies under identical streams and budgets.

External model/benchmark results must be reproduced under their official protocols and must not be inferred from this synthetic executor.