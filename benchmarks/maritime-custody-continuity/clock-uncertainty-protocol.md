# Clock and Event-Time Uncertainty Protocol — Maritime Custody Continuity P1

## Purpose

Define how a source adapter may turn a source event timestamp into a bounded event-time enclosure before the temporal partial-order primitive is allowed to compare it with another source's event.

The custody state machine must not assume that a formatted timestamp is an exact physical clock reading. The interval-order module deliberately accepts already-normalized enclosures and does not manufacture clock bounds.

## Non-interchangeable time fields

Every adapter must preserve distinct raw values and meanings for:

1. **Physical/business event time** — the time the source asserts the event or business step occurred, including the profile's meaning (e.g. start, completion, estimated occurrence).
2. **Sensor observation time** — when a sensor sampled a physical quantity, if present.
3. **Source event-entry/creation time** — when the source application created the event message or record.
4. **Repository record time** — when an event repository recorded the record.
5. **Publication time** — when the source made the event available to consumers, if known.
6. **Ingestion/receipt time** — when the receiving adapter obtained the record.
7. **Retrieval time** — when a snapshot/file/API response was fetched.
8. **Decision/as-of time** — the cutoff at which a downstream reconstruction or decision is replayed.

These fields are not aliases. In particular:
- DCSA Information Model distinguishes Event DateTime (when an event occurred or will occur) from Event Created DateTime (when the event entry was created) and separately models event classification.
- EPCIS distinguishes eventTime from recordTime; recordTime is repository bookkeeping, not a statement about the real-world event.
- A recent retrieval does not make an old schedule or forecast current.
- A time-zone offset resolves a local representation to an instant; it does not establish clock accuracy.
- A timestamp precision/lexical resolution is not itself an error bound.

## Exact source profile and raw preservation

For every normalized time assertion retain:
- exact raw source lexeme/bytes and source document/event commitment;
- source system, source event ID, profile name and exact profile/schema version;
- raw field name and its declared semantics;
- original timezone/offset lexeme and any named time-zone context;
- timestamp precision and documented rounding/truncation semantics;
- time scale, epoch, unit and leap-second/conversion policy;
- source device/host/clock identity where available;
- source event-entry/publication/observation/ingestion times as separate fields;
- mapping version, adapter build identity and all normalization findings.

Never rewrite the raw lexeme after normalization. Store normalized UTC/TAI/other instants as derived fields with a reversible, versioned mapping record.

RFC 9557 updates RFC 3339 offset semantics: a timestamp ending in Z identifies a UTC instant while leaving the local offset unknown; +00:00 indicates UTC as the preferred reference point. Preserve the original lexeme and the distinction. Do not use timezone normalization to infer local-zone identity or clock synchronization.

## Constructing a conservative event-time interval

A source event timestamp may be represented as a point plus a qualified bound only when the source and clock profile support that interpretation.

Candidate uncertainty contributors, each with its own evidence and validity interval:
- clock synchronization offset/error bound at the event time;
- oscillator holdover/drift since the last valid synchronization observation;
- timestamp capture-path latency bounds;
- timestamp quantization/rounding error, only when the source specifies the rounding or truncation rule;
- adapter parsing/normalization/conversion error;
- instrument-specific measurement latency, only where the event timestamp is derived from a sensor.

For a declared instant estimate t and a defensible total worst-case error bound u, an interval may be constructed as [t-u, t+u]. The interval must be outward-rounded in the target time unit. If uncertainty components have no validated dependence model, use a conservative bound (e.g. sum of absolute component bounds), not an unjustified root-sum-square. If one required component is unknown, do not invent a finite bound.

**Do not assume a point timestamp's printed precision is a symmetric ±half-unit measurement error.** The source could round, truncate, cache, batch, or generate the value through an application process. Obtain the field's actual timestamp-generation contract.

**Do not add source reporting delay to physical event-time error.** Event time and publication/ingestion time are separate dimensions. A report can arrive late with a well-bounded event time, or arrive immediately with an unreliable event time.

## Clock synchronization is a claim requiring evidence

A clock profile may reference IEEE 1588 Precision Time Protocol, NTP, GNSS, a hardware clock or another synchronization method. It must still retain the actual device/clock identity, grandmaster/reference identity, profile, last synchronization evidence, maximum offset bound, path/asymmetry assumptions, holdover/drift bound, alarm/lock state and validity window.

IEEE 1588-2019 describes the PTP protocol and says sub-microsecond synchronization is achievable in appropriately designed systems. This is not a universal per-device guarantee and does not make a given event timestamp proof-grade by itself. A PTP label without current clock-quality evidence is insufficient.

Where clock synchronization is unavailable, stale, alarmed, untraceable or unbounded, retain an estimate or Unknown status. An adapter must not promote it to QualifiedEnclosure.

## Partial-order rules

The temporal-order module compares intervals only when their exact time-basis identifiers match.

For A=[a_earliest,a_latest] and B=[b_earliest,b_latest]:

- A is strictly before B only if a_latest < b_earliest.
- A is strictly after B only if b_latest < a_earliest.
- Overlap or touching is UnresolvedOverlap.
- Different or unconverted time bases are IncomparableTimeBasis.
- If either bound is estimate-only, disjointness is an estimated order, not a proof-grade order.
- Overlap does not establish simultaneity. Simultaneity needs an independent joint-event certificate for the exact events and the applicable joint physical/business semantics.

No epsilon is added to create an order. No source-preference, insertion-order, lexical-ID or ingestion-time tie-break may be presented as physical chronology.

## Required adapter dispositions

Use distinct dispositions for:
- malformed/unsupported source timestamp;
- timezone/offset ambiguous or invalid under the selected source profile;
- unknown or unsupported time scale;
- source profile mapping unsupported;
- missing event-time semantics;
- source clock quality unavailable/stale/alarmed;
- uncertainty bound unavailable;
- qualified interval constructed;
- estimate-only interval constructed;
- cross-profile conversion unavailable;
- pairwise order proven;
- pairwise order estimated;
- overlapping interval unresolved;
- time-basis mismatch/incomparable.

The disposition must not silently collapse into a boolean timestamp-valid flag.


## Implemented P1 clock-profile constructor

The executable Rust primitive now includes a typed ClockBoundProfile. A caller cannot produce a proof-grade TimeEvidence by setting QualifiedEnclosure alone; direct construction is rejected. The profile must bind:

- source clock/device ID and profile ID;
- exact normalized time-basis ID;
- opaque calibration-evidence reference;
- synchronization method and current state (Locked or Holdover; Alarmed/Unknown refuse);
- calibration/validity window;
- maximum offset, holdover-drift, timestamp capture-path and rounding-error bounds.

All four error components are mandatory. Unknown is represented as None and fails qualification. The current reference implementation adds the stated worst-case tick bounds conservatively with checked arithmetic and derives [source_timestamp - total_error, source_timestamp + total_error]. It rejects event timestamps outside the calibration validity window, arithmetic overflow, missing components and unqualified clock state. Each derived TimeEvidence retains the clock/profile/calibration identifiers and total error bound.

This constructor is an explicit evidence contract, not an authentication protocol. The calibration reference is opaque and this crate does not dereference it, check its signature, validate the clock's issuer, independently audit the PTP/NTP measurements, or prove the timestamp-generation path. A production adapter must verify those inputs and must not put Some(0) for a component merely because no data exists; zero is permitted only where a reviewed source contract establishes zero or non-applicability.

## P1 falsification corpus

Freeze fixtures for:
1. disjoint qualified intervals in each direction;
2. overlapping intervals with distinct and identical midpoints;
3. touching interval endpoints;
4. identical singleton intervals without a joint-event certificate;
5. estimate-only disjoint intervals;
6. mixed qualified/estimate-only disjoint intervals;
7. differing time-basis IDs without a conversion receipt;
8. stale synchronization evidence and holdover drift beyond its policy window;
9. unknown timestamp rounding semantics;
10. source event time vs source event creation time inversion;
11. EPCIS eventTime vs recordTime substitution attempt;
12. DCSA planned/estimated/actual classification substitution attempt;
13. RFC 9557 Z vs +00:00 semantics and preserved source lexemes;
14. future-published event excluded at historical decision cutoff;
15. adapter normalization revision changes without raw evidence change;
16. positive common time-unit scaling and input permutation;
17. interval widening never upgrades an unresolved/estimated pair into a proof-grade relation.

## P1/P2 benchmark measures

Report:
- proof-grade ordering precision and recall on independently timed ground-truth event pairs;
- false-ProvenBefore and false-ProvenAfter rates (hard gate: zero on the frozen adversarial suite);
- estimated-order rate;
- unresolved-overlap rate;
- incomparable-time-basis rate;
- fraction of actual event pairs with qualified clock evidence (order coverage);
- interval width by source, device, event type and location;
- clock bound calibration/coverage against independent reference instrumentation;
- source reporting latency separately from physical event-time uncertainty;
- adapter loss, malformed and unknown-field rates;
- deterministic replay, input-permutation invariance, runtime and memory.

Publish the coverage-risk curve: proof-grade order error versus fraction of event pairs for which the verifier is willing to assert an order. A system that orders every pair but has false orders is not better than a system that abstains selectively.

Do not claim universal clock accuracy from PTP/NTP use. Measure per-source/device/profile against independent reference instrumentation. Preregister the event types, operating conditions, clock profiles and validation window before reporting coverage.

## Integration boundary

- #1930 owns the operational observation envelope.
- #1955 owns DCSA JIT and PCO cross-standard time semantics and reviewed conversion tables.
- #1650 owns decision-time applicability/freshness for forecast inputs.
- #4003 owns DGT source-specific field semantics, source admission and rights.
- #3410 and #4113 own the broader generic/physics interval-order research contracts.
- This custody package owns only the conservative pairwise interval relation used to protect physical handoff reconstruction.

P0 custody reconstruction still uses exact synthetic ticks. The included partial-order primitive is a separate P1 building block; it does not yet validate or replace the P0 state-machine ordering. Before real data can use it, source adapters must construct time enclosures with qualified clock evidence and preserve the source time profiles above.
