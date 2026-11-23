# Reliability-gated decision-aware acquisition

**Status:** isolated composition of existing admission and planning kernels with synthetic tests. No physical acquisition or reliability estimate is performed.

**Owners composed:**
- #7082 owns domain/protocol/campaign-block reliability admission for a valid qualified acquisition.
- #7098 owns projection of outcome-set separation onto decision-incompatible hypothesis pairs.
- #7145 owns Pareto action selection using decision-relevant separation, conditional protection-level change, latency and energy.
- #7138 owns the later consume-once attempt/outcome/reconciliation lifecycle.
- #6900 owns the fault inventory and dependency-aware hypothesis construction.

## Policy

The composed planner applies reliability as a **hard eligibility gate**:

1. Validate candidate action identities and conditional protection-level inputs.
2. Call the existing `admit_active_evidence_reliability` function with a frozen domain/protocol policy and the action-specific source-justified campaign-block contracts.
3. Preserve the exact admitted and rejected action summaries, including probability intervals, confidence error, campaign block counts, empirical valid-success fraction, source/cohort/method commitments and rejection reasons.
4. Send only admitted and resource-feasible candidates to the decision-aware planner.
5. Rank admitted candidates using the existing decision-aware Pareto policy; do not multiply probability, information gain or protection-level reduction into one scalar.

A high reliability lower bound does not make an action decision-relevant. A strong discriminator does not make an unsupported or unreliable action eligible. Each contract retains its own meaning and evidence.

## Explicit outcomes

The wrapper distinguishes:
- `PlanProduced`;
- `NoReliabilityAdmittedActions`;
- `NoResourceFeasibleActions`;
- `NoActionImprovesProtectionOrDecisionDiscrimination`.

Rejected candidates remain visible with #7082's reason. A candidate with no reliability contract is not silently assigned a default probability. Reliability-admitted actions excluded by availability, freshness, dependency-group or hard latency/energy constraints are separately listed.

If the protection target is already satisfied, the underlying planner returns its no-sensing target-satisfied disposition. It does not require a meaningless acquisition to prove the already-satisfied condition.

## Independent validation boundary

This wrapper calls the #7082 admission function itself; it does not accept a caller-constructed list claiming that an action is admitted. It also validates every decision report's action identity and duplicate status before reliability filtering, so reports cannot smuggle an unknown action through an all-rejected/no-resource branch.

The underlying admission kernel still consumes externally computed probability intervals. It validates their shape, source/domain/protocol commitments, campaign-block uniqueness, observed success fraction and policy thresholds. It does not estimate or authenticate the interval, prove campaign independence, or establish physical acquisition quality.

The wrapper does not:
- execute or retry an action;
- reconcile an unknown executor outcome;
- update a hypothesis set or protection region;
- prove that a qualified payload is independent or discriminating;
- grant NORMAL or any other authority transition.

The #7138 attempt ledger is still required after a selected action is actually executed. Only a fresh, qualified observation followed by the independent posterior/fault-envelope audit can support a new navigation state.

## Synthetic tests

Seven synthetic tests cover:
- a reliability-rejected action with a larger conditional PL reduction being excluded while a reliability-admitted cross-decision discriminator remains eligible;
- higher/lower admitted reliability bounds not overriding decision value or becoming a utility multiplier;
- weak and missing contracts remaining explicit rejections;
- duplicate and malformed contracts failing closed through #7082;
- resource-infeasible but reliability-admitted actions remaining visible and not being planned;
- a target-already-satisfied profile producing no unnecessary sensing plan;
- unknown or duplicate decision reports rejecting even when the action would otherwise be excluded by the reliability gate.

## Claim ceiling

A passing synthetic test validates only composition of the declared policy, external reliability intervals, campaign-block summaries, conditional outcome-set reports and planner logic. It does not establish field acquisition probability, source authenticity, statistical independence, physical sensor success, PHMI, physical fault containment or safety certification.
