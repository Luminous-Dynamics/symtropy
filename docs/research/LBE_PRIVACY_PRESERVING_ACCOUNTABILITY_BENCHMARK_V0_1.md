# LBE-BENCH-005: Privacy-Preserving Evidence and Verifiable Accountability v0.1

**Status:** research and benchmark protocol  
**Tracking:** [issue #3135](https://github.com/Luminous-Dynamics/symtropy/issues/3135)  
**Related:** BENCH-003 #3110, BENCH-004 #3123, position lifecycle #3125, kernel PR #3091  
**Principle:** make accountability verifiable without making participants universally observable.

## 1. Research question

Can an institution provide independently verifiable evidence that a decision, report or correction exists and has not been silently rewritten, while disclosing only the information required to evaluate that claim?

A transparent institution is not one in which every person's statement, identity, movement and activity are public. Integrity, authenticity, availability and confidentiality are distinct properties and must be tested separately.

## 2. Two-plane evidence architecture

### Plane A — minimally disclosed accountability stream

The accountability plane contains only what is needed to establish event order and integrity:
- case/record-scoped opaque identifiers;
- monotonic per-record sequence;
- event type and state-transition class;
- predecessor commitment and current payload commitment;
- signed receipt or inclusion proof;
- optional coarse time window only where the use case requires it.

It does not contain raw names, email addresses, phone numbers, global DIDs, testimony, private claim text, exact locations or unnecessary precise timestamps.

### Plane B — private evidence and identity vault

The private plane contains:
- encrypted position text, rationale and evidence artifacts;
- the mapping between an authenticated principal and a case-scoped actor reference;
- scoped authorization and delegation grants;
- access history, retention and key lifecycle;
- selected disclosure packages for affected participants and independently authorized reviewers.

A disclosure package should bind the disclosed object to its context and receipt. It should prove the necessary event's inclusion/integrity without revealing unrelated evidence, participants or cases.

## 3. Identifier and commitment design

### Record-scoped actor references

The host adapter authenticates the principal and derives a record-scoped reference, for example from a keyed derivation over the principal and record identifier. Do not use a global stable identifier in the public accountability stream by default. Authorization must be evaluated on the authenticated principal, not on an untrusted actor reference supplied by the client.

The kernel's current actor_id field is a host-asserted identifier and has no cryptographic assurance by itself. The host should pass a scoped pseudonymous actor reference; it must retain the private identity mapping only where required for source-authentication, appeal or legal obligations.

### Payload commitments

A plain unkeyed hash of predictable position text, low-entropy reason strings or a short evidence label can be dictionary-attacked. Public commitments must use a construction suited to the threat model (for example, a salted commitment or keyed commitment with the secret held by the host), and the canonical serialization must be fixed and versioned.

The event commitment should bind:
- schema/version;
- record scope;
- sequence;
- event kind;
- prior receipt/commitment;
- canonical encrypted or private-payload reference;
- relevant authorization context.

Do not invent cryptographic guarantees by concatenating strings and hashing them informally. Use reviewed, versioned cryptographic primitives and canonical serialization.

## 4. Verifiable log model

The public verifier should be able to check:
- a signed receipt for an event;
- event inclusion in a log/checkpoint;
- consistency between successive checkpoints;
- correct sequence and predecessor linkage;
- the host's claim about the event's scope and commitment.

It should not need access to the full private case payload.

The design should compare Merkle inclusion/consistency proofs and signed checkpoint approaches with ordinary centralized audit logging. A host-held in-memory vector is not an independently verifiable transparency log.

## 5. Privacy and security threat model

Model at least:
- public observer correlating event types, times and rare scenarios;
- honest-but-curious operator;
- privileged insider with event/index access;
- colluding analysts combining several disclosures;
- malicious participant attempting to infer another participant's source identity;
- compromised storage provider attempting truncation, rollback, substitution or selective disclosure;
- unauthorized, conflicted or expired reviewer;
- coercive authority demanding excessive disclosure;
- stale caches or old URLs after withdrawal;
- compromised key-management or identity-provider boundary.

State assumptions explicitly. An attacker with access to the source identity provider or participant endpoint may be able to re-identify a participant. Do not promise absolute anonymity.

## 6. Lifecycle interaction

Position corrections, withdrawals and reinstatements are source-authenticated events. Their public lifecycle markers and receipts may remain while sensitive evidence payload access changes according to retention policy, key revocation and applicable obligations.

Do not publish the withdrawn text in a public immutable log and then claim it has been erased. Define which data is:
- public and intentionally permanent;
- private and encrypted;
- disclosed to a specific authorized party;
- retained for audit/legal reasons;
- eligible for restricted access or cryptographic erasure after review.

Correction and withdrawal must not silently delete prior decision history or permit administrator-authored withdrawals. Key destruction is not a universal substitute for a retention policy or independently reviewed legal process.

## 7. Adversarial scenario suite

| ID | Attack or failure | Required observation |
|---|---|---|
| PRIV-01 | Dictionary attack on a public commitment to low-entropy text | Private text is not recoverable from an unkeyed predictable digest |
| PRIV-02 | Stable pseudonym correlates the same person across multiple records | Default case-scoped aliases prevent trivial cross-case matching |
| PRIV-03 | Exact timestamp plus rare hazard type identifies a small group | Metadata disclosure is minimized/coarsened; residual leakage is quantified |
| PRIV-04 | Colluding reviewers combine separate disclosures | Disclosure policies prevent unrelated-case inference where feasible |
| PRIV-05 | Withdrawn text remains available via stale cache/URL | Access revocation and cache/retention behavior are tested |
| PRIV-06 | Ciphertext is moved across records or fields | Context binding/authenticated decryption rejects substitution |
| PRIV-07 | Operator hides an event while maintaining a local log view | Independent checkpoint/inclusion verification exposes inconsistency |
| PRIV-08 | Unauthorized or conflicted reviewer requests a private record | Deny-by-default authorization and purpose checks reject access |
| PRIV-09 | Log is truncated or forked after leadership turnover | Independently anchored checkpoints expose rollback/fork |
| PRIV-10 | Emergency disclosure grant outlives its authority | Expiry/revocation is enforced and audited |

## 8. Metrics and acceptance gates

Report dimensions independently:
- public direct-identifier leakage: **0**;
- unauthorized disclosure accepted: **0**;
- modeled invalid receipt, truncation and fork cases accepted: **0**;
- valid inclusion/consistency verification success: **100%**;
- cross-case linkability and re-identification attack success under declared attacker knowledge;
- inference accuracy from public timing/type metadata;
- sensitive-field exposure in logs, search indexes, backups and analytics;
- selective-disclosure correctness;
- p50/p95 encryption, decryption and proof-verification latency;
- storage, key-management, operational and accessibility overhead;
- appeal/remedy latency under privacy restrictions.

Do not collapse these into a single privacy score. Rights and unauthorized disclosure are hard gates. Performance and utility are measured separately.

## 9. Baselines

Compare:
1. plaintext append-only event log;
2. centralized encrypted database with ordinary audit logs;
3. encrypted log with stable global pseudonyms;
4. encrypted private evidence with record-scoped actor references;
5. selective disclosure with cryptographically verifiable event commitments/checkpoints.

Use the same scenario, participants, role structure and information exposure wherever possible. Publish attacker assumptions and test artifacts.

## 10. Research comparison

- **NIST SP 1800-29** discusses how aggregated security-log metadata can reveal user activity beyond what is obvious from individual records, and recommends minimization, pseudonymization, disassociation and access control.
- **NIST SP 800-226** gives guidance for evaluating differential-privacy guarantees and warns that privacy claims must be evaluated against the actual implementation and threat model.
- A **September 2026 study** examines field-level selective encryption in structured logs under data-at-rest/index-leakage assumptions and measures security, utility and performance trade-offs.
- Existing transparency-log systems show that append-only integrity and inclusion/consistency proofs are established building blocks. LBE should reuse reviewed primitives rather than claim that append-only logging is new.

The proposed contribution is the application of privacy-preserving, independently verifiable evidence to civic/economic processes involving dissent, care, dependency, essential entitlements and source-authenticated corrections/withdrawals.

## 11. Claim ceiling

This protocol does not prove that the LBE implementation is anonymous, private, tamper-proof or superior. It defines an adversarial evaluation programme. Any cryptographic design requires formal specification, independent implementation review, key-management analysis and external testing.

## 12. References

- NIST, [SP 1800-29: Data Confidentiality — Detect, Respond to, and Recover from Data Breaches](https://www.nccoe.nist.gov/publication/1800-29/), February 2024.
- NIST, [SP 800-226: Guidelines for Evaluating Differential Privacy Guarantees](https://csrc.nist.gov/pubs/sp/800/226/final), March 2025.
- [Selective encryption of sensitive fields in structured logs: Quantifying security–utility–performance tradeoffs](https://doi.org/10.1016/j.jisa.2026.104550), *Journal of Information Security and Applications*, September 2026.
- Google, [Trillian transparent logging design](https://github.com/google/trillian/blob/master/docs/TransparentLogging.md).
- BENCH-003: [issue #3110](https://github.com/Luminous-Dynamics/symtropy/issues/3110)
- BENCH-004: [issue #3123](https://github.com/Luminous-Dynamics/symtropy/issues/3123)
- Source-authenticated position lifecycle: [issue #3125](https://github.com/Luminous-Dynamics/symtropy/issues/3125)
- Kernel: [PR #3091](https://github.com/Luminous-Dynamics/symtropy/pull/3091)
