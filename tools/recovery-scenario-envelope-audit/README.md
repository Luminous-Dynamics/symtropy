# Recovery Scenario Envelope Audit

Standalone, deterministic Rust auditor for the family-held-out applicability contract in Symtropy. This tool evaluates only the declared synthetic profile and evidence conditions; it does not predict hazards or authorize work.

## Run

Run the unit and mutation tests with Cargo against this crate's manifest. Run the evaluator with the default embedded corpus, or pass a corpus path. For an exact source-bound receipt, supply the full 40-character source commit using the source-commit option. The evaluator writes a JSON receipt to standard output and exits nonzero on a malformed corpus, contract violation, or expected-outcome mismatch.

## Contract

- Family coverage is exact-profile and version-bound. The taxonomy version must match, classification must be REVIEWED, and its reference must use the frozen synthetic review namespace. One unrecognized family or an incomplete/unknown classification forces scope reassessment.
- Profile/criteria regime mismatch is out of scope.
- Missing or stale required evidence holds an in-scope case.
- BLOCK is a non-compensatory gate; UNKNOWN never passes.
- Every disposition is review-only and actuation-disabled.
- Output order and canonical corpus digest are independent of case/profile row order and order of set-valued tag arrays.
- The frozen corpus is synthetic. An audit pass is finite contract conformance, not safety qualification, model accuracy, empirical coverage, or real-world performance.

See docs/tech/RECOVERY_SCENARIO_ENVELOPE_AUDIT_V0_1.md and issue #1913 for the full contract and benchmark protocol.
