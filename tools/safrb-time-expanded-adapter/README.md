# SAFRB time-expanded single-commodity adapter v0

This crate maps only the frozen synthetic SAFRB v0.1 profile into the shared flow kernel.

It validates exact node/edge identities, one-step time chronology, integer lower/upper capacity bounds, one quantity unit, opening-stock reconciliation, route aliases to shared physical-resource capacity gates, matching per-window capacity evidence, and service obligations at beneficiary nodes.

It builds separate lower and upper graph instances, returning both certificates and per-window delivery. It also enumerates a minimum hypothetical capacity uplift and marks the changed capacity evidence as hypothetical_counterfactual_not_admitted.

The generic kernel owns only graph mathematics. Mycelix owns identity, issuer recognition, legal access, evidence lineage, privacy and common-cause dependency closure. Symthaea owns decision-aware evidence acquisition. Sol-Atlas owns read-only projection. Power Fabric owns the beneficiary service profile. This adapter does not authenticate measurements, establish legal rights, convert units, qualify field capacity, estimate prices, authorize action, or prove actual beneficiary service.

A successful lower-envelope result means only that the declared synthetic graph meets its per-window floors. A failed or malformed input returns an error; unsupported schema/model forms are not coerced into a single-commodity graph.

The source includes known-answer and adversarial tests. Rust formatting, compilation, Clippy and tests were not available in the authoring environment; they remain mandatory on the exact PR head.
