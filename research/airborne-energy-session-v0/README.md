# Aerial energy session v0

This work implements the initial schema, frozen fixtures and independent Rust verifier for [AIRBORNE-ENERGY-SESSION-001 #4080](https://github.com/Luminous-Dynamics/symtropy/issues/4080).

## Ownership

This directory owns the aerial opportunity/session lifecycle only: discover, admit, acquire, track, energize, depart, confirm safe-off, reacquire and recover. It composes, but does not duplicate:

- #3980 site/PCC allocation and ground–air coexistence;
- #3885 aircraft mission energy;
- #3902 stationary aircraft docking;
- #3458 optical link physics/safety;
- #3472 source-to-load metrology;
- #3950 evidence identity and pass composition.

## Files

- `session.schema.json`: closed JSON Schema 2020-12 contract.
- `session.NOT_RUN.template.json`: explicit NOT_RUN template with no receiver, opportunity, event or energy evidence.
- `session.synthetic-reacquire.example.json`: synthetic positive trace with two distinct power-active intervals separated by departure, safe-off and fresh admission/reacquisition.
- `session.adversarial-revoked-authorization.example.json`: intentionally hostile trace that attempts POWER_ACTIVE with revoked authorization and local protection inhibited.
- `tools/airborne-energy-session-validator`: independent Rust semantic verifier and offline CLI.
- `tools/airborne-energy-session-validator/tests/cli_fixtures.rs`: invokes the CLI against the positive, NOT_RUN and hostile fixtures.

## Semantic invariants enforced by the Rust verifier

1. Event IDs are unique; sequence numbers are unique, contiguous and start at zero; timestamps are monotonic.
2. Each event's `from_state` equals the preceding state, starting at UNAVAILABLE.
3. Every transition is checked against the declared finite-state graph.
4. Every event binds the exact receiver ID and trajectory digest, opportunity ID and envelope digest, allocation ID and digest, and one monotonic clock ID. Cross-session or stale-revision receipts are rejected. Clock uncertainty is retained per event rather than silently discarded. Site allocation, opportunity envelope and local-protection permits each carry validity intervals, checked against the full clock-uncertainty interval.
5. A transition into ACQUISITION_PENDING from ADMISSION_PENDING requires a fresh matched receiver identity, valid authorization, valid allocation, admitted envelope and in-domain position. The allocation receipt must cover the event time.
6. A transition into TRACKED_NO_POWER requires fresh admission and tracking evidence.
7. A transition into POWER_ACTIVE requires a tracked no-power state, fresh admission, in-domain position, current allocation, valid envelope, a valid local-protection receipt digest and `PERMIT_SYNTHETIC_ONLY`.
8. Leaving POWER_ACTIVE requires local protection INHIBIT. SAFE_OFF additionally requires independent `CONFIRMED_SYNTHETIC` de-energization evidence.
9. Reacquisition returns through fresh admission and acquisition. An earlier power permit is not reused.
10. Terminal COMPLETE is only reachable from SAFE_OFF. An incomplete trace cannot be reported as complete.
11. The energy ledger is recomputed with checked integer arithmetic: source = source-to-channel loss + channel input; channel input = channel-to-receiver loss + receiver DC; receiver DC = receiver-to-battery loss + useful battery energy. Overflow and unexplained residuals are rejected.
12. A separately authored literal edge-list oracle is compared to the production transition predicate over every declared state pair.
13. Unknown fields, unsupported bearer/field combinations and invalid evidence dispositions fail closed.

The schema is the wire/data-shape contract; the Rust verifier independently checks the subset of shape and all semantic invariants required by v0. The current crate does not execute a general-purpose JSON Schema 2020-12 engine, so the CI result must not be described as a full JSON Schema conformance run.

## Running the verifier

From the repository root:

```sh
cargo test --locked -p airborne-energy-session-validator
cargo run --locked -p airborne-energy-session-validator -- research/airborne-energy-session-v0/session.synthetic-reacquire.example.json
```

The positive synthetic fixture emits `PASS_SYNTHETIC_SESSION`. The NOT_RUN template exits successfully but reports `final_state=NOT_RUN` and zero events; it is not a pass or physical result. The revoked-authorization fixture exits non-zero with a field-specific rejection. Exit code 2 indicates CLI usage or file-read error.

## Evidence and safety boundary

All current examples are synthetic. The `PERMIT_SYNTHETIC_ONLY` marker is not a hardware permit, interlock input or approval. Local hardware protection must remain independent. This verifier is not a flight controller, beam controller, safety case or energy-transfer controller.

No RF/optical power is authorized by this schema. No aircraft control, energized test, laser operation, aviation approval, spectrum authorization or safety certification is implied. Passing these software fixtures does not qualify physical coexistence, EMC, power-transfer efficiency, battery reserve, tracking performance or airborne operation.

## Explicit v0 boundaries

- One session trace binds exactly one receiver, one bearer-specific energy opportunity, one opportunity-envelope revision and one site-allocation receipt. Reacquisition means returning to that same opportunity and revalidating the same receipt. v0 does not support switching to a different site/bearer or replacing the allocation/envelope revision mid-trace. A changed opportunity or allocation starts a new episode; mission-level composition across those episodes belongs to #3885 and site orchestration #3980.
- The synthetic energy fixture uses integer Wh so that its arithmetic is exact and easy to inspect. This is **not** a metrology-resolution recommendation. Sub-watt or short-duration receiver experiments can require µWh/J resolution, calibrated meter transfer functions, timestamps and uncertainty. Physical energy record precision remains owned by #3472; do not adopt integer Wh as the universal measurement unit.
- The current schema uses one top-level opportunity and allocation object per episode. Any future multi-opportunity trace must introduce a versioned per-epoch receipt collection and independent oracle, not silently mutate the object model or reuse stale IDs.
