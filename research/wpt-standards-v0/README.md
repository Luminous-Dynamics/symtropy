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


## South African authority records (added 2026-11-08)

Five additional records separate standards from the national authorization context:

- **National Radio Frequency Plan 2026** — ICASA announced publication on 24 July 2026 (Minister approval 2 July 2026; Government Gazette 55048, Notice 4044). The 2026 plan, not the 2021 edition, is the current allocation reference to evaluate band-by-band.
- **Equipment Authorisation Regulations, 2022** — published in Government Gazette 46146, Notice 1949. Product-specific scope and any exemption remain unassessed.
- **ICASA Spectrum Licensing** — the authority page describes Electronic Communications Act section 31 spectrum-licensing requirements. Actual frequency assignment, radio classification and exemptions remain configuration-specific.
- **ICASA Type Approval** — the authority page describes Electronic Communications Act section 35(1) equipment approval requirements, subject to the Act. This is not itself a product approval or exemption.
- **SACAA UAS information page** — the authority overview lists autonomous unmanned aircraft among the exclusions from Part 101, but the page does not identify the alternative regulatory/approval regime for autonomous aircraft. The register therefore marks overall applicability UNKNOWN, not OUT_OF_SCOPE or permitted.

Official sources:

- https://www.icasa.org.za/legislation-and-regulations/national-radio-frequency-plan-2026
- https://www.icasa.org.za/uploads/files/Equipment-Authorisation-Regulations-2022.pdf
- https://www.icasa.org.za/pages/spectrum-licensing
- https://www.icasa.org.za/pages/type-approval
- https://www.caa.co.za/industry-information/uas/

These records are jurisdictional research anchors. They do not replace the authoritative Gazette/Act text, a band-specific review, product-specific classification, direct SACAA/ICASA interpretation or specialist legal review.


### 85 kHz IPT and Annex B exemption boundary

A further important overlap was verified against the current NRFP 2026 interactive table and the official ICASA Annex B amendment:

- The NRFP table's 84–86 kHz row lists radionavigation allocation/footnote 5.60 and also shows inductive-loop / RFID typical-application references.
- Annex B lists **Inductive Applications at 9–135 kHz**, with a field-strength limit of **42 dBµA/m at 10 m**, performance standard **SANS 300 330**, and additional restrictions for levels above 42 dBµA/m.
- Therefore the 85 kHz IPT lane sits inside the Annex B frequency interval, but this does **not** establish that a high-power WPT charger qualifies for the exemption. The apparatus must fit the defined application category and meet all specified limits, performance standards, EMC/safety and any additional restrictions. Product classification and measured field strength at the prescribed distance are unresolved.

Sources:
- https://app3.icasa.org.za/ (NRFP 2026; 84–86 kHz row)
- https://www.icasa.org.za/uploads/files/Notice-to-amend-Annexure-B-of-the-Radio-Frequency-Spectrum.pdf (GG 48643, Notice 1822, 23 May 2023)
- https://www.icasa.org.za/uploads/files/Amendment-To-The-Radio-Frequency-Spectrum-Regulations-2015.pdf

The register records this as RADIO_SPECTRUM_EXEMPTION with airborne applicability UNKNOWN. It is not an exemption determination, spectrum licence or approval to operate.
