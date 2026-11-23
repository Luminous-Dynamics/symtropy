# Maritime cargo event-capture completeness v0.1

Standalone synthetic-first Rust benchmark for the acquisition path between physical operations and admitted event records.

## Core distinction

`Occurrence → Detection → Durable capture → Source-semantic admission`

These are four separate propositions. A source may return a valid, signed, replayable export while a physical operation was never detected or the event was lost before persistence. A delivery acknowledgement does not establish durable persistence. A complete-looking event stream is not a qualified observation channel.

This crate owns none of the operational source adapters, canonical container identity, DCSA/EPCIS parsers, metrology, custody state machine or causal estimator. It is a read-only P0 contract that composes later with #4003, #6562, #6662, #6631, #6783 and #6945.

## P0 behavior

- Require a versioned source capture profile bound to source, event class, validity window, configuration digest and documented sequence semantics.
- Require a separately qualified independent reference with complete coverage of the exact same event class and time window.
- Reject a reference whose declared failure domains overlap the source capture path; different device IDs do not prove independence.
- Match independently referenced events to source records by exact event ID and event time.
- Count a source record only when durably persisted and visible at the declared as-of cutoff.
- Treat exact duplicate delivery as idempotent; conflicting reuse of an event ID as incomplete.
- Permit a known-no-event interval only when the independent reference explicitly qualifies complete coverage and reports no events.

Dispositions:
- `QUALIFIED_WITHIN_DECLARED_ENVELOPE` — complete only within the exact tested source/configuration/event-class/window.
- `INCOMPLETE` — an expected event is missing/non-durable, an unexpected record appears, or an event identity conflicts.
- `NOT_IDENTIFIABLE` — no independent reference, incomplete/unqualified reference coverage, or shared failure domains.
- `NOT_EVALUABLE` — source profile mismatch or unsupported sequence semantics.
- `UNKNOWN` — required profile is absent.

## Frozen corpus

`fixtures/capture-v0.1.tsv` contains 14 hand-authored expected dispositions. Unit tests add metamorphic controls for event removal, shared failure domains, acknowledgements without persistence, historical cutoffs, exact duplicates and conflicting event identities.

All cases are synthetic. The test helper creates the candidate input; the TSV independently declares the expected disposition. This is a reference contract, not a physical test campaign.

## Run

    cargo fmt --manifest-path benchmarks/maritime-cargo-capture-completeness/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/maritime-cargo-capture-completeness/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/maritime-cargo-capture-completeness/Cargo.toml --locked --all-targets -- -D warnings

## Claim ceiling

A pass proves only this synthetic evaluator's declared rules. It does not qualify any real terminal, sensor, PLC, OCR, gate, rail, broker, API, archive or receiver; it does not establish physical occurrence, detection probability, legal access, source truth, identity, custody, delivery, port performance or network effects. Actual source-specific qualification requires independent reference instrumentation, rights-cleared records, metrology/clock traceability, configuration control, and an independently adjudicated physical test protocol.
