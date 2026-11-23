# Standards and interoperability mapping

**Purpose:** use established interfaces and engineering standards where relevant; avoid creating an unnecessary parallel standards universe. This is a design mapping, not a certification or compliance claim.

| Reference | Current source position | SAFRB / Luminous design action | Boundary |
|---|---|---|---|
| ISO 20815:2026 | Production assurance and reliability management for oil/gas and lower-carbon assets, upstream through downstream. | Map asset availability, reliability, assurance processes, failure consequences and programme evidence to the applicable concepts. | Do not claim conformity without licensed text review, process implementation and independent audit. |
| ISO 14224:2016 | Reliability and maintenance data collection/exchange taxonomy for petroleum, petrochemical and natural gas equipment; current version confirmed in 2022. | Use its equipment taxonomy, failure mode and maintenance-data discipline for later pipeline/refinery reliability datasets. | It is not a fuel cargo event or inventory accounting standard. |
| ISO 23247-5:2026 | Digital thread principles for connecting and maintaining manufacturing digital twins across lifecycle. | Adopt the digital-thread concept: stable IDs, data lineage, versioned model/input/output artifacts and traceable lifecycle changes. | The standard is manufacturing-oriented; apply concepts carefully, not as a claim of oil/gas sector applicability. |
| GS1 EPCIS 2.0.1 | Current EPCIS event-sharing specification, published 1 July 2025. | Map movement, aggregation, custody, location, business step and event-time data to EPCIS-compatible event semantics where appropriate. | Preserve domain extensions explicitly; avoid making EPCIS carry high-rate SCADA telemetry. |
| W3C Verifiable Credentials Data Model 2.0 | W3C Recommendation, 15 May 2025. | Use standard issuer/holder/verifier roles and verifiable credential structures for lab certificates, inspections and attestations. | A valid signature proves issuer/integrity under a trust policy; it does not prove sensor calibration or physical truth. |
| OSDU | SLB Lumi describes alignment with OSDU for subsurface and operations data interoperability. | Prefer established energy data platforms and adapters over a proprietary monolith when industrial partners already use OSDU. | Not a replacement for facility-specific process and OT interfaces. |
| NIST SP 800-82 Rev. 4 IPD | Initial public draft released 21 Sep 2026; comment period ends 30 Nov 2026. | Incorporate its OT context, segmentation, asset management, monitoring and zero-trust principles; review AI/model artefact governance as a potential comment. | Draft guidance, not a final requirement. |

## Interoperability policy

1. Preserve original source payloads and raw digests.
2. Translate to canonical internal types without destroying original units, identifiers or source timestamps.
3. Store transformation/version details and the evidence for every normalization.
4. Export accepted events to EPCIS-compatible structures where semantic fit exists.
5. Package credentials using W3C VC 2.0-compatible formats and established cryptographic suites.
6. Keep high-frequency telemetry in industrial historians / time-series stores; place evidence references and meaningful state transitions in the provenance layer.
7. Do not equate standards compatibility with safety, certification or measurement validity.
