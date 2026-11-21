# Power restoration observation evidence v0.1

**Owner:** Symtropy #6397  
**Request contract:** #6369 / PR #6370  
**Case contract:** #6346 / PR #6347  
**FLARE comparator:** #6363  
**Programme:** #6320 / #5241

## Purpose

A sensing request is not an observation. This crate validates the data-only result envelope that connects each active-information request to its later outcome and content-addressed evidence artifacts.

It pins the frozen case and normalized action proposal, accounts for every information request, preserves request/completion/availability timing, evaluates freshness at the presentation step, and checks declared source ancestry and common-cause groups.

It does not execute sensors, call utilities, inspect a feeder, run power flow, grant authority, or validate service restoration.

## Core semantics

- Exactly one result record is required for every information-request action in the proposal. Pending, fulfilled, partial, denied, timed-out, unavailable and failed outcomes remain explicit.
- Pending requests cannot contain completion or evidence. Fulfilled/partial results require evidence artifacts. Non-result outcomes cannot carry evidence.
- Evidence cannot be presented before its availability step. Observation and availability timestamps must be monotonic and consistent with result completion.
- Freshness is evaluated at the declared presentation step against each artifact's maximum age. Stale, conflicting and unverified evidence stays in the report; none is silently dropped.
- Evidence records bind a SHA-256 content digest to a producer/source record. Producer attestation references and digests are required but **are not cryptographically verified** by this crate.
- Source lineage is a directed acyclic graph. Missing parent sources and cycles are rejected.
- Pairwise dependence checks detect shared source ancestry and shared common-cause groups. If no overlap is declared, the report says independence is unproven. It never infers independence merely from different channel names or absent lineage.

## Hashing

The case digest uses the case contract's canonical SHA-256. The proposal digest is calculated from normalized JSON: object keys are sorted by serde_json's default map representation and array order is preserved. This is a semantic JSON digest, not a hash of the original whitespace/byte encoding. The Python CI check uses sorted keys, compact separators and UTF-8.

## Report interpretation

A finding called `fresh_current_candidate` means only that the producer-declared quality is current and its declared maximum age has not elapsed. It is not sensor truth, authenticity, physical validity, or an authorization to act.

The report always marks producer attestation as `reference_present_not_cryptographically_verified`. A later trust-root/signature verifier must independently validate attestation bytes against approved issuer keys before an authenticated-source claim can be made.

Dependence findings are deliberately conservative:
- shared source ancestry: known dependence;
- shared common-cause group: known common cause;
- no declared overlap with complete declared lineage: independence still unproven;
- partial/unknown lineage: dependence cannot be excluded.

## Validation

The isolated Rust 2024 crate uses the case and action contracts by local path. Its unit suite covers request/result accounting, exact proposal/case binding, time ordering, freshness, outcome/evidence consistency, duplicate identities, lineage integrity, common-cause detection, and the prohibition on false independence/authenticity claims.

Run:

    cargo fmt --manifest-path tools/power-restoration-observation-evidence/Cargo.toml --all -- --check
    cargo clippy --manifest-path tools/power-restoration-observation-evidence/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/power-restoration-observation-evidence/Cargo.toml --all-targets

The fixture contains synthetic IDs, timestamps, digests and evidence. It is not a real sensor output or measured restoration result.

## Claim boundary

Synthetic structural, temporal and declared-lineage qualification only. No producer authenticity, sensor truth, statistical independence, physical diagnosis, electrical feasibility, switching authority or service restoration is established.
