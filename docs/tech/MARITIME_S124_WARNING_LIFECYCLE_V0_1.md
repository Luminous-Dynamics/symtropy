# S-124 Navigational Warning Lifecycle Contract v0.1

**Status:** synthetic profile/lifecycle seed. It is not a native GML/XSD conformance result, warning-service completeness claim, ECDIS implementation, vessel receipt record or navigation authority.

## Pinned product context

The IHO Geospatial Information Registry identifies S-124 Navigational Warnings Edition 2.0.0 as its first operational edition, dated 28 March 2025, with an S-100 5.2.0 dependency. The registry identifies separately signed Edition 2.0.0 Feature and Portrayal Catalogues. The general IHO S-158 Edition 1.0.0 maturity statement remains implementation/testing only; a product-specific ZIP listing is not proof of an operationally approved validator.

References:
- IHO S-124 product record: https://registry.iho.int/productspec/view.do?category=product_ID&domainS=ALL&idx=218&product_ID=S-124&statusS=5
- IHO S-100 document register: https://registry.iho.int/document/list.do
- IHO S-98 ECDIS handling of valid/cancelled S-124 datasets: https://metanorma.github.io/iho-s-98/documents/2.0.0/main/document.html

Exact Edition 2.0.0 Product Specification, DCEG, GML schema, signed Feature Catalogue and signed Portrayal Catalogue still need to be acquired, byte-pinned and rights-reviewed before any native-profile claim. The profile metadata explicitly marks that lock as pending.

## Paradigm: warning state is a versioned event history

A navigational warning is not a static hazard polygon. Its identity is producer + message series + serial number, not serial number alone. Its effective state depends on issue/publication time, updates/reissues, cancellation, validity interval, and the current in-force bulletin.

Keep separate:
- event time, publication time, retrieval time and decision cutoff;
- warning validity and cancellation effective time;
- immutable warning-event history versus the derived current view;
- warning identity versus revision/supersession;
- bulletin identity, publication/retrieval, completeness and membership;
- geometry as supplied versus approximate/qualitative location text;
- source provenance and digest versus publisher authenticity;
- published warning versus evidence of delivery to a particular vessel/device.

A warning that is published does not prove a ship subscribed, received it, displayed it, acknowledged it, or acted on it. Missing delivery evidence remains Unknown.

## v0.1 synthetic contract

The accompanying JSON Schema and corpus are a lifecycle contract seed, not a native S-124 XSD validator. The 17 cases cover:
- an active warning with no vessel receipt evidence;
- series-scoped serial identity and collision controls;
- cancellation and self-cancellation/expiry;
- future validity;
- stale or incomplete in-force bulletins;
- warning membership missing from a complete, current bulletin;
- duplicate and out-of-order event delivery;
- update/cancellation references that target the wrong message series;
- reappearance of a superseded revision;
- textual location qualifiers promoted to false geometric precision;
- a delivery claim without a source receipt artifact.

The bulletin is an independently versioned source object. Absence from a stale or incomplete bulletin is Unknown, not proof that no warning exists. A warning omitted from a complete and current bulletin cannot be promoted as in-force merely because a prior dataset remains cached.

## Benchmarks and promotion

Targets for the eventual independent evaluator:
- 100% agreement with a separately implemented finite-corpus oracle;
- zero cancelled, expired or superseded warnings represented as active;
- zero duplicate/out-of-order events corrupting current state; historical events retained;
- bulletin completeness and freshness checked independently;
- zero promotion from published data to vessel receipt/display/acknowledgement;
- exact source/catalogue revision and corpus digest binding;
- p50/p95/p99 propagation and convergence latency reported separately from source age.

Operational evaluation should add held-out message-series/provider families, real-world but rights-cleared warning samples, clock skew, delayed cancellation, repeated bulletin publication and interrupted delivery. Do not use random event-level splitting where warnings from the same series appear in both training and test sets.

## Claim ceiling

A passing v0.1 lifecycle evaluator would establish only the declared synthetic identity, event, validity, bulletin and claim-boundary semantics. It would not establish warning truth, global service completeness, on-board receipt, human awareness, route safety, ECDIS certification or permission to navigate.
