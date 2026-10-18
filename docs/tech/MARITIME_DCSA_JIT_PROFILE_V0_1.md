# MARITIME-CROSS-STANDARD-001: DCSA JIT timestamp profile v0.1

**Status:** Draft reference parser and synthetic tests; compile, test and CI qualification pending.  
**Implementation:** `crates/domains/symtropy-maritime-evidence/src/dcsa_jit.rs`  
**Tracking:** [Symtropy #1955](https://github.com/Luminous-Dynamics/symtropy/issues/1955)  
**Related PCO boundary:** [#1930](https://github.com/Luminous-Dynamics/symtropy/issues/1930), [draft PR #1940](https://github.com/Luminous-Dynamics/symtropy/pull/1940)

## Standards are not interchangeable schemas

The IAPH-IHMA PCO Guide provides a small common baseline: terminal/berth identification, planned arrival at the Pilot Boarding Place, and planned departure from berth. DCSA JIT uses a timestamp message with separate event-classifier, operation, facility, phase and service codes. The DCSA JIT adoption guide for v1.2.0-beta1 documents the EST -> REQ -> PLN negotiation path and ACT as the actual-event classifier.

Official references:
- DCSA JIT repository and adoption pack: https://github.com/dcsaorg/DCSA-JIT
- DCSA timestamp classification: https://github.com/dcsaorg/DCSA-JIT/blob/master/doc/Module%209%20part%203%20-%20classifying%20timestamps.md
- DCSA requested berth arrival example: https://github.com/dcsaorg/DCSA-JIT/blob/master/doc/Module%209%20part%202%20-%20receiving%20an%20RTA-Berth.md
- IAPH PCO Guide resource page: https://www.iaphworldports.org/products/
- IMO launch note for the PCO Guide: https://greenvoyage2050.imo.org/port-call-optimization-guide-launched-at-imo/

## v0.1 DCSA subset

The parser currently supports only the following classifiers, operations and facility types:
- `eventClassifierCode`: `EST`, `REQ`, `PLN`, `ACT`;
- `operationsEventTypeCode`: `ARRI`, `DEPA`;
- `facilityTypeCode`: `BRTH`, `PBPL`;
- phase/service fields must be absent or null until a reviewed combination table is added.

Location constraints are deliberate:
- `BRTH` requires a location name, UN location code, facility code and facility code-list provider.
- `PBPL` requires a location name and UN location code, must not carry terminal facility codes, and supports arrival only in this version.
- If a field is duplicated in modern and legacy positions, the values must agree; conflicting vessel IMO or UN location codes fail closed.
- `eventDateTime` is parsed as a timezone-qualified timestamp. Source update time, ingest time and validity window come from caller-supplied `DcsaIngestContext`, never from the event timestamp.

The parsed type retains the DCSA classifier, operation, facility/location semantics, codes, location name, vessel identifier, optional publication/voyage fields, source profile label, context version and parsed timestamp. The shared RFC 3339 parser distinguishes `Z`, explicit `+00:00`, and `-00:00` (UTC timestamp with unknown local offset) while preserving source text. It does not map every DCSA message to PCO automatically.

## Explicit cross-profile candidates

The helper `pco_field_candidate` indicates only two semantic candidates:
- DCSA `PLN + ARRI + PBPL` -> PCO planned arrival at Pilot Boarding Place;
- DCSA `PLN + DEPA + BRTH` -> PCO planned departure from berth.

This helper does not construct a PCO observation or resolve canonical port/call identity. A reviewed adapter still needs explicit identity crosswalks and must preserve source-profile provenance. It deliberately rejects ETA-Berth / RTA-Berth / PTA-Berth as PCO Pilot Boarding Place arrival, and it rejects actual berth departure as PCO planned departure.

## Frozen synthetic fixtures and tests

The module has seven authored Rust tests and uses four frozen JSON fixtures:
- `dcsa-jit-eta-berth.json` — estimated berth arrival; negative control for accidental PCO conversion.
- `dcsa-jit-pta-pbp.json` — planned pilot boarding place arrival.
- `dcsa-jit-ptd-berth.json` — planned berth departure.
- `dcsa-jit-invalid-pbpl-terminal.json` — invalid terminal facility fields on PBPL.

Other tests check duplicate aliases, unsupported classifiers/phase, offset preservation, missing timezone, and EST/REQ/PLN changing time meaning without changing the event location. All payload values are synthetic. They are shaped from public documentation examples, not copied from an operational endpoint.

The DCSA JIT repository README reports a 4,500+ timestamp-combination test collection including positive and negative examples. It is a potential source for test-design review, not a corpus already imported here. Before reuse, pin a commit, inspect the repository and submodule licenses/provenance, and preserve original expected outcomes.

## Qualification targets (proposed, not measured)

- 100% classification accuracy across every frozen allowed/forbidden combination in the declared v0.1 subset.
- Zero unsupported DCSA combinations silently accepted.
- Zero cross-profile location/time-meaning conversions that lose semantics.
- 100% consistency between modern and legacy duplicate fields when both are present.
- 100% deterministic parse/validation outcomes under JSON key reordering.
- Report every unsupported code combination explicitly rather than inflating acceptance coverage.

## Claim ceiling

This is a strict-subset parser and semantic reference, not a DCSA certification, full JIT server, API client or negotiation agent. It performs no network I/O and does not create, post, accept, activate or settle a schedule or offer. Real testbed access, schema-version pinning, license review, Rust tests/Clippy and independent API conformance remain future gates.
