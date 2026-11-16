# AeroWeft Model-Evidence Requalification Oracle

A dependency-free, synthetic-first Rust evaluator for aviation model-qualification claim impact under bitemporal change events.

It preserves the distinction between effective time and recorded/knowledge time, resolves exact dependency-digest transitions, and reports claim-specific reliance status under a frozen dependency profile and declared change-feed coverage.

It does not implement a generic event store, trusted clock, currentness service, EvidenceLease, cryptographic trust-root, physical model validation, or operational authority. It is a narrow benchmark consumer intended to compose Mycelix #6219/#1087/#2631/#1207/#181 and AeroWeft Track T.

Positive status is scoped to the synthetic dependency and coverage assumptions supplied to this evaluator. All IDs, digests, times and events in tests are fictional.
