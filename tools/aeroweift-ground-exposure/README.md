# AeroWeft Ground Exposure Evidence Oracle

Synthetic-first Rust evaluator for exact, time-scoped ground-exposure evidence packages.

The evaluator binds population snapshots, impact-location distributions, consequence/severity models, spatial grid/CRS identity, operational/contingency/buffer zone geometry, cell coverage and bitemporal cutoffs. It aggregates supplied joint-exposure bounds and uses a conservative union bound for the probability that at least one person is impacted. It does not multiply independent averages or assume spatial cells are independent.

This crate does not ingest real population data, rasterize geometry, estimate impact distributions, authenticate source bytes, predict casualties, calculate regulatory GRC, set ground-risk buffers or authorize flight. All values in tests are fictional. Positive output is only `QUALIFIED_FOR_DECLARED_EXPOSURE_REVIEW` under the frozen synthetic profile.
