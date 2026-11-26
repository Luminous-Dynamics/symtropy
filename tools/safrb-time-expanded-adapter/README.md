# SAFRB time-expanded single-commodity adapter v0

This crate maps only the frozen synthetic SAFRB v0.1 profile into the shared flow kernel.

It validates exact node/edge identities, one-step time chronology, integer lower/upper capacity bounds, one quantity unit, opening-stock reconciliation, route aliases to shared physical-resource capacity gates, matching per-window capacity evidence, and service obligations at beneficiary nodes.

It builds separate lower and upper graph instances. It reports (1) joint all-obligation feasibility, (2) each beneficiary window's individual maximum under lower and upper bounds, and (3) the allocation in one joint flow witness. Independent maxima are not additive when obligations share capacity and are never presented as a jointly feasible schedule. Isolated maxima include full primal/dual flow certificates with graph digests and min-cuts. To bound solver cost, they are computed only when the joint lower-bound problem is infeasible and there are at most 32 obligations; they are not computed when joint lower-bound service is already feasible, are explicitly omitted above that limit, and are skipped during counterfactual search. The v0.1 profile supports at most 128 obligations and one beneficiary obligation per time index. It also binary-searches a monotone minimum hypothetical capacity uplift and marks changed capacity evidence as hypothetical_counterfactual_not_admitted.

The generic kernel owns only graph mathematics. Mycelix owns identity, issuer recognition, legal access, evidence lineage, privacy and common-cause dependency closure. Symthaea owns decision-aware evidence acquisition. Sol-Atlas owns read-only projection. Power Fabric owns the beneficiary service profile. This adapter does not authenticate measurements, establish legal rights, convert units, qualify field capacity, estimate prices, authorize action, or prove actual beneficiary service.

A successful lower-envelope result means only that the declared synthetic graph can satisfy all listed obligations simultaneously. An individual per-window maximum is a separate diagnostic and does not prove joint feasibility or a beneficiary guarantee. A failed or malformed input returns an error; unsupported schema/model forms are not coerced into a single-commodity graph.

The source includes known-answer and adversarial tests. Rust formatting, compilation, Clippy and tests were not available in the authoring environment; they remain mandatory on the exact PR head.
