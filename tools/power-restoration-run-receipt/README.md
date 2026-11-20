# Power restoration benchmark run receipt v0.1

**Owner:** Symtropy #6352  
**Stacked on:** planner-neutral case #6347 and action contract #6350  
**Programme:** Power Fabric #5241

## Purpose

A planner output is not a benchmark result. A benchmark result is a content-bound chain from frozen case and observations to planner proposal, independent validation, transition execution and independent scoring. This crate verifies the shape and cross-references of that chain, accounts for every recorded attempt and calculates a canonical SHA-256 over the receipt payload.

The CLI is read-only. It does not execute actions, recalculate a power flow, verify a service trajectory or cryptographically verify a collector signature.

## Receipt contents

- Frozen case reference/digest and paired scenario, replicate and seed.
- Planner, model/provider/revision, prompt/configuration, adapter and component identities.
- Observation and proposal digest for every decision epoch.
- Full ordered attempt ledger, including malformed, rejected, timed-out, failed, human override and abstention events.
- Independent validator receipts and transition-engine state-before/state-after receipts.
- Hardware/runtime profile, declared budgets and measured time/token/tool/solver/cost use.
- Full trajectory digest and independent scorer receipt with each metric kept in its native unit.
- Optional collector attestation. A verified status requires an external verification receipt reference and digest whose signed payload digest equals this receipt's canonical digest.

## Important semantics

- The declared attempt count must equal the number of event records, and event sequence numbers must be contiguous from zero. An explicitly incomplete log is never called a complete run.
- Every applied action requires both an accepted independent validator receipt and an applied transition receipt. Rejected actions require a validator rejection/inconclusive receipt and have no transition receipt.
- Every transition is bound to its decision step, proposal digest and before/after state digests.
- A completed/horizon-reached run without an independently accepted scoring receipt is not a scored benchmark result.
- Even a present/accepted scoring receipt remains unverified until the collector attestation is independently verified. This crate consumes that external verification receipt; it does not pretend to verify an asymmetric signature itself.
- Aborted/failed/cancelled runs remain receipts with their attempts. They must not disappear from the denominator.
- Metrics are not summed across units. kWh, minutes, m³, customer-hours and monetary costs remain distinct metric rows.

## Synthetic fixture

The bundled fixture is a structural adapter example, not a measured run. It contains five attempt events: two successful planner calls, one validator-rejected unsafe switch action, one applied action supported by separate validator and transition receipts, and one timed-out tool call. It includes an independent score receipt with weighted ENS (kWh), time to 90% restoration (minutes) and potable-water shortfall (m³).

The collector attestation is intentionally absent in the fixture. The CLI must therefore report a score with unverified collector provenance, never a fully independently attested run. The unit test constructs a synthetic verified-attestation case and checks the resulting status, but this is only a conformance test of reference handling—not cryptographic security validation.

## Source-level tests

15 Rust unit tests are authored for complete attempt accounting, applied-action receipt chaining, validator rejection, digest tampering, collector-digest mismatch, count mismatch, incomplete logs, aborted-run retention, score acceptance, explicit maturity, native-unit separation, duplicate metrics, deterministic replay and unknown-field rejection.

The authoring environment does not include rustc/Cargo. Tests are authored but not locally executed. Exact-head CI must run format, Clippy, unit tests and CLI replay. No runner pass is claimed here.

## Trust boundary

SHA-256 proves content identity, not issuer identity. The collector attestation contains a trusted external verification receipt reference and digest; downstream evidence systems must verify that receipt against their key/identity registry. The report says independently scored run receipt only when the receipt digest, external collector attestation and scorer acceptance are all present and consistent.

## Claim boundary

Benchmark run accounting and content-reference verification only. No physical, operational, safety, service, field or SOTA qualification is inferred from the receipt alone.
