# Maritime Human Re-entry Benchmark v0.1

**Tracking:** [MARITIME-HUMAN-REENTRY-001 #4440](https://github.com/Luminous-Dynamics/symtropy/issues/4440)  
**Owners composed:** Symtropy #3893 / #1612; Symthaea #11185 / #11173; Mycelix #5974  
**Status:** synthetic contract and scenario seed; no human-participant or vessel trial

## Objective

Measure whether the information contract and control-transfer protocol make a remote operator's re-entry task possible, bounded and auditable. This benchmark tests *the system's handover design*, not a human's competence or fitness.

A person being logged in is not evidence that they can reconstruct the current vessel state, identify the active control mode, understand the hazard, or intervene in time. The system must therefore treat operator availability, operator awareness, command authority and successful transfer as separate claims.

## Core design

A takeover request is a time-bounded transaction, not a notification:

1. The local vessel controller remains in its current bounded mode and keeps the declared contingency available.
2. The system publishes a versioned situation packet with source, observation time, age, uncertainty, active mode, local action and authority scope.
3. The operator reconstructs the situation, may request clarification, and chooses accept, decline or hold.
4. An acceptance is valid only for the exact packet revision, vessel configuration, mode, function and unexpired authority epoch.
5. Local/remote authority transfer is atomic. Missing, late, duplicated, revoked or ambiguous acknowledgements cannot create a successful transfer.
6. On timeout or communication loss, the predeclared local fallback remains in force. The system must not invent a new fallback or assume that silence means consent.
7. Returning to a higher-autonomy mode requires fresh evidence, a matching configuration, required checks and scoped authorization.

## Situation packet minimum fields

- Vessel identity and exact configuration revision.
- Active operating mode and function allocation.
- Decision time, clock basis, source observation times, delivery times, validity limits and uncertainty.
- Ownship state with its source lineage and time basis.
- Target tracks with source class (local sensor, fused track, AIS report, V2X claim, simulator-only truth), uncertainty and prediction horizon.
- Current local controller/contingency state, constraints and action authority.
- Escalation reason, relevant evidence, contradictions, missing inputs and alternatives.
- Operator/master role, scope, lease epoch, acknowledgement deadline and transfer state.
- Exact digests/revisions for source artifacts, transformations, policy and model.

AIS/V2X messages are not local sensor truth. Several derived views from one upstream observation are not independent corroboration. Simulator truth is never deployment evidence.

## Scenario contract

The machine-readable corpus is in [`../../benchmarks/maritime/human-reentry-corpus-v0.1.json`](../benchmarks/maritime/human-reentry-corpus-v0.1.json); its schema is [`MARITIME_HUMAN_REENTRY_CASE_V0_1.schema.json`](./MARITIME_HUMAN_REENTRY_CASE_V0_1.schema.json).

Each trace binds:
- exact case ID and synthetic fixture class;
- vessel configuration, active mode, decision clock and takeover deadline;
- packet freshness/completeness and source-role state;
- declared operator availability (synthetic scenario input only);
- authority validity/scope, communications state and local contingency;
- ordered events with monotonic offsets;
- an expected disposition and hard invariants.

The first corpus contains 33 unique cases: positive, negative, ambiguity, recovery and metamorphic controls. It is intentionally contract-focused; it does not model human cognition or ship physics.

## Baseline arms

Compare on the same event tape, evidence availability, deadline and compute budget:

1. Alert-only status panel.
2. Conventional radar/AIS/ECDIS-style display with the same source data.
3. Structured situation packet with explicit source, age, uncertainty and active mode.
4. Structured packet plus event timeline, local contingency and bounded choices.
5. Evidence-constrained Symthaea explanation versus the same structured packet without generated prose.

No arm receives hidden scenario truth or extra sensor observations. Separate information quality from presentation quality.

## Metrics

Report independently:
- correct active-mode / function / authority reconstruction;
- critical-state and constraint reconstruction, including omissions;
- stale, conflicting, unsupported or source-dependent evidence detection;
- clarification, abstention and refusal correctness;
- false acceptance of invalid or expired transfer;
- duplicate, overlapping or gap-producing authority transitions;
- alert-to-correct-disposition latency;
- local fallback continuity and recovery disposition;
- unnecessary intervention, missed intervention and alert burden;
- source-to-packet trace completeness and independent replay agreement.

Do not collapse these into a weighted autonomy score. Safety/authority failures are non-compensatory. A system that rejects every transfer also fails positive controls and coverage.

## Frozen hard gates

- No handover success on a missing, late, stale, wrong-scope, revoked or expired acknowledgement.
- No simultaneous local and remote command authority.
- No control gap caused by an unacknowledged transfer.
- No mode re-entry without fresh evidence and scoped approval.
- No future, simulator-only or hidden-oracle data in a deployment packet.
- Every event, packet revision, acknowledgement, lease epoch and fallback outcome is traceable.
- Positive controls must be accepted when their declared prerequisites are complete and valid.

Deadlines are case-specific. Do not invent one universal takeover-time threshold: the value must come from the vessel/function hazard analysis, qualified local fallback, operating profile and human-factors review.

## Human-participant boundary

A synthetic evaluator can test event sequencing, information availability, timing rules and authority integrity. It cannot measure vigilance, situation awareness, workload, competence or human response distributions.

Any later human-participant study needs qualified maritime human-factors design, ethics review, informed consent, realistic simulation and safe stopping rules. It must not use the synthetic evaluator as a personnel fitness or certification instrument.

## Validation ladder

- H0: JSON/schema/corpus integrity, uniqueness, monotonic event time and positive-control presence.
- H1: dependency-light deterministic evaluator and independent transition oracle.
- H2: exact-head synthetic replay with mutation controls and source/authority boundary checks.
- H3: composition with exact pinned ODD and Symthaea owner-native artifacts.
- H4: simulator-in-the-loop interface study; physical/trajectory outcomes only when a qualified physics owner provides them.
- H5: human-participant evaluation under separate review; no live ship actuation.

## Current implementation status

The v0.1 contribution now includes:
- This research specification, strict Draft 2020-12 case schema and 33-case synthetic corpus.
- A standalone offline Rust evaluator under `benchmarks/maritime-human-reentry/`, with strict typed deserialization, actor/event validation, deadline and authority-lease checks, fallback/reconciliation dispositions, and exact-input SHA-256 reporting.
- A CLI and seven authored unit tests covering the frozen corpus, duplicate acknowledgement versus duplicate lease, partition reconciliation, AIS source-role limits, simulator-truth exclusion, stale re-entry approval, non-monotonic event rejection and unknown-field rejection.
- An exact-head workflow pinned to Rust 1.96.0 for format, test, Clippy and corpus execution.

Static preflight is not a Rust execution result. No Rust format/test/Clippy/CLI pass is claimed until the exact-head workflow reports success. No human study, simulator, hydrodynamic model, live ROC or vessel trial is claimed.
