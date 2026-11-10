# S-129 Under Keel Clearance Management Lifecycle Contract v0.1

**Status:** synthetic product-lifecycle and applicability seed. It is not an S-129 native GML validator, UKC solver, navigational alarm, physical safety case or clearance to transit.

## Pinned product context

The IHO Geospatial Information Registry identifies S-129 Under Keel Clearance Management Edition 2.0.0 as the first operational edition, dated 27 December 2024, based on S-100 Edition 5.2.0. The registry identifies signed Feature and Portrayal Catalogues. The S-129 specification describes Pre-plan, Actual Plan and Actual Update datasets, and says dataset updates are made by whole-dataset replacement. Actual Updates may be needed every five to ten minutes, depending on conditions and use.

References:
- IHO S-129 product record: https://registry.iho.int/productspec/view.do?category=product_ID&domainS=ALL&idx=219&product_ID=S-129&searchValue=&statusS=5
- S-129 Edition 2.0.0 specification: https://metanorma.github.io/iho-s-129/documents/2.0.0/document.html
- IHO S-100 document register: https://registry.iho.int/document/list.do

The exact product specification, DCEG/application schema, GML XSD, signed Feature Catalogue, signed Portrayal Catalogue and product-specific S-158 package still require byte-pinned acquisition and rights/maturity review. The corpus does not claim S-158 validator approval.

## Paradigm: UKC is a vessel-route-time envelope

A single depth or clearance Boolean is not enough. Preserve the source plan's vessel and route identity, route revision, plan purpose, maximum draught, time windows, control points, non-navigable and almost-non-navigable areas, source-product lineage, geometry and vertical reference.

The three purposes are not interchangeable:
1. Pre-plan: indicative windows for voyage/logistics planning.
2. Actual Plan: detailed vessel- and waterway-specific passage plan.
3. Actual Update: updated Actual Plan that replaces the prior dataset as a whole.

An Actual Update is not a patch to merge into an older plan. Its replacement scope includes plan metadata and every feature collection. No old control point, almost-non-navigable area or non-navigable area may survive as current unless it is present in the replacement dataset itself.

A recent retrieval is not proof that the underlying source observations are fresh. Source time, generation time, retrieval time, validity and evaluation cutoff remain separate.

## v0.1 synthetic contract

The accompanying JSON Schema and 18-case corpus cover:
- valid Actual Plan, without implying physical UKC correctness;
- valid whole-dataset Actual Update with an explicit supersession reference;
- partial replacement and incomplete feature-collection replacement;
- stale feature retention after supersession;
- wrong vessel, route and route version;
- actual draught exceeding the declared plan limit;
- stale and expired plan data;
- Pre-plan presented as an Actual Plan;
- missing vertical datum and S-100 dependency mismatch;
- non-monotonic update sequence or missing supersession reference;
- dataset generated after the decision cutoff and out-of-order updates;
- wrong source product edition.

The 600-second freshness threshold in this synthetic fixture is a benchmark parameter, not a universal normative S-129 update interval. The published five-to-ten-minute Actual Update example is an input to cadence testing, not a general guarantee that every area/provider publishes at that rate.

## Benchmarks and promotion

Targets for the eventual independent evaluator:
- 100% agreement with an independently authored finite-corpus oracle;
- 100% full-dataset replacement on Actual Updates;
- zero stale features from superseded datasets;
- zero current applicability for wrong-vessel, wrong-route, wrong-route-version, expired or stale plans;
- stale detection latency and stale-current misrepresentation duration reported separately (target zero stale-current misrepresentation);
- 100/100 deterministic replays under permutations of fields that are semantically unordered;
- every derived geometry/route/vertical transform carries parent digests and transform revision.

Physical validation is a separate programme: reproduce rights-cleared reference cases, compare UKC/squat/trim models with calibrated measurements, and report false-safe, false-no-go and abstention rates by regime. Keep S-129 metadata conformance, physical-model validity and operational transit authorization as three distinct qualifications.

## Claim ceiling

A passing v0.1 lifecycle evaluator would establish only the synthetic identity, versioning, freshness, whole-replacement and source-applicability contract. It would not prove physical UKC, bathymetric correctness, vessel stability, model accuracy, prevailing waterway conditions, pilot/master/VTS acceptance, an alarm, navigational safety or permission to transit.
