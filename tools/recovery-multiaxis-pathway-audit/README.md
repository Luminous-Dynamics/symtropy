# Recovery Multi-Axis Pathway Audit

Standalone Rust evaluator for the 864-case factorial matrix plus four targeted singleton-hazard controls (868 synthetic cases total). It recomputes scope, evidence sufficiency, hard-gate status, lead-time adequacy, disposition and lock-in warning for every synthetic case rather than trusting the labels stored in the fixture.

## Run

Run formatting and tests with Cargo using this crate manifest, then run the auditor with the corpus path and `--source-commit` bound to the exact Git commit. It emits a sorted JSON receipt and returns a nonzero exit code for malformed input, a contract mismatch, a changed expected label, aggregate-count mismatch or a violated claim ceiling.

## What it tests

- Known single and paired hazards, including a capacity-sensitive pair tested against habitat-only and route-capacity-only controls under normal/degraded capacity.
- Out-of-profile mixed family; changed criteria and a superficially reverted regime that has a different version ID.
- Complete/incomplete classification, fresh/stale/conflicting evidence, normal/degraded capacity.
- Remaining lead time vs reversible and irreversible action requirements.
- Non-compensatory PASS/BLOCK/UNKNOWN gates, scope precedence and explicit lock-in warning.
- Input-order invariance, duplicate identities, false labels, invalid lead time and the zero-actuation ceiling.
- A two-future temporal trace: shared observed prefix must have identical actions; future evidence may not be referenced before its available day. Mutation tests intentionally violate both rules.

The corpus is synthetic. Passing establishes only finite contract conformance, not field safety, predicted performance, legal compliance, or authorization to perform work.

See `docs/tech/MULTIAXIS_RECOVERY_PATHWAY_BENCHMARK_V0_1.md` and issue #1927.
