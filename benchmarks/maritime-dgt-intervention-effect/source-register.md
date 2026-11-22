# Candidate DGT source register — inspected 21 November 2026

This register distinguishes source existence and visible fields from source acquisition, rights clearance, field meaning and physical-event qualification. The following PDFs are hosted by the DGT / ICTSI CDN. They were visually inspected in the research session as PDFs, but their underlying bytes were not saved in this crate, independently hashed or authorized for downstream reuse.

| Artifact | Source-declared snapshot time | Visible source-native fields/states | What it may support | What it does not prove |
|---|---|---|---|---|
| [DGT vessel update, 05:00](https://cdnweb-dgt.ictsi.com/users/user36/DGT%20VESSELS%20UPDATE%20%4005H00_3.pdf) | 21 Nov 2026, 05:00 | Vessel name; voyage/visit-like identifiers; berth; ETA; date/time working started; raw status (including COMPLETED, WORKING and NIT); balance to discharge/load; completion fields | Time-bounded terminal status snapshot, after field dictionary, source identity and call-identity validation | Complete movement history; meaning of NIT; exact physical lines-fast; cargo loading/discharge reconciliation; vessel departure or inland delivery |
| [DGT firm 21-day berth plan](https://cdnweb-dgt.ictsi.com/users/user13/DGT%20Firm%2021%20day%20berth%20plan%2020.11.2026.pdf) | 20 Nov 2026, 13:19:36 | Berth/bollard timeline; vessel labels and visit-like codes; scheduled dates/times; some historical actual-time fields; fender/repair annotations | Versioned berth-plan snapshot and declared resource constraints, after schedule/actual semantics and revision policy are admitted | Physical all-fast/lines-fast, cargo start/end, confirmed departure, or all resource compatibility details |
| [DGT export stack dates](https://cdnweb-dgt.ictsi.com/users/user13/DGT%20EXPORTS%20STACK%20DATES%202026-11-20.pdf) | Confirmed as of 20 Nov 2026 | Vessel/voyage and DCT codes; open/close stack windows; FIRM / PROVISIONAL / CANCELLED; service/line; comments | Export-stack window status snapshot, subject to explicit revision tracking | Gate-in, cargo availability, vessel loading, sailed quantity or final export delivery |

## Source-native semantics and unresolved questions

### Vessel update

Preserve raw status lexemes, balance to discharge, balance to load, date started working, time started working, and completion fields as separate source fields. The visible sheet uses COMPLETED, WORKING and NIT; NIT is not interpreted pending a terminal-provided data dictionary. The document's note directs readers to import-storage updates for completion dates / last-free-days, so the vessel status sheet is not a full cargo dwell or release history.

The vessel name, voyage, DCT code and visit/rotation identifiers must not be assumed to be one interchangeable primary key. Seek a terminal-issued stable call/visit identifier and its revision semantics.

### Berth plan

Retain the source timestamp and revision. The diagram includes historic actual-time values and future berth windows, as well as fender recovery/repair annotations around bollards 72/73 and a separate 4×2 fender repair annotation. Do not infer the exact operational meaning of these annotations without the terminal's engineering / berth-planning dictionary. A time value labelled actual in a planning document still requires confirmation of which event it denotes and who entered/verified it.

### Export stack dates

Preserve FIRM, PROVISIONAL, and CANCELLED verbatim. The PDF warns stack dates depend on terminal space and may change. A firm stack window is not proof that the cargo was gated in, that the box was loaded, or that the vessel sailed.

## Admission record required before use

For each retrieved artifact, record:
- exact bytes, SHA-256, byte count, media type, canonical URL, retrieval time and permitted-use/legal basis;
- source-declared snapshot/publication time separately from retrieval time and event/valid time;
- source publisher/issuer, identity of terminal account/source, schema/profile version and revision/version ID where available;
- complete raw fields and raw status lexemes before normalization;
- field dictionary, units, timezone, locale/date parsing, missing-value, null, correction and cancellation semantics;
- source-local vessel-call/visit identity and crosswalk provenance between the three files;
- revision history: never overwrite a prior snapshot with a new plan;
- independent event evidence and adjudication for claims that a source field corresponds to a physical event.

## Composition boundaries

- #4003 owns DGT source acquisition, rights, definitions, identifiers and snapshot semantics.
- #6562 / #6565 own AIS-inferred geofence observation only; those are not physical port-call events.
- #1930 / #1955 own port-call event semantics and DCSA profile handling.
- #6493 / #6496 own conserved vessel/cargo stock; they should ingest only validated event stages.
- #6525 / #6526 and #6557 / #6560 own waiting cohort, censoring and identifiability.
- #6578 owns DGT intervention lifecycle and cross-stage / cross-mode impact accounting.
- #3683 owns complete coastal-to-inland service-chain evaluation.

No data in these PDFs is presently embedded as operational rows in this crate. The only pinned data fixture is data/dgt-package-announcements.csv, whose ceiling is ANNOUNCED.

## Source inventory refresh — 22 November 2026

The official [DGT Terminal Updates](https://dgt.ictsi.com/terminalupdates) page was re-inspected on 22 November 2026. It lists daily vessel-update links, import-storage files, export-stack files and firm berth plans through 21 November. The source landing page and PDF snapshots were visually inspected, but **the source bytes have not been acquired into this repository, hashed or rights-cleared**.

New directly inspected artifacts:

| Artifact | Embedded/source-declared time | Visible distinctions | Admission ceiling |
|---|---|---|---|
| [DGT firm 21-day berth plan, 21 Nov](https://cdnweb-dgt.ictsi.com/users/user13/DGT%20Firm%2021%20day%20berth%20plan%2021.11.2026.pdf) | 21 Nov 2026, 13:02:01 | Berth/bollard timeline, vessel visit/call labels, berth windows, fender-repair annotation | Versioned planning assertion; repair note is not independent proof of resource outage/capacity |
| [DGT export stack dates, 21 Nov](https://cdnweb-dgt.ictsi.com/users/user13/DGT%20EXPORTS%20STACK%20DATES%202026-11-21.pdf) | Confirmed as of 21 Nov 2026 | FIRM / PROVISIONAL distinctions; stack windows; explicit notice that dates depend on available space and can change | Stack-window status only; not gate-in, loading or export departure |
| [DGT import storage, 21 Nov](https://cdnweb-dgt.ictsi.com/users/user13/DGT%20IMPORT%20STORAGE%202026-11-21.pdf) | 21 Nov 2026 | GP, reefer and IMDG fields, full/empty, unassigned and appointment/storage-related columns | Yard/storage snapshot only; unit, denominator and field definitions still require the terminal dictionary |
| [DGT Vessel Update @05H00 page entry](https://cdnweb-dgt.ictsi.com/users/user23/DGT%20VESSELS%20UPDATE%20SPREADSHEET_27.pdf) | Embedded report heading says 22/11/2026 23H00 | Vessel/visit table; legend NYI = Not Yet In, INW = In Not Working, OWI = On Way In | Time basis is unqualified; see temporal-admission anomaly below |

### Temporal-admission anomaly: @05H00 versus embedded 23H00

The DGT landing-page link label says @05H00, while the inspected PDF heading says 22/11/2026 23H00. At 07:04 Africa/Johannesburg on 22 November this would appear to be a future report time **if** interpreted as South African local time. The PDF does not state the timestamp's time zone or report-generation convention, so this is not yet proof of a future timestamp: it is an unresolved source-label / embedded-time / time-basis conflict.

Do not treat this document as the current @05H00 observation, do not compare it directly against a 05H00 snapshot from another date, and do not use the embedded time for event ordering until DGT clarifies the time zone and generation semantics. Preserve the raw page label, PDF heading, retrieval time, PDF byte digest (once rights permit acquisition), and time-zone unknown as separate values. Preserve NYI / INW / OWI exactly; do not conflate them with the earlier NIT string.

### New P1 tests

- Landing-page link label versus embedded report-time conflict.
- Embedded report timestamp apparently later than retrieval/decision time, but with missing timezone.
- Snapshot/publication/retrieval/decision time kept distinct.
- Berth repair annotation versus independently verified asset availability.
- FIRM / PROVISIONAL stack dates versus realized gate-in/loading.
- Yard GP / reefer / IMDG fields with unresolved units/denominators.
- Daily source snapshots remain immutable and historical replay uses only records visible at the selected cutoff.

This discovery does not change the source-admission ceiling: none of these artifacts is yet an admitted operational record.

## Industry conformance comparator — standards version and scope

DCSA's public conformance register is a useful API/protocol baseline, but certification is specific to a standard release, role/module and the scenarios actually exercised. A conformance result must not be promoted into proof that a source event is authentic, physically true, legally authoritative or that a shipment was delivered. Reproduce the applicable DCSA scenarios for the exact public profile used; then evaluate custody truth separately on independently adjudicated traces.

- DCSA Standards Public Reference: https://dcsa.org/standards
- DCSA Conformance Register: https://dcsa.org/standards/conformance

For this benchmark compare source-native TOS / interchange history, DCSA public event exchange, GS1 EPCIS 2.0.1 / CBV 2.0.0, relational reconciliation, and the independent Symtropy reference. Keep standards/API conformance, timestamp semantics, issuer authentication, physical handoff proof, and operational service outcomes as separate scorecards.
