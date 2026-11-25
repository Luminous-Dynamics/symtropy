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

### 2.2 Three-axis replay coordinates

Every replay is keyed by three independent cutoffs, not one overloaded timestamp:

`T_subject`: the physical/effective-time question being reconstructed (for example, whether calibration was valid when an observation was acquired).

`T_evidence`: the receiver-observed evidence-inclusion cutoff, backed by an owner-native receipt/checkpoint. This determines which exact event bytes had entered the evidence view.

`T_trust`: the trust-knowledge cutoff and immutable trust-policy/status snapshot used to assess issuer authority, signatures, timestamp authorities and key compromise.

The replay coordinate is:

`ReplayQuery = (T_subject_interval, T_evidence, T_trust, trust_policy_id, trust_policy_digest, verifier_profile)`

The same `T_subject` may be replayed with different `T_evidence` and `T_trust`. A trust-policy identifier without an immutable policy/status snapshot is insufficient.

A replay report must persist this complete coordinate, not merely cite a human-readable policy name. A policy ID may be reused or its content changed; therefore the exact policy digest is mandatory. The verifier profile/version is separate from the policy digest. Production systems should additionally bind the concrete verifier binary/build digest in the owner-native execution receipt. The synthetic corpus includes a synthetic historical policy digest and a distinct synthetic current-policy digest to test that trust-only replay changes trust context without silently advancing the evidence watermark.


### 2.3 Three historical query modes

Every replay must label which question it answers.

**AS_KNOWN_AT(T)**

Use the original subject-time cutoff, original receiver evidence watermark and original trust snapshot. This is the historical decision view. Events received after the evidence watermark cannot influence it, even if their asserted effective time is retroactive.

**RECOMPUTED_WITH_CURRENT_TRUST(T)**

Keep the original subject-time cutoff and original evidence watermark, but reassess the admitted event bytes using current trust/revocation information. This answers: "Would we still trust the evidence we actually had then?" It does not add late-arriving events.

**RECOMPUTED_WITH_CURRENT_EVIDENCE_AND_TRUST(T)**

Keep the original subject-time question but advance both the evidence receipt cutoff and trust snapshot. This answers: "Given everything received and trusted now, what do we currently conclude about that historical subject time?" A later-received, retroactively effective revocation can change this new report without rewriting either earlier report.

Every report must retain its exact three-axis query coordinate and link to its predecessor/reassessment. New reports are append-only. No later query overwrites a historical decision.

The three views are intentionally non-equivalent. A signature trusted under the historical policy may later be found compromised; a revocation may also arrive late but claim an earlier effective time. Preserve the original decision, the trust-only reassessment and the full retrospective reassessment separately.

### 2.4 Effective time versus knowledge time

For event E and evaluation snapshot T, the event may influence an as-of result only if:

1. the exact event bytes were received and admitted by the snapshot's evidence watermark;
2. the event's asserted effective time is no later than T;
3. the event's issuer, signature, authority and timestamp evidence satisfy the trust policy pinned to T;
4. the event is in the correct certificate, instrument, method and subject scope;
5. the event is not ambiguous because of gaps, forks, conflicting corrections or uncertain chronology.

A known-but-not-yet-effective event is `NOT_YET_EFFECTIVE`. An event that exists only in a later receipt is `NOT_KNOWN_AT_SNAPSHOT`. Missing evidence or uncertain source chronology is `TRUST_UNRESOLVED` or `ABSTAIN`, never an implicit pass.

### 2.5 Acquisition time is not report/evaluation time

The event effective-time comparison must use the subject's physical acquisition-time interval, not the timestamp at which a later report is generated. `evaluated_at_unix_ms` is report/query metadata; it is not a substitute for the measurement's acquisition interval.

For an event whose effective time is (t_e) and acquisition-time interval is ([t_a^-,t_a^+]):

- `t_e <= t_a^-`: the event is effective for every acquisition time supported by the interval.
- `t_e > t_a^+`: the event is not yet effective for the acquisition.
- `t_a^- < t_e <= t_a^+`: chronology overlaps the acquisition uncertainty; report `TIME_BOUNDARY_AMBIGUOUS` and do not choose a favorable side.

A later report time cannot retroactively move the physical acquisition time. A late-arriving revocation can still change a new full retrospective report if its effective time precedes the acquisition interval; it cannot make a revocation effective before the time it actually claims to apply.

The benchmark includes both a revocation effective after acquisition but before report generation (must remain `NOT_YET_EFFECTIVE` for that observation) and a revocation whose effective time falls inside the acquisition uncertainty interval (must abstain). This is distinct from whether the event was known at the evidence cutoff.

### 2.6 Measurement-time uncertainty

A timestamp is not infinitely precise. Preserve an acquisition-time interval or estimate plus uncertainty. For calibration validity `[valid_from, valid_until)`, a conservative positive binding requires the entire credible acquisition interval to be contained in the validity interval.

If the acquisition uncertainty interval crosses either calibration boundary, report `TIME_BOUNDARY_AMBIGUOUS` and block/abstain. Do not round toward the favorable side, silently increase timestamp precision, or treat a millisecond-normalized field as a millisecond-accurate clock.

Clock quality must remain distinct from measurement uncertainty. The uncertainty budget for the measurand may include timing effects where physically relevant, but the clock uncertainty and the measurement-result uncertainty are not the same field.

## 3. Event evidence envelope

The benchmark evaluates an envelope that references owner-native event/artifact records. It does not require every field to be implemented in Symtropy or stored in a duplicate system.

| Field | Purpose | Required disposition |
|---|---|---|
| Source namespace + issuer identity + immutable event ID | Resolve an event under a trust-policy-bound source namespace; prevent aliases from evading duplicate detection | Missing/unverified/conflicting namespace-to-issuer binding blocks |
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

### 3.1 Evidence checkpoint integrity and observer coverage are separate

A receiver receipt timestamp and a valid checkpoint digest do not prove that the observer saw every source that could have influenced the historical decision. The replay contract therefore separates two owner-native attestations:

**Evidence-view checkpoint integrity**
- Exact checkpoint reference and digest.
- Receiver-inclusion watermark covered by the checkpoint.
- The checkpoint commits to the exact event/receipt view used by the replay.
- Digest mismatch or watermark mismatch rejects the view.
- This proves the committed view was not changed after checkpoint creation; it does not prove that the view contains every event that should exist.

**Observer-coverage qualification**
- Exact observer profile ID and qualification/receipt reference.
- Explicit covered source roots/channels, exclusions and unresolved channels in the owner-native BASALT receipt.
- Coverage status: qualified complete, incomplete, or unverified.
- Incomplete coverage causes abstention; unverified coverage cannot support a negative assertion such as "no revocation existed."
- A signed coverage receipt proves who made the coverage assertion, not that the underlying observation method is complete. The profile must be independently qualified.

The synthetic corpus uses `SYNTHETIC_VALID` and `SYNTHETIC_QUALIFIED_COMPLETE` as fixture labels only. It does not cryptographically verify a checkpoint or qualify an observer. Production replay must consume Mycelix/AeroCommons checkpoint evidence and BASALT's qualified coverage receipt, not create another ledger or observer inside Symtropy.

### 3.2 Receipt evidence is not a timestamp field

The `receiver_first_seen_at` value is not sufficient by itself. The event envelope must also carry a receiver-receipt disposition, an immutable receipt reference and a digest of the exact event bytes covered by that receipt.

For the synthetic corpus, `receiver_receipt_status=SYNTHETIC_VALID` is an oracle-controlled fixture label, not a cryptographic verification result. It requires a non-null receipt reference and `receiver_receipt_digest_sha256` exactly equal to `payload_sha256`. `UNVERIFIED` or `ABSENT` receipts cannot establish snapshot inclusion solely from the source's claimed `issuer_recorded_at` or `receiver_first_seen_at`. An invalid receipt/digest blocks or rejects according to the applicable challenge.

A production adapter must replace these synthetic labels with an actual owner-native receiver receipt, verifiable inclusion proof or independently retained source bytes and receipt record. It must bind the exact event digest, receiver identity, durable receipt time and applicable evidence-inclusion watermark. The benchmark does not invent another receipt ledger.

A trusted timestamp imprint is separately recorded as `trusted_timestamp_imprint_sha256`. A timestamp token whose imprint does not equal the canonical event digest is invalid timestamp evidence. A valid receiver receipt and a trusted timestamp token are different evidence mechanisms and must not be collapsed into one Boolean.

For a timestamp status declared `valid`, the synthetic contract additionally requires a non-null `trusted_timestamp_at_unix_ms` and an imprint exactly equal to the canonical payload digest. A status label without the matching imprint is rejected. This fixture rule tests internal consistency; it does not cryptographically verify an RFC 3161 token, TSA certificate chain, policy OID, nonce, accuracy or revocation status. Those require a real verifier adapter and independently governed trust policy.


### 3.3 Source-scoped event identity and idempotent delivery

An immutable event identity is the composite key `(source_namespace, event_id)`, not a globally unique display string. Two independent sources may legitimately use the same local event-ID text. They must remain distinct events and must not collide merely because the string matches.

For one composite key:
- repeated delivery of the same canonical payload digest is idempotent: apply the semantic event once, preserve every receipt/delivery reference, and report that duplicate delivery was deduplicated;
- a repeated key with a different canonical payload digest is equivocation/conflicting history: do not select the first or last payload; keep the identity ambiguous and abstain;
- a correction target must resolve using the same composite identity and exact certificate/instrument/method scope;
- a source namespace must itself be bound to a stable issuer/source identity and trust-policy scope; an untrusted caller cannot evade duplicate detection by inventing a new namespace.

This is not a permission to ignore replay. It distinguishes at-least-once transport delivery of the same immutable source event from two conflicting source assertions under the same event identity.

### 3.4 Canonicalization and immutable source bytes

The event digest must be computed over a versioned canonical representation. The canonicalization algorithm, schema/profile version and digest algorithm are part of the verification record.

Retain:
- original source bytes and source-native representation;
- canonicalized event bytes or reproducible canonicalization inputs;
- canonicalization profile and implementation version;
- digest algorithm and digest;
- signature and timestamp-token references;
- receiver receipt and any owner-native inclusion proof.

Do not replace the source with a normalized form. Normalization and derived representations are new artifacts with parent references.

### 3.5 Trusted time evidence

RFC 3161 Time-Stamp Protocol is the first comparator for a timestamp authority. A valid token over the event digest can support the claim that the committed data existed no later than the token's asserted time, subject to its cryptographic and policy validation. It does **not** validate `effective_at`, event truth, physical acquisition, issuer authority or custody before the token.

RFC 4998 Evidence Record Syntax is a comparator for long-term integrity evidence and timestamp renewal. The benchmark should exercise evidence preservation when algorithms, certificates or timestamp credentials approach expiry or become unsuitable.

Do not operate a new timestamp authority as part of this work. Consume independently verifiable evidence from an appropriate external provider or existing organizational service.

### 3.6 Trust-policy snapshot

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

A reinstatement event cannot silently restore calibration validity while the owner's lifecycle semantics and authority chain remain unqualified. The v0.1 reference model abstains on reinstatement events, and the corpus includes a revocation followed by an apparently valid reinstatement. Likewise, corrections must resolve to a prior event under the same source namespace and certificate scope; dangling and cross-certificate targets remain unresolved.

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

### 7.1 Historical replay scale and temporal-database comparison

The tournament must compare semantics and resource cost against existing temporal-data approaches rather than treating three query modes as a new database invention.

**Frozen baseline arms**
1. **Current-state/latest-row baseline:** intentionally unsafe negative control showing how backdating, corrections and revocation overwrite can lose historical decisions.
2. **Append-only event log with valid/event/transaction time:** conventional event-sourced temporal model, preserving source event time and system/receiver transaction time.
3. **Bitemporal relational baseline:** valid-time and transaction-time views with append-only corrections and a separate trust-status history.
4. **NIST IR 8536-aligned traceability links:** owner-native, cryptographically linked event references and selective disclosure; this is an interoperability comparator, not an implementation to clone.
5. **AERO three-cutoff replay:** acquisition interval, receiver evidence watermark and trust-policy/status snapshot with immutable report lineage.

Do not label a baseline “bitemporal” or “tritemporal” without documenting its actual supported semantics. The test must distinguish event time, valid time, transaction/receipt time, and trust-knowledge time. A trust-status history is not automatically covered by a database's bitemporal feature.

**Synthetic scale ladder (prospective):**
- 10^4 events: functional and indexing smoke.
- 10^5 events: correction DAG, duplicate, gap/fork and trust-history workloads.
- 10^6 events: historical query and retrospective-reassessment workload.
- 10^7 events: stress/retention workload if the available hardware and memory budget permit; do not make this a release blocker without a declared resource profile.

Generate fixed-seed histories with controlled event-time skew, late-arrival distributions, correction depth/fan-out, issuer count, trust-policy changes, duplicate deliveries, gaps, forks and checkpoint intervals. Freeze input manifests and expected answers independently of the implementations.

**Workloads and measures:**
- `AS_KNOWN_AT` query latency and exact historical result;
- `RECOMPUTED_WITH_CURRENT_TRUST` incremental cost while preserving the original receiver watermark;
- `RECOMPUTED_WITH_CURRENT_EVIDENCE_AND_TRUST` cost of incorporating late retroactive evidence;
- correction-chain and fork/gap detection latency;
- append/receipt ingest throughput and checkpoint creation;
- p50/p95/p99 query latency, CPU time, peak memory, index bytes/event, retained snapshot bytes/event and cold-start/recovery time;
- number of events scanned vs events causally relevant, incremental recomputation ratio, and cost of a full rebuild;
- concurrency tests in which evidence receipt and trust/revocation changes occur while historical queries are executing.

Report absolute numbers and environment details, not only speedup. Compare exact correctness first; a faster implementation that has future-knowledge bleed or loses a historical report is disqualified. For operational advantage, target at least 2x lower p95 latency or 30% lower retained-index/compute cost than the strongest semantically equivalent baseline at the same correctness and resource envelope; these are prospective stretch targets, not achieved results or universal performance claims.

This benchmark deliberately does not create a temporal database or ledger in Symtropy. It measures how the aviation evaluator composes existing owner-native event, receipt, trust and artifact systems.

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

### 8.1 Temporal-database prior art and claim boundary

The underlying idea of representing more than one time dimension is established prior art. A 1994 temporal-database paper, “Resolution of time concepts in temporal databases,” distinguishes event time, valid time and transaction time and explains why event time is needed alongside valid and transaction time to preserve retroactive/proactive updates, error corrections and delayed updates (DOI: https://doi.org/10.1016/0020-0255(94)90059-0). A May 2026 systematic review of bitemporal databases analyses 54 studies and identifies continuing gaps including limited industrial-scale empirical validation and underdeveloped integration with AI-driven predictive analytics: https://doi.org/10.1007/s42488-026-00162-x.

Therefore AERO does **not** claim to invent tritemporal databases, event sourcing, bitemporal queries or temporal provenance. Its narrower engineering contribution is a safety-oriented replay contract that composes:
- subject-time acquisition interval and uncertainty;
- issuer event/effective/recorded-time assertions;
- receiver-observed evidence inclusion and digest-bound receipt/checkpoint;
- independently versioned issuer-authority, revocation and trust-policy snapshot;
- explicit historical, trust-only and full-retrospective queries;
- non-overwritable reports bound to exact policy digest and verifier profile.

This is a domain-specific assurance composition over established temporal concepts, with physical measurement and engineering-claim boundaries. Novelty must be demonstrated through matched correctness, completeness, independent replay and verification economics—not by renaming the time model.

### 7.2 Reproducible reference-model scale harness

The research-only Rust reference model includes `benches/temporal_replay_scale.rs`, a deterministic in-memory benchmark using the crate's existing Serde dependencies.

Run:
```bash
cargo bench --locked --package symtropy-aero-material-qual --bench temporal_replay_scale -- 10000,100000 5
```

The first argument is a comma-separated list of event counts; the second is iteration count. The default is 10,000 events and three iterations. Larger 100,000 / 1,000,000 / 10,000,000 runs are opt-in. The output is CSV with event count, replay mode, iterations, median/minimum latency and resulting disposition.

The routine CI workflow runs only the 10,000-event, one-iteration smoke and checks all three replay modes return `ACCEPTED_FOR_BINDING_ONLY`. It is a compile/functional smoke, **not** a performance result or competitive benchmark.

A published performance claim requires repeated runs on pinned hardware/OS/toolchain, peak resident memory, CPU time, evidence-view size, events scanned, warm/cold state, p50/p95/p99 latency and a matched append-only/bitemporal baseline. Include fixture-generation cost separately from replay time. The synthetic digests/receipt labels in the harness are not cryptographic evidence. Do not use a successful benchmark run as evidence of trusted source coverage, physical correctness or qualification.

## 8. Standards and research comparators

- **NIST IR 8536 (final, 9 September 2026):** Manufacturing Supply Chain Traceability Meta-Framework. It defines interoperable, cryptographically verifiable traceability links, temporally ordered provenance, selective disclosure and cross-ecosystem event exchange while preserving existing standards. Treat it as an architecture/interoperability comparator and map AERO's event envelope to its concepts; do not create a competing manufacturing traceability framework. NIST's September 2026 announcement stated its Python reference implementation was in final review and that the public repository would follow approval; check the project page before integrating or claiming implementation parity: https://csrc.nist.gov/pubs/ir/8536/final and https://www.nccoe.nist.gov/projects/supply-chain-traceability-principles-manufacturing-meta-framework.
- **ISO 10303-242:2025 (AP242 Edition 4):** published 25 August 2025 and the current published baseline for this benchmark. ISO/CD 10303-242 Edition 5 was registered in February 2026; its committee-draft comment period closed 22 April 2026 and the draft was referred back to the working group. Edition 5 is watch-only until published; retain edition/profile identifiers and build a differential compatibility probe rather than silently upgrading the benchmark: https://www.iso.org/standard/84300.html and https://www.iso.org/standard/93277.html.
- **ISO 23952:2020 (QIF Edition 1):** ISO records this edition as reviewed and confirmed on 14 June 2026, so it remains the current published QIF baseline as of this research revision: https://www.iso.org/standard/77461.html.
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


### 9.1 Upstream readiness audit (25 November 2026)

These owners are real integration dependencies, but their current public work is not yet a production qualification basis:

| Owner / tracker | Current public state | What AERO may consume | What AERO must not claim yet |
|---|---|---|---|
| Mycelix AeroCommons [PR #6347](https://github.com/Luminous-Dynamics/mycelix/pull/6347) | Open draft; exact-head verification and review remain separate | `CalibrationValidityV1`, instrument/evidence binding, half-open validity interval, physical-unit packet identity and exact artifact byte/hash receipts | Signature/issuer authentication, trusted source/receiver time, custody, QIF/AP242 conformance, measurement truth or engineering acceptance |
| Symthaea BASALT [PR #9203](https://github.com/Luminous-Dynamics/symthaea/pull/9203) | Open; timeline auditor explicitly keeps `confirmatory_eligible=false` | Resource digest/generation timelines, supplied event-digest checks, declared dependency pins, stale-evidence closure and re-verification plans | Trusted change capture, complete observation of all real inputs, producer authenticity or confirmatory eligibility |
| Symthaea trusted runner [#9210](https://github.com/Luminous-Dynamics/symthaea/issues/9210) and coverage qualification [#9239](https://github.com/Luminous-Dynamics/symthaea/issues/9239) | Both open | Future runner-issued event/observer receipts when independently qualified | A self-declared `dependency_coverage=complete` flag or signed receipt alone is not proof that every influential input was observed |
| Symthaea ASSURE [#2618](https://github.com/Luminous-Dynamics/symthaea/issues/2618) | Open lifecycle-semantics work | Append-only predecessor/currentness lineage and explicit invalidation/requalification transitions | `TerminalInSuppliedView` is not global currentness through a verified checkpoint; a successor is not valid merely because it exists |
| Symthaea uncertainty provenance [#7412](https://github.com/Luminous-Dynamics/symthaea/issues/7412) | Open; its issue states current empirical `MetricValue` snapshots lack observed sample-unit and CI-generation provenance; upstream #7411 fails closed until resolved | Future versioned observation-level sample-unit, CI-method/version, confidence-level and estimator provenance after implementation and exact tests | Do not represent existing empirical metrics or the AERO pilot as uncertainty-qualified merely because a metric contract declares an interval policy |
| Symthaea evidence-action selector [#13112](https://github.com/Luminous-Dynamics/symthaea/issues/13112) | Open issue with a delivered Rust audit/VOI benchmark slice described | Candidate baseline for admissible next-test selection, EVSI/minimax-regret and multi-resource Pareto comparison | Do not claim a production integrated selector or replace qualified human/authority decisions without exact-head validation and owner-native outcome receipts |

**Integration consequence:** AERO's current status remains a synthetic aviation-semantic contract. The adapters should preserve explicit `UNKNOWN`, `UNQUALIFIED`, `INCOMPLETE` and `NOT_SUPPORTED` states when an upstream owner has not yet established its own claim. A downstream AERO report must never upgrade an upstream evidence record's assurance level.

The temporal benchmark's event receipt, uncertainty and historical replay vectors are intended to exercise these owner boundaries, not to implement their generic ledgers, runners, lifecycle engines or estimators inside Symtropy.

AERO v0.1/v0.2 remains a synthetic software contract. The temporal assurance benchmark must not claim to authenticate actual lab records, establish physical custody, validate material allowables, demonstrate NDE probability of detection, qualify an aircraft component, authorize maintenance release or establish airworthiness.

## 10. Frozen temporal replay corpus

The machine-readable first slice is:

- `docs/research/aviation/examples/aero_temporal_replay_corpus_v0_1.json`
- `docs/research/aviation/schemas/aero_temporal_replay_corpus_v0_1.schema.json`

It contains 29 synthetic controls: a positive no-event binding case; known-but-future-effective and known-effective revocations; a late backdated revocation excluded from the original evidence snapshot but included in a full retrospective replay; revocation effective after acquisition but before report generation; revocation effectivity overlapping acquisition-time uncertainty; calibration-window boundary ambiguity; valid timestamp/imprint positive and mismatching-imprint negative controls; unknown issuer authority; a sequence gap; conflicting same-namespace duplicate payloads; idempotent repeated delivery; identical local IDs under distinct source namespaces; an unverified namespace alias from the same issuer; an issuer-claimed but unverified receipt; unknown compromise effective time; correction cycle, unresolved correction target, cross-certificate correction target, unqualified reinstatement; structurally valid but unqualified corrections, equal-time correction targets and correction-of-correction chains; unqualified/incomplete observer coverage; evidence-checkpoint digest mismatch; and source clock rollback.

Every case freezes `AS_KNOWN_AT`, `RECOMPUTED_WITH_CURRENT_TRUST` and `RECOMPUTED_WITH_CURRENT_EVIDENCE_AND_TRUST` dispositions over distinct evidence and trust cutoffs. Event effective-time comparison is against acquisition time, not report time. Timestamp status `valid` also requires a non-null token time and exact event-digest imprint equality. The fixture deliberately permits duplicate event IDs so that the duplicate-conflict challenge is representable; uniqueness is a semantic rule to be evaluated, not a JSON Schema uniqueness rule over complete objects.

**Oracle status: `NOT_INDEPENDENTLY_REVIEWED`.** The corpus is a contract fixture and is schema-validated in CI. It is not yet wired to the v0.2 Rust evaluator and must not be described as a passing semantic oracle. Independent review should precede evaluator integration.

## 11. Execution sequence

1. Independently review the existing 31 v0.2 dispositions and the Rust event replay test.
2. Independently review the 20 temporal replay vectors and extend them with generated correction histories, sequence/fork properties and boundary cases.
3. Define an owner-native event receipt mapping with Mycelix and BASALT maintainers; do not add duplicate state.
4. Resolve duplicate event identity before correction-reference resolution in the evaluator; add conflicting-payload permutation regression tests.
5. Add a verifier adapter for supplied signature/timestamp evidence only after its source format and trust policy are pinned.
6. Validate against independently retained source receipts and a rights-cleared, non-flight-critical physical pilot under #7396.
7. Promote claims only after independent laboratory, metrology and engineering review.
