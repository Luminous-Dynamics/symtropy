# MARITIME-PCO-001: Evidence-Preserving Observation Contract v0.1

**Status:** initial Rust reference contract; compiler/test/Clippy status unverified.  
**Implementation:** crates/domains/symtropy-maritime-evidence  
**Tracking:** [Symtropy #1930](https://github.com/Luminous-Dynamics/symtropy/issues/1930)  
**Standard references:** [IAPH announcement and minimum dataset](https://www.iaphworldports.org/pickup/21900/), [IMO GreenVoyage2050 launch notice](https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/), and [official PCO Guide PDF](https://greenvoyage2050.imo.org/wp-content/uploads/2026/03/Guide-IAPH-IHMA-PCO-Guide-Final.pdf).

## Why this slice exists

A solver is only as safe as the evidence it consumes. Port-call systems can contain multiple reports for one event, delayed corrections, timezone differences, stale master data, and planned times that are mistaken for actual times. Picking the most recently ingested record is not a general conflict-resolution policy.

The IAPH–IHMA PCO Guide takes a basics-first approach: identify terminal and berth, and exchange planned arrival at the Pilot Boarding Place (PTA-PBP) and planned departure from berth (PTD). It bases semantics on IMO/IHO concepts and requires ISO 8601 date/time with timezone for timestamps. This crate maps the core concepts while retaining the source field code and original time text. It is not an official PCO SDK or a certified API adapter.

### Explicit PCO-to-contract mapping

| PCO Guide concept | Typed field | Typed value | Mapping constraint |
|---|---|---|---|
| Terminal identifier | TerminalIdentifier | Identifier | Must match an explicitly configured terminal registry entry. |
| Berth identifier | BerthIdentifier | Identifier | Must match an explicitly configured berth registry entry. |
| Planned time of arrival at Pilot Boarding Place (PTA-PBP) | ArrivalAtPilotBoardingPlace | Timestamp with Planned meaning | PCO Guide API example: dateTimeArrivalPlanned.plannedTimeOfArrival; preserve the source text; do not recast it as ETA or actual arrival. |
| Planned time of departure from berth (PTD) | DepartureFromBerth | Timestamp with Planned meaning | PCO Guide API example: dateTimeDeparturePlanned.plannedTimeOfDeparture; preserve the source text; do not recast it as forecast or actual departure. |

The Guide's sample API payload also carries pilotBoardingPlaceName alongside the planned arrival timestamp and berthName alongside the planned departure timestamp. A real adapter must map those location values to validated stable identifiers; names alone must not be treated as globally unique identifiers.

The enum separates event location (arrival or departure) from time meaning (planned, requested, estimated, forecast or actual). A production adapter must map a source field only after checking the source's published definition; the generic enum does not itself establish that a particular provider field means the same thing as the PCO field. The table deliberately does not invent numeric source-field codes.


## Contract

Envelope identity mapping follows the PCO Guide's sample operational response:
- shipReportingSystemEntryPortCode -> port_id, retained verbatim as the source port code;
- shipStayReferenceNumber -> port_call_id, retained verbatim as the source port-call reference;
- shipIMONumber -> ship_imo_number, retained verbatim as the vessel identifier.

This crate does not resolve aliases or canonicalize location identifiers. Any mapping into an internal canonical port/berth ID must be provided by an explicit, audited mapping layer.

Each observation carries stable observation, port, port-call, vessel and source identifiers; the semantic field and exact source field code; a typed identifier or typed timestamp with planned/requested/estimated/forecast/actual meaning; source-event time, ingestion time and optional validity end; schema version; original timestamp text; source UTC offset; normalized UTC nanoseconds; and the original pilot boarding place or berth name where applicable. A JSON adapter reads the Guide's sample planned-arrival and planned-departure response envelopes; it does not guess source-event or ingestion timestamps from the scheduled event timestamp.

The timestamp parser accepts a strict RFC 3339 subset: YYYY-MM-DDTHH:MM:SS[.1-9 digits](Z or signed HH:MM offset). It rejects timezone-less values, invalid calendar values, leap-second notation and precision beyond nanoseconds rather than guessing. A broader adapter must test any wider ISO 8601 syntax used by its actual provider. The original string is retained. Offset interpretation follows [RFC 9557](https://www.rfc-editor.org/info/rfc9557/): `Z` and `-00:00` indicate a known UTC instant whose local offset is unknown, while `+00:00` indicates UTC as the preferred reference point. The exact source lexeme remains preserved even when two notations have the same offset-knowledge semantics.

## Safe conflict semantics

Observations are grouped by port, port call, field and time meaning. Thus planned and actual arrival are separate groups; they do not conflict merely because the instants differ. Same-semantic observations from distinct offsets are consistent when their normalized UTC instant agrees. Different values for the same semantic key produce an explicit conflict with all source candidates preserved. The crate never applies last-write-wins or source ranking.

An observation with an unknown terminal/berth identifier, malformed field/value pairing, duplicate observation ID, stale source event, future-dated source/ingestion metadata, invalid validity window or expired validity is rejected with a reason. If otherwise-valid records reuse the same port-scoped call reference for different vessel identifiers, the entire call identity is quarantined; changing vessel identity cannot split one ambiguous call into individually uncontested fields. The same call reference at different ports is allowed. A future timestamp inside a planned/requested/estimated/forecast payload is allowed when the source metadata is fresh; future actual-event values beyond allowed clock skew are rejected. Master-data registries are explicit policy inputs; empty registries reject all terminal/berth references. Parsed timestamp fields are private and can only be created by the parser, so callers cannot construct a mismatched source string/UTC instant pair through a struct literal.

## Frozen synthetic tests

The Rust module includes 26 authored test cases covering timestamp normalization, the RFC 3339 `Z` / explicit `+00:00` / `-00:00` offset distinction, malformed formats, planned-versus-actual semantics, PCO arrival/departure JSON mapping, malformed/missing PCO fields, equal-instant different-offset reports, conflicts and permutation invariance, stale/future source metadata, unknown identifiers, field/value mismatch, duplicate observation IDs, validity windows, conflict-filtered projection, source/ingestion ordering, conflicting location context, and conflicting vessel identities for one port-scoped call reference. A negative control confirms that the same call reference at different ports is allowed. Tests consume frozen JSON fixtures:
- `fixtures/pco-arrival-valid.json` — supported planned-arrival payload with explicit timezone offset.
- `fixtures/pco-departure-valid.json` — supported planned-departure payload.
- `fixtures/pco-arrival-missing-location.json` — must reject an absent pilot boarding place.
- `fixtures/pco-arrival-malformed-time.json` — must reject a timestamp without a timezone.

All fixture values are synthetic; only the field/path shape is based on the published Guide. No testbed/live API was queried.

Exact-head CI has progressed beyond formatting and into strict Clippy. The previous run failed on `clippy::collapsible_if` in the future-actual guard; the guard now uses a single let-chain condition. A new exact-head run is queued. The Rust test and Clippy qualification remains pending; no pass is claimed until the new run reports it.

## Falsification and performance gates

Before an operational adapter is considered, qualification should report each of these dimensions separately:

- 100% exact UTC-nanosecond agreement for the frozen supported timestamp vectors, with original source lexemes and offset retained.
- 100% of planned/requested/estimated/forecast/actual vectors retain their meaning across normalization and resolution; no cross-meaning conflict.
- Zero malformed field/value pairs, stale source records, expired validity windows, unknown terminal/berth IDs, duplicate IDs or future source metadata silently accepted.
- 100% report equality across permutations of the same evidence set, including conflict candidate ordering.
- Zero conflicted semantic keys returned by the uncontested projection.
- Explicit counts and reasons for rejected records, consistent groups and conflicts; do not compress these into one opaque quality score.
- Measure 1,000 / 10,000 / 100,000-observation batches for p50/p95 runtime and peak memory on a named, reproducible host. Establish a measured baseline before choosing a latency SLO; never trade correctness for throughput.

## Claim ceiling and next integration steps

This first slice validates an in-memory contract, not the source's authority or truth. It does not authenticate source signatures, query a live PCO API, resolve entity aliases automatically, apply port-specific trust rankings, generate a cryptographic evidence digest, or change a berth/schedule. The report is deterministic over normalized inputs but does not yet issue a cryptographic replay receipt. Do not treat this as end-to-end provenance or official API conformance.

Next gates:
1. Implement a field-by-field PCO Guide mapping table with source examples, exact semantic definitions and known-loss notes.
2. Add fixture vectors from the official PCO API testbed or a port-authorized endpoint once permitted; clearly label synthetic values meanwhile.
3. Add a cryptographic digest and versioned canonical serialization only after selecting an established implementation and updating the lockfile—not a hand-rolled hash.
4. Run cargo fmt --check, crate-scoped cargo test, cargo clippy -- -D warnings, then verify exact-head CI evidence.
5. Only then build a read-only adapter for Symtropy/Sol Atlas. Mycelix retains authority over participant consent, offer activation, schedule changes and settlement.
