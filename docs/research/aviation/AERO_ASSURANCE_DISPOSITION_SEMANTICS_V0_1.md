# AERO Assurance Disposition Semantics v0.1

**Status:** research contract; proposed semantics and synthetic vectors, not independently adjudicated  
**Authority ceiling:** review disposition only; no physical qualification, airworthiness, certification or work authorization

## 1. The shift: assurance is a vector, not a score

AERO should not compress evidence integrity, producer authority, semantic validity, applicability, chronology, model credibility, observer completeness and argument strength into a single confidence number.

Those are independent obligations. A high result on one axis cannot compensate for failure on another. The normative object is therefore a **typed assurance state vector**. The familiar disposition label is a deterministic projection for routing and display; it is not the complete decision state.

For a claim episode, retain at least these axes independently:

1. claim identity and physical/configuration context;
2. payload integrity;
3. producer authority under the selected trust-policy snapshot;
4. semantic validity of the owner record;
5. applicability to the subject, configuration and context of use;
6. receiver knowledge at the selected as-known cutoff;
7. model credibility and required validation strata;
8. dependency/observer coverage for the relevant source, change class and interval;
9. defeater status and resolution evidence;
10. replay and verifier completeness.

Each obligation carries its own evidence state, applicability, authority, temporal status, observer coverage, verification state, evidence references and reason codes. Never derive one axis from another merely because they share a document or signature.

## 2. Four-valued evidence semantics

For each proposition (p), record two independent bits:

- (s(p)=1) when admissible evidence supports (p);
- (r(p)=1) when admissible evidence rebuts (p).

This yields four states:

| State | (s) | (r) | Meaning |
|---|---:|---:|---|
| `NEITHER` | 0 | 0 | Neither support nor rebuttal is established |
| `SUPPORTED_ONLY` | 1 | 0 | Support exists; no admissible rebuttal is known |
| `REFUTED_ONLY` | 0 | 1 | Rebuttal exists; no admissible support is known |
| `BOTH` | 1 | 1 | Conflicting support and rebuttal coexist |

This is the four-valued evidence discipline used by Belnap-style logics. It is preferable to classical true/false when engineering evidence is incomplete or contradictory.

The information order is:

[
	exttt{NEITHER} preceq 	exttt{SUPPORTED_ONLY} preceq 	exttt{BOTH}
quad	ext{and}quad
	exttt{NEITHER} preceq 	exttt{REFUTED_ONLY} preceq 	exttt{BOTH}
]

`SUPPORTED_ONLY` and `REFUTED_ONLY` are incomparable. New evidence can move a proposition from supported to both; it must not silently erase the earlier rebuttal. An evidence count or confidence average must never collapse `BOTH` into `SUPPORTED_ONLY`.

The state is calculated only from admissible evidence: payload integrity, owner/producer authority, semantic validation, applicability and time must be evaluated as separate gates before an item contributes support or rebuttal. An unauthenticated assertion is not admissible merely because its contents look plausible.

### 3.2 Intrinsic hard gates and policy authority

The v0.1 projection treats claim identity, evidence/payload integrity and producer authority as intrinsic mandatory hard gates. Every state vector must include each kind, and no instance may mark one optional or non-hard. Missing or downgraded intrinsic gates produce `UNVERIFIED`, never reviewability.

This protects against the most direct self-declared-policy bypass. It does **not** solve the broader policy-authority problem: whether every other obligation is mandatory, which domain strata are required, and which gates apply to a particular intended use must be bound to a verified claim specification / requirement profile and its exact digest. The present reference evaluator receives those additional obligations as caller-supplied inputs; it does not authenticate a requirement profile or acceptance authority. Until that profile binding exists, this module is a reference projection and must not be used as a standalone production approval gate.

## 3. Orthogonal gates and open-world semantics

The proposition state is only one axis. Each obligation also records:

- **Applicability:** `IN_SCOPE`, `OUT_OF_SCOPE`, or `UNKNOWN`.
- **Authority:** `AUTHORIZED`, `DENIED`, `UNKNOWN`, or `NOT_REQUIRED`.
- **Temporal knowledge:** `KNOWN_AT_CUTOFF`, `NOT_KNOWN_AT_CUTOFF`, `TEMPORAL_CONFLICT`, or `NOT_TEMPORAL`.
- **Observer coverage:** `COMPLETE`, `INCOMPLETE`, `UNKNOWN`, or `NOT_APPLICABLE`.
- **Verification:** `VERIFIED`, `FAILED`, `UNVERIFIED`, or `NOT_RUN`.

This prevents the following invalid substitutions:

- matching digest (
otRightarrow) authorized producer;
- valid signature (
otRightarrow) semantically valid record;
- semantically valid record (
otRightarrow) applicable to this physical article;
- issuer event time (
otRightarrow) receiver knew the evidence at that time;
- no observed change (
otRightarrow) complete observation;
- reproducible calculation (
otRightarrow) correct engineering conclusion.

The system is open-world. Missing records, unavailable observers and empty impact sets are not evidence of absence unless the source inventory, observation interval, change classes and event-delivery coverage are themselves qualified.

### 3.1 Do not collapse subject-time effectivity into receiver knowledge

AERO has two independent temporal questions:

- **Subject-time effectivity:** was the event / requirement / configuration effective for every possible physical acquisition instant? The existing `effectivity_interval_v0_1` comparator consumes hard event-effective and acquisition-time intervals and returns `DEFINITELY_EFFECTIVE`, `DEFINITELY_NOT_YET_EFFECTIVE`, `TIME_BOUNDARY_AMBIGUOUS` or `INVALID_TIME_INTERVAL`.
- **Receiver knowledge:** had the evidence actually been received by the selected `as_known_at` cutoff? A backdated issuer assertion does not move receiver-first-seen earlier.

The disposition evaluator consumes the existing `EffectivityRelation` through an explicit Rust conversion into its normalized state-vector enum. The normalized values `BOUNDARY_AMBIGUOUS` and `INVALID_INTERVAL` correspond to the comparator's `TIME_BOUNDARY_AMBIGUOUS` and `INVALID_TIME_INTERVAL`. A Rust `From<EffectivityRelation>` conversion and test cover all four mappings. `NOT_EFFECTIVE_AT_SUBJECT_TIME` is distinct from `NOT_KNOWN_AT_SNAPSHOT`; boundary ambiguity abstains and invalid intervals reject the temporal assertion. It does not recompute source chronology or substitute issuer-asserted time for receipt time.

Physical/acquisition bounds and event-effective bounds are owned by the interval comparator; receiver receipt watermarks and trust-policy commitments remain owner/decision-context inputs. The issuer-asserted event timestamp is retained with the source artifact and must not be inferred from either of those watermarks. The evaluator is a projection layer, not a replacement for the owner-native temporal ledger.

## 4. Deterministic disposition projection

The full state vector is normative. A primary label is derived using the following fixed presentation precedence; all component states and all applicable reason codes remain visible:

1. **REJECTED** — verified counterevidence establishes a mandatory hard-gate failure, a required identity/integrity mismatch, a trusted verifier returns `FAILED` for a mandatory hard gate, or an authenticated policy snapshot explicitly denies required producer authority. A `BOTH` conflict alone is not enough to claim the proposition is false.
2. **OUT_OF_SCOPE** — the claim's declared context of use is affirmatively outside the applicable domain.
3. **NOT_KNOWN_AT_SNAPSHOT** — evidence necessary to evaluate the claim was first received after the selected as-known cutoff.
4. **UNVERIFIED** — required payload, producer authority, owner receipt or verifier result cannot be trusted or has not been run. A completed, trusted negative verification is a failure finding, not merely an absence of verification.
5. **INCOMPLETE** — one or more mandatory obligations remain `NEITHER`, conflicting, uncollected, or dependent on incomplete/unknown observer coverage; an effectivity boundary is ambiguous; or a material defeater remains unresolved.
6. **NOT_EFFECTIVE_AT_SUBJECT_TIME** — an event is definitely not yet effective for any physical acquisition instant in the declared acquisition interval. This says nothing about whether its evidence was received by the historical cutoff.
7. **REVIEWABLE_FOR_INDEPENDENT_DECISION** — every mandatory obligation is supported, in scope, authorized where required, known at the cutoff, verified, and free of unresolved material defeaters; relevant observers are complete; and any required event is effective at subject time.

The precedence is a stable user-interface projection, not a safety ranking or compensating score. When multiple conditions occur, report the selected label **and** every obligation-level blocker. For example, a late evidence item and an independently verified wrong-subject record must preserve both facts even though the primary projection is `REJECTED`.

`REVIEWABLE_FOR_INDEPENDENT_DECISION` means only that the case is sufficiently evidenced for a competent independent decision-maker to review. It does not mean accepted, safe, qualified, approved, airworthy or certified.

## 5. Defeaters and generated hypotheses

A defeater must have an explicit status and evidence trail:

- `CANDIDATE`: a reviewer or model proposed a possible defeater; it is a lead, not established counterevidence.
- `UNRESOLVED`: a material defeater requires investigation and blocks reviewability.
- `CONFIRMED`: admissible evidence supports the defeater.
- `REFUTED`: admissible evidence rebuts the defeater; retain the history and rationale.
- `DISMISSED`: an authorized reviewer closed it with a recorded rationale.

Symthaea may generate candidate defeaters, alternative explanations and acquisition plans. A generated hypothesis cannot become `CONFIRMED` without independently verifiable evidence. Candidate generation is not an adverse finding and cannot itself reject a claim; conversely, an unresolved material defeater cannot disappear because the positive argument is lengthy.

## 6. Temporal replay is an explicit query

Every evaluation names a replay mode and immutable snapshots. The v0.1 Rust projection preserves the selected replay mode in its result; it does not itself fetch or reconstruct snapshots. The caller must supply already-resolved, owner-verified obligation states and policy context. Replay orchestration remains a separate layer:

- `HISTORICAL_AS_KNOWN`: evidence and trust policy as known at the original cutoff;
- `CURRENT_POLICY_COUNTERFACTUAL`: same evidence cut evaluated under a named current trust policy;
- `CURRENT_AS_KNOWN`: evidence and policy known at the current evaluation time.

Never rewrite the historical result when a key is later revoked, a record corrected or a trust policy changed. Produce a new counterfactual result linked to the original snapshot. Store physical/acquisition time, issuer-asserted time, receiver-first-seen time and trust-policy effective/observation time separately.

## 7. Observer completeness and selective requalification

An unaffected claim may be preserved only when the dependency relation and the observer are both qualified for the relevant source, interval and change class. If observer coverage is `UNKNOWN` or `INCOMPLETE`, the impact state is unknown/incomplete, not empty.

This is a two-sided assurance objective:

- avoid missed invalidations by requiring complete observation before preservation;
- avoid indiscriminate requalification by proving non-dependence under a complete observer contract.

AERO consumes owner-native dependency and lifecycle results; it must not silently replace Symtropy Track U or Mycelix/AeroCommons ledgers and invalidation services.

## 8. Reference evaluation algorithm

The projection is a pure function over a frozen state vector and named policy snapshot:

```text
derive_disposition(vector):
    preserve every obligation result and reason code

    if any mandatory hard gate has verified REFUTED_ONLY evidence
       or a required authority predicate is explicitly DENIED under a verified policy snapshot:
        return REJECTED

    if claim context is affirmatively outside its declared domain:
        return OUT_OF_SCOPE

    if a required evidence item is NOT_KNOWN_AT_CUTOFF:
        return NOT_KNOWN_AT_SNAPSHOT

    if a required trust / payload / authority predicate is UNVERIFIED
       or its verifier is UNVERIFIED / NOT_RUN:
        return UNVERIFIED

    if any mandatory proposition is NEITHER or BOTH
       or a material defeater is UNRESOLVED
       or required observer coverage is UNKNOWN / INCOMPLETE
       or subject-time effectivity is BOUNDARY_AMBIGUOUS:
        return INCOMPLETE

    if subject-time effectivity is DEFINITELY_NOT_YET_EFFECTIVE:
        return NOT_EFFECTIVE_AT_SUBJECT_TIME

    if subject-time effectivity is INVALID_INTERVAL:
        return REJECTED

    if every mandatory obligation passes every applicable axis:
        return REVIEWABLE_FOR_INDEPENDENT_DECISION
```

An implementation must be deterministic for a frozen vector, policy snapshot and canonicalization version. It must preserve the complete blocker set, not only the first matching branch. The schema and synthetic vectors in this directory are contract fixtures, not an independently adjudicated oracle.

## 9. Required properties and adversarial tests

1. **Non-compensation:** one failed mandatory gate cannot be outweighed by any number of passing optional obligations.
2. **Conflict preservation:** `BOTH` never becomes supported-only through aggregation, sorting or score normalization.
3. **Evidence monotonicity:** adding evidence preserves prior support and rebuttal; a contradiction can make the state less reviewable.
4. **Temporal immutability:** later receipt or policy change does not mutate an earlier as-known snapshot.
5. **Unknown propagation:** unknown observer coverage cannot produce a complete empty impact set.
6. **Authority separation:** payload validity cannot satisfy producer authority or work authorization.
7. **Defeater provenance:** a generated candidate is not a confirmed defeater; unresolved material defeaters block reviewability.
8. **Permutation invariance:** manifest ordering, graph ordering and filesystem location do not change normalized results.
9. **Independent implementation:** oracle and evaluator share no decision helper, reason-code table or generated expected output.
10. **No approval capability:** no disposition or Symthaea recommendation grants authority to perform a physical test or modify an aircraft.

## 10. Relationship to prior art and intended contribution

Four-valued logic, assurance cases, formal argument analysis and traceability are not new. ACCESS and goal-directed ASP work already explore formalized assurance cases and semantic analysis. The research contribution is the empirically tested composition of those techniques with owner-scoped provenance, bitemporal replay, observer-qualified impact, non-compensatory gates and portable independent reconstruction.

The first Rust reference evaluator is now in `crates/domains/symtropy-aero-material-qual/src/assurance_disposition_v0_1.rs`. A separately implemented JSON-level integration oracle in `tests/assurance_disposition_oracle_v0_1.rs` cross-checks all published vectors, 12,288 finite product states and obligation-order permutations without calling the production evaluator's helpers. This is implementation diversity / consistency testing; it is not independent domain adjudication of the expected outcomes. It deserializes the typed obligation contract, derives the non-compensatory projection, preserves sorted reason codes and blocker identities, and always reports authority/physical qualification/certification as not granted or not claimed. Its unit tests execute the ten published synthetic vectors and test evidence conflict, missing versus unverified evidence, temporal precedence, observer unknowns, hard-gate failures, defeaters and optional-obligation handling. This remains an implementation of the proposed projection, not an independent oracle: a later Lean or TLA+ model and a separately implemented oracle are still required. Neither formal proof nor passing software tests establishes physical safety; evidence quality and context-specific engineering validation remain separate.

## 11. Non-claims

This contract does not establish physical truth, model credibility, structural integrity, material allowables, airworthiness, regulatory compliance, certification or authority to conduct tests. All included vectors are synthetic and proposed. Independent domain adjudication is required before they can be treated as benchmark oracles.
