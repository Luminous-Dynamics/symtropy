# Maritime interoperability profile research v0.1

**Status:** design research and profile mapping only. This document does not implement IHO S-100 parsing, certify any dataset, perform hydrographic calculations, or authorize navigation.

## Finding: use official product specifications as versioned adapters, not as a new internal ontology

The International Hydrographic Organization (IHO) S-100 family is a strong interoperability seam for the marine-freight work. Its current *Standards in Force* page lists the following product editions with entry into force on 1 January 2026:

| Product | Edition listed in force | Intended role in this architecture |
|---|---:|---|
| S-101 Electronic Navigational Chart | 2.0.0 | Chart-feature profile; source identity and applicable chart/update metadata must survive intake |
| S-102 Bathymetric Surface | 3.0.0 | Bathymetric coverage profile; preserve depth reference, uncertainty/quality information, coverage, and edition |
| S-104 Water Level Information for Surface Navigation | 2.0.0 | Water-level observations/predictions; preserve the vertical reference, validity interval, product/status and time semantics |
| S-111 Surface Currents | 2.0.0 | Current information; preserve direction/speed units, reference frame, spatial/time coverage and quality |
| S-124 Navigational Warnings | 2.0.0 | Warning records; preserve issuer, warning identifier, issue/revision and validity/withdrawal state |
| S-129 Under Keel Clearance Management | 2.0.0 | Interoperability profile relevant to UKC management; not itself evidence that a local clearance calculation is valid or authorized |

Sources: [IHO Standards in Force](https://iho.int/en/s-100-standards-in-force-0), [IHO S-101 to S-199 product specifications](https://iho.int/en/iho-s-101-to-s-199), and [S-100 developer resources and test data](https://iho-ohi.github.io/S100Resources/). Re-check the authoritative registry and exact edition at implementation/review time; do not treat this table as a hard-coded, permanently current registry.

**Critical distinction:** a dataset being public, having an S-100-shaped encoding, or conforming to a product schema does not prove publisher authenticity, currentness, correctness for a particular location, permission for a proposed use, or qualification for navigation. Likewise, general bathymetry used for planning/modeling must not be silently promoted into an official navigation source. NOAA NCEI explicitly distinguishes its Bathymetric Data Viewer planning/modeling data from data suitable for navigation: [NCEI Bathymetric Data Viewer](https://www.ncei.noaa.gov/maps/bathymetry/).

## Recommended data path

1. **Discover** a source/product and retrieve its exact metadata, edition, producer/provider identity, spatial and temporal coverage, update/revision identifier, and use terms.
2. **Capture** original bytes without normalization. Record the exact request/source URI, retrieval time, response metadata and digest. A digest verifies byte identity only against bytes actually held.
3. **Record a receipt** using the existing Symtropy v0.1 provenance/byte-integrity contract. Keep its claim ceiling: the verifier is offline-only and always emits NotAdmitted.
4. **Validate a product profile** in a separate versioned adapter. S-100 product edition, S-100 edition, producer/code, dataset/cell identity, issue/update sequence, native encoding, validity/coverage and product-specific quality semantics belong in the profile, not in generic unit strings.
5. **Normalize only as a new artifact.** Each transform has its own input digest(s), tool/version, declared CRS/datum/unit operations, parameter set, output digest, and validation report. Preserve source bytes and unknown fields/qualifiers; never silently rewrite a datum, interpolate a missing field, or collapse preliminary and verified status.
6. **Apply a declared use/freshness policy.** Separate publication time, capture/retrieval time, observation/survey time, validity time, data coverage, update age and policy-evaluation time. A failed refresh, absent warning, missing tile, or expired update means unknown/stale/indeterminate—not clear water or zero risk.
7. **Admit to a scenario only under explicit capability/model gates.** The manifest must bind exact source receipt(s), product-profile version, transform receipts, model/capability census, scenario/configuration and output. S-129, S-102, or any other product cannot bypass Symtropy's model-credibility and independent-qualification gates.

## Cross-project ownership

- **Symtropy:** immutable input receipts, deterministic scenario manifest/replay, source-to-run binding and explicit Unsupported / Unknown outcomes. No new physical model is implied by ingesting a product.
- **Sol Atlas (sol-atlas-leptos):** renderer-neutral asset discovery, geospatial/temporal projection, visible uncertainty/status and click-through provenance. Rendering must not promote evidence or qualification.
- **Symthaea:** route/vessel/energy/terminal trade-off analysis only after inputs and supported model paths are admitted. Separate infeasibility constraints from Pareto objectives; do not let cost, throughput or a high confidence estimate compensate for a failed hard constraint.
- **Mycelix:** cargo, pack, provider, handoff and port-call event provenance using existing logistics and EPCIS/CBV interoperability work. Event integrity, custody interpretation, delivery acceptance, conformity and legal/operational authority remain separate.

Reference: [GS1 EPCIS/CBV](https://www.gs1.org/standards/epcis) defines interoperable supply-chain visibility events about what, when, where, why and how assets move; use it as an adapter rather than replacing Mycelix's existing evidence semantics.

## First safe vertical slice

Use frozen synthetic S-100-shaped metadata/fixture examples for S-102, S-104 and S-111 profile tests before attempting real payload parsing. Then select one small, explicitly permitted public example only after its source-specific licence, edition, coverage, quality metadata and redistribution constraints have been reviewed. Do not download or publish real facility-security data merely because it is accessible.

Required negative cases:

- mutable latest or missing product edition/update identity;
- valid digest but wrong/unavailable source artifact;
- S-104 water-level value missing vertical reference or validity semantics;
- S-102-like bathymetry missing depth reference, uncertainty/quality or coverage;
- S-111-like current missing units, direction/reference-frame or time semantics;
- S-124-like warning that is expired, withdrawn, superseded or from an unqualified source;
- two products share a URL but differ in edition/cell/update sequence;
- normalization changes CRS, datum, units, grid, or timestamps without a transform receipt;
- correlated provider feeds incorrectly counted as independent corroboration;
- missing/failed update rendered as zero, safe, clear, or current;
- schema-conforming data promoted to navigation clearance, seaworthiness, or engineering approval;
- renderer, Symthaea recommendation or Mycelix event mutates source/qualification status.

## Claim ceiling

Passing these checks can establish metadata/receipt/profile conformance for an exact artifact and ruleset. It cannot establish that a chart is current for voyage planning, bathymetry is accurate enough for clearance, a vessel is seaworthy, a berth is safe, a route is navigable, a forecast is physically correct, or a real-world operation is authorized. Operational navigation remains with the official products, approved equipment/procedures, competent operators and applicable authorities.
