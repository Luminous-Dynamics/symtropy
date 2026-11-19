# GeoFusion Evidence Standards Interoperability Crosswalk

**Tracking:** [GEOFUSION-EVIDENCE-RECEIPTS-001 / #6158](https://github.com/Luminous-Dynamics/symtropy/issues/6158)  
**Runtime verifier:** [research/geofusion-evidence-verifier](../../../../research/geofusion-evidence-verifier/README.md)  
**Research snapshot:** 19 November 2026

## Paradigm: compose standards, do not invent an evidence platform

The GeoFusion evidence envelope should remain a **small domain profile** for physical boundaries, carriers, intervals, measurement/model applicability, uncertainty and allocation semantics. It should not become a replacement for established system packaging, research-object metadata, provenance graphs, payload integrity manifests or signatures.

The architecture is layered:

| Layer | Standard / owner | What it establishes | What it does not establish |
|---|---|---|---|
| Whole simulation system | SSP 2.0.1 | Tool-independent component topology, connections, parameterization, units and packaging | Master algorithm quality or physical validity |
| Individual model execution interface | FMI 3.0.2 | FMU packaging and component execution contracts | Federation-wide algorithm or energy conservation |
| Payload inventory / transfer integrity | BagIt 1.0 (RFC 8493) | Complete file manifest and checksum verification | Active-attack authenticity, provenance meaning or source truth |
| Research object context | RO-Crate 1.3 | Dataset/package metadata, files, software, people, equipment and contextual description | Cryptographic issuer trust or scientific validity |
| Provenance semantics | W3C PROV-O / PROV-DM | Entity, activity, agent, usage, generation, derivation and attribution relationships | Physical truth or rights authorization |
| Signed attestations | DSSE / in-toto | Exact subjects, signed predicates and verifiable attestation envelopes under a trust policy | Physical correctness, calibration truth or authority outside the policy |
| GeoFusion domain semantics | This profile / #6158 | Source interval, boundary, carrier, calibration/model disposition, uncertainty and first-law allocation | General-purpose provenance or archive format |

## Standards and pinned references

### SSP 2.0.1 — the full system, not another FMU wrapper

The Modelica Association's SSP 2.0.1 standard (5 May 2026) is the tool-independent format for describing, parameterizing, packaging and exchanging connected simulation systems. It complements FMI: FMI exchanges an individual model, while SSP describes the system of interconnected models. SSP supports hierarchical structures, connector/signal flow, parameterization, unit definitions and conversion/checking, package resources and traceability metadata.

- [SSP 2.0.1 specification](https://ssp-standard.org/docs/2.0.1/)
- [SSP current documentation](https://ssp-standard.org/docs/main/)
- [SSP standard repository](https://github.com/modelica/ssp-standard)

**GeoFusion decision:** use SSP 2.0.1 as the system topology package in the later multi-FMU campaign. Do not create a proprietary JSON graph for the same component connections. Keep carrier-specific energy accounting and thermal-isolation predicates in the GeoFusion domain contracts, because SSP unit checking alone does not prove first-law closure.

### FMI 3.0.2 — component runtime interface

The official FMI downloads page lists the complete FMI 3.0.2 package and interface specifications. FMI is supported by a broad tool ecosystem and is the correct interface baseline for FMU interoperability.

- [FMI downloads and complete package](https://fmi-standard.org/downloads/)
- [FMI 3.0.2 specification](https://fmi-standard.org/docs/3.0.2/)

**GeoFusion decision:** use FMI for per-FMU execution, clocks and step records; use SSP for the system graph; use the trace verifier for a bounded record of time/step semantics. Do not infer that FMI conformance proves the master algorithm or the physical model.

### BagIt 1.0 (RFC 8493) — complete payload inventory

BagIt defines a hierarchical bag with payload files and metadata tag files. A valid bag is complete and has every declared payload/tag checksum successfully verified. BagIt 1.0 requires support for SHA-256 and SHA-512 manifests. RFC 8493 explicitly describes integrity against corruption; it is not designed to be secure against active attacks.

- [RFC 8493](https://www.rfc-editor.org/rfc/rfc8493)

**GeoFusion decision:** use a BagIt bag for a distributable evidence bundle containing raw meter/solver payloads, SSP files, FMU binaries, parameter/configuration manifests, schemas, receipts and replay outputs. Its manifest should cover every payload file exactly once. Keep the current runtime verifier's SHA-256 exact-byte check as the source-interval-specific semantic check, not as a substitute for BagIt completeness.

### RO-Crate 1.3 — research object and contextual metadata

RO-Crate 1.3 is the current long-term specification release. It provides a JSON-LD metadata document for aggregating and describing files and other resources for distribution, reuse, publishing, preservation and archiving. Its contextual model can describe software, equipment, people, methods and the provenance context of a dataset.

- [RO-Crate specification and version history](https://www.researchobject.org/ro-crate/specification/)
- [RO-Crate 1.3](https://www.researchobject.org/ro-crate/specification/1.3/)

**GeoFusion decision:** package the BagIt payload bundle as the data inventory and use RO-Crate for contextual description and relationships. The GeoFusion evidence envelope should be represented as a domain-specific entity/profile within the crate, not a parallel crate descriptor.

### W3C PROV-O — provenance graph semantics

PROV-O is the W3C ontology for expressing the PROV data model. PROV-DM distinguishes entities, activities and agents, with relationships such as usage, generation, derivation, attribution and association.

- [W3C PROV-O Recommendation](https://www.w3.org/TR/prov-o/)
- [W3C PROV-DM](https://www.w3.org/TR/prov-dm/)

**GeoFusion decision:** represent raw payloads, calibration certificates, FMUs, SSP packages, solver outputs and receipts as entities; represent measurement, simulation, transformation and verification runs as activities; represent software, institutions and responsible operators as agents. Only assert `prov:wasDerivedFrom` where the actual input/output lineage is known. Never create all-to-all derivation edges because one activity happened to consume multiple inputs and generate multiple outputs.

### DSSE / in-toto — attestation, not scientific authority

Existing Luminous work owns attestation design and trust policy:

- [PROTECT-EVIDENCE-ATTESTATION-001 / #5560](https://github.com/Luminous-Dynamics/symtropy/issues/5560)

**GeoFusion decision:** attest to exact BagIt/RO-Crate/SSP and evidence-receipt digests using DSSE/in-toto. Verify signer identity through a pinned trust policy. Keep signature validity, identity trust, purpose authorization, calibration validity and physical truth as distinct states. A trusted signature must not upgrade a synthetic fixture into a measured result.

## Reuse existing Luminous owners

Do not create parallel format or semantics implementations:

- [#5524 — PROV-O and RO-Crate lossless portability](https://github.com/Luminous-Dynamics/symtropy/issues/5524) owns the standards-native portability and round-trip benchmark.
- [#5559 — JSON-LD 1.1 and PROV constraint equivalence](https://github.com/Luminous-Dynamics/symtropy/issues/5559) owns standards-level semantic equivalence and validation.
- [#5560 — DSSE/in-toto attestations and trust policy](https://github.com/Luminous-Dynamics/symtropy/issues/5560) owns signatures and signer/trust-policy semantics.
- [#5724 — RO-Crate 1.3 / PROV-O portable ISRU evidence](https://github.com/Luminous-Dynamics/symtropy/issues/5724) is an existing physical-science evidence profile that can share the same portability substrate.
- [#1467 — federated co-simulation boundary](https://github.com/Luminous-Dynamics/symtropy/issues/1467) owns the broad FMI/SSP federation boundary.
- [#5871 — multirate co-simulation tournament](https://github.com/Luminous-Dynamics/symtropy/issues/5871) owns generic coupling algorithm comparisons.

The GeoFusion profile should own only geothermal/fusion-specific source, carrier, boundary, interval, thermodynamic applicability and energy-allocation rules.

## Proposed benchmark gates

Report separate conformance axes. Do not collapse them into a single score.

| Benchmark | Target |
|---|---:|
| BagIt payload inventory completeness | 100% of payload files listed exactly once |
| BagIt checksum verification | 100% of payload and tag manifest checksums match |
| Tampered or missing payload accepted | 0 |
| RO-Crate import/export semantic loss | 0 required-field / contextual-link loss on frozen fixtures |
| PROV graph unresolved required references | 0 |
| Inferred or unsupported derivation edges | 0 |
| SSP component/connection/parameter/unit round-trip | 100% preserved across at least two independent tool implementations |
| DSSE/in-toto tamper or wrong-subject acceptance | 0 |
| Untrusted signer admitted for a restricted purpose | 0 |
| Unknown/stale/synthetic evidence silently upgraded | 0 |
| GeoFusion boundary/carrier/interval/allocation mismatch accepted | 0 |

Also publish p50/p95 export/import time, memory, package size, semantic-diff counts, unresolved-reference counts and time to independent audit.

## Implementation order

1. Keep the current single-source-interval verifier small and exact.
2. Build an SSP 2.0.1 synthetic two-component topology for geothermal source → isolated heat exchanger → thermal buffer, with declared units and parameterization.
3. Produce a BagIt bundle containing the SSP file, FMU(s), raw fixture, envelope, allocation receipt and replay artifacts.
4. Wrap that bundle in RO-Crate 1.3 and map its entity/activity/agent lineage through the existing #5524/#5559 profile.
5. Add DSSE/in-toto subject attestation through #5560 only after the package bytes and semantic identities are frozen.
6. Run two independent import/validation implementations and preserve every unsupported feature, warning and semantic diff.

## Claim ceiling

A BagIt checksum pass is byte integrity. RO-Crate conformance is packaging/context interoperability. PROV-O conformance is provenance semantics. SSP/FMI conformance is simulation interchange. DSSE/in-toto verification is an attestation/trust-policy result. None alone proves source truth, meter accuracy, calibration validity, solver physical validity, geothermal resource quality, fusion performance, plant safety or operational authorization.
