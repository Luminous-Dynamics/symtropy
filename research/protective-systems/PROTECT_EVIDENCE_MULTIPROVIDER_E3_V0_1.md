# Protective Evidence E3 — DataCite, OGC Records and OpenLineage

**Snapshot:** 16 November 2026  
**Owner:** PROTECT-EVIDENCE-MULTIPROVIDER-001 #5598  
**Branch:** `research/protect-evidence-multiprovider-e3-v0-1`  
**Parent:** E2 semantic portability #5524 / PR #5556  
**Status:** three synthetic provider fixtures, pinned source manifest, RO-Crate 1.3 package, OpenLineage 2.0.2 schema snapshot, Rust adapter gate and 30 tests written; exact-head hosted CI pending

## 1. The paradigm shift: multi-standard semantics without flattening

Evidence integration fails when a generic intermediary treats every provider record as the same kind of object. A DOI metadata record, a geospatial catalog record and an observed pipeline run are related, but they are not interchangeable.

E3 treats the common evidence record as a *projection*, never as a replacement for native provider data:
- preserve original native bytes and provider identity;
- make mapping decisions explicit;
- retain unknown/extension fields without silently discarding them;
- keep metadata discovery, data lineage and execution evidence separate;
- report exactly which standard/profile checks were applied;
- abstain when a record cannot support a stronger claim.

The package includes the exact E1 context envelope. E1 says `execution_status=NOT_EXECUTED` and `run_id=null`. The OpenLineage START/COMPLETE pair is a separate synthetic run of the E3 adapter conformance fixture; its facet labels the E1 analysis as context only, not as that run.

## 2. Standards and industry baselines

### DataCite REST API and Metadata Schema 4.7

The DataCite REST API returns JSON using JSON:API. The public metadata retrieval API does not require authentication and is designed for harvesting Findable DOI records. This makes it useful for source identity and citation metadata, but the REST response itself is not an authenticated scientific result.

DataCite Metadata Schema 4.7 was released on 3 March 2026. E3 specifically exercises three 4.7 additions:
- `SWHID` as a `relatedIdentifierType`;
- `Other` as a `relationType`;
- `relationTypeInformation` for explanatory text when `Other` is used.

The DataCite REST payload's `schemaVersion` remains the generic `http://datacite.org/schema/kernel-4` URI; it does not by itself encode the 4.7 minor version. The E3 manifest therefore declares the 4.7 field-profile target separately. The Rust gate checks the REST JSON:API envelope, the selected 4.7 fields, and core SWHID syntax (`swh:1:<object-type>:<40 lowercase hex>`). The all-zero SWHID fixture value is a syntactic sentinel only; it is not asserted to resolve in Software Heritage. The gate does **not** claim full XML/XSD validation of a DataCite registration.

References:
- DataCite REST API and JSON:API response format: https://support.datacite.org/docs/rest-api
- DataCite response structure: https://support.datacite.org/docs/how-do-i-query-the-rest-api-and-whats-in-the-response
- DataCite Schema 4.7 release history: https://schema.datacite.org/versions.html
- DataCite 4.7 additions: https://datacite-metadata-schema.readthedocs.io/en/4.7/introduction/version-update/
- Related identifier type `SWHID`: https://datacite-metadata-schema.readthedocs.io/en/4.7/appendices/appendix-1/relatedIdentifierType/
- `Other` and `relationTypeInformation`: https://datacite-metadata-schema.readthedocs.io/en/4.7/properties/relatedidentifier/

### OGC API - Records Part 1: Core 1.0

OGC API - Records standardizes discovery and access to catalog metadata. It is not a data payload format and does not make a referenced geospatial payload available or valid.

The E3 fixture uses the GeoJSON Feature record encoding and retains:
- server-assigned record `id`;
- top-level `conformsTo`;
- `properties.created`, `updated`, `type`, `title`, `description`, `language` and `keywords`;
- top-level `geometry`, `extent` and `links`;
- explicit CRS84 for the spatial bbox;
- a foreign extension at `properties.e3Extension`.

The polygon is a fictional unit-square geometry used only to test the record adapter. It is not a real-world location or a physical dataset. The verifier checks that the bbox matches the geometry, that the exterior ring closes, and that coordinate reference information remains explicit; it does not reproject coordinates or validate a referenced raster/vector payload.

The standard's GeoJSON requirements include a Feature object, geometry, extent and a profile link in the record's links. E3 manually checks this selected record profile. It does not test a live OGC API server's endpoints, HTTP negotiation, catalogs, pagination, filtering or the full OGC conformance suite.

References:
- OGC API - Records Part 1: Core 1.0 (20-004r1): https://docs.ogc.org/is/20-004r1/20-004r1.html
- OGC standard overview: https://www.ogc.org/standards/ogcapi-records/
- Official record GeoJSON schema: https://schemas.opengis.net/ogcapi/records/part1/1.0/openapi/schemas/recordGeoJSON.yaml

### OpenLineage RunEvent 2.0.2

OpenLineage records observed state transitions for a job run. Its model separates Job, Run, input datasets and output datasets. A run needs a START event and one terminal event (COMPLETE, ABORT or FAIL); the E3 fixture uses START followed by COMPLETE for the synthetic adapter test run.

The official OpenLineage 2.0.2 JSON Schema is vendored without modification, pinned by its schema `$id`, source repository and GitHub blob SHA, and used to validate each event. E3 also enforces semantic invariants across the event pair:
- the run UUID and job identity are stable;
- START precedes COMPLETE;
- input/output dataset namespace and names are retained;
- the exact OpenLineage 2.0.2 `schemaURL` fragment is retained on both events;
- event timestamps use canonical UTC `Z` form for deterministic ordering;
- the custom E3 evidence facet is retained on both events;
- the facet records the E1 analysis ID and its `NOT_EXECUTED` state as context only, with `notASourceExecution=true`.

The schema snapshot is from the OpenLineage 2.0.2 specification. No production job or E1 model execution is claimed.

References:
- OpenLineage specification: https://github.com/OpenLineage/OpenLineage/blob/main/spec/OpenLineage.md
- Pinned JSON Schema identity: https://openlineage.io/spec/2-0-2/OpenLineage.json
- OpenLineage Run Cycle: https://openlineage.io/docs/spec/run-cycle/
- Run/job/dataset naming: https://openlineage.io/docs/spec/naming/

## 3. E3 package contents

Package root:

`fixtures/protective-evidence-multiprovider-e3-v0.1/`

The RO-Crate 1.3 package contains seven File entities:
1. DataCite JSON:API DOI metadata response;
2. OGC API - Records Core 1.0 GeoJSON Feature;
3. OpenLineage START/COMPLETE event bundle;
4. exact E1 evidence envelope copied byte-for-byte;
5. E3 source/rights/profile manifest;
6. Draft 2020-12 schema for the E3 manifest;
7. official OpenLineage 2.0.2 JSON Schema snapshot.

The E3 manifest pins source provider identity, native profile, rights/use ceiling, byte length and SHA-256 for each provider record. It separately pins the OpenLineage schema asset. The E1 context envelope has its own digest and explicit `CONTEXT_ONLY_NOT_A_RUN` relationship.

## 4. Rust gate

Binary: `protective-multiprovider-e3`

The gate:
- validates the full E3 manifest against its pinned Draft 2020-12 JSON Schema;
- verifies exact byte lengths and SHA-256 values for each provider fixture and schema asset;
- checks the packaged RO-Crate 1.3 descriptor, root Dataset, hasPart and File inventories;
- checks DataCite JSON:API identity, the generic kernel-4 schemaVersion URI, the 4.7 SWHID/Other/relationTypeInformation field combination, rights statement and JSON:API extension preservation;
- checks OGC Core/JSON conformance classes, GeoJSON Feature structure, created/updated metadata, links/profile, polygon closure, CRS84 bbox and bbox/geometry agreement;
- validates each OpenLineage event with the unmodified official 2.0.2 JSON Schema, then checks the shared run/job identity and START/COMPLETE ordering;
- compares the E1 context envelope byte-for-byte to the parent E1 fixture and rejects any attempt to represent its `NOT_EXECUTED` status as the E3 OpenLineage run.

The common provider projections are summaries, not replacements. The native source bytes remain in the crate, while known extension roots are enumerated as preserved but not flattened into the common profile.

## 5. Benchmark programme

| Benchmark | Prospective target |
|---|---:|
| Native source identity preservation | 100% |
| Source byte length/SHA-256 verification | 100% |
| Known required profile-field preservation | 100% |
| Unknown/extension roots retained or explicitly reported | 100% |
| OpenLineage 2.0.2 schema validity for the frozen event corpus | 100% |
| E1 NOT_EXECUTED promoted to a real run | 0 |
| Fabricated DOI/provider authentication | 0 |
| Fabricated CRS or silent coordinate transformation | 0 |
| Rights/license or claim-ceiling escalation | 0 |
| Semantic projection field loss on frozen fixtures | 0 |
| Adapter/reviewer effort and runtime | p50/p95 measured on frozen multi-provider corpus |

A genuine SOTA comparison requires at least three independently maintained live/sandbox provider fixtures, a frozen test corpus, reference validators, field-by-field semantic diffs, and a matched manual-mapping baseline. The present three records are synthetic fixtures and do not establish superiority over DataCite, OGC or OpenLineage implementations.

## 6. Verification status and limitations

- Three provider fixtures, E1 context, manifest, schemas and RO-Crate package are committed.
- OpenLineage events are checked against the vendored official 2.0.2 schema.
- The E3 Rust gate has 30 unit tests written.
- Formatting, Clippy, Rust compilation, test execution, executable output and independent RO-Crate validation need exact-head hosted CI evidence. No E3 pass is claimed yet.
- The DataCite adapter is a JSON:API + selected Schema 4.7 field-profile check, not full XSD validation.
- The OGC adapter is a selected Core/GeoJSON record-profile check, not full server conformance.
- The OpenLineage adapter validates RunEvent JSON Schema 2.0.2, but not a live lineage service or physical/scientific correctness.
- The RO-Crate validator is a separate beta-stage test-only oracle and does not authenticate a source, grant a license or prove model validity.

All E3 records are static synthetic fixtures. No external API calls, DOI registrations, writes, credentials, private data or physical actions are performed.
