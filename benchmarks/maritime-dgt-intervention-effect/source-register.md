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
