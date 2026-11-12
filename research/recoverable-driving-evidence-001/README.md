# RECOVERABLE-DRIVING-EVIDENCE-001

**Status:** finite-record Rust reference auditor; simulator integration and external evidence authentication are out of scope  
**Canonical issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4553  
**Benchmark protocol:** https://github.com/Luminous-Dynamics/symtropy/issues/4550

## Purpose

This standalone crate checks whether a supplied synthetic campaign record is internally consistent with the v0.1 recoverable-driving benchmark's branch, counterfactual, privilege and denominator rules.

It is intentionally dependency-free. It does not parse untrusted network payloads, run a simulator, verify cryptographic signatures, establish that an event is physically exogenous, evaluate SAM, train or select a policy, or certify driving safety. A passing report means only that the finite typed records satisfy this crate's structural checks.

## Validated invariants

- Expected and observed episode IDs and branch IDs reconcile exactly.
- Completed, failed, incomplete and censored episodes remain represented in the denominator.
- Branch IDs and per-branch actor/event identities are unique.
- Parent branches exist, remain within the same episode and share the declared decision-boundary identity.
- Branch parent links are acyclic.
- Replayed actor responses require identical actor-policy revision, policy-input digest, action digest and randomness digest to the parent record. If the actor-policy input changes, its response must be marked recomputed.
- Reused random keys must be explicitly shared with the parent and must belong to an intervention-invariant exogenous event. The random key and sampled-value digest must match the parent.
- Query input fields must be policy-visible and available when the query is issued. Query issuance must precede answer and outcome availability.
- During frozen evaluation, teacher blending, online updates, exploration noise, teacher input and future outcome access are rejected.
- Record counts are bounded; findings and summary order are deterministic.

The auditor can detect contradictory declarations. It cannot prove an upstream claim such as "this random variable is exogenous" or authenticate the source that made that claim.

## API

The public entry point is `validate_campaign(&CampaignRecord) -> ValidationReport`.

The caller constructs typed records with explicit expected episode/branch manifests, episode dispositions, branch parents, actor-policy response provenance, random-event pairing, policy/query information provenance and frozen-evaluation controls.

The report includes:
- valid/rejected disposition;
- expected and observed episode/branch counts;
- completed/failed/incomplete/censored counts;
- actor and random-event record counts;
- sorted, deduplicated findings.

The pure library entry point is `validate_campaign(&CampaignRecord) -> ValidationReport`. The PR also adds a bounded JSON CLI adapter that deserializes typed campaign records, invokes the same library validator and writes a machine-readable report. It is not a second implementation of the audit rules.

## CLI

Build on Rust 1.96.0:

```sh
cargo build --locked --manifest-path research/recoverable-driving-evidence-001/Cargo.toml --bin recoverable-driving-evidence
```

Run:

```sh
recoverable-driving-evidence campaign.json report.json
```

Exit codes:
- `0`: campaign structurally valid;
- `1`: campaign structurally rejected (a report is still written);
- `2`: invocation, file, size or typed-JSON error.

The input is bounded to 64 MiB. Unknown fields are rejected by the typed structs. The output contains valid/rejected disposition, denominator summary and deterministic finding names. Unkeyed digests and source declarations are not authenticated by this adapter.

## Run

From this directory:

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

The repository workflow runs these checks using Rust 1.96.0.

## Claim ceiling

Passing the unit tests establishes conformance of this implementation to its finite known-answer fixtures. A passing audit establishes only internal structural consistency of the exact supplied records. It does not establish that the simulator is correct, that exogenous/invariant labels are true, that counterfactual physics are valid, that a policy is competent, that SAM is correct, or that any vehicle is safe or roadworthy.
