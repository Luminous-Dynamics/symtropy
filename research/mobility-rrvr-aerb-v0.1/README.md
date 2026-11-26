# AERB — Ambiguity and Evidence Resolution Benchmark v0.1

**Status:** protocol and schema workstream, 2026-11-26. Synthetic process-model work only; no simulator, HIL, physical, RoboRecover or SOTA result is claimed.

**Tracking issue:** [Symtropy #7631](https://github.com/Luminous-Dynamics/symtropy/issues/7631)

**Related work:** #7564 deterministic fault harness · #7565 evidence adapter · #7583 benchmark overlay · #7586 replay auditor · #7595 RoboTwin exact replay verifier · #7598 paired inference · isolated paired inference [PR #7629](https://github.com/Luminous-Dynamics/symtropy/pull/7629) · Symthaea [#13807](https://github.com/Luminous-Dynamics/symthaea/issues/13807) / [#13809](https://github.com/Luminous-Dynamics/symthaea/pull/13809).

## 1. Research question

When a command acknowledgement is lost or an execution trace is incomplete, can an agent preserve the set of physically plausible execution histories, identify what observations would distinguish them, and choose a decision disposition whose worst-case consequence is acceptable—without guessing which history actually happened?

AERB moves evaluation from "did the robot eventually recover?" to:

1. What possible execution histories remain consistent with the evidence available at the decision cut?
2. Does the agent's declared support still contain the hidden execution truth?
3. Which permitted observation best reduces worst-case decision loss before the deadline?
4. Does the agent abstain when the record is stale, incomplete, disputed or outside its validated operating envelope?
5. Does the execution layer independently reconcile the physical effect before a subsequent attempt?

The counterfactual worlds are alternative hypotheses, not claims that multiple histories physically occurred. Exactly one hidden execution history is used as evaluator truth per episode.

## 2. Paradigm: possible-history execution rather than point-guessing

For a lost acknowledgement, preserve distinct hypotheses:

- not_dispatched: the command did not leave the coordinator;
- dispatched_no_effect: command was dispatched, no effect occurred;
- applied_once_ack_lost: intended effect occurred once, acknowledgement was lost;
- partial_effect: only part of the intended effect occurred;
- duplicate_effect: one-shot effect occurred more than once;
- stale_authority_effect: effect occurred under a superseded authority epoch or queue generation;
- unresolved_command_holder: another known holder/forwarder may still create an effect because inventory is incomplete;
- open_world_residual: an unmodelled effect-producing path or system-boundary mismatch remains possible, so the catalog cannot yet be treated as exhaustive.

A scenario may permit a subset or extension of these states. State IDs, world transition rules, outcome supports and hidden truth are defined in the evaluator package. The open-world residual is retained until the declared system boundary and effect-producing component/holder inventory have been independently qualified; a command-holder list supplied by the same coordinator is not sufficient evidence to remove it. AERB v0.1 does not itself issue that closure attestation. The candidate receives only the public episode snapshot and its permitted observation menu.

**Do not collapse unknown into no-effect. Do not turn a prediction into a physical observation. Do not let a successful task outcome prove command integrity.**

## 3. Two physically separate data products

AERB uses two distinct artifacts and processes:

### Candidate-visible episode

Contains the frozen command context, decision cut, admitted observations, their timestamps/clock domains/source roles, and the permitted probe menu with costs/deadlines. It contains no hidden truth ID, world outcome supports, likelihoods or evaluator loss matrix.

The candidate snapshot digest in the submission is SHA-256 over RFC 8785 JSON Canonicalization Scheme (JCS) bytes of the complete candidate-visible episode artifact. The synthetic submission fixture binds to its exact episode fixture; do not copy that digest to a modified episode.

### Evaluator-only world pack

Contains the one true world for each episode, the possible-world catalog, the world-specific physical effect and authority state, the probe outcome-support sets, and the frozen decision-loss table. It must remain in an independently permissioned evaluator process/environment during confirmatory runs.

The JSON schemas in schemas/ define these separate contracts. Synthetic public fixtures exist only for unit tests and conformance; they are not a secret evaluation set. The candidate command context binds an execution_scope_id, system_boundary_manifest_sha256 and effect_producing_inventory_profile_sha256 so all closure evidence is assessed against a declared boundary.

## 4. Candidate output contract

A candidate submission declares:

- exact candidate episode and evidence snapshot digest;
- model/checkpoint/profile digests;
- the set of possible world IDs it believes remain consistent with the evidence;
- one non-actuating disposition: hold_reconcile, request_observation, or abstain;
- optionally one probe ID from the permitted menu;
- reason codes and a short structured explanation tied to observation IDs.

The candidate cannot dispatch, retry, fence an endpoint, append evidence, adjudicate Applied/NoEffect, or authorize a robot action. It can recommend a disposition only. A separate execution owner enforces permissions and executes permitted probes.

A candidate may conservatively retain every possible world. This should preserve support coverage but often leads to high worst-case decision loss. A narrow support may reduce worst-case loss but is penalized when it excludes evaluator truth. This creates a real calibration/utility trade-off rather than rewarding confident guessing.

### Evidence refinement invariant (required for sequential extensions)

Across successive decisions in one attempt, admitted evidence may narrow the candidate's possible-world support only when the evidence is valid, decision-cut-bounded, scope-matched and its outcome support excludes the removed worlds. Stale, missing, unknown, disputed, correlated-but-unmodelled or wrong-scope observations must never shrink the support set; they preserve uncertainty or force abstention. Contradictory evidence must expand the support or produce a typed conflict/abstention rather than silently overwrite the prior history. The v0.1 artifact schema is one immutable decision snapshot; a sequential v0.2 trace will bind predecessor report digests and test this monotonicity rule explicitly.

### Scope-closure receipt and the open-world residual

The separate schema schemas/aerb_scope_closure_receipt_v0_1.schema.json defines a receipt for a claimed complete inventory of effect-producing components, command holders and forwarding edges within one declared execution scope. The receipt is admissible for narrowing open_world_residual only when:

- its execution scope, system-boundary manifest digest and inventory-profile digest match the episode command context;
- the independent evidence adapter verifies the signature envelope, source artifacts, verifier profile and receipt provenance;
- the disposition is closed within the declared scope, all three completeness flags are true, and unresolved holder/forwarder/unaccounted-producer counts are zero;
- the receipt's decision frontier is not later than the candidate's decision cut and its source roles are independent of the coordinator whose inventory is under test.

A JSON field reading verification_status = verified is not self-authenticating. The upstream adapter must actually verify the evidence and signature; AERB v0.1 only defines the receipt contract. Even a valid receipt closes only the named boundary and stated assumptions. It does not prove that the physical universe has no effects outside that boundary, and it cannot be issued by the same coordinator whose command-holder inventory is being challenged.

## 5. Reuse Symthaea's existing robust decision-value evaluator

Do not implement a second minimax/EVSI engine.

Reuse research/decision-predictability-benchmark/src/robust.rs as the robust decision-value calculation:

For a declared set of possible worlds S, available dispositions A, and frozen loss L(a,w),

\[
R(S)=\min_{a\in A}\max_{w\in S}L(a,w)
\]

For probe p, its possible outcome z restricts the world set to S_z. The post-observation worst-case risk is

\[
R_p=\max_z R(S_z), \qquad \Delta_p=R(S)-R_p.
\]

\(\Delta_p\) is a guaranteed worst-case reduction under the supplied finite support model. It is **not** EVSI, mutual information or an expected risk reduction. Probe acquisition cost, result delay, permission and deadline remain separate dimensions. Do not subtract them from task loss unless a defensible, preregistered common utility scale exists.

Use the Bayesian EVSI path only when prior probabilities and joint observation likelihoods are independently justified and frozen. Correlated evidence channels require a joint model or an explicitly justified conditional-independence contract. HDC cosine, covariance distance, Mahalanobis distance and raw prediction error are not probabilities.

The robust evaluator computes value; it does not execute probes. Its Rust type boundary is not a process-security boundary.

## 6. Initial campaign: process-model lane

Start with #7564's six paired scenarios and add independently frozen cut points for:

- effect-before-acknowledgement loss;
- delayed/reordered/duplicated acknowledgement;
- partial effect;
- endpoint restart and deduplication-state loss;
- old authority epoch / queue-generation effect;
- coordinator restart during reconciliation;
- cancellation racing with in-flight command;
- missing command holder / newly discovered forwarding edge;
- stale or disputed observation;
- event-time, acquisition-time and ingest-time divergence;
- source clock regression and clock-domain substitution;
- sensor outage during a decision deadline.

For each episode:

1. Freeze scenario, command, action prefix, endpoint profile, authority epoch, queue generation, fault schedule and cluster identity.
2. Generate evaluator-only truth from the process model.
3. Expose only the candidate-visible snapshot up to its declared decision cut.
4. Collect the candidate's world-support declaration and non-actuating disposition/probe request.
5. Execute permitted probes through the harness; preserve their raw outputs and execution receipts.
6. Reconcile the candidate support with evaluator truth and probe outcomes.
7. Compute robust minimax value from the candidate-declared support and the evaluator's frozen loss/support model.
8. Record all unknown, stale, disputed, invalid and unresolved episodes; no silent exclusions.

The first lane is synthetic process-model evidence. The simulator adapter must independently supply simulator reset/action/effect receipts and raw observations before any simulator claim. HIL and physical tiers are separate.

## 7. Required baselines

Compare under matched information, probe menu, deadlines and resource budgets:

- immediate retry (synthetic negative control only; not a proxy for all industrial robot systems);
- fixed wait / delayed reconciliation;
- journal-only;
- endpoint deduplication and fencing;
- temporal command-holder closure;
- fixed-cadence observation;
- event/threshold-triggered observation;
- uncertainty-triggered observation;
- entropy/EIG selection;
- Bayesian EVSI when assumptions are justified;
- prior-free robust minimax probe selection;
- Symthaea HDC/CfC trajectory representation plus robust probe ranking.

For HDC/CfC comparisons, preserve typed numeric observations; use HDC for representation and temporal models for forecasting. Do not treat PredictiveHdcEncoder's missing/zero-prediction sentinel as a genuine anomaly. The existing symthaea-manipulator same-model one-step predictor remains a consistency/oracle control, not an independent dynamics model.

## 8. Separate endpoints

Do not aggregate these into one score:

**Execution integrity**
- false Applied / false NoEffect / false closure;
- duplicate, partial and stale-epoch effects;
- unsafe retry before reconciliation;
- stale/unknown/disputed observations;
- missing command-holder inventory.

**Epistemic integrity**
- hidden truth retained in the candidate's possible-world support;
- support-set size and world-support precision/recall by regime;
- unsupported world elimination and invalid evidence-driven support shrinkage;
- abstention precision/recall and risk-versus-coverage;
- decision-cut/future leakage;
- clock-domain and source-role confusion.

**Decision quality**
- minimax loss before/after observation;
- guaranteed worst-case loss reduction;
- decision regret under hidden truth;
- safe disposition by deadline;
- cost/delay/permission-constrained probe utility.

**Recovery utility**
- native task success and recovery-level outcome;
- time to reconciliation and safe recovery;
- false holds / safe retry availability.

**Resources**
- p50/p95/p99 observer and end-to-end latency;
- CPU/GPU time, memory, data bytes, observation count, communications and measured energy.

Hard-safety endpoints cannot be traded against utility or efficiency.

## 9. Statistical and leakage protocol

- Preregister estimands, scenario roster, possible-world catalog version, hidden truth generation, decision losses, probe supports, cluster unit, candidate/model profiles, deadlines and stopping rules.
- Split by scenario, source trajectory cluster, task, policy/checkpoint, endpoint profile and simulator configuration; do not split adjacent frames from the same episode across training and test.
- Keep hidden truth, probe outcome supports, losses, labels and likelihoods in a separately permissioned evaluator process.
- Candidate outputs must bind to the exact evidence snapshot and decision cut.
- Resample whole scenario/trajectory clusters for treatment comparisons.
- Report exact scheduled, valid, invalid, unresolved and analyzed denominators.
- Zero observed failures is not zero risk. Any one-sided zero-event bound must state its independent-cluster sampling assumptions; #7629 keeps v0.1 descriptive until these are independently verified and coverage is validated.

## 10. Proposed acceptance gates

These are targets, not achieved results:

1. 100% of candidate inputs bind to one immutable snapshot digest and decision cut.
2. 100% of post-cut, stale, duplicate, disputed, wrong-scope or non-monotonic records are rejected or trigger typed abstention.
3. 100% of valid synthetic evaluator episodes have hidden truth inside the generator's complete possible-world catalog.
4. Candidate support coverage is reported by regime; no narrow support can be scored as safe if it excludes evaluator truth. The open-world residual remains in candidate support unless an independently verified, scope-bound closure artifact is admitted.
5. 100% of requested probes come from the allowed menu and satisfy budget/deadline checks before execution.
6. Probe outcome supports cover all declared evaluator states; uncovered supports reject.
7. Zero false Applied / NoEffect / closure in the preregistered synthetic known-answer negative controls.
8. Deterministic replay of frozen scenario, input, candidate output and evaluator configuration yields identical canonical reports.
9. A second independent evaluator recomputes world-support coverage, decision loss and probe-value metrics.
10. At least one preregistered case shows lower worst-case decision loss than fixed cadence and entropy/EIG baselines without increasing hard-safety events; otherwise publish the null result.

## 11. Higher evidence tiers

- **Tier 0 — software / known-answer:** schema, parser, candidate/evaluator isolation tests, finite-world minimax conformance.
- **Tier 1 — process model:** #7564 deterministic harness and #7565 domain-qualified adapter.
- **Tier 2 — simulator:** independent simulator reset/action/physical-effect receipts and raw observations, pinned simulator/controller/render profile.
- **Tier 3 — HIL:** independently measured hardware/controller/sensor path, clock and fault-injection receipts.
- **Tier 4 — physical:** independently adjudicated effect and safety outcomes under an approved robot test plan.

No tier promotion by metadata. RoboRecover host reproduction remains conditional on author archive/checkpoints/simulator assets and the separately qualified three-attempt replay gate; RoboTwin's all-state/all-camera verifier remains separate.

## 12. Claim ceiling

AERB v0.1 is a new synthetic benchmark protocol and schema workstream. It does not establish SOTA superiority, calibrated sensor reliability, causal identification, physical effect truth, simulator/HIL/physical qualification or robot safety certification. Its first result must be a reproducible, independently challengeable process-model experiment—not a leaderboard claim.
