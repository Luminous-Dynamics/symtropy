# Command Evidence Receipt v0.1

**Status:** implementation authored on a stacked branch; exact-head CI and compilation are pending. This is a deterministic SHA-256 commitment over the supplied audit inputs, not a signature, trusted timestamp, durable transparency log, or hardware attestation.

## Paradigm shift: from an audit verdict to a reproducible evidence object

The command-evidence auditor checks whether each expected request has a legal lifecycle. This layer creates a stable receipt that binds that verdict to the exact campaign and evidence inputs. A score without its campaign revision, scenario manifest, arm revision, platform/controller context, and trace identity is not independently reproducible.

The receipt is intended to travel with every scored trial. Campaign aggregation can then reject or quarantine records whose receipts are missing, malformed, incompatible with the evaluator version, or inconsistent with the registered scenario and policy digests.

## Receipt contents

`CommandEvidenceReceiptContext` binds:

- Campaign ID and campaign revision.
- Exact scenario-manifest digest and scenario ID.
- Benchmark arm and exact arm source revision.
- Policy, task-profile, plant/model, and native-controller configuration digests.
- Trace collector identity and evaluator/canonical-encoding version.

At construction time, the context's native-controller configuration digest must also equal the controller-configuration digest in every expected command identity; a mismatch prevents receipt creation.

The receipt contains the independent auditor summary and five commitments:

- `context_digest`: campaign/scenario/arm/evaluator identity.
- `expected_identity_digest`: expected commands sorted by request ID.
- `event_stream_digest`: complete evidence stream in its captured global order.
- `audit_summary_digest`: all lifecycle counts produced by the auditor.
- `receipt_digest`: root commitment over the schema version and four component digests.

All digests use SHA-256 with a versioned domain separator. Every variable-length string is length-prefixed; integers are encoded little-endian at fixed widths. The encoder feeds fields incrementally into the hasher instead of allocating a second serialized copy of the full event stream. Expected identities are sorted because their input slice order is not semantically meaningful. Events are **not** sorted: their original global order is part of the trace, even when per-request order is independently checked by the auditor.

No creation timestamp is included. Identical context, expected identities, and ordered events must yield a byte-identical receipt.

## API

- `create_command_evidence_receipt(context, expected, events)` first runs `audit_command_evidence`; invalid or incomplete traces cannot produce a receipt.
- `verify_command_evidence_receipt(receipt, context, expected, events)` reruns the audit and every digest computation, then requires complete receipt equality.
- Schema version is explicit. Unknown versions fail closed.

## What the digest does—and does not—prove

SHA-256 makes the commitment deterministic and detects a mismatch when the receipt is compared with an independently retained trusted copy. It does **not** authenticate who created the trace, protect a mutable receipt stored beside mutable logs, establish trusted time, or prove the event came from a motor drive or physical actuator. A hash is not a signature and not an append-only storage system.

Before production use, add:
1. Canonical semantic command and controller configuration serialization at the native boundary.
2. Authenticated event capture, with collector identity and key rotation/revocation rules.
3. Append-only storage or a transparency log with externally anchored checkpoints.
4. A hardware/application witness independent of the command producer.
5. Restart, rollback, truncation, replay, clock-domain substitution, and storage-corruption fault injection.
6. Independent reproduction of receipts from retained raw inputs.

Do not put receipt construction or hashing in the real-time motor-control loop. Generate receipts out-of-band from captured traces.

## Conformance tests authored

The tests cover deterministic receipts under expected-identity permutation, sensitivity to global event interleaving, binding to policy/context, recomputation during verification, unsupported schema rejection, empty context rejection, and refusal to issue receipts for stale-epoch evidence.

These are authored tests, not a measured pass until exact-head CI succeeds.

## Benchmark protocol

Freeze both a valid corpus and an adversarial mutation corpus before performance testing.

Integrity target:
- 100% detection for every seeded context, identity, epoch, deadline, event-order, truncation, event-duplication, missing-terminal, and audit-summary mutation.
- 0 false rejection of valid complete traces.
- Report each mutation class denominator, detection numerator, and false-positive numerator separately.
- A missing trace, missing receipt, unknown schema, or unverifiable event source is **incomplete/unknown**, never a success.

Scale target:
- 1,000 / 10,000 / 100,000 / 1,000,000 events per receipt.
- Report receipt creation and verification events/second, p50/p95/p99 latency, peak resident memory, bytes/event, and serialized receipt size.
- Report CPU and memory overhead separately from the robot control loop; the receipt path is offline and should contribute exactly zero blocking time to control updates by architecture.

For the end-to-end recovery study, preserve separate axes:
1. Task capability and recovery success.
2. Command authority integrity (stale/expired admissions or applications, unresolved lifecycle events).
3. Evidence completeness/authenticity.
4. Real-time behavior (deadline misses, p50/p95/p99 jitter and latency).
5. Resource cost (CPU, memory, energy).

Do not combine them into a single weighted score that allows task reward to compensate for an unsafe dispatch or missing evidence.

## Industry positioning

ROS 2 Control's controller manager provides lifecycle management, command limits, error handling, and fallback-controller behavior; its documentation warns that some lifecycle queries are not real-time safe. The receipt layer consumes evidence out-of-band and complements, rather than replaces, those mechanisms.

NIST's response-robot program develops repeatable elemental capability tests and encourages mission owners to set task-specific thresholds. This work adds auditable command/evidence integrity alongside those task-level tests; it is not an alternative to NIST/ASTM test methods or a functional-safety certification.

RoboDojo's reproducible sim-and-real evaluation, seed-controlled tasks, and capability dimensions are useful models for the task-level half of this program. Its manipulation success rates are not directly comparable to command-audit integrity rates.

## Acceptance gates

- Every valid fixture yields a receipt that is stable across repeated runs and host architectures.
- Expected-identity permutation does not change the receipt.
- Any event order, context, identity, or summary mutation changes the relevant digest or fails the audit.
- Receipt verification reproduces every field from original inputs.
- Unknown schema versions and invalid traces fail closed.
- Exact-head CI passes on Ubuntu, Windows, and macOS before validation is claimed.
- No claim of hardware authenticity, safety certification, measured recovery improvement, or SOTA superiority is made from this module alone.
