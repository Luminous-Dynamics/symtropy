# Protective Evidence E4 — Typed semantic projections and loss accounting

**Snapshot:** 16 November 2026  
**Owner:** PROTECT-EVIDENCE-SEMANTIC-PROJECTION-001 #5643  
**Branch:** `research/protect-evidence-semantic-projection-e4-v0-1`  
**Parent:** E3 multi-provider adapters #5598 / PR #5637  
**Status:** Rust tagged-union projection prototype, 41 unit tests written, Cargo/CI integration authored; exact-head hosted execution pending

## 1. Paradigm shift: a typed federation, not a universal flat schema

The E3 work established three very different evidence sources: DOI registration metadata, geospatial catalog records and job-run events. A common flat record that treats each as the same thing would erase meaning. The E4 change is to model a *federation of typed views* with a common assurance envelope, not to build another generic metadata schema.

The adapter returns a tagged union:
- `DataCite`: persistent resource identifier, creator/title/publisher/resource type, publication year, related identifiers, rights declarations and the native REST schema URI;
- `OgcRecords`: catalog record identifier, conformance classes, descriptive properties, geometry, spatial extent and record links;
- `OpenLineage`: observed run identity, job namespace/name, producer, RunEvent schema URL, start and terminal event times, inputs/outputs and native run facets.

These variants are intentionally not interchangeable. An OGC catalog ID is not a DOI. A RunEvent run ID is not a dataset ID. A rights statement is not an authenticated license. A catalog geometry is not the underlying geospatial payload.

## 2. Explicit mapping/loss taxonomy

Every consumed field mapping is classified:

| Class | Meaning | E4 rule |
|---|---|---|
| `EXACT_NATIVE` | Source value is represented in a typed field without changing its meaning | No unit, identity, authority or relation inference |
| `NORMALIZED_EQUIVALENT` | A documented transformation changes representation but preserves declared semantics | Versioned rule ID; source receipt and original bytes remain separate |
| `DERIVED` | Value is calculated from source observations | Explicit rule ID and must not masquerade as source-native data |
| `PRESERVED_RAW_ONLY` | Field remains in the provider-specific raw extension/native payload but is not flattened into the common mapped view | Preserve its JSON value and source bytes |
| `UNKNOWN_OR_UNSUPPORTED` | Adapter does not understand a provider field or facet | Preserve it, report its path and make no guess |

The presence of an unknown property is not a reason to drop or default it. E4 adds unrecognized DataCite attributes, OGC properties and OpenLineage facets to the typed source-specific preservation areas and emits an explicit unknown mapping decision.

### Explicit E4 transformation examples

- OGC bbox JSON numbers are represented as an array of `f64`; this is `NORMALIZED_EQUIVALENT` under `E4-OGC-NUMERIC-BBOX-001`. The CRS URI is carried as an exact string. No coordinate transformation or CRS inference occurs.
- The OpenLineage terminal state is `DERIVED` under `E4-OPENLINEAGE-TERMINAL-STATE-001`, because it is read from the observed event pair. It is not a forecast and is not the E1 model result.
- The DataCite generic `kernel-4` schema URI remains unchanged. The 4.7 minor-version field-profile target is a separate manifest fact.
- DataCite rights URI null remains null. It is not promoted into an assumed license.

## 3. Semantic round-trip vs byte identity

The projection envelope is serialized to JSON, deserialized into the tagged Rust types, and compared for structural equality. The report carries separate integrity and semantic identities:
- raw source byte length and SHA-256 per provider record;
- a separate E1 context receipt bound to the exact envelope bytes;
- a serialization SHA-256 for the Rust output representation;
- a semantic projection fingerprint using RFC 8785 JSON Canonicalization Scheme (JCS) and SHA-256;
- field mapping decisions and classification counts.

The semantic fingerprint includes provider identity/profile, rights/use boundaries, complete typed native views and mapping decisions. It deliberately excludes source file paths, raw source byte lengths and raw source SHA-256 values; those remain in the source receipts. Therefore harmless JSON whitespace or object-key-order changes can change a raw source digest without changing the semantic fingerprint. A change to a provider title, identity, rights statement, extension or mapping decision does change the semantic fingerprint.

E4 pins `serde_json_canonicalizer = 0.4.1` and checks published RFC 8785 number/string and UTF-16 key-order vectors. Object member names are canonicalized recursively; array order is preserved. The E4 JCS input profile rejects integer-valued JSON numbers outside the exact safe-integer range `[-(2^53-1), 2^53-1]`; larger exact identifiers/counters must be represented as strings. This is a deliberately conservative E4 profile restriction to avoid cross-language numeric ambiguity, not a claim that RFC 8785 itself defines this exact bound as its full number domain.

A strict recursive JSON parser rejects duplicate object member names before projection. This prevents a parser's duplicate-key resolution policy from silently deciding which provider value becomes evidence. Complete provider-native source objects are retained in each typed view, so unmapped top-level and nested fields remain reconstructable; unknown fields are recorded as `UNKNOWN_OR_UNSUPPORTED` rather than receiving an invented interpretation. Raw-byte identity and semantic equality are not interchangeable.

A projection round-trip pass means the E4 typed envelope survives its own serialization boundary. It does not mean that every field has been normalized into a common schema or that every provider API implementation is fully conformant.

## 4. E1 context boundary

The E4 envelope includes a separate E1 context receipt bound to the exact embedded E1 envelope bytes. The OpenLineage source facet must match the E1 analysis ID and its `NOT_EXECUTED` status, with null E1 run ID and `CONTEXT_ONLY` relationship.

The OpenLineage run remains the synthetic E3 adapter-conformance run. It must not be confused with, or used to imply, execution of the E1 model analysis.

## 5. Implementation and tests

Binary: `protect-evidence-semantic-projection-e4`

It constructs three tagged provider views, classifies the explicit mappings, retains unprojected provider-native values, creates source receipts, and verifies the typed projection serialization/deserialization round-trip.

**32 unit tests are written**, covering:
- positive provider tagged-union and report round-trip;
- DataCite DOI/rights/schema/related-identifier distinctions;
- DataCite raw metadata and unrecognized native attribute preservation;
- OGC record/geometry/extent/CRS/link semantics and extension retention;
- OpenLineage terminal-state derivation, canonical UTC timestamp ordering, matching start/terminal job+schema identity, and preservation of full native events/facets;
- E1 context receipt identity and E1 NOT_EXECUTED isolation;
- explicit normalized-equivalent and derived mapping rule IDs;
- semantic equality vs raw-byte digest change under whitespace/key-order rewriting;
- duplicate JSON object-member rejection, including nested objects;
- RFC 8785 published numeric/string and UTF-16 property-order vectors;
- JCS-safe integer bounds, array-order preservation and semantic fingerprint stability/change controls;
- stable serialization and source receipt hashes;
- report-level distinction between serialization digest and JCS semantic fingerprint.

Formatting, Clippy, compilation, tests and executable output must be evidenced on the exact branch head. A test count is not a pass.

## 6. Benchmarks and industry comparison

| Benchmark | Prospective target |
|---|---:|
| Typed projection serialize/deserialize equality | 100% on frozen corpus |
| Required native mapped-field preservation | 100% |
| Every consumed field has a mapping class | 100% |
| Baseline mapping ledger completeness | 52 explicit decisions, 0 unclassified fixture fields |
| Unknown/unmapped provider fields retained or explicitly reported | 100% |
| Silent default coercions | 0 |
| DOI/provider identity drift | 0 |
| CRS/unit/timezone semantic drift | 0 |
| Rights/license/claim-ceiling escalation | 0 |
| E1 NOT_EXECUTED represented as a completed E1 run | 0 |
| Semantic projection differs under a declared equivalent rewrite | 0 |
| RFC 8785 reference-vector failures | 0 |
| Duplicate JSON object members accepted | 0 |
| Semantic fingerprint changes under raw whitespace/object-key reorder | 0 |
| Semantic fingerprint fails to change on a meaning-changing mutation | 0 |
| False semantic equivalence accepted | 0 |
| Human mapping/review time, CPU and memory overhead | p50/p95 measured |

E4 does not aim to replace DataCite, OGC API - Records or OpenLineage. It aims to beat hand-written ETL and manual crosswalk processes on a measured provider corpus by reducing silent field loss, false inferences and reviewer effort.

A defensible SOTA comparison requires:
1. at least three independently maintained provider fixtures beyond the synthetic examples;
2. independent normative schema/validator implementations;
3. a frozen cross-provider semantic corpus with exact transformation/version manifests;
4. a matched manual-mapping baseline;
5. field-level false-equivalence and loss rates;
6. independent reproduction of p50/p95 review effort, runtime and memory.

No superiority claim follows from internal fixture round-trip tests alone.

## 7. Claim ceiling

All E3/E4 data remains static synthetic fixture data. The gate does not authenticate an external provider, register or resolve the DOI-shaped fixture, authorize a license, prove the OGC-referenced resource contents, establish an executed E1 model run, validate a scientific model or demonstrate physical resilience. The result remains `ABSTAIN_SYNTHETIC_METADATA_ONLY`.
