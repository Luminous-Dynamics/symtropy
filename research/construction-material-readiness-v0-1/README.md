# Construction Material Readiness Core — v0.1

**Status:** synthetic, read-only work-package material readiness evaluator. No stock, custody, approval or inspection mutation; no certification or safety authority.

Parent: [CONSTRUCTION-MATERIAL-READINESS-001 #5321](https://github.com/Luminous-Dynamics/symtropy/issues/5321).

Owner composition:
- Symtropy #3331: component/material requirements, process/quality/inspection qualification and as-built context.
- Mycelix #5149: lot identity, verified vs asserted provenance, chain-of-custody, authority receipts, correction, revocation, dispute and supersession.
- Mycelix #4901: inventory state, logistics lifecycle, dispatch, receipt, custody handoff and owner-issued reservation.
- Symtropy #4937/#4964: reference-floor work packages, kitting, material buffers and vertical logistics.
- Symtropy #4957/#5232: accepted-output and whole-floor economics.
- Symtropy #4974/#5233: spatial occupancy and workface resource constraints.

## V0.1 semantics

- Material requirement pins project, floor, work package, task, design and specification revisions, material class/grade, unit, quantity and approved substitutions.
- Source-owned inventory reservations must match the current site epoch as well as project, lot, work package, design, location and use window; prior-epoch reservations are Superseded.
- Lot projection composes owner-issued provenance and quality receipts, logistics/recipient acceptance and inventory reservation receipts.
- Each source receipt is checked for lot/project/revision scope and currentness at the assessment time.
- Dispatch/in-transit is not delivery; delivery is not recipient acceptance.
- Supplier assertion is not qualified evidence; revoked/rejected evidence blocks readiness.
- Available stock without a current reservation for the exact work package/location/use window is not ready.
- Substitutions require an exact material/grade match and an approval bound to the current design revision.
- Unit conversion is forbidden unless a future owner-qualified conversion profile is explicitly added.
- Kit allocation checks each required line and sums use of a lot across the full kit so the same reserved quantity cannot be counted twice.
- Assessment is a pure read-only projection. The adapter never mutates upstream owner records.

## Local commands

```sh
cargo fmt --manifest-path research/construction-material-readiness-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-material-readiness-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-material-readiness-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

## Claim ceiling

V0.1 qualifies only deterministic behavior over caller-supplied synthetic source projections. It does not verify signatures, certificates, physical composition, real stock quantity, custody truth, structural suitability, product conformity, statutory approval or site safety. If an upstream source cannot provide a versioned, exact-subject, current disposition, this projection remains HeldUnknown.
