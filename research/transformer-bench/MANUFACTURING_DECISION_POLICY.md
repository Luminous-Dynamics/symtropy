# Manufacturing decision-policy contract

**Status:** synthetic policy validator and review contract. Utility values in the fixture are illustrative only, not a recommended transformer acceptance policy.

The action×outcome table binds a product/test scope to a single policy ID/revision, decision case, utility unit, source-artifact digest, approval-artifact digest, effective interval and safety-policy revision. A table is valid only when the complete matrix is present, action/outcome keys are unique, metadata and units are consistent, and exactly one action is explicitly marked as release.

For each outcome marked `release_prohibited=true`, the utility of the release action must be strictly below at least one non-release action. This is a structural guard against a decision model that rewards release on an explicitly prohibited state; it is not a safety case or proof that the remaining utilities are appropriate.

Run:

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin decision-policy-validate -- \
  research/transformer-bench/fixtures/manufacturing-decision-policy-synthetic.csv \
  2026-10-30T00:00:00Z /tmp/decision-policy-report.csv
```

The as-of timestamp is explicit and canonical UTC. A policy outside its validity interval is reported as BLOCKED; the validator does not silently reuse it.

SHA-256 fields are syntax-checked only. The validator does not authenticate source or approval artifact bytes or signatures. Use the evidence receipt workflow to verify exact artifact bytes and the organization's controlled approval process to establish authority.

Symtropy owns this decision-policy contract. Symthaea consumes the utility matrix only for expected-value-of-sample-information ranking after all independent eligibility/conformity gates have passed. Missing, stale, incomplete or unsupported policy evidence must cause abstention. EVSI cannot change acceptance limits or authorize shipment.

Tracked by [PWR-MFG-DEC-001 #2993](https://github.com/Luminous-Dynamics/symtropy/issues/2993) and [Symthaea PWR-MFG-EVSI-001 #9232](https://github.com/Luminous-Dynamics/symthaea/issues/9232).
