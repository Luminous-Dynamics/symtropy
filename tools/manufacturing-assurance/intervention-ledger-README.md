# Governed intervention transaction ledger (v0.1)

A dependency-free Rust **offline record verifier** for a proposed manufacturing intervention. It audits an immutable event record; it has no network client, PLC integration, machine driver, actuator binding, or command-emission path.

The central design decision is to treat an intervention as a transaction with separately checkable admission, authority, execution, measurement, recovery and post-recovery stages. A boolean "approved" field or model-confidence score is insufficient.

## Build and run

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/intervention_ledger.rs -o /tmp/mfg-intervention-tests
/tmp/mfg-intervention-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/intervention_ledger.rs -o /tmp/mfg-intervention
/tmp/mfg-intervention tools/manufacturing-assurance/fixtures/valid-intervention-ledger.tsv
```

Or run the repository reference suite:

```sh
bash tools/manufacturing-assurance/run-tests.sh
```

## Event sequence

- **PLAN** binds an asset, scope, action, target quantity/unit, permitted numeric bounds, three SHA-256-shaped digests (action envelope, precondition evidence and assurance case), validity interval and whether human approval is mandatory.
- **CHECK** records precondition, assurance and interlock checks, with status, timestamp and evidence digest. The PRECONDITION and ASSURANCE digests must match the plan. Every check must pass before command issue.
- **AUTH** records an identified HUMAN or SYSTEM principal, the exact asset/scope/action, grant/reject decision, approval time, expiry and receipt digest. A plan requiring human approval must have a HUMAN grant. The latest matching authority event per principal supersedes earlier events, so a later REVOKED or REJECTED record cancels an earlier GRANTED record.
- **AUTH** is accepted only after every required admission check and before command issue.
- **EXEC** records one unique command ID, executor, exact approved action digest, command issue/acknowledgment times and result.
- **REQUIRE** declares every required outcome metric, unit and acceptance interval *before* execution. A successful transaction cannot omit an inconvenient metric.
- **VERIFY** records a measurement and acceptance interval. The verifier computes pass/fail from the actual value. POST_ACTION and POST_RECOVERY are separate stages.
- **RECOVERY** records ROLLBACK, SAFE_STATE or CONTAIN with issue/acknowledgment, result and evidence digest.

All timestamps use canonical UTC `YYYY-MM-DDTHH:MM:SSZ`. The plan and authority intervals are half-open `[from,until)`. Every required admission check must occur before approval, and approval must occur before command issue.

## Result states

- `BLOCKED`: required checks, authority, digest matching, chronology or other admission conditions do not pass.
- `ABORTED`: the command executor explicitly rejected the command, with no follow-on events.
- `ISSUED_UNVERIFIED`: command acknowledgment exists, but no post-action verification has been recorded.
- `VERIFIED`: all post-action measurements are within their declared acceptance ranges.
- `RECOVERY_REQUIRED`: a post-action measurement is outside its acceptance interval or recovery verification is incomplete.
- `RECOVERED`: a failed post-action metric is followed by acknowledged recovery and post-recovery verification of every failed metric, all within range.
- `RECOVERY_FAILED`: recovery itself failed or post-recovery measurements remain outside range.

The evaluator requires post-recovery verification to cover the full set of failed post-action metrics. It does not let a passing subset close the transaction.

## Frozen controls

The Rust reference now has a warning-denied unit-test suite covering the frozen transaction and seeded fault cases. Use the exact commit to count the tests; The current branch adds controls for:
- missing/failed preconditions, assurance checks and interlocks;
- plan-bound precondition/assurance digest mismatch;
- absent, wrong-action, wrong-asset/scope, rejected, late or system-only authority;
- command digest mismatch and out-of-bounds plan values;
- missing command acknowledgment, missing declared metric or missing post-action verification;
- post-action failure with missing, failed or chronologically invalid recovery;
- approval before completion of all admission checks; post-recovery measurement before acknowledgment, incomplete failed-metric coverage and out-of-range result;
- duplicate execution records, malformed measurement digests, invalid intervals and non-finite measurements;
- deterministic replay and explicit no-actuation claim ceiling.

Test counts may change as this reference is strengthened. Check the exact commit's source and CI summary rather than relying on this descriptive count.

## Safety boundary

This tool never:
- sends messages to equipment;
- writes to a PLC, CNC, robot, MES or SCADA system;
- proposes or selects a process parameter;
- chooses a recovery action;
- grants authority or authenticates an identity;
- establishes a safe operating envelope.

It only checks the event record and declared policy. A `VERIFIED` result means the supplied record satisfies this local synthetic policy. It does not prove that an intervention was safe, lawful, causally beneficial or physically effective.

The SHA-256 fields are syntax-checked and compared to declared bindings only. This version does not read the referenced bytes, verify signatures, authenticate clocks or validate measurement-system traceability. A production extension would need independent signature verification, content-addressed artifact bytes, trusted time, revocation handling, calibrated sensor provenance and review by qualified safety authority.

## Relationship to the assurance-case graph

Issue #3178 / PR #3178 adds a machine-checkable assurance-case graph with explicit claims, evidence, assumptions, defeaters and action-specific authority. This ledger is the transaction-history companion:
- the PLAN's assurance digest should identify the exact assurance-case evaluation manifest/output;
- AUTH should bind the authorized principal to the same asset, scope and action;
- EXEC should bind the command digest to the approved action envelope;
- VERIFY/RECOVERY evidence should become evidence nodes in the graph.

A future adapter should bind all references to immutable run/artifact digests, not mutable PR links or filenames.
