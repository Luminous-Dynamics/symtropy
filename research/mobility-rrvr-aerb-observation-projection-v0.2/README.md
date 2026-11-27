# AERB Candidate Evidence Snapshot v0.2

**Status:** candidate-side interchange proposal and synthetic fixture. Not a production evidence service or physical observation.

## Purpose

AERB v0.1 observations currently carry source/clock/quality metadata, an artifact digest and an observation-kind string, but not a typed result payload. A digest commits to bytes; it does not show the candidate what the endpoint, sensor or closure verifier reported.

This v0.2 snapshot is a candidate-visible projection of source-qualified evidence at one exact decision cut. It is separate from the candidate's policy submission and separate from evaluator-only world/support/loss artifacts.

## Contents

- `schemas/aerb_candidate_evidence_snapshot_v0_2.schema.json`: a strict candidate evidence envelope and typed payload union.
- `fixtures/candidate_evidence_snapshot_synthetic_v0_2.json`: three synthetic process-model observations:
  1. a lost command acknowledgement;
  2. a typed end-effector pose measurement with unit, frame, uncertainty and raw-data digest;
  3. an unresolved temporal-closure receipt, held because the command-holder inventory is incomplete.

All fixture sources are `model_oracle` in the `process_model` evidence domain. They are not physical sensors, independent adjudicators or real temporal-closure receipts.

## Typed payloads

The payload union composes the existing RRVR Evidence Capsules v0.1 evidence types where available:
- effect/fault observations with explicit coverage and calibration provenance;
- endpoint fence receipts;
- network-edge closure receipts;
- command-holder inventory;
- temporal-closure receipt projection bound to graph/profile/epoch/generation;
- typed physical measurements (pose, force, torque, contact, gripper/actuator state);
- command acknowledgements as transport facts separate from physical effects.

The payload is visible to the candidate; hidden truth IDs, evaluator outcome supports, priors/likelihoods, loss tables, seed and future records are forbidden. Raw artifacts are bound by digest. Larger artifacts can only be referenced by an opaque artifact ID for a host-mediated, read-only, size/type-bounded retrieval path; no arbitrary URI or filesystem path is exposed.

## Temporal closure is not NoEffect

A temporal-closure `closed` result concerns whether an old command can still execute through a declared and qualified command-holding graph. It does not establish whether the command acted before closure. Physical effect observations and coverage remain separate. An incomplete/unbounded/unqueryable path stays Hold/Unknown and retains the open-world residual.

Reuse:
- RRVR Evidence Capsules #7515 for evidence identity, source metadata, parentage, intervals and payload semantics;
- domain-qualified trace adapter #7565;
- temporal-closure owner #7433 / draft PR #7435;
- AERB policy / decision owners #7631, #7694 and #7728.

Do not create a second evidence store, source registry, trust store, clock authority, command graph, closure verifier or minimax engine.

## Candidate/evaluator boundary

The candidate process receives only this immutable evidence snapshot and the permitted probe catalogue. It does not receive evaluator-only truth/support/loss/likelihood data. The snapshot's schema/digest/profile identity does not authenticate the source or establish sensor calibration, independence or physical truth. Blind evaluation still needs separately permissioned processes/environments and independent evidence verification.

## Local checks

The focused workflow parses JSON, checks the decision cut, unique IDs, parent ordering, evidence-domain/source-role mapping, payload types, unresolved temporal-closure handling and absence of evaluator-only fields. These are schema/fixture structural checks only; a full JSON Schema implementation, Rust adapter and independent evaluator remain follow-on work under #7728.

## Claim ceiling

Synthetic process-model interchange only. No simulator/HIL/physical qualification, calibrated sensor claim, physical NoEffect claim, safety certification, benchmark improvement or SOTA result is claimed.


## Digest and attestation boundary

The schema exposes `evidence_profile_sha256`, `adapter_revision_sha256`, each capsule's `content_sha256`, and raw artifact digests. These fields bind the snapshot to explicit producer/profile identities; they do not authenticate a source or prove a sensor result.

The current Evidence Capsules v0.1 digest is deterministic for its pinned Rust/Serde representation but is not a language-neutral canonicalization contract. The new snapshot includes floating-point measurement values, so polyglot producers/verifiers must not assume that a matching decimal JSON rendering implies matching canonical bytes.

Reuse #7563 for the cross-language digest/attestation decision. Before independent language implementations exchange v0.2 snapshots, select and version either a published canonicalization profile (e.g. RFC 8785 with duplicate-key rejection and known-answer vectors) or exact immutable raw-byte hashing. Keep content integrity, source authentication, independence, calibration and physical truth separate.
