# AERO Temporal Evidence Assurance v0.1

**Tracker:** [AERO-TEMPORAL-ASSURANCE-001 #7455](https://github.com/Luminous-Dynamics/symtropy/issues/7455)  
**Related:** #7342, #7352, #7395, #7396; PR #7383  
**Status:** research and benchmark specification; no trusted-time integration or cryptographic verification implemented  
**Claim ceiling:** synthetic historical replay and evidence-envelope semantics only  
**Owners:** Symtropy (aviation semantics and evaluation); Mycelix/AeroCommons (owner-native source, artifact and physical-unit evidence); Symthaea BASALT/ASSURE/CEB (freshness, lifecycle and claim support); competent laboratories and authorities (measurement validity and acceptance)

## 1. Purpose

The v0.2 evidence-binding evaluator distinguishes three timestamps:

- `effective_at`: when an event claims to apply;
- `recorded_at`: when the event claims to have entered the record;
- `evaluated_at`: the historical knowledge snapshot.

Observation acquisition time is a separate fourth time. This is a useful bitemporal foundation, but the timestamp fields remain declared metadata. A deterministic replay can still be historically wrong when a source clock is wrong, an event was backdated, an event is missing, a log was forked, or the trust policy used for the replay is not pinned.

This specification defines a bounded path from declared chronology to **verifiable historical evidence replay**, without creating another ledger, PKI, artifact store, provenance graph or qualification lifecycle.

The central design rule is:

> Preserve what was asserted, what was independently observed, what was cryptographically evidenced, and what policy was used to interpret it. Never collapse those into one timestamp or one trust bit.

## 2. Temporal semantics

### 2.1 Six distinct temporal facts

| Fact | Meaning | Evidence class |
|---|---|---|
| `acquired_at` | Claimed physical measurement time | Source assertion plus acquisition-clock evidence |
| `effective_at` | Time from which an event claims to apply | Issuer assertion |
| `issuer_recorded_at` | Source system's claimed commit/record time | Issuer assertion |
| `receiver_first_seen_at` | First durable receipt of exact event bytes by a consuming system | Receiver observation |
| `trusted_timestamp_at` | Time in an independently verifiable timestamp token over a canonical digest | External timestamp evidence |
| `evaluated_at` | Requested historical query cutoff | Evaluation input |

Preserve source-native timestamp text, time zone/offset, precision, clock identifier, synchronization state, uncertainty, original record ID and normalization procedure. A normalized UTC epoch value is a comparison field, not a substitute for the original source evidence.

`issuer_recorded_at` is not interchangeable with `receiver_first_seen_at`. A source may claim it recorded an event on Monday while the receiving evidence system first received it on Thursday. That difference is itself important evidence.

### 2.2 Two historical query modes

Every replay must label which question it answers.

**AS_KNOWN_AT(T)**

Reconstruct the decision from evidence durably received by snapshot T, using the trust-policy version and evidence-inclusion watermark pinned to that snapshot. Evidence first received after T must not influence the result.

**RECOMPUTED_WITH_CURRENT_TRUST(T)**

Re-evaluate the same historical acquisition/effective-time question using the current trust and revocation information, while retaining and linking the original AS_KNOWN_AT(T) report. The later evaluation is a new report, never an overwrite.

These modes answer different questions. For example, a signature that was trusted under the historical policy may later be discovered to have been issued by a compromised key. The system should preserve both the original decision context and the current reassessment.

### 2.3 Effective time versus knowledge time

For event E and evaluation snapshot T, the event may influence an as-of result only if:

1. the exact event bytes were received and admitted by the snapshot's evidence watermark;
2. the event's asserted effective time is no later than T;
3. the event's issuer, signature, authority and timestamp evidence satisfy the trust policy pinned to T;
4. the event is in the correct certificate, instrument, method and subject scope;
5. the event is not ambiguous because of gaps, forks, conflicting corrections or uncertain chronology.

A known-but-not-yet-effective event is `NOT_YET_EFFECTIVE`. An event that exists only in a later receipt is `NOT_KNOWN_AT_SNAPSHOT`. Missing evidence or uncertain source chronology is `TRUST_UNRESOLVED` or `ABSTAIN`, never an implicit pass.

### 2.4 Measurement-time uncertainty

A timestamp is not infinitely precise. Preserve an acquisition-time interval or estimate plus uncertainty. For calibration validity `[valid_from, valid_until)`, a conservative positive binding requires the entire credible acquisition interval to be contained in the validity interval.

If the acquisition uncertainty interval crosses either calibration boundary, report `TIME_BOUNDARY_AMBIGUOUS` and block/abstain. Do not round toward the favorable side, silently increase timestamp precision, or treat a millisecond-normalized field as a millisecond-accurate clock.

Clock quality must remain distinct from measurement uncertainty. The uncertainty budget for the measurand may include timing effects where physically relevant, but the clock uncertainty and the measurement-result uncertainty are not the same field.

## 3. Event evidence envelope

The benchmark evaluates an envelope that references owner-native event/artifact records. It does not require every field to be implemented in Symtropy or stored in a duplicate system.

| Field | Purpose | Required disposition |
|---|---|---|
| Source namespace + immutable event ID | Prevent ambiguous cross-source identity | Missing/duplicate/conflicting identity blocks |
| Subject/resource and scope IDs | Bind event to exact certificate, instrument, method and physical evidence | Cross-scope event blocks |
| Event type + schema/profile version | Define semantics under a pinned contract | Unknown required type/profile abstains |
| Canonical payload digest | Bind exact event content | Digest mismatch rejects |
| Issuer identity + signature reference | Bind assertion to an accountable source | Missing or invalid evidence blocks |
| Issuer authority scope | Establish what the issuer was permitted to assert | Unknown/expired/out-of-scope authority blocks |
| Issuer asserted effective/recorded times | Preserve source claims | Never trusted merely because signed |
| Receiver first-seen time + receipt reference | Independently bound first observed receipt | Missing receipt is explicitly weaker |
| Trusted timestamp token/reference | Prove digest existed no later than timestamp under TSA policy | Verify token, imprint, policy, certificate and time |
| Source sequence / predecessor commitment | Detect gaps, forks and equivocation where supplied by the owner | Missing expected sequence/checkpoint remains unresolved |
| Correction / supersession target | Define append-only change semantics | Same-scope, acyclic, unique target required |
| Trust-policy and verifier version | Make replay reproducible | Must be pinned in report |

A timestamp token is evidence about the committed digest, not proof that the asserted event occurred at its stated effective time. A signature establishes attribution/integrity under a key policy; it does not alone establish the signer's authority over a calibration certificate or the truth of the underlying measurement.

### 3.1 Canonicalization and immutable source bytes

The event digest must be computed over a versioned canonical representation. The canonicalization algorithm, schema/profile version and digest algorithm are part of the verification record.

Retain:
- original source bytes and source-native representation;
- canonicalized event bytes or reproducible canonicalization inputs;
- canonicalization profile and implementation version;
- digest algorithm and digest;
- signature and timestamp-token references;
- receiver receipt and any owner-native inclusion proof.

Do not replace the source with a normalized form. Normalization and derived representations are new artifacts with parent references.

### 3.2 Trusted time evidence

RFC 3161 Time-Stamp Protocol is the first comparator for a timestamp authority. A valid token over the event digest can support the claim that the committed data existed no later than the token's asserted time, subject to its cryptographic and policy validation. It does **not** validate `effective_at`, event truth, physical acquisition, issuer authority or custody before the token.

RFC 4998 Evidence Record Syntax is a comparator for long-term integrity evidence and timestamp renewal. The benchmark should exercise evidence preservation when algorithms, certificates or timestamp credentials approach expiry or become unsuitable.

Do not operate a new timestamp authority as part of this work. Consume independently verifiable evidence from an appropriate external provider or existing organizational service.

### 3.3 Trust-policy snapshot

An evaluation report must pin at least:
- accepted trust roots and certificate/status evidence;
- issuer identity and authority scope policy;
- timestamp authority policy and token-validation result;
- signature, digest and canonicalization algorithms;
- revocation/compromise information known at the snapshot;
- evidence receipt/inclusion watermark;
- validator implementation/version and schema/profile version.

Retrospective key compromise or trust-root changes may change the current reassessment. They must not erase the original as-known report.

## 4. Correction and event-stream rules

The existing v0.2 same-certificate correction checks remain a baseline. The next profile should additionally test:

- correction of a prior correction or supersession;
- cancellation and reinstatement with explicit scope and authority;
- cycles and invalid correction chains;
- two events with the same ID but different payload digests;
- duplicate IDs in separate source namespaces;
- same event sequence/ID with conflicting predecessor commitments;
- source sequence gaps and a missing final event;
- forked histories and equivocation;
- event replay from a stale replica;
- incomplete receiver replication and explicit receipt watermarks;
- correction of the wrong instrument, method, calibration evidence record or certificate artifact;
- issuer authorization changes and signer-key compromise.

Correction is not a generic "latest row wins" rule. The evaluator must not infer an effective current state from array order, wall-clock order alone, or whichever correction was received last. When owner-native semantics do not specify a unique authority-qualified successor, preserve the conflict and abstain.

A signed hash chain can expose mutation or ordering changes within a committed history. It does not prove that the producer committed every event that should exist. Completeness requires independent source checkpoints, observer coverage or another qualified receipt mechanism. A producer's self-declared `complete` flag is not independent completeness evidence.

## 5. Threat model

The frozen challenge set should include, at minimum:

| Threat | Example | Required result |
|---|---|---|
| Backdating | Event claims Monday but arrives Thursday without earlier independent receipt | Trust unresolved / block |
| Clock rollback | Source clock moves backwards or is reset | Clock anomaly; no favorable chronology |
| Future-effective event | Known before T but effective after T | Not yet effective |
| Late-arriving event | First receipt after T | Excluded from AS_KNOWN_AT(T); included in later replay |
| Timestamp substitution | Valid token over a different digest | Reject |
| Canonicalization mismatch | Whitespace/order normalization changes signed bytes under an undeclared profile | Reject or unresolved |
| Wrong authority | Correct signature from an issuer without scope for this certificate/method | Block |
| Compromised key | Key compromise discovered after original snapshot | Preserve historical report; create current-trust reassessment |
| Sequence gap | Event 103 is received but 102 is absent | History incomplete / abstain |
| Fork/equivocation | Same predecessor has two conflicting successors | History unresolved |
| Truncated export | Plausible packet omits the final revocation | Completeness unresolved |
| Correction race | Two corrections compete for one target | Contradictory history / block |
| Time uncertainty | Acquisition interval intersects calibration expiry | Boundary ambiguous / block |
| Digest A→B→A | Content returns to a previous digest | Generations and event history remain monotonic |
| Long-term crypto expiry | Old timestamp evidence relies on an unsuitable algorithm/certificate | Renewal or preservation gap reported |

All challenges must retain positive, negative and unknown controls. An all-negative suite can be gamed by an evaluator that simply rejects everything.

## 6. Metamorphic and model-based invariants

1. **No future-knowledge bleed:** adding an event first received after T cannot change AS_KNOWN_AT(T).
2. **Effective-time correctness:** a known event whose effective time is after T cannot affect the state at T.
3. **Late-evidence preservation:** adding late evidence creates a new evaluation without mutating prior reports.
4. **Permutation invariance:** record order does not change the normalized result for the same event set, watermark and policy.
5. **Receipt monotonicity:** receiver watermarks cannot move backward; a new replica must not silently erase a previously observed event.
6. **No silent truncation:** sequence gaps, forks or missing checkpoints are unresolved.
7. **Digest integrity:** any mutation to signed/timestamped event content invalidates the corresponding commitment.
8. **Correction uniqueness:** every accepted correction chain is acyclic, same-scope and uniquely resolvable at the snapshot.
9. **Policy reproducibility:** same evidence watermark, trust snapshot and verifier version yield identical normalized reports.
10. **Clock conservatism:** uncertainty intervals overlapping a validity boundary never produce a positive binding.
11. **Claim separation:** evidence authenticity, event chronology, calibration binding, measurement validity and physical qualification remain distinct statuses.
12. **Owner non-duplication:** Symtropy consumes existing owner-native Mycelix/BASALT/ASSURE/CEB records and does not create another ledger or state machine.

Use property-based generation for event permutations, timestamp boundaries, correction DAGs and receipt-watermark sequences. Use model-based tests for state transitions and fork/gap detection. The expected oracle must be independently authored and reviewed rather than inferred from evaluator output.

## 7. Benchmark tournament

Freeze the same event corpus, policy inputs, hardware, data budget and expected dispositions for all arms:

1. **Declared-time baseline:** current v0.2 self-declared `effective_at`, `recorded_at` and evaluation snapshot semantics.
2. **Owner-signed envelope:** source-signed canonical event plus native source sequence/predecessor commitments and receiver receipt.
3. **Trusted-time arm:** owner-signed envelope plus independently verified RFC 3161 timestamp token.
4. **Long-term integrity arm:** trusted-time evidence plus RFC 4998-style archive evidence/renewal.
5. **Manual reconstruction:** qualified reviewer using source-native records, retained receipts and trust-policy history.

No arm is presumed to win. The benchmark compares assurance and operational cost separately.

### Metrics

| Dimension | Metric |
|---|---|
| Historical correctness | Exact agreement with independent AS_KNOWN_AT and current-trust oracle |
| False acceptance | Backdated, wrong-digest, wrong-authority, forked or incomplete history incorrectly accepted |
| Completeness | Sequence/checkpoint gaps detected; omitted events detected |
| Trust | Invalid signature, timestamp, authority and compromise cases correctly rejected/blocked |
| Time | Boundary ambiguity, clock drift/rollback detection and timestamp precision |
| Corrections | Unique valid-chain resolution versus contradiction/abstention |
| Replay | Byte-identical normalized results under equivalent ordering and pinned policy |
| Usability | Manual reconstruction time and reviewer disagreement |
| Performance | p50/p95 validation latency, memory and evidence payload overhead |
| Operations | Timestamp cost, renewal burden, receipt delay and recovery effort |

### Prospective software gates

- 100% agreement with an independently authored, independently reviewed oracle on the bounded corpus.
- Zero false acceptance of wrong-digest, wrong-authority, backdated, forked, incomplete or boundary-ambiguous histories.
- Zero future-knowledge bleed.
- Deterministic replay for fixed evidence and policy snapshots.
- Every incomplete, unknown, unsupported or ambiguous state remains explicit.
- All positive controls remain accepted at the declared binding-only claim ceiling.

These are research targets, not regulatory limits, physical qualification or certification criteria.

## 8. Standards and research comparators

- **NIST IR 8536 (September 2026):** Manufacturing Supply Chain Traceability Meta-Framework. Its cryptographically verifiable linked provenance, temporal ordering, interoperable payloads and selective-disclosure approach are the closest current manufacturing architecture comparator. Reuse its principles and existing owner infrastructure; do not implement a competing framework.
- **RFC 3161 / RFC 5816:** Internet X.509 Time-Stamp Protocol. Comparator for independent timestamp evidence over a data digest.
- **RFC 4998:** Evidence Record Syntax. Comparator for long-term evidence integrity and timestamp renewal.
- **W3C PROV-O:** reusable Entity, Activity, Agent and derivation vocabulary. Map only what is needed by the benchmark.
- **ISO/IEC 17025:2017:** current laboratory competence standard. This profile does not claim laboratory accreditation or replace the standard's requirements.
- **NIST metrological traceability policy:** a measurement result's traceability depends on an unbroken calibration chain with uncertainty contributions; calibrated equipment alone does not establish result traceability.

The architecture must distinguish five independent claims: artifact integrity, issuer attribution/authority, chronology/receipt, metrological traceability, and physical/engineering validity. Success in one is not evidence of the others.

## 9. Integration map and boundaries

- **Mycelix/AeroCommons:** exact physical-unit/build packet, source records, artifact digest/byte length and owner-native event/receipt references.
- **Symthaea BASALT:** resource generations, freshness, dependency and observer-coverage receipts.
- **Symthaea ASSURE:** append-only qualification lifecycle and supplied-view history.
- **Symthaea CEB:** engineering claim support, contradiction and claim closure.
- **Symthaea uncertainty provenance:** acquisition-time and measurement uncertainty, dependence and sampling metadata.
- **Symtropy:** aviation-specific event scope, calibration/configuration semantics, historical query modes, corpus and benchmark disposition.

AERO v0.1/v0.2 remains a synthetic software contract. The temporal assurance benchmark must not claim to authenticate actual lab records, establish physical custody, validate material allowables, demonstrate NDE probability of detection, qualify an aircraft component, authorize maintenance release or establish airworthiness.

## 10. Execution sequence

1. Independently review the existing 31 dispositions and the Rust event replay test.
2. Add future-effective, clock-uncertainty and conflicting-payload controls to the frozen temporal corpus.
3. Define an owner-native event receipt mapping with Mycelix and BASALT maintainers; do not add duplicate state.
4. Build generated correction histories and sequence/fork property tests around the existing evaluator.
5. Add a verifier adapter for supplied signature/timestamp evidence only after its source format and trust policy are pinned.
6. Validate against independently retained source receipts and a rights-cleared, non-flight-critical physical pilot under #7396.
7. Promote claims only after independent laboratory, metrology and engineering review.
