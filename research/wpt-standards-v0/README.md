# WPT standards applicability register v0

**Status:** source-checked research metadata; not a compliance assessment, legal determination or certification.

The register inventories document lifecycle, source-described technical scope and the current status of aircraft-applicability review. UNASSESSED means no aircraft applicability decision has been made; it does not mean applicable, inapplicable or safe.

## Files

- standards.tsv — tab-separated source records with a frozen 20-column header.
- tools/wpt-standards-register/src/main.rs — dependency-free Rust validator and CLI.

TSV is used for v0 so every source claim remains inspectable in a plain diff. Tabs/newlines are forbidden inside field values. All rows must use the exact header width.

## Initial source records

1. **IEEE/IEC 63184-2025.** The official IEEE page describes a WPT human-exposure assessment standard focused on inductive WPT. A material source discrepancy is preserved: the page title/working-group descriptor says 3 kHz–30 MHz, while descriptive body text says 1 kHz–30 MHz. Do not silently select one value.
2. **CISPR PAS 38:2025.** The official IEC scope distinguishes the device operating-frequency envelope (0 Hz–400 GHz, designed operation up to 10 m) from emissions requirements (150 kHz–18 GHz). Its scope excludes devices classified as radio apparatus and explicitly does not set RF-exposure requirements. National radio requirements remain a separate assessment.
3. **IEC TR 63377:2022.** Published technical report describing human-exposure assessment methods for radiative WPT from 30 MHz to 300 GHz, including focused/narrow-beam systems.
4. **IEEE P3735.** IEEE SA identifies an Active PAR approved 10 September 2025; the project page lists no approved active standard. It is a standards-development project, not a published standard.

## Official sources

- https://standards.ieee.org/ieee/63184/10156/
- https://webstore.iec.ch/en/publication/67222
- https://webstore.iec.ch/en/publication/68552
- https://standards.ieee.org/ieee/3735/12230/

Source access date: 2026-11-08. Recheck lifecycle and scope before reuse.

## Validator invariants

- Exact 20-column header and row width; no empty/edge-whitespace fields.
- Unique record/document IDs and HTTPS source URLs.
- Explicit lifecycle, document-type, hazard-domain and applicability vocabularies.
- ACTIVE_PAR cannot be represented as PUBLISHED or ACTIVE_STANDARD.
- UNASSESSED aircraft applicability requires NOT_REVIEWED clause/reviewer placeholders.
- Assessed applicability requires a clause locator and named reviewer.
- Source title/body discrepancies must be declared, not silently normalized.
- Every record carries a claim ceiling.

## Claim ceiling

A passing validator proves only metadata consistency. It does not interpret purchased standards text, decide aircraft applicability, provide legal advice, establish safety, authorize energization or flight, or demonstrate compliance. Clause-level interpretation requires authorized full-text access and independent specialist review in the relevant jurisdiction.
