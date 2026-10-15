# Maritime Data Receipt v0.1

**Purpose:** preserve exactly which input bytes a future maritime scenario consumed, alongside the source revision, retrieval details, units, datums, quality state, uncertainty declaration, freshness policy and terms review.

**Status:** provenance and byte-integrity only. The sample is wholly synthetic; no live NOAA/USACE payload is claimed to have been retrieved or represented here.

## Primary-source research and ingestion implications

### NOAA CO-OPS water levels and currents

The [CO-OPS Data Retrieval API documentation](https://api.tidesandcurrents.noaa.gov/api/uat/) lists product-specific parameters and limits. It states that datum is mandatory for water-level products (except air gap), describes `water_level` as preliminary or verified 6-minute data depending on availability, and `hourly_height` as verified hourly data. Verified water-level data are processed monthly; availability timing varies by station. Retrieval products and ranges have their own length limits.

Receipt implications:
- carry the exact product, station identity, request URI, retrieval UTC timestamp, response status/content type, output byte length and SHA-256 of the exact saved response body;
- require a vertical datum for `water_level` and preserve the source's preliminary / verified state;
- never convert preliminary to verified because the request succeeded or because the payload parsed;
- separate data timestamp and retrieval time; implement a product- and use-specific freshness policy before any scenario decision;
- preserve raw bytes, including source flags and original formatting. Any normalized derivative must be a distinct artifact with its own digest and transform record.

### NOAA NCEI hydrographic data / BAG

[NCEI NOS Hydrographic Survey Data and Products](https://www.ngdc.noaa.gov/mgg/bathymetry/noshdb/) describes Bathymetric Attributed Grid (BAG) files containing depth grids, position and uncertainty grids, and source metadata. The [NCEI Bathymetric Data Viewer](https://www.ncei.noaa.gov/maps/bathymetry/) explicitly warns that its displayed data are for planning/modeling and are not suitable for navigation; it directs users to official NOAA nautical charts for navigation.

Receipt implications:
- preserve the original BAG/download, survey identifiers/dates, CRS, depth units, uncertainty grid and source metadata;
- do not infer under-keel clearance from an unaudited raster or confuse planning data with an official navigation product;
- do not publish a sensitive facility detail just because its input was obtainable publicly.

### USACE port-facility discovery

The [USACE Navigation and Civil Works Decision Support Library](https://ndclibrary.sec.usace.army.mil/) currently lists a September 2026 Navigation Facilities publication (document identifier 09022026) describing nearly 12,000 ports, waterway facilities, docks, anchorages and other points of interest. The record describes attributes including berthing-space length, depth, deck elevation, access and facility identity. The same library marks a previous August 2026 publication as superseded, demonstrating why mutable `latest` is not a stable dataset revision.

Receipt implications:
- pin the exact published file/document identifier and retrieval date rather than recording only a library landing page or `latest`;
- retain source facility IDs and record status, publication age and all relevant units;
- a public catalog is a discovery lead, not proof that a value is current, correct for a specific design, or licensed for arbitrary redistribution.

## Receipt fields

The normative structure is [`MARITIME_DATA_RECEIPT_V0_1.schema.json`](./MARITIME_DATA_RECEIPT_V0_1.schema.json). The Rust checker additionally enforces source-kind-specific constraints and hashes the exact local artifact bytes without network access or normalization.

- **Identity:** schema ID/version, receipt ID, publisher, product, dataset ID, pinned dataset revision.
- **Retrieval provenance:** local-fixture / HTTP GET / manual download, request URI, UTC retrieval timestamp, status and reported content type.
- **Artifact identity:** repository-relative raw file path, media type, exact byte length, hash algorithm and SHA-256.
- **Meaning and quality:** data domain, CRS, vertical datum when relevant, versioned unit vocabulary, time coverage, observation/survey time, source verification state, uncertainty statement.
- **Freshness:** a named policy state and a required policy description. This schema records the policy; it does not calculate whether a datum is fresh enough for a particular decision.
- **Terms:** licence name/URI, review status, reviewer, review time, and hash of the exact terms text reviewed. This is a review record, not an automatic legal determination.
- **Claim ceiling:** explicit prohibition on treating byte-integrity verification as physical truth, engineering qualification, navigation clearance, or approval to operate.

## Fail-closed rules in v0.1

The Rust verifier rejects:
- duplicate JSON keys, unknown or missing fields, unsupported schema versions, invalid enum values and trailing JSON;
- mutable/ambiguous dataset revisions (`latest`, `unknown`, `unspecified`);
- source/artifact hashes or byte lengths that do not match the exact local bytes;
- absolute, traversal, drive-prefixed or backslash paths supplied by the receipt;
- unrecognized units or a unit outside the frozen vocabulary for the declared domain;
- water-level records lacking an explicit vertical datum and hydrography records lacking a CRS;
- public-dataset records without HTTPS source/request URIs, retrieval time and a recorded reviewed-permitted terms decision, reviewer/time and terms digest;
- public-dataset records marked synthetic/unknown or water-level-prediction records not labelled as predictions;
- any receipt that claims navigation authorization or authority to assert physical truth.

A SHA-256 match proves only that the artifact bytes equal the digest recorded in the receipt. It does not prove that the dataset publisher is authentic, that the data are accurate/current, that the licence review is legally correct, or that an analysis is safe. A receipt verifier always reports **NotAdmitted**.

## Synthetic sample

`crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-payload.json` contains a made-up 1.23 m value with a synthetic timestamp. The receipt binds its exact 293-byte file digest. It is a parser and provenance test only—not an actual NOAA reading.

## Qualification boundary

Before any public dataset is used in a real scenario:
1. retrieve and preserve an exact versioned payload and source metadata;
2. record licensing/terms review and hash the exact terms version considered;
3. verify schema, source state, units, CRS/datum and provenance separately from content quality;
4. define and test a product-specific freshness rule against the intended analysis time;
5. treat unknown/stale/unverified values as blocked or indeterminate;
6. keep engineering model admission, independent physical validation and operational authority as separate gates.

The data-receipt tool is intentionally offline and is not a simulation runner. Publicly available planning data, authenticated provenance, deterministic replay and byte-level integrity do not constitute physical observation, navigational authority, seaworthiness, engineering certification or approval for physical operation.
