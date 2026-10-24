# Site Evidence Adapter Contract (Draft)

- **Status:** interface proposal only; no production adapter, validated site dataset, or geotechnical result is claimed.
- **Date:** 2026-10-13
- **Purpose:** define a versioned, read-only boundary between Sol Atlas's geospatial visualization/data catalogues and Symtropy's housing-engineering case.
- **Related documents:** [Housing engineering RFC](housing-engineering-rfc.md), [solver adapter evaluation](solver-adapter-evaluation.md), [Sol Atlas](https://github.com/Luminous-Dynamics/sol-atlas-leptos), [Sol Atlas PR #104](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/104), [Sol Atlas PR #101](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/101), and [Mycelix Earth PR #3776](https://github.com/Luminous-Dynamics/luminous-dynamics/pull/3776).

## 1. Boundary decision

Sol Atlas is a located public renderer/core project with useful geographic display primitives and active proposals for provenance-bearing utility observations and Earth-systems catalogues. It is **not currently demonstrated to be an authoritative housing-site assessment pipeline**. The open PRs are still draft/pending/queued; their tests and planned ingestion must not be described as passed or deployed.

The adapter should initially exchange versioned JSON plus separate source/output artifacts. Do not link the AGPL-declared `sol-atlas-core` directly into the Symtropy Rust workspace until the project has an explicit licensing/redistribution decision. A process boundary is a technical design choice, not by itself a legal conclusion.

## 2. Projection object

Use a versioned `SiteEvidenceContextV1` object as a *pointer and provenance envelope*, not a giant duplicated raster container. It identifies the project/site geometry, evidence sources, processed outputs, allowed interpretations, and missing investigations. Large source bytes and analysis outputs remain in an artifact store; the Symtropy housing manifest registers their exact SHA-256 and safe artifact URI.

Required top-level fields:

| Field | Requirement |
|---|---|
| `schema_id` | Stable identifier, e.g. `luminous.site-evidence-context`. |
| `schema_version` | Exact compatible version; unknown major versions block ingestion. |
| `case_id` / `site_id` | Stable IDs; the projection cannot silently change site identity. |
| `created_at` | RFC 3339 timestamp for the projection. |
| `site_geometry` | Geometry, coordinate reference system, axis order, and provenance. A map-drawn boundary is not automatically a surveyed parcel boundary. |
| `observations` | Zero or more separately typed source observations and derived layers. Empty, absent, and unknown are distinct states. |
| `artifact_refs` | References to raw snapshots, aligned products, masks, outputs, and reports; each resolved artifact has an independently recorded SHA-256. |
| `limitations` | Explicit non-inferences and investigation needs, not just free-form confidence prose. |

## 3. Per-observation contract

Every observation MUST record:

- stable `observation_id`, `phenomenon`, subject/footprint, and semantic data state;
- data state: `OBSERVED`, `DERIVED`, `FORECAST`, `SCENARIO`, `CATALOGUE_METADATA`, or `UNKNOWN`;
- source dataset/provider, collection/product and version, source item/scene ID where applicable, source URI, publication/acquisition time, source-observation time, and adapter receive time;
- declared CRS (including coordinate axis order), native pixel size/resolution if gridded, footprint/coverage, no-data fraction, quality-mask identity, horizontal and vertical accuracy where relevant, vertical datum for elevation, and uncertainty or an explicit `unknown`;
- processing/transform chain with each tool name/version, tool artifact SHA-256, parameter/config digest, input artifact digests, output artifact digest, CRS transformation, resampling/interpolation, calibration/scaling, band mapping, and quality-mask policy as applicable;
- exact units for every quantitative field, metric method, thresholds, and the validity domain;
- reuse/license state, required attribution, source limitations, prohibited inferences, and whether a survey or specialist investigation remains required.

Use RFC 7946 GeoJSON in WGS 84 longitude/latitude coordinate order (equivalent to OGC:CRS84) for interchange geometry. Keep native projected-grid CRS, transformation parameters, vertical datum, and survey reference system in explicit sidecar metadata/artifacts; do not invent a custom CRS member or silently relabel projected coordinates as longitude/latitude. GeoJSON interchange coordinates are not a substitute for a professional coordinate/survey specification.

## 4. Source and derived data are different

Keep the layers and evidence stages separate:

1. **Catalogue metadata found** means a provider advertises an item. It does not mean raster bytes were acquired.
2. **Source bytes captured** means exact source bytes or provider metadata response were retained and hashed. It does not mean the values were analysed.
3. **Preprocessed source ready** means the declared CRS/grid, scale factors, nodata semantics, QA masks and time/window have been applied and recorded. It does not mean the metric is validated.
4. **Derived measurement produced** means a versioned method emitted a result and retained inputs, masks, parameters and outputs. It does not mean the method is suitable for engineering use.
5. **Engineering applicability reviewed** means someone competent has evaluated the source vintage, spatial and temporal resolution, uncertainty, validity domain and intended use.
6. **Site requirement satisfied** is a separate per-claim engineering conclusion and must cite the applicable rule/criterion, validated analysis, and required professional review. Never infer it from earlier stages.

The GIBS tile helper is for browse/display imagery, not analysis-ready reflectance. The current proposed NDWI/MNDWI function expects already aligned reflectance samples and upstream provider-specific QA masks. An inventory plan for CHIRPS/WaPOR is metadata, not processed rainfall, water productivity, flood levels or geotechnical data. Sentinel/Landsat STAC search produces catalogue metadata; it is not a validated terrain model or site assessment.

## 5. Mandatory semantics for housing use

- Null/no-data/unknown must never become zero, “clear”, dry ground, no flood risk, or no constraint.
- A scenario or forecast must never be displayed or exported as an observation.
- Coarse DEM, satellite imagery, H3 cells, or a globe's sphere-coordinate transform must not be used as parcel-grade geometry, a foundation bearing-capacity estimate, groundwater elevation, liquefaction determination, structural design load, or statutory flood-design level.
- Solar climatology is not an installed-system yield guarantee; a weather display is not a design climate record.
- A public road or utility marker is not proof of legal access, service capacity, connection approval, easement rights or service availability at the plot.
- Land cover and water-index estimates are not automatically wetlands delineations, flood studies, contamination surveys, or legal constraints.
- Utility telemetry/observations are read-only decision support; no site projection may add a SCADA write path, command, payment, permit, or actuation authority.
- Every site-related engineering claim names which evidence stage it relies on; missing, stale, unlicensed, low-coverage, out-of-domain, or unreviewed evidence yields `INSUFFICIENT_EVIDENCE`, `BLOCKED`, or `REQUIRES_REVIEW`, never inferred success.

## 6. First practical interface

A read-only adapter should emit a small `site-context.json` plus an artifact directory. The initial pilot should not fetch or decode imagery in Symtropy. Let the source-specific provider/processing project own acquisition and QA, then validate the emitted artifact and metadata at this boundary.

Minimal run receipt:
- site/case ID and projection schema/version;
- source catalog query and time interval;
- source item IDs, source terms/reuse state, and raw response/source hashes;
- output hashes and complete transform-chain digests;
- dataset coverage, accuracy/resolution, valid-area fraction, and missing-data counts;
- per-layer limitations and prohibited inferences;
- toolchain/adapter version and executable/container hash;
- outcome, timestamp and deterministic report digest.

No live municipal/private utility records should be added until access permissions, scope, retention, redaction and operator-approved interfaces are documented.

## 7. Acceptance tests before pilot use

- Reject an unknown major schema version, missing source identity, undeclared units, unsafe URI, malformed digest, missing output artifact, and CRS without an axis-order convention.
- Reject a changed source item/version/hash or changed transform/configuration with a previously pinned evidence contract.
- Reject an `OBSERVED` label for a scenario/planned layer, and reject unknown/no-data coercion to numeric zero.
- Reject output generated below minimum valid-pixel/area coverage or without required cloud/shadow/nodata QA masks.
- Reject raster results without aligned grids, scale-factor records, band mapping, CRS transformations, or vertical-datum metadata where those apply.
- Ensure catalogue-only records cannot satisfy a site condition; source-byte capture cannot satisfy a derived-measurement claim; derived measurements cannot satisfy an engineering requirement without applicability review.
- Ensure a globe-rendered point, H3 cell or coarse elevation tile cannot be interpreted as survey-grade site geometry or geotechnical investigation.
- Verify artifact hashes end-to-end and preserve source/receive/processing times.
- Run deterministic synthetic data first. If a source catalogue or raster isn't available, mark the stage blocked rather than inventing or substituting a public demo layer.

## 8. Gauteng / City of Johannesburg pilot source register (candidate only)

**Jurisdiction caveat:** Johannesburg is a research baseline, not a selected parcel or confirmed authority. Before using this profile, resolve the actual municipality, erf/township identity, zoning scheme, title conditions, and applicable code editions. If the chosen parcel is outside the City of Johannesburg, substitute that municipality's current official submission and planning sources rather than reusing CoJ rules.

| Source/custodian | Useful first-stage evidence | Not enough to conclude | Gate before engineering use |
|---|---|---|---|
| [Chief Directorate: National Geospatial Information (CD:NGI)](https://ngi.dlrrd.gov.za/index.php/what-we-do/maps-and-geospatial-information) and [aerial imagery program](https://ngi.dlrrd.gov.za/index.php/what-we-do/aerial-photography-and-imagery) | Official mapping, topographic data, elevation products and aerial imagery. The published imagery page describes 0.5 m GSD products for 2008–2016 and 0.25 m GSD imagery from 2017 onward. | Availability of a given flight/tile, its capture date, vertical/horizontal accuracy, cadastral boundary or a surveyed level. Pixel size is not positional accuracy. | Acquire the actual product/tile; retain its metadata, capture vintage, datum/CRS, accuracy, licence, footprint and digest. Confirm product-level availability and terms. |
| [Council for Geoscience (CGS) interactive map](https://maps.geoscience.org.za/) and [geological map service](https://maps.geoscience.org.za/hosting/rest/services/GEOLOGY_V3_MIL1/MapServer) | Regional geology, mapped engineering-geology/geology layers and desktop hazard/context screening. The service exposes map layers at regional scales including 1:50 000 and 1:250 000. | Foundation bearing capacity, depth to bedrock, mine void/stability clearance, groundwater level, dolomite-risk certification or site-specific soil parameters. | Record scale and layer vintage; make targeted enquiry for relevant CGS data where appropriate; require a competent geotechnical investigation for design parameters and unresolved ground hazards. |
| [Department of Water and Sanitation (DWS) spatial information/map shop](https://www.dws.gov.za/SLIM/mapshop.aspx) and [DWS planning document portal](https://www.dws.gov.za/docPortal/) | Catchment/water-management context and discovery of relevant water-resource reports or spatial layers. | Current parcel-scale design flood level, stormwater capacity, flood-line determination, wetland delineation or proof that an area is flood-safe. | Record the exact layer/report, release date, scale, basin, CRS and rights. Require a qualified hydrology/drainage study and applicable authority confirmation where the design depends on flood or runoff behaviour. |
| [City of Johannesburg Construction Permit Management System](https://cpms.joburg.org.za/) and [submission verification checklist](https://cpms.joburg.org.za/asset_uplds/docs/Checklists/Submission-verification.pdf) | Applicable CoJ process and submission-document prerequisites. The current public checklist includes title deed, zoning information, a registered Surveyor-General diagram, and SANS 10400 forms; the CPMS describes building-plan submissions by an appropriately registered professional. | A map point as legal erf proof, an unverified zoning inference, approval, or permit based on automated screening. These requirements are CoJ-specific, not a universal municipal schema. | Match legal parcel identity across title deed/SG diagram and the applicable municipal records. Track professional submission, municipal comments and approval as separate signed/issued evidence records. |
| [National Building Regulations and Building Standards Act 103 of 1977](https://www.gov.za/documents/national-building-regulations-and-building-standards-act-16-apr-2015-1302) and [regulations](https://www.gov.za/nso/documents/notices/national-building-regulations-and-building-standards-act-regulations-27-may-2008) | Legal framework for jurisdiction-specific building approval and standards applicability. | A complete, current, licensed set of applicable SANS clauses or proof that a design meets them. | Determine applicable amended instruments and SANS editions with the authority/qualified professional; track edition, effective date, clause/location and lawful access. Do not copy licensed standards text into public datasets without permission. |

These are source candidates for discovery and preliminary constraints only. Source links can move, products can be stale or unavailable, and a public service may be display-only or constrained by use terms. Every imported layer still passes the per-observation provenance and quality gates in §3–§5. A missing source or failed download stays `UNKNOWN`/blocked; it must not be replaced with invented numbers or an unrelated demo layer.

## 9. Readiness status

As of the 2026-10-14 status snapshot, this adapter is **not implemented**. Sol Atlas PR #104 is at head `b561102f77df76f779b4b5c30a2cccb361074afe`; exact-head run [`38509723929`](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38509723929) passed `fmt`, `test-native`, and `check-wasm`, including canonical evidence-digest guards for derived water observations. This validates code/tests only; no production imagery was acquired or evaluated. Sol Atlas PR #101 is at head `3791125bafc6e60013242fc379341855fa574bd2`; catalog-validation run [`38510250201`](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38510250201) passed structural validation and 15 Python unit tests, and Rust `Check` run [`38510250259`](https://github.com/Luminous-Dynamics/sol-atlas-leptos/actions/runs/38510250259) passed `fmt`, `check-wasm`, and `test-native`. These are catalog/contract checks; CHIRPS/WaPOR raster products have not been acquired or processed. Mycelix Earth PR #3776's earlier STAC validation run (`38423502726`) concluded `failure`; the available job-log retrieval did not provide diagnostic text, so the reason remains unresolved. Catalogue metadata discovery is not completed raster science. The next gate is to capture and inspect a source/output corpus, then complete a qualified site-data applicability review before any real dwelling consumes these layers.
