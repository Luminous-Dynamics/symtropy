# AeroWeft Longitudinal Model Monitoring Adapter

Synthetic-first aviation adapter for interpreting already-produced sequential e-process receipts from Symtropy #3588. It does not implement or recompute an e-process, likelihood ratio, CUSUM, drift detector or change-point algorithm.

The adapter verifies scope binding, upstream receipt shape, alpha allocation, per-stream look sequencing, independent replication-unit counts, evidence coverage, and dual time-scale identity. It latches an anytime-valid boundary crossing once observed: a later e-value decrease cannot erase a prior valid crossing.

A no-signal status is emitted only when coverage is complete through both cutoffs, the domain and measurement/reference quality are acceptable, and the minimum independent-unit requirement is met. Repeated telemetry rows are not counted as independent flights/encounters.

All digests and owner evidence states are structural inputs. This crate does not authenticate receipt bytes, signer/issuer authority, validate a physical model, diagnose the cause of drift, authorize flight or issue operational commands.
